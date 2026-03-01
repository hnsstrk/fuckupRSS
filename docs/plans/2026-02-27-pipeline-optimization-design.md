# Ollama Pipeline-Optimierung — Implementierungsplan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ollama-Pipeline optimieren, damit die GPU bei Remote-Betrieb durchläuft statt zwischen Artikeln an/aus zu gehen.

**Architecture:** Vier Änderungen: (1) `keep_alive` auf "5m" setzen, (2) `unload_model()` zwischen LLM- und Embedding-Phase, (3) `/api/embed` Batch-Endpunkt für Embeddings, (4) LLM-Concurrency konfigurierbar machen.

**Tech Stack:** Rust (Tauri Backend), reqwest HTTP-Client, Ollama API, Svelte 5 Frontend

---

## Kontext

- **GPU:** NVIDIA RTX 3080 Ti, 12 GB VRAM auf Ganymed (192.168.177.22)
- **Server-Config:** `OLLAMA_MAX_LOADED_MODELS=1`, `OLLAMA_NUM_PARALLEL=4`
- **Modelle:** ministral-3:latest (Text), snowflake-arctic-embed2 (Embeddings)
- **Problem:** Artikel werden sequentiell verarbeitet, GPU geht zwischen Requests aus

---

### Task 1: `keep_alive` von "30m" auf "5m" ändern

**Files:**
- Modify: `src-tauri/src/ollama/mod.rs:157` (EmbeddingRequest keep_alive)
- Modify: `src-tauri/src/ollama/mod.rs:446` (GenerateRequest keep_alive)
- Test: `cargo check --manifest-path src-tauri/Cargo.toml`

**Step 1: Beide `keep_alive`-Werte ändern**

In `src-tauri/src/ollama/mod.rs`, Zeile 377 (in `generate_embedding()`):
```rust
// VORHER:
keep_alive: "30m".to_string(),
// NACHHER:
keep_alive: "5m".to_string(),
```

In `src-tauri/src/ollama/mod.rs`, Zeile 446 (in `generate()`):
```rust
// VORHER:
keep_alive: "30m".to_string(),
// NACHHER:
keep_alive: "5m".to_string(),
```

**Step 2: Verify**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Kompiliert ohne Fehler

**Step 3: Commit**

```bash
git add src-tauri/src/ollama/mod.rs
git commit -m "perf: keep_alive von 30m auf 5m reduzieren (Server-Default)"
```

---

### Task 2: `unload_model()` Methode implementieren

**Files:**
- Modify: `src-tauri/src/ollama/mod.rs` — neue Methode `unload_model()`
- Modify: `src-tauri/src/commands/ai/batch_processor.rs:1127` — nach LLM-Phase aufrufen
- Test: `cargo check --manifest-path src-tauri/Cargo.toml`

**Step 1: `unload_model()` in OllamaClient hinzufügen**

In `src-tauri/src/ollama/mod.rs`, nach `generate()` (nach Zeile 505), neue Methode einfügen:

