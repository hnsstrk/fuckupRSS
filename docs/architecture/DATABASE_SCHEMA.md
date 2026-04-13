# Database Schema Reference

This document provides a comprehensive overview of the fuckupRSS SQLite database schema.

**Related documentation:**
- Database patterns and best practices: See [CLAUDE.md](../../CLAUDE.md#database-patterns--best-practices)
- Full schema implementation: `src-tauri/src/db/schema.rs`
- Technical specification: `docs/ROADMAP.md` chapters 6b + 10

---

## Core Tables

### `pentacles` - Feed Sources

Stores RSS/Atom feed subscriptions (named after Illuminatus! terminology for "Pentacle" = Feed source).

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `url` | TEXT | Feed URL |
| `title` | TEXT | Feed title |
| `description` | TEXT | Feed description |
| `site_url` | TEXT | Website URL |
| `icon_url` | TEXT | Favicon URL |
| `last_sync` | TEXT | Last synchronization timestamp |
| `sync_interval` | INTEGER | Sync interval in minutes |
| `created_at` | TEXT | Creation timestamp |

### `fnords` - Articles

Stores articles/entries from feeds (named after Illuminatus! terminology for articles with hidden meanings).

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `pentacle_id` | INTEGER | FK to pentacles |
| `guid` | TEXT | Unique article identifier from feed |
| `title` | TEXT | Article title |
| `link` | TEXT | Article URL |
| `content_raw` | TEXT | RSS feed content (excerpt) |
| `content_full` | TEXT | Full article text (from Hagbard's Retrieval) |
| `content_hash` | TEXT | Hash for change detection |
| `author` | TEXT | Article author |
| `published_at` | TEXT | Publication timestamp |
| `status` | TEXT | 'concealed' (unread), 'illuminated' (read), 'golden_apple' (favorite) |
| `has_changes` | INTEGER | TRUE if article has revisions |
| `summary` | TEXT | AI-generated summary |
| `political_bias` | INTEGER | -2 to +2 (left to right) |
| `sachlichkeit` | INTEGER | 0-4 (objectivity score) |
| `article_type` | TEXT | Article type classification: `news`, `analysis`, `opinion`, `satire`, `ad`, `unknown` (Default: `unknown`) |
| `embedding` | BLOB | 1024-dim article embedding |
| `processed_at` | TEXT | AI processing timestamp |
| `created_at` | TEXT | Creation timestamp |

**Indizes:**
- `idx_fnords_article_type` auf `article_type`

### `fnord_revisions` - Article Version History

Tracks changes to articles over time (for "Fnord" detection - seeing what others don't see).

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `fnord_id` | INTEGER | FK to fnords |
| `title` | TEXT | Titel zu diesem Zeitpunkt |
| `author` | TEXT | Autor zu diesem Zeitpunkt |
| `content_raw` | TEXT | Feed-Content zu diesem Zeitpunkt |
| `content_full` | TEXT | Volltext zu diesem Zeitpunkt |
| `published_at` | DATETIME | Veröffentlichungsdatum laut Feed |
| `content_hash` | TEXT | SHA256 von content_full oder content_raw |
| `revision_at` | DATETIME | Wann diese Version erfasst wurde |
| `revision_number` | INTEGER | Fortlaufende Nummer (1 = Original, 2 = erste Änderung, etc.) |
| `changes_summary` | TEXT | JSON mit geänderten Feldern: `{"title": true, "content": true, ...}` |

**Indizes:**
- `idx_revisions_fnord` auf `fnord_id`
- `idx_revisions_date` auf `revision_at DESC`

#### Revisionsverwaltung (Fnord History)

Artikel können sich ändern - sei es durch Korrekturen, Updates oder "stille" Änderungen. fuckupRSS speichert **alle Versionen** eines Artikels und macht Änderungen sichtbar.

**Was wird auf Änderungen geprüft?**

| Feld | Prüfung | Speicherung |
|------|---------|-------------|
| `title` | Ja | In Revision |
| `content_raw` | Ja (Hash) | In Revision |
| `content_full` | Ja (Hash) | In Revision |
| `author` | Ja | In Revision |
| `published_at` | Ja | In Revision |
| `summary` (KI) | Nein | Nur aktuell |

**Änderungserkennung beim Sync:**

```
Feed-Sync
    │
    ├─► Artikel existiert bereits? (via GUID)
    │       │
    │       ├─► Nein ──► Neuer Artikel, Revision 1 anlegen
    │       │
    │       └─► Ja ──► Content-Hash vergleichen
    │                   │
    │                   ├─► Hash identisch ──► Keine Aktion
    │                   │
    │                   └─► Hash unterschiedlich ──► Neue Revision anlegen
    │                                               ──► has_changes = TRUE
    │                                               ──► Volltext neu abrufen
```

**Kennzeichnung in der UI:**
- `●` = Normale Artikel
- `⚡` = Artikel mit Änderungen (Fnord!)
- `[🔄 3]` = 3 Revisionen vorhanden

---

## Kategorien (13 Fixed Categories)

### `sephiroth` - Categories

Fixed set of 13 categories (named after Kabbalah's Sephiroth, the tree of life's emanations).

**Categories:** Technik, Politik, Wirtschaft, Wissenschaft, Kultur, Sport, Gesellschaft, Umwelt, Sicherheit, Gesundheit, Verteidigung, Energie, Recht

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `name` | TEXT | Category name |
| `description` | TEXT | Category description |
| `parent_id` | INTEGER | FK for subcategories |
| `icon` | TEXT | Font Awesome icon class |

### `fnord_sephiroth` - Article-Category Mapping

Links articles to their categories with provenance tracking.

| Column | Type | Description |
|--------|------|-------------|
| `fnord_id` | INTEGER | FK to fnords |
| `sephiroth_id` | INTEGER | FK to sephiroth |
| `source` | TEXT | 'ai' or 'manual' |
| `confidence` | REAL | 0.0-1.0 confidence score |
| `assigned_at` | TEXT | Assignment timestamp |

---

## Immanentize Network (Semantic Keyword Graph)

The "Immanentize Network" is a semantic knowledge graph of keywords, named after "Immanentize the Eschaton" from Illuminatus!

### `immanentize` - Keywords

Central keyword storage with embeddings for semantic similarity.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `name` | TEXT | Keyword text (unique) |
| `embedding` | BLOB | 1024-dim embedding (snowflake-arctic-embed2) |
| `quality_score` | REAL | Calculated quality score |
| `canonical_id` | INTEGER | FK for merged synonyms |
| `keyword_type` | TEXT | 'person', 'organization', 'location', 'concept', 'event', 'product', 'unknown' |
| `created_at` | TEXT | Creation timestamp |
| `updated_at` | TEXT | Last update timestamp |

### `immanentize_sephiroth` - Keyword-Category Association

Links keywords to categories based on article associations.

| Column | Type | Description |
|--------|------|-------------|
| `immanentize_id` | INTEGER | FK to immanentize |
| `sephiroth_id` | INTEGER | FK to sephiroth |
| `association_count` | INTEGER | Number of associations |
| `updated_at` | TEXT | Last update timestamp |

### `immanentize_neighbors` - Keyword Co-occurrence Network

Tracks semantic relationships between keywords.

| Column | Type | Description |
|--------|------|-------------|
| `immanentize_id_a` | INTEGER | FK to immanentize |
| `immanentize_id_b` | INTEGER | FK to immanentize |
| `cooccurrence` | INTEGER | Co-occurrence count |
| `embedding_similarity` | REAL | Cosine similarity of embeddings |
| `updated_at` | TEXT | Last update timestamp |

### `immanentize_clusters` - Topic Clusters

Groups related keywords into thematic clusters.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `name` | TEXT | Cluster name |
| `centroid` | BLOB | Cluster centroid embedding |
| `created_at` | TEXT | Creation timestamp |

### `immanentize_daily` - Daily Keyword Counts

Tracks daily keyword frequency for trend analysis.

| Column | Type | Description |
|--------|------|-------------|
| `immanentize_id` | INTEGER | FK to immanentize |
| `date` | TEXT | Date (YYYY-MM-DD) |
| `count` | INTEGER | Occurrences on that date |

### `fnord_immanentize` - Article-Keyword Mapping

Links articles to their extracted keywords with provenance.

| Column | Type | Description |
|--------|------|-------------|
| `fnord_id` | INTEGER | FK to fnords |
| `immanentize_id` | INTEGER | FK to immanentize |
| `source` | TEXT | 'ai', 'statistical', or 'manual' |
| `confidence` | REAL | 0.0-1.0 confidence score |
| `assigned_at` | TEXT | Assignment timestamp |

### `dismissed_synonyms` - Ignored Synonym Suggestions

Stores user-dismissed synonym merge suggestions.

| Column | Type | Description |
|--------|------|-------------|
| `keyword_a_id` | INTEGER | FK to immanentize |
| `keyword_b_id` | INTEGER | FK to immanentize |
| `dismissed_at` | TEXT | Dismissal timestamp |

---

## Embeddings & Vector Search

### Embedding-Modell Details

**Standard:** `snowflake-arctic-embed2` (1024-dim, multilingual)

| Eigenschaft | Wert |
|-------------|------|
| Modell | snowflake-arctic-embed2 |
| Dimensionen | 1024 |
| Sprachen | 74 (inkl. Deutsch, Englisch) |
| Größe | 1.2 GB |
| VRAM | ~2-3 GB |
| Kontext | 8192 Tokens |

**Vorteile der Modellwahl:**
- Explizite deutsche Sprachunterstützung (CLEF-Benchmarks)
- Bessere Performance bei kurzen Texten/Keywords
- Matryoshka Representation Learning (MRL) für Kompression
- Apache 2.0 Lizenz

**Bei Modellwechsel:** Alle Keywords müssen neu eingebettet werden (Settings -> Wartung -> Embeddings generieren), da unterschiedliche Dimensionen nicht kompatibel sind.

**Alternative:** `bge-m3` (100+ Sprachen, ebenfalls 1024-dim)

---

### `vec_immanentize` - Keyword Vector Index

SQLite-vec virtual table for O(log n) KNN search on keyword embeddings.

| Column | Type | Description |
|--------|------|-------------|
| `rowid` | INTEGER | Matches immanentize.id |
| `embedding` | FLOAT[1024] | Vector data |

**Index type:** Cosine distance, IVF index

### `vec_fnords` - Article Vector Index

SQLite-vec virtual table for article similarity search.

| Column | Type | Description |
|--------|------|-------------|
| `rowid` | INTEGER | Matches fnords.id |
| `embedding` | FLOAT[1024] | Vector data |

**Index type:** Cosine distance, IVF index

### `embedding_queue` - Pending Embedding Jobs

Queue for background embedding generation.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `immanentize_id` | INTEGER | FK to immanentize |
| `priority` | INTEGER | Processing priority |
| `created_at` | TEXT | Queue timestamp |

---

## Keyword Type Detection

### `keyword_type_prototype` - Type Prototype Embeddings

Stores prototype embeddings for keyword type classification.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `keyword_type` | TEXT | Type name |
| `embedding` | BLOB | Prototype embedding |
| `example_keywords` | TEXT | JSON array of examples |
| `created_at` | TEXT | Creation timestamp |

---

## User Tags

### `tags` - Tag Definitions

User-defined tags for organizing articles.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `name` | TEXT | Tag name (unique) |
| `color` | TEXT | Hex color code |
| `created_at` | TEXT | Creation timestamp |

### `fnord_tags` - Article-Tag Mapping

Links articles to user-defined tags.

| Column | Type | Description |
|--------|------|-------------|
| `fnord_id` | INTEGER | FK to fnords |
| `tag_id` | INTEGER | FK to tags |
| `created_at` | TEXT | Assignment timestamp |

---

## Stopwords

### `stopwords` - Stopword Management

Words excluded from keyword extraction.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `word` | TEXT | Stopword (unique, lowercase) |
| `is_system` | INTEGER | TRUE if system-defined |
| `created_at` | TEXT | Creation timestamp |

---

## Recommendations (Operation Mindfuck)

### `recommendation_feedback` - User Feedback

Tracks user interactions with recommendations for personalization.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `fnord_id` | INTEGER | FK to fnords |
| `action` | TEXT | 'save', 'hide', or 'click' |
| `created_at` | TEXT | Action timestamp |

---

## Statistical Analysis & Learning System

### `corpus_stats` - Document Frequencies

Stores corpus-wide statistics for TF-IDF calculation.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `term` | TEXT | Term (unique) |
| `document_frequency` | INTEGER | Number of documents containing term |
| `updated_at` | TEXT | Last update timestamp |

### `bias_weights` - Learning Weights

Adaptive weights learned from LLM rejections and user corrections.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `weight_type` | TEXT | 'keyword_boost', 'category_term', 'category_boost' |
| `key` | TEXT | Keyword or category name |
| `weight` | REAL | Weight value (0.1-3.0) |
| `correction_count` | INTEGER | Number of corrections |
| `updated_at` | TEXT | Last update timestamp |

**Weight types:**
- `keyword_boost`: Adjusts keyword extraction confidence
- `category_term`: Adjusts individual term weights for category matching
- `category_boost`: Adjusts overall category confidence

---

## AI Cost Tracking

### `ai_cost_log` - API Cost Log

Tracks token usage and estimated costs for OpenAI-compatible API providers. Used for monthly cost limit enforcement.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key (autoincrement) |
| `provider` | TEXT | API provider name (e.g. 'openai') |
| `model` | TEXT | Model used for the request |
| `input_tokens` | INTEGER | Number of input tokens consumed |
| `output_tokens` | INTEGER | Number of output tokens consumed |
| `estimated_cost_usd` | REAL | Estimated cost in USD |
| `created_at` | TEXT | Timestamp (default: current datetime) |

**Indizes:**
- `idx_ai_cost_log_created` auf `created_at DESC`
- `idx_ai_cost_log_provider` auf `provider`

---

## Briefings (KI-generierte Nachrichten-Zusammenfassungen)

### `briefings` - AI-generated Briefings

Stores AI-generated news briefings (daily/weekly) that summarize the most important articles and trending keywords of a given period.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key (autoincrement) |
| `period_type` | TEXT | `'daily'` or `'weekly'` (CHECK constraint) |
| `period_start` | DATETIME | Start of the summarized period |
| `period_end` | DATETIME | End of the summarized period |
| `content` | TEXT | AI-generated briefing text (structured JSON or legacy markdown) |
| `top_keywords` | TEXT | Comma-separated trending keywords for the period |
| `article_count` | INTEGER | Number of articles used for the briefing |
| `article_refs` | TEXT | JSON-Array mit Artikel-Referenzen fuer Frontend-Navigation (index, fnord_id, title, source) |
| `model_used` | TEXT | AI model used for generation |
| `created_at` | DATETIME | Creation timestamp (default: current datetime) |

**Constraints:**
- `UNIQUE(period_type, period_start)` - Nur ein Briefing pro Typ und Zeitraum

**Indizes:**
- `idx_briefings_created` auf `created_at DESC`
- `idx_briefings_period` auf `(period_type, period_start DESC)`

**Generierung:**
- Bis zu 15 aktuelle Artikel mit Zusammenfassung als Input
- Trending Keywords aus `immanentize_daily` als zusätzlicher Kontext
- Prompt erzeugt strukturiertes Briefing (Überblick, Top-5 Themen, Trends)

---

## Story Clusters (Themenbezogene Artikel-Gruppen)

### `story_clusters` - Topic Clusters

Groups related articles by topic for perspective comparison. Clusters are discovered automatically by analyzing article embedding similarities using a Union-Find algorithm.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key (autoincrement) |
| `title` | TEXT | Cluster title (generated from common keywords) |
| `summary` | TEXT | Optional cluster summary |
| `perspective_comparison` | TEXT | AI-generated perspective comparison text |
| `article_count` | INTEGER | Number of articles in the cluster |
| `created_at` | DATETIME | Creation timestamp (default: current datetime) |
| `updated_at` | DATETIME | Last update timestamp (default: current datetime) |

### `story_cluster_articles` - Cluster-Article Mapping

Links articles to their story cluster with similarity scores.

| Column | Type | Description |
|--------|------|-------------|
| `cluster_id` | INTEGER | FK to story_clusters (ON DELETE CASCADE) |
| `fnord_id` | INTEGER | FK to fnords (ON DELETE CASCADE) |
| `similarity_score` | REAL | Cosine similarity score to cluster (0.0-1.0) |

**Primary Key:** `(cluster_id, fnord_id)`

**Indizes:**
- `idx_sca_fnord` auf `fnord_id`

**Clustering-Algorithmus:**
- Artikel mit Embeddings der letzten N Tage werden geladen
- Ähnlichkeitspaare via `vec_fnords` KNN-Suche (k=50) identifiziert
- Cosine-Similarity-Threshold: **0.78** (nur Paare darüber werden verbunden)
- Union-Find-Algorithmus gruppiert transitiv verbundene Artikel
- Cluster müssen mindestens 3 Artikel und 2 verschiedene Quellen haben
- Cluster älter als 30 Tage werden automatisch gelöscht

---

## Named Entity Recognition (NER)

### `entities` - Named Entities

Stores named entities (persons, organizations, locations, events) extracted from articles via LLM-based NER.

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key (autoincrement) |
| `name` | TEXT | Entity name (original form) |
| `entity_type` | TEXT | `'person'`, `'organization'`, `'location'`, or `'event'` (CHECK constraint) |
| `normalized_name` | TEXT | Normalized name for deduplication (lowercase, titles removed) |
| `article_count` | INTEGER | Number of articles mentioning this entity |
| `first_seen` | DATETIME | First occurrence timestamp |
| `last_seen` | DATETIME | Last occurrence timestamp |

**Constraints:**
- `UNIQUE(normalized_name, entity_type)` - Deduplizierung über normalisierte Form + Typ

**Indizes:**
- `idx_entities_type` auf `entity_type`
- `idx_entities_normalized` auf `normalized_name`

**Normalisierung:**
- Kleinschreibung, Trimming
- Entfernung gängiger Titel/Prefixe (Dr., Prof., Mr., Herr, etc.)
- Zusammenfassung mehrerer Leerzeichen

### `fnord_entities` - Article-Entity Mapping

Links articles to their extracted named entities with mention counts.

| Column | Type | Description |
|--------|------|-------------|
| `fnord_id` | INTEGER | FK to fnords (ON DELETE CASCADE) |
| `entity_id` | INTEGER | FK to entities (ON DELETE CASCADE) |
| `mention_count` | INTEGER | How often the entity is mentioned in the article |
| `confidence` | REAL | Extraction confidence (default: 0.8) |

**Primary Key:** `(fnord_id, entity_id)`

**Indizes:**
- `idx_fnord_entities_entity` auf `entity_id`

---

## Settings & Configuration

### `settings` - Application Settings

Key-value store for application settings. Einstellungen werden in der SQLite-Datenbank gespeichert, nicht in einer externen config.toml.

| Column | Type | Description |
|--------|------|-------------|
| `key` | TEXT | Setting key (PRIMARY KEY) |
| `value` | TEXT | Setting value (als String) |

**Implementierte Settings:**

| Key | Typ | Default | Beschreibung |
|-----|-----|---------|--------------|
| `locale` | String | `'de'` | Sprache: `de`, `en` |
| `theme` | String | `'mocha'` | Theme: `mocha`, `macchiato`, `frappe`, `latte` |
| `showTerminologyTooltips` | Boolean | `'true'` | Illuminatus!-Tooltips anzeigen |
| `ai_text_provider` | String | `'ollama'` | Active text generation provider (`ollama` or `openai`) |
| `ollama_url` | String | `'http://localhost:11434'` | Ollama server URL (local or remote) |
| `openai_base_url` | String | `'https://api.openai.com'` | OpenAI-compatible API base URL |
| `openai_api_key` | String | `''` | API key for OpenAI-compatible provider |
| `openai_model` | String | `'gpt-4.1-nano'` | Model for text generation |
| `cost_limit_monthly` | Float | `'5.0'` | Monthly cost limit in USD (enforced via `ai_cost_log`) |

**Geplante Settings (spätere Phasen):**

| Key | Typ | Beschreibung |
|-----|-----|--------------|
| `syncInterval` | Integer | Sync-Intervall in Minuten |
| `syncOnStart` | Boolean | Sync bei App-Start |
| `embeddingModel` | String | Modell für Embeddings |

**Persistenz-Mechanismus:**
- Frontend liest/schreibt via `get_setting`/`set_setting` Tauri Commands
- Werte werden als Strings gespeichert (JSON-Serialisierung bei komplexen Typen)
- Änderungen werden sofort in die Datenbank geschrieben

### `hardware_profiles` - Performance Profiles

Hardware configurations for batch processing.

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT | Profile ID (unique) |
| `name` | TEXT | Profile name |
| `description` | TEXT | Profile description |

**Note:** The `ai_parallelism` column is deprecated and no longer used. Articles are processed sequentially.

---

## Entity Relationship Overview

```
pentacles (Feeds)
    |
    +-- fnords (Articles)
            |
            +-- fnord_revisions (Version History)
            +-- fnord_sephiroth (Category Mappings)
            +-- fnord_immanentize (Keyword Mappings)
            +-- fnord_tags (User Tags)
            +-- fnord_entities (Entity Mappings)
            +-- recommendation_feedback (User Feedback)
            +-- story_cluster_articles (Cluster Membership)
            +-- vec_fnords (Vector Index)

sephiroth (Categories)
    |
    +-- fnord_sephiroth
    +-- immanentize_sephiroth

immanentize (Keywords)
    |
    +-- fnord_immanentize
    +-- immanentize_sephiroth
    +-- immanentize_neighbors (Self-referential)
    +-- immanentize_daily (Trends)
    +-- dismissed_synonyms
    +-- vec_immanentize (Vector Index)
    +-- embedding_queue

entities (Named Entities)
    |
    +-- fnord_entities

story_clusters (Topic Clusters)
    |
    +-- story_cluster_articles

tags
    |
    +-- fnord_tags

briefings (Standalone)
stopwords (Standalone)
corpus_stats (Standalone)
bias_weights (Standalone)
settings (Standalone)
hardware_profiles (Standalone)
keyword_type_prototype (Standalone)
ai_cost_log (Standalone)
```

---

## Source Types

The `source` field in mapping tables indicates provenance:

| Source | Description | Typical Confidence |
|--------|-------------|-------------------|
| `ai` | Generated/validated by LLM | 0.7-1.0 |
| `statistical` | Extracted by TF-IDF/frequency analysis | 0.5-0.9 |
| `manual` | Added by user | 1.0 |

---

## Notes

1. **Embedding dimensions:** All embeddings use 1024 dimensions (snowflake-arctic-embed2, siehe "Embedding-Modell Details")
2. **Vector search:** Uses sqlite-vec extension with cosine distance, O(log n) KNN
3. **Timestamps:** All timestamps are ISO 8601 format (TEXT)
4. **WAL mode:** Database uses Write-Ahead Logging for concurrent access
5. **Settings persistence:** Alle Einstellungen werden in der `settings`-Tabelle gespeichert (Key-Value-Store)
6. **Revisionsverwaltung:** Artikeländerungen werden vollständig in `fnord_revisions` protokolliert
7. **AI cost tracking:** Token usage and estimated costs for OpenAI-compatible providers are logged in `ai_cost_log`, with monthly limits enforced via `cost_limit_monthly` setting
8. **Article type:** Artikel werden durch LLM-Analyse als `news`, `analysis`, `opinion`, `satire`, `ad` oder `unknown` klassifiziert (Migration 26)
9. **Briefings:** KI-generierte Nachrichten-Zusammenfassungen (täglich/wöchentlich) werden in `briefings` gespeichert (Migration 27)
10. **Story Clusters:** Thematisch verwandte Artikel werden via Embedding-Ähnlichkeit (>0.78) und Union-Find zu Clustern gruppiert (Migration 28)
11. **Named Entities:** Personen, Organisationen, Orte und Events werden via LLM-basierter NER extrahiert und in `entities`/`fnord_entities` gespeichert (Migration 29)
