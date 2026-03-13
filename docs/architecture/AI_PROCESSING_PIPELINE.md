# AI Processing Pipeline Reference

> This document provides a comprehensive reference for the AI processing pipeline in fuckupRSS.
> For the main developer guide, see [CLAUDE.md](../../CLAUDE.md).

## Table of Contents

1. [Pipeline Overview](#pipeline-overview)
2. [AI Provider Abstraction](#ai-provider-abstraction)
3. [Content Fields](#content-fields-in-fnords)
4. [Greyface Alert (Bias-Erkennung)](#greyface-alert-bias-erkennung)
5. [Prompt-Design](#prompt-design)
6. [Statistical Text Analysis](#statistical-text-analysis)
7. [Bias Learning System](#bias-learning-system)
8. [Advanced Keyword Extraction](#advanced-keyword-extraction)
9. [Article Clustering](#article-clustering-batch-optimization)
10. [Relevant Modules](#relevant-modules)

---

## Pipeline Overview

The AI processing pipeline consists of 7 sequential stages that transform raw RSS content into analyzed, categorized, and searchable articles. Additional features (Briefings, Story Clustering) operate on already-processed articles.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         AI PROCESSING PIPELINE                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. HAGBARD'S RETRIEVAL                                                     │
│     └─ Fetch full text for ALL new articles (automatic after sync)          │
│                                                                             │
│  2. DISCORDIAN ANALYSIS                                                     │
│     └─ Summarize, categorize, extract keywords via AiTextProvider           │
│     └─ Includes Article Type Classification (news/analysis/opinion/...)     │
│                                                                             │
│  3. ARTICLE EMBEDDING (Extended)                                            │
│     └─ title + summary + content_full (bis 4000 chars) → Embedding          │
│                                                                             │
│  4. GREYFACE ALERT                                                          │
│     └─ Bias detection (political_bias: -2 to +2, sachlichkeit: 0-4)        │
│                                                                             │
│  5. IMMANENTIZE NETWORK                                                     │
│     └─ Keyword graph processing                                             │
│                                                                             │
│  6. NAMED ENTITY RECOGNITION (NER)                                          │
│     └─ Extract persons, organizations, locations, events via LLM            │
│                                                                             │
│  7. POST-PROCESSING FEATURES                                                │
│     ├─ Briefings: AI-generated news summaries (daily/weekly)                │
│     └─ Story Clusters: Group similar articles for perspective comparison    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Stage 1: Hagbard's Retrieval

Full-text extraction for all articles after RSS sync.

| Aspect | Details |
|--------|---------|
| **Trigger** | Automatic after feed sync |
| **Input** | RSS article link/URL |
| **Output** | Full article text |
| **Storage** | `fnords.content_full` |
| **Technology** | `readability` crate |

- All articles are fully fetched, not just truncated feeds
- Full text stored in `content_full`
- `content_raw` remains for change detection and fallback display

### Stage 2: Discordian Analysis

AI-powered summarization, bias detection, keyword validation, and article type classification.

| Aspect | Details |
|--------|---------|
| **Provider** | Configurable via `AiTextProvider` (Ollama or OpenAI-compatible) |
| **Default Model** | ministral-3:latest (Ollama) or configurable (OpenAI-compatible) |
| **Input** | `content_full` ONLY (no fallback) |
| **Primary Output** | Summary, political_bias, sachlichkeit, article_type |
| **Secondary Output** | Validated keywords |
| **Optional Output** | Categories (only if statistical derivation seems wrong) |
| **Ollama API** | `/api/chat` mit System/User Message Split |
| **Structured Output** | JSON Schema Validierung (statt `format: "json"`) |
| **keep_alive** | `5m` (Modell wird nach 5 Minuten Inaktivitaet entladen) |
| **Concurrency** | Konfigurierbar via `ollama_concurrency` Setting (Standard: 1) |

- **Uses ONLY `content_full`** - no fallback to `content_raw`
- Articles without full text are not suggested for analysis
- Individual articles can be re-analyzed anytime (button in ArticleView)
- "Re-analyze all" available in Settings with progress display
- **Categories are primarily derived from keyword network** (statistical)
- LLM categories serve only as optional validation/fallback
- **Article Type Classification:** Jeder Artikel wird als `news`, `analysis`, `opinion`, `satire`, `ad` oder `unknown` klassifiziert (gespeichert in `fnords.article_type`)

### Stage 3: Article Embedding (Extended)

Vector embedding generation for semantic similarity search. Uses extended embedding input that combines title, summary, and article content for better semantic representation.

| Aspect | Details |
|--------|---------|
| **Model** | snowflake-arctic-embed2 (oder konfigurierbar via `EmbeddingProvider`) |
| **Dimensions** | 1024 |
| **Input** | `build_embedding_text()`: Title + Summary + content_full (bis 4000 chars) |
| **Max Input** | 4000 Zeichen (DEFAULT_EMBEDDING_MAX_CHARS), nutzt snowflake-arctic-embed2 8.192 Token Kontext |
| **Fallback** | Falls `content_full` leer: `content_raw` als Fallback |
| **Storage** | `fnords.embedding` + `vec_fnords` virtual table |
| **Index** | sqlite-vec with O(log n) KNN |
| **API** | `/api/embed` (Batch-Endpunkt, mehrere Texte pro Request) |
| **Provider** | Konfigurierbar via `EmbeddingProvider` Trait (Ollama oder OpenAI-compatible) |

**Extended Embedding Input (`build_embedding_text`):**
```
Title
\n\n
Summary (falls vorhanden)
\n\n
Content (bis zum verbleibenden Zeichenbudget)
```

- Automatic after successful Discordian Analysis
- Enables similar article discovery, semantic search, and **Story Clustering**
- Vor dem Embedding-Schritt wird das LLM-Modell explizit aus dem VRAM entladen (`unload_model()`), damit das Embedding-Modell ausreichend VRAM hat
- Die erweiterte Embedding-Berechnung (`build_embedding_text`) ist in `data_persistence.rs` definiert

### Stage 4: Greyface Alert

Mehrdimensionale Bias-Erkennung und Quellenqualitätsbewertung.

| Dimension | Bereich | Beschreibung |
|-----------|---------|--------------|
| `political_bias` | -2 bis +2 | Politische Tendenz (Links bis Rechts) |
| `sachlichkeit` | 0 bis 4 | Sachlichkeitsgrad (Emotional bis Faktisch) |
| `source_credibility` | 1 bis 5 | Quellenqualität (Sterne-Bewertung) |
| `article_type` | Enum | Artikel-Kategorie (news, analysis, opinion, etc.) |

Detaillierte Informationen zu allen Dimensionen: siehe [Greyface Alert (Bias-Erkennung)](#greyface-alert-bias-erkennung).

### Stage 5: Immanentize Network

Keyword graph processing and semantic network building.

Processing steps:
1. **New Keywords**: Generate embedding via snowflake-arctic-embed2
2. **Category Association**: Update `immanentize_sephiroth` table
3. **Neighbor Update**: Calculate co-occurrence + embedding similarity
4. **Synonym Detection**: Flag pairs with `embedding_similarity > 0.92`

### Stage 6: Named Entity Recognition (NER)

LLM-basierte Extraktion von benannten Entitäten aus Artikeln.

| Aspect | Details |
|--------|---------|
| **Provider** | Configurable via `AiTextProvider` (wie Discordian Analysis) |
| **Input** | Title + content_full (bis 3000 chars) |
| **Output** | Entities mit Name, Typ, Mention-Count |
| **Entity-Typen** | `person`, `organization`, `location`, `event` |
| **Storage** | `entities` + `fnord_entities` Tabellen |
| **Structured Output** | JSON Schema mit `ner_schema()` |
| **Batch** | `extract_entities_batch` verarbeitet bis zu 50 Artikel |

**NER-Workflow:**
1. Artikel ohne Entities werden identifiziert (nur bereits LLM-analysierte Artikel)
2. LLM extrahiert Entities mit Typ und Mention-Count pro Artikel
3. Entity-Namen werden normalisiert (lowercase, Titel entfernt, Whitespace bereinigt)
4. Deduplizierung über `(normalized_name, entity_type)` UNIQUE Constraint
5. `article_count` und `last_seen` werden bei wiederholter Erkennung aktualisiert

**Entity-Typen und Beispiele:**

| Typ | Beispiele |
|-----|-----------|
| `person` | Angela Merkel, Elon Musk |
| `organization` | Bundestag, EU, Microsoft |
| `location` | Berlin, Washington D.C. |
| `event` | Klimagipfel 2025, Bundestagswahl |

### Stage 7: Post-Processing Features

Features, die auf bereits verarbeiteten Artikeln aufbauen.

#### Briefings (KI-generierte Nachrichten-Zusammenfassungen)

| Aspect | Details |
|--------|---------|
| **Trigger** | Manuell vom Benutzer (daily/weekly) |
| **Input** | Top 15 Artikel mit Zusammenfassung + Top 20 Trending Keywords |
| **Output** | Strukturiertes Briefing (Überblick, Top-5 Themen, Trends) |
| **Storage** | `briefings` Tabelle |
| **Provider** | Configurable via `AiTextProvider` |
| **Sprache** | Deutsch |

**Briefing-Workflow:**
1. Zeitraum berechnen (daily: 24h, weekly: 7 Tage)
2. Top-15 Artikel mit Zusammenfassung aus dem Zeitraum laden
3. Trending Keywords aus `immanentize_daily` für den Zeitraum laden
4. Prompt mit Artikel-Liste und Keywords zusammenbauen
5. LLM generiert strukturiertes Briefing
6. Briefing in `briefings` Tabelle speichern (UNIQUE per Typ+Zeitraum)

#### Story Clustering (Thematische Artikel-Gruppierung)

| Aspect | Details |
|--------|---------|
| **Trigger** | Manuell vom Benutzer |
| **Input** | Artikel mit Embeddings der letzten N Tage |
| **Output** | Cluster verwandter Artikel mit optionalem Perspektivvergleich |
| **Storage** | `story_clusters` + `story_cluster_articles` |
| **Similarity Threshold** | 0.78 (Cosine Similarity) |
| **Min. Cluster-Größe** | 3 Artikel, 2 verschiedene Quellen |
| **Algorithmus** | Union-Find auf Embedding-Ähnlichkeitsgraph |

**Clustering-Workflow:**
```
1. Artikel mit Embeddings laden (letzte N Tage)
       │
2. Für jeden Artikel: KNN-Suche via vec_fnords (k=50)
       │
3. Ähnlichkeitspaare filtern (similarity >= 0.78)
       │
4. Union-Find: Transitiv verbundene Artikel gruppieren
       │
5. Cluster filtern (>= 3 Artikel, >= 2 Quellen)
       │
6. Cluster-Titel aus gemeinsamen Keywords generieren
       │
7. Optional: LLM-Perspektivvergleich (compare_perspectives)
```

**Perspektivvergleich:**
- Vergleicht die Berichterstattung verschiedener Quellen über dasselbe Thema
- Analysiert übereinstimmende Fakten, Schwerpunkte, Widersprüche
- Ergebnis wird in `story_clusters.perspective_comparison` gespeichert

---

## AI Provider Abstraction

Text generation is decoupled from any specific backend through the `AiTextProvider` trait defined in `src-tauri/src/ai_provider/mod.rs`. This allows switching between local Ollama and remote OpenAI-compatible APIs without changing pipeline code.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       AI PROVIDER ARCHITECTURE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────────────────────────────────────┐                          │
│  │         AiTextProvider (trait)                 │                          │
│  │  generate_text(model, prompt, json_schema)     │                          │
│  │  is_available()                                │                          │
│  │  provider_name()                               │                          │
│  │  suggested_concurrency()                       │                          │
│  └──────────────┬───────────────┬────────────────┘                          │
│                 │               │                                            │
│    ┌────────────▼──┐   ┌───────▼───────────────┐                           │
│    │ OllamaText    │   │ OpenAiCompatible       │                           │
│    │ Provider      │   │ Provider               │                           │
│    │               │   │                        │                           │
│    │ - wraps       │   │ - reqwest HTTP client  │                           │
│    │   OllamaClient│   │ - /v1/chat/completions │                           │
│    │ - /api/chat   │   │ - token usage tracking │                           │
│    │   endpoint    │   │ - supports OpenAI,     │                           │
│    │ - auto-       │   │   Together.ai, Mistral,│                           │
│    │   prepends    │   │   Groq, etc.           │                           │
│    │   /no_think   │   │                        │                           │
│    └───────────────┘   └────────────────────────┘                           │
│                                                                             │
│  ┌──────────────────────────────────────────────┐                          │
│  │         EmbeddingProvider (trait)              │                          │
│  │  generate_embedding(text)                      │                          │
│  │  generate_embeddings_batch(texts)              │                          │
│  │  embedding_dimensions()                        │                          │
│  │  provider_name()                               │                          │
│  └──────────────┬───────────────┬────────────────┘                          │
│                 │               │                                            │
│    ┌────────────▼──┐   ┌───────▼───────────────┐                           │
│    │ OllamaEmbed-  │   │ OpenAiEmbed-          │                           │
│    │ dingProvider   │   │ dingProvider           │                           │
│    │ /api/embed     │   │ /v1/embeddings         │                           │
│    └───────────────┘   └────────────────────────┘                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Trait-Signaturen

**AiTextProvider:**
```rust
async fn generate_text(
    &self,
    model: &str,
    prompt: &str,
    json_schema: Option<serde_json::Value>,  // JSON Schema statt bool json_mode
) -> Result<GenerationResult, AiProviderError>;
```

**EmbeddingProvider:**
```rust
async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AiProviderError>;
async fn generate_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, AiProviderError>;
fn embedding_dimensions(&self) -> usize;
```

**Wichtig:** `json_schema` ersetzt den früheren `json_mode: bool` Parameter. Provider, die JSON Schema Validierung unterstützen (Ollama 2025+, OpenAI), validieren die Antwort direkt gegen das Schema. Für andere Provider wird auf plain JSON Mode zurückgefallen.

### Provider Selection

The active text generation provider is configured via the `ai_text_provider` setting in the database:

| Setting Value | Provider | Description |
|---------------|----------|-------------|
| `ollama` (default) | `OllamaTextProvider` | Local Ollama instance, auto-prepends `/no_think` |
| `openai_compatible` | `OpenAiCompatibleProvider` | Any OpenAI-compatible API endpoint |

Additional settings for `openai_compatible`:
- `openai_base_url` - API base URL (e.g. `https://api.openai.com`, `https://api.together.xyz`)
- `openai_api_key` - Bearer token for authentication
- `openai_model` - Model name to use (e.g. `gpt-4.1-nano`, `meta-llama/Llama-3-70b`)

### Cost Tracking

When using the OpenAI-compatible provider, token usage is tracked for cost estimation. The `ai_cost_log` SQLite table records per-request token counts (input/output) returned by the API. Ollama does not report token usage in non-streaming mode, so its entries have `NULL` token counts.

### Embeddings

Embeddings werden über das separate `EmbeddingProvider` Trait verwaltet. Es unterstützt sowohl Ollama als auch OpenAI-kompatible Embedding-APIs:

| Provider | Modell | Dimensionen | API-Endpunkt |
|----------|--------|-------------|--------------|
| `OllamaEmbeddingProvider` | snowflake-arctic-embed2:latest | 1024 | `/api/embed` (Batch) |
| `OpenAiEmbeddingProvider` | text-embedding-3-small | konfigurierbar (default: 1024) | `/v1/embeddings` |

Die Konfiguration erfolgt über `EmbeddingProviderConfig` und `create_embedding_provider()`.

### Source Modules

| Module | Purpose |
|--------|---------|
| `src-tauri/src/ai_provider/mod.rs` | `AiTextProvider` + `EmbeddingProvider` Traits, Factory-Funktionen |
| `src-tauri/src/ai_provider/ollama_provider.rs` | `OllamaTextProvider` + `OllamaEmbeddingProvider` |
| `src-tauri/src/ai_provider/openai_provider.rs` | `OpenAiCompatibleProvider` (reqwest-based) |
| `src-tauri/src/ai_provider/openai_embedding_provider.rs` | `OpenAiEmbeddingProvider` (reqwest-based) |

---

## Content Fields in fnords

The `fnords` table contains two distinct content fields:

| Field | Purpose | Source | Used For |
|-------|---------|--------|----------|
| `content_raw` | RSS feed content (excerpt) | Feed Sync | Change detection, fallback display |
| `content_full` | Full webpage text | Hagbard's Retrieval | ALL AI analysis |

**Critical Rule**: All AI analyses use exclusively `content_full`. Articles without full text are not analyzed.

```sql
-- Articles ready for analysis
SELECT id, title FROM fnords
WHERE content_full IS NOT NULL
  AND summary IS NULL;

-- Articles missing full text
SELECT id, title FROM fnords
WHERE content_full IS NULL;
```

---

## Greyface Alert (Bias-Erkennung)

Das Greyface Alert System bewertet Artikel auf vier Dimensionen, um Nutzer auf potentielle Einseitigkeit oder Qualitätsprobleme hinzuweisen.

```
┌─────────────────────────────────────────────────────────────┐
│                     GREYFACE ALERT                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Politische Tendenz        Sachlichkeit                    │
│  ◀━━━━━━━━●━━━━━━━━▶       ◀━━━━━━━━━━●━━▶                 │
│  Links    Mitte   Rechts   Emotional   Sachlich            │
│                                                             │
│  Quellenqualität           Kategorie                       │
│  ★★★★☆                     📰 Nachricht                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Dimension 1: Politische Tendenz (political_bias)

| Wert | Bedeutung | Beispiel-Indikatoren |
|------|-----------|----------------------|
| -2 | Stark links | Kapitalismuskritik, Klassenkampf-Rhetorik |
| -1 | Leicht links | Soziale Gerechtigkeit, Umverteilung positiv |
| 0 | Neutral/Mitte | Ausgewogene Darstellung, multiple Perspektiven |
| +1 | Leicht rechts | Marktliberalismus, Traditionswerte positiv |
| +2 | Stark rechts | Nationalismus, Anti-Establishment |

**Datentyp:** INTEGER (-2 bis +2)
**UI-Darstellung:** Slider oder farbige Skala

### Dimension 2: Sachlichkeit

| Wert | Bedeutung | Indikatoren |
|------|-----------|-------------|
| 0 | Stark emotional | Superlative, Ausrufezeichen, Clickbait, Angstmache |
| 1 | Emotional | Wertende Adjektive, einseitige Wortwahl |
| 2 | Gemischt | Fakten mit Meinung vermischt |
| 3 | Überwiegend sachlich | Faktenbasiert mit leichter Färbung |
| 4 | Sachlich | Neutrale Sprache, Quellenangaben, Fakten |

**Datentyp:** INTEGER (0 bis 4)
**UI-Darstellung:** 5-Stufen-Anzeige oder Prozent (0-100%)

### Dimension 3: Quellenqualität (source_credibility)

| Sterne | Bedeutung | Kriterien |
|--------|-----------|-----------|
| ★☆☆☆☆ | Fragwürdig | Keine Quellenangaben, bekannte Desinformation |
| ★★☆☆☆ | Schwach | Wenig Belege, stark meinungsgetrieben |
| ★★★☆☆ | Mittel | Einige Quellen, erkennbare Perspektive |
| ★★★★☆ | Gut | Solide Recherche, transparente Methodik |
| ★★★★★ | Exzellent | Primärquellen, Peer-Review, etablierte Redaktion |

**Datentyp:** INTEGER (1 bis 5)
**Berechnung:** Kombination aus Feed-Basis-Wert + Artikel-Modifikatoren

**Berechnungslogik:**

```rust
fn calculate_quality(pentacle: &Pentacle, fnord: &Fnord) -> i32 {
    let mut score = pentacle.default_quality as f32;

    // Positive Modifikatoren
    if fnord.has_sources { score += 1.0; }
    if fnord.author.is_some() { score += 0.5; }
    if fnord.sachlichkeit >= 3 { score += 0.5; }

    // Negative Modifikatoren
    if fnord.is_clickbait { score -= 1.0; }
    if fnord.sachlichkeit <= 1 { score -= 0.5; }

    score.clamp(1.0, 5.0).round() as i32
}
```

### Dimension 4: Artikel-Kategorie (article_type)

| Kategorie | DB-Wert | Beschreibung |
|-----------|---------|--------------|
| Nachricht | `news` | Faktenbericht, 5 W-Fragen |
| Analyse | `analysis` | Einordnung mit Hintergrund |
| Meinung | `opinion` | Kommentar, Editorial, Kolumne |
| Satire | `satire` | Satirischer Inhalt |
| Werbung | `ad` | Sponsored Content, PR |
| Unbekannt | `unknown` | Nicht einordbar |

**Datentyp:** TEXT (enum)
**Ermittlung:** Durch KI (ministral-3)

### UI-Darstellung

Das Greyface Alert wird in der Artikel-Ansicht als kompaktes Panel dargestellt:

```
┌────────────────────────────────┐
│ GREYFACE ALERT                 │
│ Tendenz: ━━━●━━ Neutral        │
│ Sachlich: ★★★★☆               │
│ Typ: 📰 Nachricht              │
└────────────────────────────────┘
```

---

## Prompt-Design

### Haupt-Analyse-Prompt

Ein einzelner Prompt für alle Text-Tasks (Zusammenfassung, Kategorisierung, Keyword-Extraktion, Bias-Erkennung):

```
Du bist ein Nachrichtenanalyst. Analysiere den folgenden Artikel und antworte NUR mit validem JSON.

ARTIKEL:
Titel: {title}
Quelle: {source}
Inhalt: {content}

Antworte mit diesem JSON-Format:
{
  "summary": "2-3 Sätze Zusammenfassung auf Deutsch",
  "categories": ["Kategorie1", "Kategorie2"],
  "keywords": ["Stichwort1", "Stichwort2", "Stichwort3"],
  "greyface": {
    "political_bias": 0,
    "sachlichkeit": 3,
    "article_type": "news"
  }
}

REGELN:
- summary: Deutsch, neutral, nur Kernaussagen, 2-3 Sätze
- categories: Maximal 3, NUR aus dieser Liste wählen:
  Tech, Politik, Wirtschaft, Wissenschaft, Kultur, Sport, Gesellschaft, Umwelt, Sicherheit, Gesundheit
- keywords: 3-7 spezifische Begriffe (Personen, Orte, Konzepte, Organisationen)
- political_bias: -2 (stark links) bis +2 (stark rechts), 0 = neutral/ausgewogen
- sachlichkeit: 0 (stark emotional) bis 4 (sachlich/faktenbasiert)
- article_type: news | analysis | opinion | satire | ad | unknown

Antworte NUR mit dem JSON, keine Erklärungen, kein Markdown.
```

### Beispiel-Input

```
ARTIKEL:
Titel: EU verabschiedet AI Act – Strengere Regeln für KI-Systeme
Quelle: heise.de
Inhalt: Die Europäische Union hat heute den AI Act final verabschiedet.
Das Gesetz sieht vor, dass Hochrisiko-KI-Anwendungen künftig strenge
Auflagen erfüllen müssen. Verboten werden unter anderem Social-Scoring-Systeme
und biometrische Echtzeit-Überwachung. Die Verordnung tritt 2025 in Kraft.
Kritiker bemängeln, dass die Regeln Innovation bremsen könnten.
```

### Beispiel-Output

```json
{
  "summary": "Die EU hat den AI Act verabschiedet, der strenge Auflagen für Hochrisiko-KI vorsieht. Social Scoring und biometrische Echtzeit-Überwachung werden verboten. Das Gesetz tritt 2025 in Kraft.",
  "categories": ["Tech", "Politik"],
  "keywords": ["EU", "AI Act", "KI-Regulierung", "Hochrisiko-KI", "Social Scoring", "Biometrie"],
  "greyface": {
    "political_bias": 0,
    "sachlichkeit": 4,
    "article_type": "news"
  }
}
```

### JSON-Output-Format

Das LLM gibt ein strukturiertes JSON-Objekt zurück (validiert gegen `discordian_schema()`):

| Feld | Typ | Beschreibung |
|------|-----|--------------|
| `summary` | String | 2-3 Sätze Zusammenfassung auf Deutsch |
| `categories` | Array<String> | 0-1 Kategorien (nur bei Korrektur der statistischen Analyse) |
| `keywords` | Array<String> | 3-5 spezifische Schlagwörter |
| `political_bias` | Integer | -2 bis +2 |
| `sachlichkeit` | Integer | 0 bis 4 |
| `article_type` | String | news, analysis, opinion, satire, ad, unknown |
| `rejected_keywords` | Array<String> | Abgelehnte statistische Keywords (für Bias Learning) |
| `rejected_categories` | Array<String> | Abgelehnte statistische Kategorien (für Bias Learning) |

### Parsing im Rust-Backend

```rust
/// Raw-Struct mit flexibler Deserialisierung (akzeptiert Floats vom LLM)
#[derive(Deserialize)]
struct RawDiscordianAnalysisWithRejections {
    summary: String,          // flexible_deser: akzeptiert auch {name: "..."}-Objekte
    categories: Vec<String>,  // flexible_deser: akzeptiert auch Objekt-Arrays
    keywords: Vec<String>,
    rejected_keywords: Vec<String>,
    rejected_categories: Vec<String>,
    political_bias: f64,      // Floats werden gerundet
    sachlichkeit: f64,
    article_type: String,     // Default: "unknown"
}

/// Normalisierter Struct mit Integer-Werten
#[derive(Serialize)]
struct DiscordianAnalysisWithRejections {
    summary: String,
    categories: Vec<String>,
    keywords: Vec<String>,
    rejected_keywords: Vec<String>,
    rejected_categories: Vec<String>,
    political_bias: i32,      // Gerundet aus f64
    sachlichkeit: i32,
    article_type: String,     // Normalisiert via normalize_article_type()
}

// Provider-Aufruf mit JSON Schema statt json_mode
let result = provider.generate_text(
    model,
    &prompt,
    Some(discordian_schema()),  // JSON Schema für Validierung
).await?;
```

### Ollama API: Chat Endpoint mit System/User Messages

Die Ollama-Kommunikation verwendet den `/api/chat` Endpoint (statt `/api/generate`) mit System/User Message Split für bessere Prompt-Kontrolle:

```
POST /api/chat
{
  "model": "ministral-3:latest",
  "messages": [
    {"role": "system", "content": "You are a professional media analyst..."},
    {"role": "user", "content": "PRE-COMPUTED: keywords=...\n\nTitle: ...\nContent: ..."}
  ],
  "format": <JSON Schema>,
  "stream": false,
  "options": {"num_ctx": 4096, "num_predict": 4096},
  "keep_alive": "5m"
}
```

**System/User Message Templates (in `ollama/mod.rs`):**
- `DEFAULT_DISCORDIAN_SYSTEM` - System-Rolle für Analyse
- `DEFAULT_DISCORDIAN_USER` - User-Prompt mit statistischen Vordaten
- `DEFAULT_BIAS_SYSTEM` / `DEFAULT_BIAS_USER` - Bias-Analyse
- `DEFAULT_SUMMARY_SYSTEM` / `DEFAULT_SUMMARY_USER` - Zusammenfassung

### Structured Outputs (JSON Schema Validierung)

Statt des einfachen `format: "json"` Modus werden jetzt **JSON Schemas** als `format`-Parameter übergeben. Ollama validiert die Antwort direkt gegen das Schema.

**Definierte Schemas (in `ollama/mod.rs`):**

| Schema-Funktion | Felder | Verwendung |
|-----------------|--------|------------|
| `discordian_schema()` | political_bias, sachlichkeit, summary, keywords, categories, rejected_keywords, rejected_categories | Batch-Analyse mit statistischer Voranalyse |
| `discordian_simple_schema()` | political_bias, sachlichkeit, summary, keywords, categories | Einzelartikel-Analyse (ohne Rejections) |
| `bias_schema()` | political_bias, sachlichkeit | Reine Bias-Analyse |
| `synonym_schema()` | is_synonym, confidence, explanation | Synonym-Verifikation |
| `ner_schema()` | entities (array of {name, type, mentions}) | Named Entity Recognition |

### Parsing-Regeln

1. **JSON Schema Validierung:** Provider wird mit `json_schema: Some(schema)` aufgerufen. Ollama validiert die Antwort gegen das Schema, OpenAI-compatible nutzt `response_format: json_schema`
2. **Validierung:** Alle Werte werden auf gültige Bereiche geprüft (z.B. political_bias zwischen -2 und +2)
3. **Flexible Deserialization:** `flexible_deser` Module behandelt LLM-Antworten, die Objekte statt Strings zurückgeben (z.B. `{"name": "keyword"}` statt `"keyword"`)
4. **Fallback:** Bei ungültigen Werten werden Defaults verwendet (political_bias=0, sachlichkeit=2, article_type="unknown")
5. **Error Handling:** Bei komplettem Parse-Fehler wird der Artikel als "nicht analysiert" markiert

---

## Statistical Text Analysis

### Statistical-First Workflow

Statistical analysis runs **BEFORE** LLM analysis. Categories are now **primarily derived from the keyword network**, with LLM categories serving only as optional validation/fallback:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    STATISTICAL-FIRST WORKFLOW                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. STATISTICAL PRE-ANALYSIS (TF-IDF + Keyword Network)                     │
│     ├─ Extract keywords via TF-IDF with bias weights                        │
│     ├─ Derive categories from keyword-category associations                 │
│     └─ Generate keyword_candidates, category_scores                         │
│                                                                             │
│  2. LLM QUALITY CONTROL (Discordian Analysis)                               │
│     ├─ PRIMARY FOCUS: Summary quality, bias detection, objectivity          │
│     ├─ SECONDARY: Validate/filter keywords (keep good ones, add max 2 new)  │
│     ├─ OPTIONAL: Categories (only if statistical results seem wrong)        │
│     └─ Returns rejected_keywords, rejected_categories for bias learning     │
│                                                                             │
│  3. BIAS LEARNING FROM REJECTIONS                                           │
│     ├─ Rejected keywords: boost -= 0.1                                      │
│     └─ Rejected categories: term_weight -= 0.1 for matching_terms           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**LLM Focus Areas (in order of importance):**
1. High-quality summary (2-3 factual sentences)
2. Precise political bias assessment (-2 to +2)
3. Precise objectivity assessment (sachlichkeit 0-4)
4. Keyword validation and refinement
5. Category correction (only if clearly wrong)

### Analysis Methods

| Analysis | Method | Output |
|----------|--------|--------|
| Keyword Extraction | TF-IDF + Corpus-Stats | `keyword_candidates` with score |
| Category Matching | Word frequency + word lists | `category_scores` with `matching_terms` |
| LLM Validation | AiTextProvider (configurable) | `rejected_keywords`, `rejected_categories` |

### Corpus-wide TF-IDF

The system uses corpus-wide TF-IDF for better keyword extraction:

- `corpus_stats` table stores Document Frequencies
- At >= 10 articles, true IDF is used
- Before that: fallback to simple TF analysis
- Corpus stats are updated after each successful analysis

```sql
-- Check corpus stats
SELECT COUNT(*) as total_terms,
       SUM(doc_count) as total_occurrences
FROM corpus_stats;
```

### Source Types and Weights

| Source | Description | Default Weight |
|--------|-------------|----------------|
| `ai` | Generated/validated by LLM | 1.0 |
| `statistical` | Generated by TF-IDF/word frequency | 0.9 |
| `manual` | Added by user | 1.2 |

Source weights are applied to confidence values (clamped to 0.0-1.0).

### Processing Status Tracking

**Important:** Statistical and LLM analysis use different tracking mechanisms:

| Analysis Type | Tracking Field | Purpose |
|---------------|----------------|---------|
| **LLM Analysis** | `fnords.processed_at` | Timestamp when LLM analysis completed |
| **Statistical Analysis** | `fnord_immanentize.source='statistical'` | Keywords with statistical source |

**Key Behavior:**
- Statistical analysis does **NOT** set `processed_at`
- This allows LLM analysis to run after statistical analysis
- Articles are considered "LLM-processed" only when `processed_at IS NOT NULL`
- Statistical processing is tracked by checking for keywords with `source='statistical'`

```sql
-- Articles ready for LLM analysis (not yet LLM-processed)
SELECT id FROM fnords WHERE processed_at IS NULL AND content_full IS NOT NULL;

-- Articles already statistically processed
SELECT DISTINCT fnord_id FROM fnord_immanentize WHERE source = 'statistical';

-- Articles that need statistical analysis (not yet statistically processed)
SELECT id FROM fnords
WHERE processed_at IS NULL
  AND content_full IS NOT NULL
  AND id NOT IN (SELECT DISTINCT fnord_id FROM fnord_immanentize WHERE source = 'statistical');
```

---

## Bias Learning System

The system learns from two sources to improve statistical analysis over time:

### 1. LLM Rejections (Automatic)

| Rejection | Bias Adjustment |
|-----------|-----------------|
| LLM rejects keyword | `keyword_boost -= 0.1` |
| LLM rejects category | `category_term_weight -= 0.1` for each matching_term |
| LLM rejects category | `category_boost -= 0.1` general |

### 2. User Corrections (Manual)

| Correction | Bias Adjustment |
|------------|-----------------|
| Keyword removed | `keyword_boost -= 0.1` |
| Keyword added | `keyword_boost += 0.1` |
| Category removed | `category_boost -= 0.1` + term_weights |
| Category added | `category_boost += 0.1` |

### Bias Weights Storage

Weights are stored in the `bias_weights` table:

| Column | Description |
|--------|-------------|
| `weight_type` | `keyword_boost`, `category_term`, `category_boost` |
| `weight_value` | Clamped to 0.1-3.0 |
| `correction_count` | Tracks frequency of adjustments |

```sql
-- View current bias weights
SELECT weight_type, target_id, weight_value, correction_count
FROM bias_weights
ORDER BY correction_count DESC
LIMIT 20;
```

---

## Advanced Keyword Extraction

The keyword extraction system uses multiple methods with configurable options.

### Configuration Structure

```rust
pub struct KeywordConfig {
    // Standard options
    pub max_keywords: usize,           // Default: 15
    pub min_word_length: usize,        // Default: 3
    pub use_stemming: bool,            // Default: true
    pub max_categories: usize,         // Default: 5
    pub statistical_confidence: f64,   // Default: 0.8
    pub compound_confidence_factor: f64, // Default: 0.8

    // === MMR Diversification ===
    pub use_mmr: bool,                 // Default: true
    pub mmr_lambda: f64,               // Default: 0.6 (0.0=diversity, 1.0=relevance)

    // === TRISUM Multi-Centrality ===
    pub use_trisum: bool,              // Default: false
    pub trisum_pagerank_weight: f64,   // Default: 0.4
    pub trisum_eigenvector_weight: f64, // Default: 0.35
    pub trisum_betweenness_weight: f64, // Default: 0.25

    // === Levenshtein Deduplication ===
    pub levenshtein_max_distance: usize, // Default: 2
}
```

### Predefined Configurations

| Configuration | use_mmr | use_trisum | Description |
|---------------|---------|------------|-------------|
| `standard()` | true | false | Standard for single articles |
| `batch_processing()` | true | true | For batch processing (TRISUM active) |
| `high_diversity()` | true | true | Maximum keyword diversity |
| `local_extraction()` | false | false | Fallback without advanced features |

### MMR (Maximal Marginal Relevance)

MMR balances relevance vs. diversity of keywords:

```
Score(k) = lambda * Relevance(k) - (1-lambda) * max(Similarity(k, selected))
```

| Lambda Value | Effect |
|--------------|--------|
| 0.3 | More diversity |
| 0.6 | Balanced (default) |
| 0.7 | More relevance |

### TRISUM Multi-Centrality

TRISUM combines three graph centrality measures:

| Centrality | Weight | Purpose |
|------------|--------|---------|
| PageRank | 0.4 | Find important keywords |
| Eigenvector | 0.35 | Find well-connected keywords |
| Betweenness | 0.25 | Find "bridge" keywords connecting topics |

- Recommended for batch processing
- Finds keywords that connect different topic areas

### Levenshtein Deduplication

Removes near-duplicates from keyword lists:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_distance` | 2 | Maximum edit distance for deduplication |

Examples of duplicates removed:
- "Trump" vs "Trumps"
- "Analysis" vs "Analyse"
- "Economy" vs "Economic"

---

## Article Clustering (Batch Optimization)

For batch processing, similar articles can be grouped to reduce LLM calls.

### Clustering Workflow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ARTICLE CLUSTERING WORKFLOW                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. Load articles with embeddings                                           │
│                                                                             │
│  2. Agglomerative Hierarchical Clustering                                   │
│     └─ Group similar articles by embedding distance                         │
│                                                                             │
│  3. Analyze only cluster representatives via LLM                            │
│     └─ One LLM call per cluster (not per article)                          │
│                                                                             │
│  4. Transfer keywords to cluster members                                    │
│     └─ All articles in cluster receive same keywords                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Benefits

| Benefit | Impact |
|---------|--------|
| Reduced LLM calls | Often 30-50% fewer calls |
| Consistent keywords | Similar articles get same keywords |
| Faster processing | Significant speedup for large batches |

### Cluster Configuration

```rust
pub struct ClusterConfig {
    pub distance_threshold: f64,    // Default: 0.4 (Cosine distance)
    pub min_cluster_size: usize,    // Default: 2
    pub max_clusters: usize,        // Default: 0 (unlimited)
}
```

### Usage via Tauri Commands

```typescript
// Standard batch (without clustering)
await invoke('process_batch', { model, limit });

// Cluster-optimized batch
await invoke('process_batch_clustered', {
  model,
  limit,
  useClustering: true  // Optional, default: true
});
```

---

## Relevant Modules

### Statistical Analysis

| Module | Purpose |
|--------|---------|
| `src-tauri/src/text_analysis/tfidf.rs` | TF-IDF implementation |
| `src-tauri/src/text_analysis/category_matcher.rs` | Category word lists |
| `src-tauri/src/text_analysis/bias.rs` | Bias weights |
| `src-tauri/src/text_analysis/stopwords.rs` | DE/EN stopwords |

### Keyword Extraction

| Module | Purpose |
|--------|---------|
| `src-tauri/src/keywords/mod.rs` | Main extractor |
| `src-tauri/src/keywords/config.rs` | Configuration |
| `src-tauri/src/keywords/advanced.rs` | MMR, TRISUM, Levenshtein |
| `src-tauri/src/keywords/clustering.rs` | Article clustering |

### AI Integration

| Module | Purpose |
|--------|---------|
| `src-tauri/src/ai_provider/mod.rs` | `AiTextProvider` + `EmbeddingProvider` Traits, Factory-Funktionen |
| `src-tauri/src/ai_provider/ollama_provider.rs` | `OllamaTextProvider` + `OllamaEmbeddingProvider` |
| `src-tauri/src/ai_provider/openai_provider.rs` | OpenAI-compatible text generation (reqwest) |
| `src-tauri/src/ai_provider/openai_embedding_provider.rs` | OpenAI-compatible embedding generation |
| `src-tauri/src/ollama/mod.rs` | OllamaClient: `/api/chat`, JSON Schemas, Embedding-API, Prompt-Konstanten |
| `src-tauri/src/retrieval/mod.rs` | Hagbard's Retrieval (full-text fetching) |

### AI Commands

| Module | Purpose |
|--------|---------|
| `src-tauri/src/commands/ai/mod.rs` | AI command entry points (Tauri commands) |
| `src-tauri/src/commands/ai/batch_processor.rs` | Batch processing |
| `src-tauri/src/commands/ai/article_processor.rs` | Single-article AI processing |
| `src-tauri/src/commands/ai/model_management.rs` | Model listing, pulling, provider testing |
| `src-tauri/src/commands/ai/prompts.rs` | Prompt templates |
| `src-tauri/src/commands/entities.rs` | NER Pipeline (Named Entity Recognition) |
| `src-tauri/src/commands/briefings.rs` | Briefing-Generierung (daily/weekly) |
| `src-tauri/src/commands/story_clusters.rs` | Story Clustering + Perspektivvergleich |

### Database Operations

| Module | Purpose |
|--------|---------|
| `src-tauri/src/immanentize.rs` | Keyword network operations |
| `src-tauri/src/commands/ai/data_persistence.rs` | AI result persistence, cost logging, `build_embedding_text()` |
| `src-tauri/src/commands/article_analysis.rs` | Statistical article analysis |

---

## Related Documentation

- [CLAUDE.md](../../CLAUDE.md) - Main developer guide
- [docs/ANFORDERUNGEN.md](../../docs/ANFORDERUNGEN.md) - Technical specification
- [KEYWORDS_SCHEMA.md](../features/immanentize/KEYWORDS_SCHEMA.md) - Keyword database schema
- [STOPWORD_KEYWORD_REPORT.md](../archive/STOPWORD_KEYWORD_REPORT.md) - Stopword and keyword analysis report