```rust
/// Unload a model from VRAM by sending a generate request with keep_alive: "0"
pub async fn unload_model(&self, model: &str) -> Result<(), OllamaError> {
    let url = format!("{}/api/generate", self.base_url);
    let request = GenerateRequest {
        model: model.to_string(),
        prompt: String::new(),
        stream: false,
        format: None,
        options: GenerateOptions {
            num_ctx: self.num_ctx,
            num_predict: 1,
        },
        keep_alive: "0".to_string(),
    };

    let resp = self.client().post(&url).json(&request).send().await
        .map_err(|e| OllamaError::GenerationFailed(format!("Unload request failed: {}", e)))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        warn!("[Ollama] Unload model '{}' failed: {}", model, body);
    } else {
        debug!("[Ollama] Model '{}' unloaded from VRAM", model);
    }

    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Kompiliert ohne Fehler

**Step 3: Nach LLM-Phase unload aufrufen**

In `src-tauri/src/commands/ai/batch_processor.rs`, nach Zeile 1128 (nach `batch_running.store(false)`), den Modell-Unload einfügen:

```rust
// Explicitly unload LLM model to free VRAM for embedding model
{
    let ollama_url = {
        let db = state.db_conn()?;
        super::helpers::get_setting(&db, "ollama_url", "http://localhost:11434")
    };
    let unload_client = crate::ollama::OllamaClient::new(Some(ollama_url));
    if let Err(e) = unload_client.unload_model(&effective_model).await {
        warn!("[LLM] Failed to unload model: {}", e);
    }
}
```

**Hinweis:** Der OllamaClient wird hier temporär erstellt, da der Provider bereits als Arc<dyn AiTextProvider> vorliegt und keinen Zugriff auf den zugrunde liegenden OllamaClient bietet. Der HTTP-Client-Overhead ist bei einem einmaligen Unload-Request vernachlässigbar.

**Step 4: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Kompiliert ohne Fehler

**Step 5: Commit**

```bash
git add src-tauri/src/ollama/mod.rs src-tauri/src/commands/ai/batch_processor.rs
git commit -m "perf: LLM-Modell nach Analyse-Phase explizit aus VRAM entladen"
```

---

### Task 3: `/api/embed` Batch-Endpunkt für Embeddings

**Files:**
- Modify: `src-tauri/src/ollama/mod.rs:152-163` — EmbeddingRequest/Response durch Batch-Structs ersetzen
- Modify: `src-tauri/src/ollama/mod.rs:366-398` — `generate_embedding()` auf `/api/embed` umstellen
- Add: `src-tauri/src/ollama/mod.rs` — neue Methode `generate_embeddings_batch()`
- Modify: `src-tauri/src/ai_provider/mod.rs:127-136` — `EmbeddingProvider` Trait um Batch-Methode erweitern
- Modify: `src-tauri/src/ai_provider/ollama_provider.rs:100-121` — Batch-Methode implementieren
- Modify: `src-tauri/src/ai_provider/openai_embedding_provider.rs` — Batch-Methode implementieren (Fallback auf Einzelaufrufe)
- Modify: `src-tauri/src/commands/ai/batch_processor.rs:1226-1245` — Batch-Embedding nutzen
- Modify: `src-tauri/src/embedding_worker.rs:182-207` — Batch-Embedding nutzen
- Test: `cargo test --manifest-path src-tauri/Cargo.toml`

**Step 1: Embedding-Structs in ollama/mod.rs auf `/api/embed` umstellen**

Ersetze die bestehenden EmbeddingRequest/Response (Zeile 152-163):

```rust
// Embedding structs for Ollama /api/embed endpoint (batch-capable)
#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: Vec<String>,
    keep_alive: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}
```

**Step 2: `generate_embedding()` auf neuen Endpunkt umstellen**

Ersetze `generate_embedding()` (Zeile 366-398):

```rust
/// Generate embedding vector for a single text
pub async fn generate_embedding(
    &self,
    model: &str,
    text: &str,
) -> Result<Vec<f32>, OllamaError> {
    let result = self.generate_embeddings_batch(model, &[text.to_string()]).await?;
    result.into_iter().next().ok_or_else(|| {
        OllamaError::GenerationFailed("Empty embedding response".to_string())
    })
}

/// Generate embedding vectors for multiple texts in a single request
pub async fn generate_embeddings_batch(
    &self,
    model: &str,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, OllamaError> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let url = format!("{}/api/embed", self.base_url);

    let request = EmbedRequest {
        model: model.to_string(),
        input: texts.to_vec(),
        keep_alive: "5m".to_string(),
    };

    let resp = self.client().post(&url).json(&request).send().await.map_err(|e| {
        OllamaError::GenerationFailed(format!("Batch embedding request failed: {}", e))
    })?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(OllamaError::GenerationFailed(format!(
            "Batch embedding failed with status {}: {}",
            status, body
        )));
    }

    let result: EmbedResponse = resp.json().await.map_err(|e| {
        OllamaError::GenerationFailed(format!("Failed to parse batch embedding: {}", e))
    })?;

    Ok(result.embeddings)
}
```

**Step 3: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Kompiliert ohne Fehler (bestehende Aufrufe von `generate_embedding()` funktionieren unverändert)

**Step 4: `EmbeddingProvider` Trait um Batch-Methode erweitern**

In `src-tauri/src/ai_provider/mod.rs`, den `EmbeddingProvider` Trait (Zeile 127-136) erweitern:

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding vector for the given text
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AiProviderError>;

    /// Generate embedding vectors for multiple texts in a single batch
    /// Default implementation falls back to sequential single calls
    async fn generate_embeddings_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, AiProviderError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.generate_embedding(text).await?);
        }
        Ok(results)
    }

    /// The number of dimensions produced by this provider
    fn embedding_dimensions(&self) -> usize;

    /// Human-readable provider name
    fn provider_name(&self) -> &str;
}
```

