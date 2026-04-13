# Processing Pipeline

How fuckupRSS processes articles — from RSS feed to AI-analyzed content.

---

## Overview

```text
RSS Feed
  |
  v
1. Sync & Fetch -----> Raw article (title, content_raw, metadata)
  |
  v
2. Full-Text Retrieval (Hagbard's Retrieval) -----> content_full
  |
  v
3. AI Analysis (Discordian Analysis) -----> summary, categories, keywords, bias, article_type
  |
  v
4. Embedding Generation -----> vector for similarity search
  |
  v
5. Keyword Network (Immanentize) -----> co-occurrence graph, synonyms, clusters
  |
  v
6. Post-Processing -----> Theme Reports, Briefings, NER, Recommendations
```

---

## AI Providers

fuckupRSS supports two AI backends. Ollama is the default and the only provider that supports embeddings.

| Provider | Use Case | Embedding Support |
|----------|----------|-------------------|
| **Ollama** (default) | Local inference, fully offline | Yes |
| **OpenAI-compatible** | OpenAI, Together.ai, Mistral, Groq, etc. | No (falls back to Ollama) |

**Default models:** `ministral-3:latest` (text), `snowflake-arctic-embed2:latest` (embeddings, 1024 dimensions)

**Task types:** The system distinguishes between `Fast` tasks (article analysis, NER) and `Reasoning` tasks (briefings, theme reports) which use a separate reasoning model with larger context windows.

---

## Pipeline Stages

### 1. Feed Sync

Standard RSS/Atom parsing. New articles are stored with `content_raw` (the RSS snippet). Feeds are synced on a configurable interval.

### 2. Full-Text Retrieval (Hagbard's Retrieval)

For each new article, the full web page is fetched and cleaned:

1. HTTP GET the article URL
2. Readability extraction (strips ads, navigation, scripts)
3. HTML sanitization (XSS protection)
4. Result stored as `content_full`

A headless browser fallback is available for JavaScript-heavy pages.

### 3. AI Analysis (Discordian Analysis)

The core analysis step. Combines statistical text analysis with LLM-based understanding.

**Statistical pre-analysis** (runs before the LLM call):
- TF-IDF keyword extraction with corpus statistics
- Category scoring against the existing category tree
- Local keyword extraction (YAKE, RAKE, n-grams, TextRank)

**LLM analysis** (Structured Outputs via JSON Schema):
- **Summary** — 2-3 sentences in the user's language (regardless of article language)
- **Categories** — matched against existing category hierarchy
- **Keywords** — merged with statistical keywords
- **Political bias** — left/right spectrum rating
- **Objectivity** (Sachlichkeit) — emotional vs. fact-based rating
- **Article type** — news, analysis, opinion, satire, ad, or unknown
- **Rejection learning** — the LLM can reject statistically suggested keywords/categories, which feeds back into the scoring system

Results are cached by content hash to avoid re-analyzing identical articles.

### 4. Embedding Generation

After analysis, a vector embedding is generated for similarity search:

- **Input:** title + summary + content_full (up to 4000 characters)
- **Model:** snowflake-arctic-embed2 (8192 token context, 1024 dimensions)
- **Storage:** SQLite + sqlite-vec virtual table for fast vector search

The LLM model is explicitly unloaded before embedding generation to free VRAM.

### 5. Keyword Network (Immanentize)

Keywords are organized into a co-occurrence network:

- **Co-occurrence tracking** — keywords appearing in the same article strengthen their connection
- **Category associations** — keywords are linked to categories they frequently appear with
- **Synonym detection** — similar keywords are identified via embedding similarity
- **Type classification** — keywords are typed as concept, person, organization, location, or acronym

### 6. Post-Processing

These features run on top of the analyzed articles:

**Theme Reports** — Multi-signal topic detection that groups related articles:
- 5 weighted signals: embedding similarity (35%), keyword overlap (25%), entity overlap (20%), category match (10%), temporal proximity (10%)
- Agglomerative clustering with dynamic cut
- LLM generates narrative analysis for each detected theme

**Daily Briefings** — AI-generated overview of the most relevant articles:
- Hybrid scoring: trending keywords, cluster membership, article quality, category priority
- Diversity filtering: max 3 articles per source, minimum 3 categories
- LLM produces structured output with TL;DR and topic summaries

**Named Entity Recognition (NER)** — Extracts persons, organizations, locations, and events from articles via LLM.

**Recommendations** — Personalized article suggestions based on reading behavior, embedding similarity, and keyword overlap.

---

## Batch Processing

Articles are processed in batches with configurable concurrency:
- **Ollama:** sequential by default (configurable via `ollama_concurrency`)
- **OpenAI-compatible:** up to 20 parallel requests (configurable via `openai_concurrency`)
- **Retry logic:** failed articles get progressively larger context windows, marked as hopeless after 5 attempts
- **Progress events:** the frontend receives `batch-complete` events for live updates
