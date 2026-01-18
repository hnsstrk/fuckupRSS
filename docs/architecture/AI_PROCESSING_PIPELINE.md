# AI Processing Pipeline Reference

> This document provides a comprehensive reference for the AI processing pipeline in fuckupRSS.
> For the main developer guide, see [CLAUDE.md](../../CLAUDE.md).

## Table of Contents

1. [Pipeline Overview](#pipeline-overview)
2. [Content Fields](#content-fields-in-fnords)
3. [Statistical Text Analysis](#statistical-text-analysis)
4. [Bias Learning System](#bias-learning-system)
5. [Advanced Keyword Extraction](#advanced-keyword-extraction)
6. [Article Clustering](#article-clustering-batch-optimization)
7. [Relevant Modules](#relevant-modules)

---

## Pipeline Overview

The AI processing pipeline consists of 5 sequential stages that transform raw RSS content into analyzed, categorized, and searchable articles:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         AI PROCESSING PIPELINE                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. HAGBARD'S RETRIEVAL                                                     │
│     └─ Fetch full text for ALL new articles (automatic after sync)          │
│                                                                             │
│  2. DISCORDIAN ANALYSIS                                                     │
│     └─ Summarize, categorize, extract keywords via ministral                │
│                                                                             │
│  3. ARTICLE EMBEDDING                                                       │
│     └─ Generate embedding for similarity search                             │
│                                                                             │
│  4. GREYFACE ALERT                                                          │
│     └─ Bias detection (political_bias: -2 to +2, sachlichkeit: 0-4)        │
│                                                                             │
│  5. IMMANENTIZE NETWORK                                                     │
│     └─ Keyword graph processing                                             │
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

AI-powered summarization, categorization, and keyword extraction.

| Aspect | Details |
|--------|---------|
| **Model** | ministral-3:latest |
| **Input** | `content_full` ONLY (no fallback) |
| **Output** | Summary, categories, keywords, bias scores |
| **Mode** | Native JSON mode for stability |

- **Uses ONLY `content_full`** - no fallback to `content_raw`
- Articles without full text are not suggested for analysis
- Individual articles can be re-analyzed anytime (button in ArticleView)
- "Re-analyze all" available in Settings with progress display

### Stage 3: Article Embedding

Vector embedding generation for semantic similarity search.

| Aspect | Details |
|--------|---------|
| **Model** | snowflake-arctic-embed2 |
| **Dimensions** | 1024 |
| **Input** | Title + first 500 characters of content |
| **Storage** | `fnords.embedding` + `vec_fnords` virtual table |
| **Index** | sqlite-vec with O(log n) KNN |

- Automatic after successful Discordian Analysis
- Enables similar article discovery and semantic search

### Stage 4: Greyface Alert

Political bias and objectivity detection.

| Score | Range | Description |
|-------|-------|-------------|
| `political_bias` | -2 to +2 | Left (-2) to Right (+2) political leaning |
| `sachlichkeit` | 0 to 4 | Objectivity score (0 = opinion, 4 = factual) |

### Stage 5: Immanentize Network

Keyword graph processing and semantic network building.

Processing steps:
1. **New Keywords**: Generate embedding via snowflake-arctic-embed2
2. **Category Association**: Update `immanentize_sephiroth` table
3. **Neighbor Update**: Calculate co-occurrence + embedding similarity
4. **Synonym Detection**: Flag pairs with `embedding_similarity > 0.92`

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

## Statistical Text Analysis

### Statistical-First Workflow

Statistical analysis runs **BEFORE** LLM analysis. The LLM validates/corrects statistical suggestions:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    STATISTICAL-FIRST WORKFLOW                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. STATISTICAL PRE-ANALYSIS (TF-IDF + Category Matcher)                    │
│     ├─ Apply bias weights                                                   │
│     └─ Generate keyword_candidates, category_scores                         │
│                                                                             │
│  2. LLM QUALITY CONTROL (Discordian Analysis)                               │
│     ├─ Receives statistical results as context                              │
│     ├─ Validates/rejects suggestions                                        │
│     └─ Returns rejected_keywords, rejected_categories                       │
│                                                                             │
│  3. BIAS LEARNING FROM REJECTIONS                                           │
│     ├─ Rejected keywords: boost -= 0.1                                      │
│     └─ Rejected categories: term_weight -= 0.1 for matching_terms           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Analysis Methods

| Analysis | Method | Output |
|----------|--------|--------|
| Keyword Extraction | TF-IDF + Corpus-Stats | `keyword_candidates` with score |
| Category Matching | Word frequency + word lists | `category_scores` with `matching_terms` |
| LLM Validation | ministral-3 | `rejected_keywords`, `rejected_categories` |

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
| `src-tauri/src/ollama/mod.rs` | Ollama API integration |
| `src-tauri/src/ollama/mod.rs` | `discordian_analysis_with_stats` function |
| `src-tauri/src/retrieval/mod.rs` | Hagbard's Retrieval (full-text fetching) |

### Database Operations

| Module | Purpose |
|--------|---------|
| `src-tauri/src/immanentize.rs` | Keyword network operations |
| `src-tauri/src/commands/batch_processor.rs` | Batch processing |
| `src-tauri/src/commands/article_analysis.rs` | Statistical article analysis |

---

## Related Documentation

- [CLAUDE.md](../../CLAUDE.md) - Main developer guide
- [fuckupRSS-Anforderungen.md](../../fuckupRSS-Anforderungen.md) - Technical specification
- [KEYWORDS_SCHEMA.md](../features/immanentize/KEYWORDS_SCHEMA.md) - Keyword database schema
- [STOPWORD_KEYWORD_REPORT.md](../archive/STOPWORD_KEYWORD_REPORT.md) - Stopword and keyword analysis report