**Step 5: Ollama-EmbeddingProvider: Batch-Methode implementieren**

In `src-tauri/src/ai_provider/ollama_provider.rs`, innerhalb des `impl EmbeddingProvider for OllamaEmbeddingProvider` Blocks (nach `generate_embedding()`), hinzufügen:

```rust
async fn generate_embeddings_batch(
    &self,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, AiProviderError> {
    self.client
        .generate_embeddings_batch(&self.model, texts)
        .await
        .map_err(|e| match e {
            crate::ollama::OllamaError::NotAvailable(msg) => AiProviderError::NotAvailable(msg),
            crate::ollama::OllamaError::GenerationFailed(msg) => {
                AiProviderError::GenerationFailed(msg)
            }
            other => AiProviderError::GenerationFailed(other.to_string()),
        })
}
```

**Step 6: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Kompiliert ohne Fehler. OpenAI-EmbeddingProvider nutzt automatisch die Default-Implementierung (sequentiell).

**Step 7: Artikel-Embeddings im batch_processor auf Batch umstellen**

In `src-tauri/src/commands/ai/batch_processor.rs`, die sequentielle Schleife (Zeile 1223-1245) ersetzen:

```rust
let mut embed_succeeded = 0;
let embed_total = articles_for_embedding.len();

// Prepare batch: collect embedding texts
let embedding_texts: Vec<String> = articles_for_embedding
    .iter()
    .map(|(_id, title, content)| {
        let content_preview: String = content.chars().take(500).collect();
        format!("{}\n\n{}", title, content_preview)
    })
    .collect();

// Generate all embeddings in one batch request
match embedding_provider.generate_embeddings_batch(&embedding_texts).await {
    Ok(embeddings) => {
        let db = state.db_conn()?;
        for (embedding, (fnord_id, _title, _content)) in
            embeddings.iter().zip(articles_for_embedding.iter())
        {
            match crate::commands::ai::data_persistence::save_article_embedding(
                db.conn(), *fnord_id, embedding,
            ) {
                Ok(_) => embed_succeeded += 1,
                Err(e) => {
                    debug!("[Embedding] Failed to save for article {}: {}", fnord_id, e);
                }
            }
        }
    }
    Err(e) => {
        warn!("[Embedding] Batch embedding failed, falling back to sequential: {}", e);
        // Fallback: sequential processing
        for (fnord_id, title, content) in &articles_for_embedding {
            if state.batch_cancel.load(Ordering::SeqCst) {
                break;
            }
            match generate_and_save_article_embedding(
                embedding_provider.as_ref(),
                &state.db,
                *fnord_id,
                title,
                content,
            )
            .await
            {
                Ok(_) => embed_succeeded += 1,
                Err(e) => {
                    debug!("[Embedding] Failed for article {}: {}", fnord_id, e);
                }
            }
        }
    }
}
```

**Step 8: Keyword-Embeddings im embedding_worker auf Batch umstellen**

In `src-tauri/src/embedding_worker.rs`, die `process_embedding_queue()` Funktion (Zeile 188-229) ersetzen. Statt `join_all` mit Einzelaufrufen, einen Batch-Call pro Chunk:

```rust
// Process in chunks using batch embedding API
for chunk in keywords.chunks(concurrency) {
    let texts: Vec<String> = chunk
        .iter()
        .map(|(_queue_id, keyword_id, name)| format!("{}_{}", name, keyword_id))
        .collect();

    match provider.generate_embeddings_batch(&texts).await {
        Ok(embeddings) => {
            for (embedding, (queue_id, keyword_id, name)) in
                embeddings.iter().zip(chunk.iter())
            {
                if let Err(e) =
                    save_embedding_and_dequeue(&db, *queue_id, *keyword_id, embedding)
                {
                    error!("Failed to save embedding for '{}': {}", name, e);
                    failed += 1;
                } else {
                    debug!("Generated embedding for keyword: {}", name);
                    processed += 1;
                }
            }
        }
        Err(e) => {
            warn!("Batch embedding failed for chunk, falling back to sequential: {}", e);
            // Fallback: process individually
            for (queue_id, keyword_id, name) in chunk {
                match provider.generate_embedding(&format!("{}_{}", name, keyword_id)).await {
                    Ok(embedding) => {
                        if let Err(e) =
                            save_embedding_and_dequeue(&db, *queue_id, *keyword_id, &embedding)
                        {
                            error!("Failed to save embedding for '{}': {}", name, e);
                            failed += 1;
                        } else {
                            processed += 1;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to generate embedding for '{}': {}", name, e);
                        let _ = record_failure(&db, *queue_id, &e.to_string());
                        failed += 1;
                    }
                }
            }
        }
    }

    // Emit progress event after each chunk
    if let Some(handle) = app_handle {
        let queue_size = get_queue_size(&db).unwrap_or(0);
        let total = initial_total.unwrap_or(queue_size + processed + failed);
        let _ = handle.emit(
            "embedding-progress",
            EmbeddingProgress {
                queue_size,
                total,
                processed,
                failed,
                is_processing: true,
            },
        );
    }
}
```

