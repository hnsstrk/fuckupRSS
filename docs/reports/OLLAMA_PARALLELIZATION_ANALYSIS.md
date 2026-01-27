# Ollama LLM-Parallelisierung - Detailanalyse

**Erstellt:** 2026-01-27
**Analysiert von:** Claude Opus 4.5
**Hardware:** MacBook Pro M4 Pro, 48GB RAM
**Ollama-Konfiguration:** OLLAMA_NUM_PARALLEL=4 (erwartet)

---

## Inhaltsverzeichnis

1. [Zusammenfassung](#1-zusammenfassung)
2. [Aktuelle Implementierung](#2-aktuelle-implementierung)
3. [Identifizierte Probleme](#3-identifizierte-probleme)
4. [Ollama-Server-Konfiguration](#4-ollama-server-konfiguration)
5. [Konkrete Verbesserungsvorschlaege](#5-konkrete-verbesserungsvorschlaege)
6. [Fazit](#6-fazit)

---

## 1. Zusammenfassung

**Gute Nachricht:** Die LLM-Parallelisierung ist korrekt implementiert. Der Code verwendet `buffer_unordered(concurrency)` richtig.

**Kernproblem:** Der fehlende Speedup liegt hoechstwahrscheinlich an der **Ollama-Server-Konfiguration**, nicht am Client-Code.

| Aspekt | Status | Bewertung |
|--------|--------|-----------|
| `buffer_unordered` Implementation | Korrekt | OK |
| `ai_parallelism` Setting | Wird genutzt | OK |
| DB-Lock-Pattern | Korrekt (kurze Locks) | OK |
| Ollama `OLLAMA_NUM_PARALLEL` | **Moeglicherweise nicht gesetzt** | PROBLEM |
| Request-Timeouts | 120 Sekunden | OK |

---

## 2. Aktuelle Implementierung

### 2.1 Batch-Processor (`batch_processor.rs`)

Die Parallelisierung ist korrekt implementiert:

```rust
// Zeile 683-684
let locale = get_locale_from_db(&state);
let concurrency = get_ai_concurrency(&state);  // Liest ai_parallelism aus DB

info!("Starting batch processing with concurrency: {}", concurrency);

// Zeile 792-849: Parallele Verarbeitung
let stream = stream::iter(articles.into_iter().enumerate());

let results = stream
    .map(|(idx, article)| {
        // ... async closure fuer jeden Artikel
        async move {
            // OllamaClient wird PRO ARTIKEL erstellt (mit angepasstem num_ctx)
            let client = OllamaClient::with_context(None, adjusted_num_ctx);

            let (success, error) =
                process_single_article(&client, &state, &model, &locale, article, &batch_context)
                    .await;
            // ...
        }
    })
    .buffer_unordered(concurrency)  // <-- KORREKT: Parallele Ausfuehrung
    .collect::<Vec<_>>()
    .await;
```

**Bewertung:** Die Verwendung von `buffer_unordered(concurrency)` ist die korrekte Methode fuer parallele async Operationen in Rust/Tokio.

### 2.2 AI Concurrency Setting (`helpers.rs`)

```rust
// Zeile 55-70
pub fn get_ai_concurrency(state: &AppState) -> usize {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return 1,
    };
    let val: String = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'ai_parallelism'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "1".to_string());

    val.parse().unwrap_or(1).clamp(1, 10)
}
```

**Bewertung:** Setting wird korrekt aus der Datenbank geladen und auf 1-10 begrenzt.

### 2.3 Ollama Client (`ollama/mod.rs`)

```rust
// Zeile 332-337: HTTP Client mit Timeout
fn client(&self) -> Result<reqwest_new::Client, OllamaError> {
    reqwest_new::Client::builder()
        .timeout(Duration::from_secs(120))  // 2 Minuten Timeout
        .build()
        .map_err(|e| OllamaError::NotAvailable(format!("Failed to create HTTP client: {}", e)))
}

// Zeile 695-746: Generate-Funktion
async fn generate(&self, model: &str, prompt: &str, format: Option<String>) -> Result<String, OllamaError> {
    let url = format!("{}/api/generate", self.base_url);
    let client = self.client()?;

    let request = GenerateRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,  // Blocking call, wartet auf komplette Antwort
        format,
        options: GenerateOptions {
            num_ctx: self.num_ctx,
            num_predict: 4096,
        },
    };

    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| OllamaError::NotAvailable(e.to_string()))?;
    // ...
}
```

**Bewertung:**
- Kein Connection-Pooling explizit, aber `reqwest` nutzt standardmaessig Connection-Pooling.
- `stream: false` ist korrekt - wir wollen die komplette Antwort.

### 2.4 Database Lock Pattern (`batch_processor.rs`)

```rust
// Zeile 433-439: Kurze DB-Locks
let cached_result = {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return (false, Some("Database lock failed".to_string())),
    };
    check_analysis_cache(db.conn(), &content_hash)
};  // Lock wird hier sofort released!

// LLM-Call OHNE Lock gehalten
let analysis_result = client
    .discordian_analysis_with_stats_custom(...)
    .await;

// Neuer Lock nur fuer DB-Schreiboperationen
{
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(e) => return (false, Some(format!("Database lock failed: {}", e))),
    };
    // Kurze DB-Operationen
} // Lock wieder released
```

**Bewertung:** Exzellent! DB-Locks werden nur fuer kurze Operationen gehalten, nie waehrend LLM-Calls.

---

## 3. Identifizierte Probleme

### 3.1 HAUPTPROBLEM: Ollama-Server Parallelitaet

**Die Client-Seite ist korrekt implementiert, aber der Ollama-Server muss parallel Requests akzeptieren!**

Ollama-Default: `OLLAMA_NUM_PARALLEL=1` (nur ein Request gleichzeitig!)

**Symptom:** Wenn `OLLAMA_NUM_PARALLEL` nicht gesetzt ist, werden parallele Requests vom Client **seriell** vom Server abgearbeitet. Der Client sendet 4 Requests parallel, aber Ollama verarbeitet sie nacheinander.

**Loesung in `docs/guides/HARDWARE_OPTIMIZATION.md` dokumentiert:**

```bash
# macOS (vor Ollama-Start)
export OLLAMA_NUM_PARALLEL=4
ollama serve

# macOS (permanent via launchctl)
launchctl setenv OLLAMA_NUM_PARALLEL 4
```

### 3.2 Problem: OllamaClient wird pro Artikel neu erstellt

```rust
// Zeile 826: Neuer Client pro Artikel
let client = OllamaClient::with_context(None, adjusted_num_ctx);
```

**Auswirkung:** Technisch kein Problem, da `reqwest::Client` intern Connection-Pooling nutzt. Aber es ist ineffizient:
- Jeder Client hat eigene Timeout-Konfiguration
- Kein Sharing von TCP-Verbindungen zwischen parallelen Requests

### 3.3 Problem: num_ctx Variation

```rust
// Zeile 810-814: Context wird bei Retries erhoeht
let (ctx_multiplier, adjusted_num_ctx) = match article.attempts {
    0 => (1.0, num_ctx),
    1 => (1.5, ((num_ctx as f64) * 1.5) as u32),
    _ => (2.0, num_ctx * 2),
};
```

**Auswirkung:** Bei Retries mit erhoehtem `num_ctx` koennte Ollama mehr VRAM benoetigen, was die Parallelitaet einschraenkt.

### 3.4 Kein Problem: Embedding-Generierung

Die Embedding-Generierung am Ende des Batches ist korrekt parallelisiert:

```rust
// Zeile 953-970
let embedding_concurrency = concurrency.max(4); // Mindestens 4 fuer Embeddings

let embedding_results = stream::iter(articles_for_embedding)
    .map(|(fnord_id, title, content)| {
        // ...
    })
    .buffer_unordered(embedding_concurrency)
    .collect::<Vec<_>>()
    .await;
```

**Bewertung:** Korrekt implementiert.

---

## 4. Ollama-Server-Konfiguration

### 4.1 Aktuelle Empfehlungen aus der Doku

Aus `docs/guides/HARDWARE_OPTIMIZATION.md`:

| GPU | ai_parallelism (App) | OLLAMA_NUM_PARALLEL (Server) |
|-----|---------------------|------------------------------|
| 8 GB | 1 | 1 |
| 12 GB | 4 | 4 |
| 16+ GB | 8 | 8 |

**Fuer MacBook Pro M4 Pro mit 48GB RAM:** `OLLAMA_NUM_PARALLEL=8` sollte problemlos moeglich sein.

### 4.2 Verifizierung der Ollama-Konfiguration

```bash
# Pruefen ob Ollama laeuft
curl http://localhost:11434/api/tags

# Pruefen der aktuellen Modelle im Speicher
curl http://localhost:11434/api/ps
```

### 4.3 Empfohlene Ollama-Konfiguration fuer M4 Pro

```bash
# Terminal (vor ollama serve)
export OLLAMA_NUM_PARALLEL=8
export OLLAMA_MAX_LOADED_MODELS=2
export OLLAMA_FLASH_ATTENTION=1
export OLLAMA_KEEP_ALIVE=24h
ollama serve
```

Oder permanent via launchctl:

```bash
launchctl setenv OLLAMA_NUM_PARALLEL 8
launchctl setenv OLLAMA_MAX_LOADED_MODELS 2
launchctl setenv OLLAMA_FLASH_ATTENTION 1
launchctl setenv OLLAMA_KEEP_ALIVE 24h
# Danach Ollama neu starten
```

---

## 5. Konkrete Verbesserungsvorschlaege

### 5.1 PRIORITAET 1: Ollama-Server korrekt konfigurieren

**Aktion:** Sicherstellen dass `OLLAMA_NUM_PARALLEL` >= `ai_parallelism` Setting in der App.

**Verifizierung:** Im Log sollte erscheinen:
```
INFO  fuckup_rss::commands::ollama::batch_processor > Starting batch processing with concurrency: 4
```

Dann sollten 4 parallele Requests an Ollama gehen. Wenn Ollama mit `OLLAMA_NUM_PARALLEL=4` laeuft, sollten alle 4 gleichzeitig verarbeitet werden.

### 5.2 PRIORITAET 2: Shared OllamaClient (Optional)

**Aktuelle Implementierung:**
```rust
// Pro Artikel neuer Client
let client = OllamaClient::with_context(None, adjusted_num_ctx);
```

**Verbesserter Ansatz:**
```rust
// Am Anfang von process_batch:
let base_client = Arc::new(OllamaClient::with_context(None, num_ctx));

// In der async closure:
let client = if article.attempts > 0 {
    // Nur bei Retries neuen Client mit erhoehtem Context
    OllamaClient::with_context(None, adjusted_num_ctx)
} else {
    // Normaler Fall: Shared Client
    Arc::clone(&base_client)
};
```

**Erwartete Verbesserung:** Minimal (reqwest hat bereits Connection-Pooling), aber sauberer.

### 5.3 PRIORITAET 3: Logging zur Diagnose

Fuer bessere Diagnose koennte man temporaer detaillierteres Logging hinzufuegen:

```rust
// In process_single_article, vor dem LLM-Call:
let start = std::time::Instant::now();
debug!("Article {} starting LLM analysis at {:?}", fnord_id, std::time::SystemTime::now());

// Nach dem LLM-Call:
debug!("Article {} LLM analysis completed in {:?}", fnord_id, start.elapsed());
```

Wenn die Parallelitaet funktioniert, sollten mehrere "starting LLM analysis" Meldungen kurz nacheinander erscheinen, gefolgt von "completed" Meldungen ebenfalls kurz nacheinander.

### 5.4 Keine Aenderung noetig: Embedding Worker

Der Embedding Worker in `embedding_worker.rs` ist korrekt implementiert:

```rust
// Zeile 179
let concurrency = 10; // Process 10 embeddings in parallel

// Zeile 185-205: Parallele Verarbeitung in Chunks
for chunk in keywords.chunks(concurrency) {
    let futures: Vec<_> = chunk.iter().map(...).collect();
    let results = join_all(futures).await;
}
```

---

## 6. Fazit

### Was funktioniert

1. **`buffer_unordered(concurrency)`** ist korrekt implementiert
2. **`ai_parallelism`** Setting wird aus der Datenbank geladen und verwendet
3. **DB-Lock-Pattern** ist korrekt (kurze Locks, keine Locks waehrend LLM-Calls)
4. **Embedding-Generierung** ist parallel
5. **Article Embedding** am Ende des Batches ist parallel

### Was wahrscheinlich das Problem ist

**Ollama-Server-Konfiguration:** `OLLAMA_NUM_PARALLEL` ist moeglicherweise nicht oder zu niedrig gesetzt.

### Empfohlene Sofortmassnahmen

1. **Verifizieren:** `echo $OLLAMA_NUM_PARALLEL` - sollte >= dem App-Setting sein
2. **Konfigurieren:** `export OLLAMA_NUM_PARALLEL=8` (fuer M4 Pro mit 48GB)
3. **Testen:** Batch mit 20 Artikeln starten und beobachten ob mehrere gleichzeitig verarbeitet werden

### Erwartete Verbesserung nach Korrektur

| Metrik | Vorher (NUM_PARALLEL=1) | Nachher (NUM_PARALLEL=4) |
|--------|------------------------|-------------------------|
| Zeit pro 10 Artikel | ~30-50 Sekunden | ~8-15 Sekunden |
| CPU/GPU Auslastung | Niedrig (wartend) | Hoeher (parallel) |
| Speedup-Faktor | 1x | ~3-4x |

### Zusammenfassung

Der Client-Code ist **korrekt implementiert**. Das Problem liegt hoechstwahrscheinlich an der **Ollama-Server-Konfiguration**. Nach korrekter Einstellung von `OLLAMA_NUM_PARALLEL=4` (oder hoeher) sollte der erwartete Speedup eintreten.

---

*Bericht erstellt am 2026-01-27 durch Code-Analyse.*
