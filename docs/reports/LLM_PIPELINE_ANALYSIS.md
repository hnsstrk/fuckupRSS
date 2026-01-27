# LLM-Artikel-Analyse-Pipeline - Technische Analyse

**Erstellt:** 2026-01-27
**Analysiert von:** Claude Opus 4.5
**Status:** Abgeschlossen

---

## Inhaltsverzeichnis

1. [Zusammenfassung](#1-zusammenfassung)
2. [Architektur-Uebersicht](#2-architektur-uebersicht)
3. [Detaillierter Ablauf](#3-detaillierter-ablauf)
4. [Beteiligte Module](#4-beteiligte-module)
5. [LLM-Calls und Prompts](#5-llm-calls-und-prompts)
6. [Performance-Analyse](#6-performance-analyse)
7. [Bottlenecks](#7-bottlenecks)
8. [Verbesserungsvorschlaege](#8-verbesserungsvorschlaege)
9. [Priorisierte Massnahmen](#9-priorisierte-massnahmen)

---

## 1. Zusammenfassung

Die LLM-Artikel-Analyse-Pipeline von fuckupRSS verarbeitet Artikel in mehreren Phasen:

| Phase | Beschreibung | LLM-Nutzung |
|-------|--------------|-------------|
| Volltext-Abruf | Webseiten-Scraping via `readability` | Nein |
| Discordian Analysis | Zusammenfassung, Kategorien, Keywords, Bias | Ja (ministral) |
| Embedding-Generierung | Artikel- und Keyword-Embeddings | Ja (snowflake-arctic-embed2) |
| Statistische Analyse | TF-IDF, Kategorie-Matching | Nein |

**Kernproblem:** Die Verarbeitung erfolgt groesstenteils **sequentiell**, obwohl erhebliches Parallelisierungspotenzial besteht.

---

## 2. Architektur-Uebersicht

```
+------------------+     +-------------------+     +--------------------+
|   Feed Sync      |---->| Volltext-Abruf    |---->| Batch-Verarbeitung |
| (store_feed)     |     | (Hagbard's Retr.) |     | (process_batch)    |
+------------------+     +-------------------+     +--------------------+
                                                           |
                         +--------------------------------+
                         |                                |
                         v                                v
              +-------------------+            +--------------------+
              | Discordian        |            | Statistical        |
              | Analysis (LLM)    |            | Analysis (TF-IDF)  |
              +-------------------+            +--------------------+
                         |                                |
                         +--------------------------------+
                                        |
                                        v
                              +-------------------+
                              | Keyword Network   |
                              | & Persistence     |
                              +-------------------+
                                        |
                                        v
                              +-------------------+
                              | Embedding Worker  |
                              | (Background)      |
                              +-------------------+
```

---

## 3. Detaillierter Ablauf

### 3.1 Artikel-Aufnahme (Feed Sync)

**Modul:** `src-tauri/src/sync/mod.rs`

```rust
// Ablauf store_feed():
1. BEGIN Transaction
2. Feed-Metadaten speichern/aktualisieren
3. Fuer jeden Artikel:
   - Duplikat-Check via guid/url
   - INSERT INTO fnords (content_raw, ...)
   - processed_at = NULL (markiert als "nicht verarbeitet")
4. COMMIT Transaction
```

**Wichtig:** Artikel erhalten bei der Aufnahme nur `content_raw` (RSS-Feed-Inhalt). Der Volltext (`content_full`) wird spaeter abgerufen.

### 3.2 Volltext-Abruf (Hagbard's Retrieval)

**Modul:** `src-tauri/src/commands/ollama/batch_processor.rs`

```rust
// Funktion: fetch_fulltext_batch()
1. SELECT Artikel WHERE content_full IS NULL LIMIT batch_size
2. Fuer jeden Artikel:
   a. HTTP GET auf original_url
   b. readability::extract() fuer Clean-HTML
   c. UPDATE fnords SET content_full = ... WHERE id = ?
   d. Yield nach jedem Artikel (Lock-Release)
```

**Performance-Charakteristik:**
- Netzwerk-gebunden (HTTP-Requests)
- Sequentielle Verarbeitung pro Artikel
- Lock wird nach jedem Artikel freigegeben

### 3.3 Batch-Verarbeitung (Discordian Analysis)

**Modul:** `src-tauri/src/commands/ollama/batch_processor.rs`

Dies ist der **Haupt-LLM-Verarbeitungsschritt**.

```rust
// Funktion: process_batch_internal()
async fn process_batch_internal(
    state: State<'_, AppState>,
    limit: i32,
    generate_embedding: bool,
) -> Result<BatchResult, String> {
    // 1. Artikel laden (WHERE processed_at IS NULL)
    // 2. Fuer jeden Artikel SEQUENTIELL:
    //    a. Discordian Analysis (LLM-Call)
    //    b. Statistische Analyse (TF-IDF)
    //    c. Ergebnisse mergen
    //    d. Kategorien speichern
    //    e. Keywords speichern + Netzwerk updaten
    //    f. Optional: Artikel-Embedding generieren
    //    g. processed_at = NOW()
}
```

#### 3.3.1 Discordian Analysis (LLM-Call)

**Modul:** `src-tauri/src/ollama/mod.rs`

```rust
// OllamaClient::generate_discordian()
pub async fn generate_discordian(
    &self,
    text: &str,
    locale: &str,
    custom_prompt: Option<&str>,
) -> Result<DiscordianResponse, OllamaError> {
    // Prompt aufbauen
    let prompt = format!(
        "{}\n\n{}\n\nArtikel:\n{}",
        DISCORDIAN_PROMPT_BASE,
        DISCORDIAN_PROMPT_WITH_STATS,
        truncated_text
    );

    // LLM-Call
    let response = self.chat_completion(model, &prompt, num_ctx).await?;

    // JSON-Parsing der Antwort
    parse_discordian_response(&response)
}
```

**Prompt-Struktur:**
```
Du bist ein Nachrichtenanalyst...

Antworte NUR mit einem JSON-Objekt:
{
  "summary": "...",
  "categories": ["Politik", "Wirtschaft", ...],
  "keywords": ["Keyword1", "Keyword2", ...],
  "political_bias": 0,
  "bias_reasoning": "..."
}

Artikel:
[ARTIKEL-TEXT, max. 8000 Zeichen]
```

#### 3.3.2 Statistische Analyse (TF-IDF)

**Modul:** `src-tauri/src/text_analysis/tfidf.rs`

Parallel zur LLM-Analyse werden statistische Keywords extrahiert:

```rust
// TfIdfExtractor::extract_smart()
1. Text tokenisieren (Stopwords entfernen)
2. Term-Frequenzen berechnen
3. IDF aus Corpus-Statistiken laden
4. TF-IDF Scores berechnen
5. Top-N Keywords zurueckgeben
```

**Kategorie-Matching via Keyword-Netzwerk:**
```rust
// derive_categories_from_keywords()
// Leitet Kategorien aus bekannten Keywords ab
1. Keywords in immanentize-Tabelle nachschlagen
2. Kategorie-Assoziationen aggregieren
3. Nach Konfidenz gewichten
```

#### 3.3.3 Ergebnis-Merge

```rust
// Prioritaet: Statistical > LLM > Local
let final_categories = merge_categories_stat_primary(
    stat_categories,   // Aus Keyword-Netzwerk
    llm_categories,    // Aus Discordian Analysis
    local_categories,  // TF-IDF Vorschlaege
    min_confidence: 0.15,
);

let final_keywords = merge_keywords(
    llm_keywords,      // Aus Discordian Analysis
    local_keywords,    // TF-IDF Extraktion
    max_count: 15,
);
```

### 3.4 Embedding-Generierung

**Modul:** `src-tauri/src/embedding_worker.rs`

Embeddings werden in einem **separaten Background-Worker** generiert:

```rust
// process_embedding_queue()
// Parallelisierung: 10 gleichzeitige Embedding-Requests

for chunk in keywords.chunks(10) {
    let futures = chunk.iter().map(|kw| {
        client.generate_embedding(model, &kw.name)
    });

    let results = join_all(futures).await;
    // Ergebnisse speichern...
}
```

**Artikel-Embeddings:**
```rust
// generate_and_save_article_embedding()
let embedding_text = format!("{}\n\n{}", title, content_preview);
let embedding = client.generate_embedding(model, &embedding_text).await?;
// 1024-dimensionaler Vektor fuer Aehnlichkeitssuche
```

---

## 4. Beteiligte Module

| Modul | Pfad | Verantwortung |
|-------|------|---------------|
| **batch_processor.rs** | `src-tauri/src/commands/ollama/` | Haupt-Batch-Verarbeitung |
| **article_processor.rs** | `src-tauri/src/commands/ollama/` | Einzel-Artikel-Verarbeitung |
| **helpers.rs** | `src-tauri/src/commands/ollama/` | Prompt-Verwaltung, Merge-Logik |
| **data_persistence.rs** | `src-tauri/src/commands/ollama/` | Keyword/Kategorie-Speicherung |
| **mod.rs** | `src-tauri/src/ollama/` | OllamaClient, LLM-Kommunikation |
| **embedding_worker.rs** | `src-tauri/src/` | Background-Embedding-Queue |
| **tfidf.rs** | `src-tauri/src/text_analysis/` | TF-IDF Extraktion |
| **category_matcher.rs** | `src-tauri/src/text_analysis/` | Kategorie-Matching |

---

## 5. LLM-Calls und Prompts

### 5.1 Verwendete Modelle

| Modell | Verwendung | Konfigurierbar |
|--------|------------|----------------|
| `ministral-3:latest` | Discordian Analysis | Ja (Settings) |
| `snowflake-arctic-embed2` | Embeddings | Ja (Settings) |

### 5.2 LLM-Calls pro Artikel

| Call | Timing | Dauer (geschaetzt) |
|------|--------|-------------------|
| Discordian Analysis | Sequentiell | 2-5 Sekunden |
| Artikel-Embedding | Optional, nach Analysis | 0.5-1 Sekunde |
| Keyword-Embeddings | Background-Worker | 0.2-0.5 Sek. pro Keyword |

### 5.3 Prompt-Parameter

```rust
// Default-Konfiguration
const DEFAULT_NUM_CTX: u32 = 16384;  // Context Window
const MAX_CONTENT_LENGTH: usize = 8000;  // Artikel-Truncation
```

**Prompt-Variablen:**
- `{language}` - Wird durch Locale ersetzt (z.B. "Deutsch", "English")
- Kategorien sind fix (13 Sephiroth-Kategorien)

### 5.4 Response-Format

```json
{
  "summary": "2-3 Saetze Zusammenfassung",
  "categories": ["Politik", "Wirtschaft"],
  "keywords": ["Merkel", "Koalition", "Bundesregierung"],
  "political_bias": 0,
  "bias_reasoning": "Neutral berichtend..."
}
```

---

## 6. Performance-Analyse

### 6.1 Zeitmessungen (geschaetzt)

| Operation | Dauer | Parallelisiert |
|-----------|-------|----------------|
| HTTP Fetch (Volltext) | 0.5-2s | Nein |
| Discordian Analysis | 2-5s | Nein |
| TF-IDF Extraktion | 10-50ms | Nein |
| Keyword-Speicherung | 20-100ms | Nein |
| Artikel-Embedding | 0.5-1s | Nein |
| **Gesamt pro Artikel** | **3-8s** | - |

### 6.2 Batch-Verarbeitung

```rust
// Aktuelle Implementierung (SEQUENTIELL)
for article in articles {
    let analysis = client.generate_discordian(&article.content).await?;
    save_categories(conn, article.id, &analysis.categories)?;
    save_keywords(conn, article.id, &analysis.keywords)?;
    // ... weitere DB-Operationen
}
```

**Beobachtung:** Bei 100 Artikeln dauert die Batch-Verarbeitung ca. 5-13 Minuten.

### 6.3 Database-Lock-Analyse

Die aktuelle Implementierung folgt korrekten Lock-Patterns:

```rust
// Gut: Lock pro Artikel, nicht fuer gesamte Schleife
for article in articles {
    {
        let conn = db.lock().unwrap();
        // DB-Operationen
    } // Lock wird released

    // LLM-Call OHNE Lock
    let analysis = client.generate_discordian(&text).await?;

    tokio::task::yield_now().await;
}
```

---

## 7. Bottlenecks

### 7.1 Kritischer Bottleneck: Sequentielle LLM-Calls

**Problem:** Die Discordian-Analyse wird fuer jeden Artikel sequentiell ausgefuehrt.

```rust
// AKTUELL: Sequentiell
for article in articles {
    let analysis = generate_discordian(&article.content).await?;  // Blockiert
}
```

**Auswirkung:** Bei N Artikeln und 3s pro LLM-Call = N * 3s Gesamtzeit.

### 7.2 Sekundaerer Bottleneck: HTTP Fetching

**Problem:** Volltext-Abruf erfolgt ebenfalls sequentiell.

**Auswirkung:** Netzwerk-Latenz addiert sich linear.

### 7.3 Tertiarer Bottleneck: DB-Transaktionen

**Problem:** Jeder Artikel erfordert mehrere DB-Writes:
- Categories (1-5 INSERTs)
- Keywords (5-15 INSERTs)
- Keyword-Network (N*(N-1)/2 Updates fuer Co-Occurrence)

**Auswirkung:** Bei 15 Keywords = 105 Network-Updates pro Artikel.

### 7.4 Ressourcen-Engpaesse

| Ressource | Auslastung | Optimierungspotenzial |
|-----------|------------|----------------------|
| CPU | Niedrig (~10%) | Hoch - mehr Parallelitaet moeglich |
| GPU/VRAM | Mittel (~40%) | Mittel - Batching moeglich |
| Netzwerk | Niedrig | Hoch - mehr parallele Requests |
| Disk I/O | Niedrig | Mittel - Transaction-Batching |

---

## 8. Verbesserungsvorschlaege

### 8.1 Parallelisierung der LLM-Calls

**Vorschlag:** Mehrere Artikel gleichzeitig analysieren.

```rust
// VORSCHLAG: Parallele Verarbeitung
use futures::stream::{self, StreamExt};

let concurrency = get_ai_concurrency(state); // 1-10, konfigurierbar

let results: Vec<_> = stream::iter(articles)
    .map(|article| {
        let client = client.clone();
        async move {
            let analysis = client.generate_discordian(&article.content).await?;
            Ok((article, analysis))
        }
    })
    .buffer_unordered(concurrency)
    .collect()
    .await;

// Danach sequentiell DB-Updates
for (article, analysis) in results {
    save_to_db(conn, article, analysis)?;
}
```

**Erwartete Verbesserung:**
- Bei concurrency=4: ~75% Zeitersparnis
- Bei concurrency=8: ~87% Zeitersparnis

**Einschraenkungen:**
- VRAM-Limit (Ollama kann mehrere Requests parallel handlen, aber VRAM-gebunden)
- Bereits implementiert: `ai_parallelism` Setting existiert (aktuell ungenutzt)

### 8.2 HTTP-Fetching parallelisieren

```rust
// VORSCHLAG: Parallele HTTP-Requests
let concurrency = 10;

let fulltext_results = stream::iter(articles_without_fulltext)
    .map(|article| async move {
        let content = fetch_fulltext(&article.url).await;
        (article.id, content)
    })
    .buffer_unordered(concurrency)
    .collect::<Vec<_>>()
    .await;
```

**Erwartete Verbesserung:** ~80% Zeitersparnis beim Volltext-Abruf.

### 8.3 Prompt-Optimierung

**Aktueller Prompt:** ~1000 Tokens System + ~2000 Tokens Artikel = 3000 Tokens Input

**Optimierungsmoeglichkeiten:**

1. **Kuerzerer System-Prompt:**
   - Redundante Anweisungen entfernen
   - Kategorien als JSON-Array statt Liste

2. **Artikel-Truncation optimieren:**
   - Aktuell: Erste 8000 Zeichen
   - Besser: Intelligentes Truncation (Anfang + Ende + wichtige Absaetze)

3. **Few-Shot entfernen:**
   - Prompt enthaelt implizite Beispiele
   - Koennte durch praeziese Formatvorgabe ersetzt werden

**Erwartete Verbesserung:** 10-20% schnellere Responses.

### 8.4 Caching-Strategien

#### 8.4.1 Embedding-Cache

**Bereits implementiert:**
- `embedding_queue` Tabelle
- Background-Worker generiert Embeddings asynchron

**Verbesserung:**
```rust
// Vor LLM-Call pruefen, ob aehnlicher Artikel existiert
let similar = find_similar_by_embedding(title_embedding, threshold=0.95);
if let Some(existing) = similar {
    // Kategorien/Keywords vom aehnlichen Artikel uebernehmen
    copy_analysis_from(existing.id, new_article.id)?;
    return Ok(());
}
```

#### 8.4.2 Prompt-Response-Cache

```rust
// Fuer identische Artikel-Inhalte
let content_hash = hash(&article.content_full);
if let Some(cached) = analysis_cache.get(&content_hash) {
    return Ok(cached);
}
```

**Erwartete Verbesserung:** 5-10% bei Duplikaten/Updates.

### 8.5 Batch-Groessen-Optimierung

**Aktuelle Defaults:**
- `batch_size`: 20 Artikel pro Aufruf
- `fulltext_batch_size`: 50 Artikel

**Empfehlung:**
| Operation | Aktuelle Groesse | Empfohlene Groesse | Begruendung |
|-----------|-----------------|-------------------|-------------|
| Fulltext Fetch | 50 | 100 | Netzwerk kann mehr handlen |
| LLM Analysis | 20 | 10-50 (dynamisch) | Abhaengig von VRAM/Concurrency |
| Embedding | 50 | 100 | Background, Zeit unkritisch |

### 8.6 Ollama-Optimierungen

#### 8.6.1 Keep-Alive

```rust
// OllamaClient Konfiguration
client.set_keep_alive(Duration::from_secs(300)); // Modell 5 Min. im VRAM halten
```

**Aktuell:** Nicht explizit gesetzt (Ollama-Default: 5 Minuten).

#### 8.6.2 Flash Attention

Falls Hardware unterstuetzt (CUDA 11.7+):
```bash
OLLAMA_FLASH_ATTENTION=1 ollama serve
```

**Erwartete Verbesserung:** 20-40% schnellere Inference.

### 8.7 Statistische Vor-Filterung

**Idee:** Vor dem LLM-Call statistische Analyse durchfuehren und nur bei Bedarf LLM nutzen.

```rust
// Statistische Vor-Analyse
let stat_categories = derive_categories_from_keywords(conn, tfidf_keywords)?;
let stat_confidence = stat_categories.iter().map(|(_, c)| c).sum::<f64>();

if stat_confidence > 2.0 && stat_categories.len() >= 2 {
    // Statistische Ergebnisse reichen aus
    save_analysis_without_llm(article, stat_categories, tfidf_keywords)?;
} else {
    // LLM fuer unsichere Faelle
    let llm_analysis = generate_discordian(&article.content).await?;
    // ...
}
```

**Erwartete Verbesserung:** 30-50% der Artikel koennten ohne LLM verarbeitet werden (bei gut trainiertem Netzwerk).

---

## 9. Priorisierte Massnahmen

### Hohe Prioritaet (Signifikante Verbesserung, Machbar)

| # | Massnahme | Aufwand | Erwartete Verbesserung |
|---|-----------|---------|------------------------|
| 1 | **LLM-Parallelisierung aktivieren** | Mittel | 50-75% Zeitersparnis |
| 2 | **HTTP-Fetching parallelisieren** | Niedrig | 80% beim Fulltext-Abruf |
| 3 | **ai_parallelism Setting nutzen** | Niedrig | Bereits implementiert, nur aktivieren |

### Mittlere Prioritaet (Gute Verbesserung, Mehr Aufwand)

| # | Massnahme | Aufwand | Erwartete Verbesserung |
|---|-----------|---------|------------------------|
| 4 | Prompt-Laenge optimieren | Niedrig | 10-20% schnellere Responses |
| 5 | Statistische Vor-Filterung | Mittel | 30-50% weniger LLM-Calls |
| 6 | Intelligentes Artikel-Truncation | Mittel | Bessere Analyseergebnisse |

### Niedrige Prioritaet (Kleine Verbesserung, Spezialfaelle)

| # | Massnahme | Aufwand | Erwartete Verbesserung |
|---|-----------|---------|------------------------|
| 7 | Content-Hash-Cache | Niedrig | 5-10% bei Duplikaten |
| 8 | Aehnlichkeits-basiertes Kopieren | Hoch | Schwer einschaetzbar |
| 9 | Flash Attention (Server-Konfig) | Niedrig | 20-40% (hardwareabhaengig) |

---

## Anhang: Relevante Code-Stellen

### A.1 Batch-Verarbeitung Entry-Point

```
src-tauri/src/commands/ollama/batch_processor.rs:
- process_batch() - Tauri Command
- process_batch_internal() - Hauptlogik
- fetch_fulltext_batch() - Volltext-Abruf
```

### A.2 LLM-Kommunikation

```
src-tauri/src/ollama/mod.rs:
- OllamaClient::generate_discordian()
- OllamaClient::generate_embedding()
- OllamaClient::chat_completion()
```

### A.3 Statistische Analyse

```
src-tauri/src/text_analysis/tfidf.rs:
- TfIdfExtractor::extract_smart()
- CorpusStats::load_from_db()
```

### A.4 Settings-Verwaltung

```
src-tauri/src/commands/ollama/helpers.rs:
- get_ai_concurrency() - Parallelitaets-Setting
- get_num_ctx_setting() - Context-Window
- get_discordian_prompt() - Custom Prompts
```

---

*Bericht erstellt am 2026-01-27 durch automatisierte Code-Analyse.*