**Step 9: Prüfe ob `save_article_embedding` public ist**

Run: `grep -n "pub fn save_article_embedding\|fn save_article_embedding" src-tauri/src/commands/ai/data_persistence.rs`

Falls nicht public: `pub` hinzufügen.

**Step 10: Tests und Verify**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: Kompiliert und Tests laufen

**Step 11: Commit**

```bash
git add src-tauri/src/ollama/mod.rs src-tauri/src/ai_provider/mod.rs src-tauri/src/ai_provider/ollama_provider.rs src-tauri/src/commands/ai/batch_processor.rs src-tauri/src/embedding_worker.rs
git commit -m "perf: /api/embed Batch-Endpunkt für Embeddings nutzen

Statt N einzelne HTTP-Requests an /api/embeddings werden alle Texte
in einem Batch-Request an /api/embed gesendet. Reduziert Netzwerk-
Roundtrips und hält die GPU durchgehend beschäftigt.

Fallback auf sequentielle Verarbeitung bei Fehler."
```

---

### Task 4: LLM-Concurrency konfigurierbar machen

**Files:**
- Modify: `src-tauri/src/db/schema.rs:1351` — neues Setting `ollama_concurrency`
- Modify: `src-tauri/src/ai_provider/ollama_provider.rs:13-23` — Concurrency-Feld hinzufügen
- Modify: `src-tauri/src/ai_provider/ollama_provider.rs:75-77` — `suggested_concurrency()` anpassen
- Modify: `src-tauri/src/ai_provider/mod.rs:49-66` — `ProviderConfig` um Feld erweitern
- Modify: `src-tauri/src/ai_provider/mod.rs:191-203` — `create_provider()` anpassen
- Modify: `src-tauri/src/commands/ai/helpers.rs` — `get_provider_config()` liest Setting
- Modify: `src-tauri/src/commands/ai/batch_processor.rs:925-931` — Concurrency-Logik anpassen
- Modify: Settings-UI (Svelte) — neues Feld
- Test: `cargo test --manifest-path src-tauri/Cargo.toml`

**Step 1: DB-Setting hinzufügen**

In `src-tauri/src/db/schema.rs`, Zeile 1351, nach `ollama_num_ctx`:

```sql
            ('ollama_num_ctx', '4096'),
            ('ollama_concurrency', '1');
```

**Step 2: `ProviderConfig` erweitern**

In `src-tauri/src/ai_provider/mod.rs`, `ProviderConfig` (Zeile 49-66):

```rust
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub ollama_url: String,
    pub ollama_model: String,
    pub ollama_num_ctx: u32,
    /// Ollama parallel request concurrency (1 = sequential, 2-4 for remote)
    pub ollama_concurrency: usize,
    pub openai_base_url: String,
    pub openai_api_key: String,
    pub openai_model: String,
    pub openai_temperature: Option<f32>,
}
```

**Step 3: `OllamaTextProvider` um Concurrency erweitern**

In `src-tauri/src/ai_provider/ollama_provider.rs`:

```rust
pub struct OllamaTextProvider {
    client: OllamaClient,
    concurrency: usize,
}

impl OllamaTextProvider {
    pub fn new(base_url: &str, num_ctx: u32, concurrency: usize) -> Self {
        Self {
            client: OllamaClient::with_context(Some(base_url.to_string()), num_ctx),
            concurrency: concurrency.max(1),
        }
    }
}
```

Und `suggested_concurrency()` ändern:

```rust
fn suggested_concurrency(&self) -> usize {
    self.concurrency
}
```

**Step 4: `create_provider()` anpassen**

In `src-tauri/src/ai_provider/mod.rs`, Zeile 193:

