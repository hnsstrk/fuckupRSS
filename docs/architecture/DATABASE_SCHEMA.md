# Database Schema Reference

This document provides a comprehensive overview of the fuckupRSS SQLite database schema.

**Related documentation:**
- Database patterns and best practices: See [CLAUDE.md](../../CLAUDE.md#database-patterns--best-practices)
- Full schema implementation: `src-tauri/src/db/schema.rs`
- Technical specification: `fuckupRSS-Anforderungen.md` Kapitel 6b + 10

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
| `embedding` | BLOB | 1024-dim article embedding |
| `processed_at` | TEXT | AI processing timestamp |
| `created_at` | TEXT | Creation timestamp |

### `fnord_revisions` - Article Version History

Tracks changes to articles over time (for "Fnord" detection - seeing what others don't see).

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key |
| `fnord_id` | INTEGER | FK to fnords |
| `content_hash` | TEXT | Content hash at revision |
| `content_raw` | TEXT | Content at revision |
| `title` | TEXT | Title at revision |
| `detected_at` | TEXT | When change was detected |

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

## Settings & Configuration

### `settings` - Application Settings

Key-value store for application settings.

| Column | Type | Description |
|--------|------|-------------|
| `key` | TEXT | Setting key (unique) |
| `value` | TEXT | Setting value (JSON) |
| `updated_at` | TEXT | Last update timestamp |

### `hardware_profiles` - Performance Profiles

Predefined hardware configurations for batch processing.

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT | Profile ID (unique) |
| `name` | TEXT | Profile name |
| `description` | TEXT | Profile description |
| `ai_parallelism` | INTEGER | Parallel processing count |

**Predefined profiles:**
| Profile | ai_parallelism | Use Case |
|---------|----------------|----------|
| Standard | 1 | Safe for all systems |
| Moderat | 4 | Good compromise |
| Hohe Leistung | 8 | High-end hardware |

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
            +-- recommendation_feedback (User Feedback)
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

tags
    |
    +-- fnord_tags

stopwords (Standalone)
corpus_stats (Standalone)
bias_weights (Standalone)
settings (Standalone)
hardware_profiles (Standalone)
keyword_type_prototype (Standalone)
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

1. **Embedding dimensions:** All embeddings use 1024 dimensions (snowflake-arctic-embed2)
2. **Vector search:** Uses sqlite-vec extension with cosine distance
3. **Timestamps:** All timestamps are ISO 8601 format (TEXT)
4. **WAL mode:** Database uses Write-Ahead Logging for concurrent access