```rust
ProviderType::Ollama => Arc::new(ollama_provider::OllamaTextProvider::new(
    &config.ollama_url,
    config.ollama_num_ctx,
    config.ollama_concurrency,
)),
```

**Step 5: `get_provider_config()` in helpers.rs anpassen**

Suche die Funktion `get_provider_config()` und füge das neue Feld hinzu:

```rust
ollama_concurrency: get_setting(db, "ollama_concurrency", "1")
    .parse()
    .unwrap_or(1),
```

**Step 6: Concurrency-Logik im batch_processor anpassen**

In `src-tauri/src/commands/ai/batch_processor.rs`, Zeile 925-931 ersetzen:

```rust
// Ollama uses its own concurrency setting, OpenAI uses openai_concurrency
let active_concurrency = if suggested == 1 && matches!(provider_config.provider_type, crate::ai_provider::ProviderType::OpenAiCompatible) {
    openai_concurrency_setting
} else {
    suggested  // Uses ollama_concurrency for Ollama, 1 as default
};
```

**Hinweis:** Die Logik wird vereinfacht: `suggested_concurrency()` gibt jetzt den richtigen Wert zurück (1 als Default, oder was der User konfiguriert hat). Für OpenAI-Provider bleibt die separate `openai_concurrency`-Einstellung bestehen.

**Step 7: Alle Stellen finden die `OllamaTextProvider::new()` aufrufen und anpassen**

Run: `grep -rn "OllamaTextProvider::new" src-tauri/src/`

Alle Aufrufe von 2-Parameter auf 3-Parameter umstellen (drittes Argument: concurrency). In Tests ggf. Default-Wert `1` verwenden.

**Step 8: Tests**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: Kompiliert und Tests laufen

**Step 9: Settings-UI erweitern**

In der Svelte Settings-Komponente ein neues Feld für Ollama-Concurrency hinzufügen:
- Label: "Ollama Parallelität" / "Ollama Concurrency"
- Typ: Zahleneingabe (1-8, Default: 1)
- Tooltip: "Anzahl paralleler Requests an Ollama. Erhöhen bei Remote-Ollama mit genügend VRAM."
- Nur sichtbar wenn Provider = Ollama
- i18n-Keys hinzufügen

**Step 10: Commit**

```bash
git add -A
git commit -m "feat: Ollama-Concurrency konfigurierbar machen

Neues Setting ollama_concurrency (Default: 1). Erlaubt parallele
LLM-Requests an Remote-Ollama, um OLLAMA_NUM_PARALLEL auszunutzen.
Erhöht den Durchsatz bei Remote-Betrieb um ~2x bei Wert 4."
```

---

### Task 5: Dokumentation aktualisieren

**Files:**
- Modify: `docs/architecture/AI_PROCESSING_PIPELINE.md`
- Modify: `docs/guides/HARDWARE_OPTIMIZATION.md`
- Modify: `CLAUDE.md`

**Step 1: Pipeline-Docs aktualisieren**

Folgende Änderungen dokumentieren:
- `/api/embed` Batch-Endpunkt statt `/api/embeddings`
- Explizites Modell-Entladen zwischen Phasen
- Konfigurierbare Concurrency
- `keep_alive` auf 5m

**Step 2: Hardware-Optimierung aktualisieren**

- Empfehlung für `OLLAMA_NUM_PARALLEL` Einstellung
- `ollama_concurrency` Setting erklären
- VRAM-Budget-Rechnung bei parallelen Requests

**Step 3: CLAUDE.md aktualisieren**

- Neues Setting in der Settings-Übersicht
- Verweis auf geänderte Ollama-API-Nutzung

**Step 4: Commit**

```bash
git add docs/ CLAUDE.md
git commit -m "docs: Pipeline-Optimierung dokumentieren"
```

---

## Nicht in Scope

- HTTP-Timeout ändern (120s reicht)
- keep_alive konfigurierbar machen (5m hardcoded reicht)
- Beide Modelle gleichzeitig laden (VRAM zu knapp)
- Streaming für LLM-Responses (bleibt stream=false)

## Hardware-Kontext

- **GPU:** NVIDIA RTX 3080 Ti, 12 GB VRAM auf Ganymed (192.168.177.22)
- **Server-Config:** `OLLAMA_MAX_LOADED_MODELS=1`, `OLLAMA_NUM_PARALLEL=4`, `OLLAMA_FLASH_ATTENTION=1`
- **VRAM-Budget bei num_ctx=4096, 4 Slots:** ~10.8GB (1.5GB Puffer)
