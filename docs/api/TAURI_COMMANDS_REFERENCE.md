# Tauri Commands API Reference

This document provides a comprehensive reference for all Tauri commands available in fuckupRSS. These commands define the IPC (Inter-Process Communication) interface between the Svelte 5 frontend and the Rust backend.

> **Note:** For development guidelines, database patterns, and project overview, see [CLAUDE.md](../../CLAUDE.md).

---

## Table of Contents

- [Pentacles (Feeds)](#pentacles-feeds)
- [Fnords (Articles)](#fnords-articles)
- [Sync](#sync)
- [Retrieval (Full-text)](#retrieval-full-text)
- [AI Provider](#ai-provider)
- [Settings](#settings)
- [Immanentize (Keyword Quality & Synonyms)](#immanentize-keyword-quality--synonyms)
- [Hardware Profiles](#hardware-profiles)
- [Similar Articles & Semantic Search](#similar-articles--semantic-search)
- [Article Analysis (Statistical Keywords/Categories)](#article-analysis-statistical-keywordscategories)
- [Stopwords Management](#stopwords-management)
- [Article Tags](#article-tags)
- [Operation Mindfuck (Recommendations)](#operation-mindfuck-recommendations)
- [Keyword Network (Extended)](#keyword-network-extended)
- [Keyword Type Detection](#keyword-type-detection)
- [Fnord Statistics](#fnord-statistics)
- [Categories (Extended)](#categories-extended)
- [Embedding Management](#embedding-management)
- [Model Management (Extended)](#model-management-extended)
- [Cost Tracking](#cost-tracking)
- [Batch Processing](#batch-processing)
- [OPML Import/Export](#opml-importexport)
- [Settings (Extended)](#settings-extended)
- [Prompts (Extended)](#prompts-extended)
- [Data Structures](#data-structures)

---

## Pentacles (Feeds)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_pentacles` | - | `Vec<Pentacle>` | Get all feeds with article counts |
| `add_pentacle` | `url`, `title?` | `Pentacle` | Add a new feed |
| `delete_pentacle` | `id` | - | Delete a feed |

---

## Fnords (Articles)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_fnords` | `filter?` | `Vec<Fnord>` | Get articles with optional filter |
| `get_fnord` | `id` | `Fnord` | Get a single article |
| `update_fnord_status` | `id`, `status` | - | Update article status |
| `get_changed_fnords` | - | `Vec<Fnord>` | Get articles with changes |
| `acknowledge_changes` | `id` | - | Acknowledge article changes |
| `get_fnord_revisions` | `fnord_id` | `Vec<FnordRevision>` | Get revision history |

---

## Sync

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `sync_all_feeds` | - | `SyncResponse` | Synchronize all feeds |
| `sync_feed` | `pentacle_id` | `SyncResultResponse` | Synchronize a single feed |

---

## Retrieval (Full-text)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `fetch_full_content` | `fnord_id` | `RetrievalResponse` | Fetch full article content |
| `fetch_truncated_articles` | `pentacle_id?`, `limit?` | `Vec<RetrievalResponse>` | Fetch truncated articles |

---

## AI Provider

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `check_ollama` | - | `OllamaStatus` | Check Ollama availability |
| `test_ai_provider` | `provider_type`, `base_url`, `api_key?` | `ProviderTestResult` | Test AI provider connection |
| `generate_summary` | `fnord_id`, `model` | `SummaryResponse` | Generate article summary |
| `analyze_article` | `fnord_id`, `model` | `AnalysisResponse` | Perform bias analysis |
| `process_article` | `fnord_id`, `model` | `(Summary, Analysis)` | Combined summary and analysis |
| `get_unprocessed_count` | - | `UnprocessedCount` | Count unprocessed articles |
| `process_batch` | `model`, `limit?` | `BatchResult` | Batch processing |
| `pull_model` | `model` | `ModelPullResult` | Download a model |
| `get_prompts` | - | `PromptTemplates` | Get current prompts |
| `set_prompts` | `summary_prompt`, `analysis_prompt` | - | Save prompts |
| `reset_prompts` | - | `PromptTemplates` | Reset prompts to defaults |
| `get_default_prompts` | - | `DefaultPrompts` | Get default prompts |

### ProviderTestResult Structure

```typescript
interface ProviderTestResult {
  success: boolean;         // Whether the connection succeeded
  latency_ms: number;       // Response time in milliseconds
  models: string[];         // Available models from the provider
  error: string | null;     // Error message if connection failed
}
```

**Notes:**
- `provider_type` accepts `"ollama"` or `"openai_compatible"`
- For Ollama, tests by listing available models
- For OpenAI-compatible APIs, tests the `/v1/models` endpoint
- Returns `success: false` with error details on authentication failure (HTTP 401)

---

## Settings

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_settings` | - | `Settings` | Get all settings |
| `set_setting` | `key`, `value` | - | Save a setting |
| `get_setting` | `key` | `Option<String>` | Load a setting |

---

## Immanentize (Keyword Quality & Synonyms)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `calculate_keyword_quality_scores` | `limit?` | `QualityScoreResult` | Calculate quality scores |
| `get_low_quality_keywords` | `threshold`, `limit` | `Vec<LowQualityKeyword>` | Get low-quality keywords |
| `auto_prune_low_quality` | `quality_threshold`, `min_age_days`, `dry_run` | `PruneResult` | Prune low-quality keywords |
| `queue_missing_embeddings` | - | `i64` | Queue missing embeddings |
| `find_synonym_candidates` | `threshold?`, `limit?` | `Vec<SynonymCandidate>` | Find synonym candidates |
| `merge_keyword_pair` | `keep_id`, `remove_id` | `MergeSynonymsResult` | Merge keywords |
| `dismiss_synonym_pair` | `keyword_a_id`, `keyword_b_id` | - | Dismiss synonym suggestion |
| `split_compound_keywords` | `dry_run?` | `CompoundSplitResult` | Split compound keywords (e.g. "Ukraine-Krieg" → "Ukraine" + "Krieg") |
| `preview_compound_splits` | - | `Vec<CompoundSplitDetail>` | Preview which compounds would be split |

### CompoundSplitResult Structure

```typescript
interface CompoundSplitResult {
  compounds_found: number;      // Total hyphenated keywords found
  compounds_split: number;      // Keywords actually split
  components_created: number;   // New component keywords created
  articles_transferred: number; // Article associations transferred
  split_details: CompoundSplitDetail[];
}

interface CompoundSplitDetail {
  original: string;        // e.g. "Ukraine-Krieg"
  components: string[];    // e.g. ["Ukraine", "Krieg"]
  articles_affected: number;
}
```

**Notes:**
- Keywords in the NO_SPLIT list (Bundesländer, sports terms, proper nouns) are preserved
- The original compound keyword is deleted after splitting
- Article associations are transferred to all component keywords

---

## Hardware Profiles

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_hardware_profiles` | - | `Vec<HardwareProfile>` | Get available profiles |
| `save_hardware_profile` | `profile` | - | Create/update profile |
| `delete_hardware_profile` | `id` | - | Delete profile |
| `apply_hardware_profile` | `profile_id` | - | Activate profile |

### HardwareProfile Structure

```rust
struct HardwareProfile {
    id: String,
    name: String,
    description: String,
}
```

**Note:** Articles are processed sequentially (one at a time). Hardware profiles are retained for future use but no longer control parallelism.

---

## Similar Articles & Semantic Search

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `find_similar_articles` | `fnord_id`, `limit?` | `SimilarArticlesResponse` | Find similar articles |
| `get_article_embedding_stats` | - | `ArticleEmbeddingCount` | Embedding statistics |
| `generate_article_embeddings_batch` | `limit?` | `ArticleEmbeddingBatchResult` | Batch embedding generation |
| `semantic_search` | `query`, `limit?` | `SemanticSearchResponse` | Semantic search (threshold >= 0.3) |

### SimilarArticle Structure

```rust
struct SimilarArticle {
    fnord_id: i64,
    title: String,
    pentacle_title: Option<String>,
    published_at: Option<String>,
    similarity: f64,  // 0.0-1.0, Threshold >= 0.5
}
```

---

## Article Analysis (Statistical Keywords/Categories)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_article_keywords` | `fnord_id` | `Vec<ArticleKeyword>` | Get keywords with source/confidence |
| `add_article_keyword` | `fnord_id`, `keyword` | `ArticleKeyword` | Manually add keyword |
| `remove_article_keyword` | `fnord_id`, `keyword_id` | - | Remove keyword |
| `get_article_categories_detailed` | `fnord_id` | `Vec<ArticleCategory>` | Get categories with source/confidence |
| `update_article_categories` | `fnord_id`, `categories` | - | Set categories |
| `add_article_category` | `fnord_id`, `sephiroth_id` | - | Add category |
| `remove_article_category` | `fnord_id`, `sephiroth_id` | - | Remove category |
| `analyze_article_statistical` | `fnord_id` | `StatisticalAnalysis` | Statistical analysis only |
| `record_correction` | `correction` | - | Record correction for bias learning |
| `get_bias_stats` | - | `BiasStats` | Get bias statistics |

### ArticleKeyword Structure

```rust
struct ArticleKeyword {
    id: i64,
    name: String,
    source: String,      // 'ai', 'statistical', 'manual'
    confidence: f64,     // 0.0-1.0
}
```

---

## Stopwords Management

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_user_stopwords` | - | `Vec<UserStopword>` | Get user stopwords |
| `get_system_stopwords` | - | `Vec<UserStopword>` | Get system stopwords |
| `get_all_stopwords_list` | - | `Vec<UserStopword>` | Get all stopwords |
| `add_stopword` | `word` | `bool` | Add stopword |
| `add_stopwords_batch` | `words` | `i64` | Add multiple stopwords |
| `remove_stopword` | `word` | `bool` | Remove stopword |
| `get_stopwords_stats` | - | `StopwordStatsResponse` | Get statistics |
| `is_stopword_check` | `word` | `bool` | Check if stopword |
| `search_stopwords` | `query`, `limit?` | `Vec<UserStopword>` | Search stopwords |
| `clear_user_stopwords` | - | `i64` | Clear user stopwords |
| `reset_stopwords` | - | `ResetStopwordsResult` | Reset all stopwords |
| `restore_system_stopwords` | - | `ResetStopwordsResult` | Restore system stopwords |
| `export_stopwords` | - | `String` | JSON export |
| `import_stopwords` | `json` | `i64` | JSON import |

---

## Article Tags

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_all_tags` | `limit?` | `Vec<Tag>` | Get all tags |
| `get_article_tags` | `fnord_id` | `Vec<Tag>` | Get tags of an article |
| `add_article_tag` | `fnord_id`, `tag_name` | `Tag` | Add tag |
| `remove_article_tag` | `fnord_id`, `tag_id` | - | Remove tag |
| `set_article_tags` | `fnord_id`, `tag_names` | - | Set tags |

---

## Operation Mindfuck (Recommendations)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_recommendations` | `limit?` | `Vec<Recommendation>` | Get personalized recommendations |
| `get_reading_profile` | - | `ReadingProfile` | Get user reading profile |
| `get_blind_spots` | - | `Vec<BlindSpot>` | Get neglected topics |
| `get_counter_perspectives` | `fnord_id` | `Vec<CounterPerspective>` | Get alternative perspectives |
| `get_reading_trends` | `days?` | `ReadingTrends` | Get reading trends |
| `save_article` | `fnord_id` | - | Save article (positive feedback) |
| `unsave_article` | `fnord_id` | - | Unsave article |
| `hide_recommendation` | `fnord_id` | - | Hide recommendation |
| `get_saved_articles` | `limit?` | `Vec<SavedArticle>` | Get saved articles |
| `get_recommendation_stats` | - | `RecommendationStats` | Get recommendation statistics |

---

## Keyword Network (Extended)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_keywords` | `limit?`, `offset?`, `sort?` | `Vec<Keyword>` | Get keywords paginated |
| `get_keyword` | `id` | `Option<Keyword>` | Get single keyword |
| `get_keyword_neighbors` | `id`, `limit?` | `Vec<KeywordNeighbor>` | Get neighbor keywords |
| `get_keyword_categories` | `id` | `Vec<KeywordCategory>` | Get keyword categories |
| `get_category_keywords` | `sephiroth_id`, `limit?` | `Vec<Keyword>` | Get keywords of a category |
| `get_trending_keywords` | `days?`, `limit?` | `Vec<TrendingKeyword>` | Get trending keywords |
| `get_network_stats` | - | `NetworkStats` | Get network statistics |
| `search_keywords` | `query`, `limit?` | `Vec<Keyword>` | Search keywords |
| `get_keyword_trend` | `id`, `days?` | `Vec<TrendPoint>` | Get keyword trend |
| `get_network_graph` | `limit?` | `NetworkGraph` | Get graph data for visualization |
| `get_trending_comparison` | `keyword_ids`, `days?` | `TrendComparison` | Compare trends |
| `get_keyword_articles` | `id`, `limit?` | `Vec<Fnord>` | Get articles of a keyword |
| `get_cooccurring_keywords` | `id`, `limit?` | `Vec<CooccurringKeyword>` | Get co-occurring keywords |
| `create_keyword` | `name` | `CreateKeywordResult` | Create keyword |
| `delete_keyword` | `id` | - | Delete keyword |
| `rename_keyword` | `id`, `new_name` | `String` | Rename keyword |
| `prune_keywords` | `min_quality?`, `max_age_days?` | `PruneResult` | Prune keywords |
| `get_keyword_health` | - | `KeywordHealthStats` | Get health statistics |
| `merge_synonym_keywords` | - | `MergeResult` | Auto-merge synonyms |
| `cleanup_garbage_keywords` | - | `CleanupResult` | Remove garbage keywords |
| `find_similar_keywords` | `id`, `threshold?` | `Vec<SimilarKeyword>` | Find similar keywords |
| `auto_merge_similar_keywords` | `threshold?`, `dry_run?` | `AutoMergeResult` | Auto-merge similar |
| `update_keyword_types` | - | `KeywordTypeUpdateResult` | Update keyword types |
| `cleanup_keywords` | - | `KeywordCleanupResult` | Complete cleanup |

---

## Keyword Type Detection

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `init_keyword_type_prototypes` | - | - | Initialize prototypes |
| `get_prototype_stats` | - | `PrototypeStats` | Get prototype statistics |
| `generate_keyword_type_prototypes` | - | `PrototypeGenerationResult` | Generate prototypes (async) |
| `detect_single_keyword_type` | `keyword_id` | `KeywordType` | Detect keyword type |
| `update_keyword_types_hybrid` | `limit?` | `HybridUpdateResult` | Hybrid update (async) |
| `count_untyped_keywords` | - | `i64` | Count untyped keywords |
| `update_untyped_keywords` | `limit?` | `i64` | Update untyped keywords |

---

## Fnord Statistics

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_fnords_count` | `filter?` | `i64` | Count articles |
| `get_changed_count` | - | `i64` | Count changed articles |
| `reset_all_changes` | - | `i64` | Reset all changes |
| `get_fnord_stats` | - | `FnordStats` | Get general statistics |
| `get_subcategory_stats` | `sephiroth_id` | `Vec<SubcategoryStat>` | Get subcategory stats |
| `get_article_timeline` | `days?` | `Vec<TimelineEntry>` | Get article timeline |
| `get_greyface_index` | - | `GreyfaceIndex` | Get bias index |
| `get_top_keywords_stats` | `limit?` | `Vec<KeywordStat>` | Get top keywords |
| `get_feed_activity` | `days?` | `Vec<FeedActivity>` | Get feed activity |
| `get_bias_heatmap` | - | `Vec<BiasHeatmapEntry>` | Get bias heatmap |
| `get_keyword_cloud` | `limit?` | `Vec<KeywordCloudEntry>` | Get keyword cloud |

---

## Categories (Extended)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_all_categories` | - | `Vec<Sephiroth>` | Get all categories |
| `get_main_categories` | - | `Vec<Sephiroth>` | Get main categories |
| `get_subcategories` | - | `Vec<Sephiroth>` | Get subcategories |
| `get_categories_with_stats` | - | `Vec<MainCategory>` | Get with statistics |
| `get_article_categories` | `fnord_id` | `Vec<Sephiroth>` | Get article categories |
| `set_article_categories` | `fnord_id`, `category_ids` | - | Set article categories |
| `get_subcategory_names` | - | `Vec<String>` | Get subcategory names |

---

## Embedding Management

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_embedding_queue_status` | - | `EmbeddingQueueStatus` | Get queue status |
| `process_embedding_queue_now` | `limit?` | `EmbeddingQueueResult` | Process queue (async) |
| `queue_missing_embeddings` | - | `i64` | Queue missing embeddings |
| `get_embedding_queue_details` | `limit?` | `Vec<EmbeddingQueueItem>` | Get queue details |
| `calculate_neighbor_similarities` | `limit?` | `NeighborSimilarityResult` | Calculate neighbor similarities |

---

## Model Management (Extended)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `check_ollama` | - | `OllamaStatus` | Check Ollama status |
| `test_ai_provider` | `provider_type`, `base_url`, `api_key?` | `ProviderTestResult` | Test AI provider connection |
| `get_loaded_models` | - | `LoadedModelsResponse` | Get loaded models |
| `load_model` | `model` | `bool` | Load model |
| `unload_model` | `model` | `bool` | Unload model |
| `ensure_models_loaded` | `main_model`, `embedding_model` | `LoadedModelsResponse` | Ensure models are loaded |
| `pull_model` | `model` | `ModelPullResult` | Download model |

---

## Cost Tracking

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_monthly_cost` | - | `MonthlyCost` | Get current monthly cost summary |
| `get_cost_history` | `limit?` | `Vec<CostEntry>` | Get cost history entries (default: 100) |

### MonthlyCost Structure

```typescript
interface MonthlyCost {
  spent: number;       // Total spent this month in USD
  limit: number;       // Monthly cost limit from settings (default: 5.0)
  remaining: number;   // Remaining budget (limit - spent, min 0)
  percentage: number;  // Percentage of limit used (0-100)
}
```

### CostEntry Structure

```typescript
interface CostEntry {
  id: number;
  provider: string;           // e.g. "ollama", "openai_compatible"
  model: string;              // Model name used
  input_tokens: number;       // Input tokens consumed
  output_tokens: number;      // Output tokens generated
  estimated_cost_usd: number; // Estimated cost in USD
  created_at: string;         // ISO timestamp
}
```

**Notes:**
- Cost data is stored in the `ai_cost_log` table
- Monthly cost is calculated from the start of the current month
- The cost limit is configurable via the `cost_limit_monthly` setting (default: 5.0 USD)
- When the cost limit is reached, AI processing is blocked with a `CostLimitReached` error

---

## Batch Processing

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_unprocessed_count` | - | `UnprocessedCount` | Count unprocessed |
| `get_failed_count` | - | `FailedCount` | Count failed |
| `get_hopeless_count` | - | `HopelessCount` | Count hopeless |
| `process_batch` | `limit?` | `BatchResult` | Process batch (async) |
| `cancel_batch` | - | - | Cancel batch |

---

## OPML Import/Export

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `parse_opml_preview` | `content` | `OpmlPreview` | OPML preview |
| `import_opml` | `content`, `selected_feeds?` | `ImportResult` | Import OPML |
| `export_opml` | - | `String` | Export OPML |

---

## Settings (Extended)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_settings` | - | `HashMap<String, Value>` | Get all settings |
| `set_setting` | `key`, `value` | - | Save setting |
| `get_setting` | `key` | `Option<String>` | Load setting |
| `get_system_theme` | - | `String` | Get system theme |
| `get_log_levels` | - | `Vec<String>` | Get available log levels |
| `set_log_level` | `level` | - | Set log level |

---

## Prompts (Extended)

| Command | Parameter | Return | Description |
|---------|-----------|--------|-------------|
| `get_default_prompts` | - | `DefaultPrompts` | Get default prompts |
| `get_prompts` | - | `PromptTemplates` | Get current prompts |
| `set_prompts` | `summary_prompt`, `analysis_prompt` | - | Save prompts |
| `reset_prompts` | - | `PromptTemplates` | Reset prompts |
| `reset_articles_for_reprocessing` | `fnord_ids?` | `i64` | Reset articles for reprocessing |

---

## Data Structures

### Source Types and Weights

| Source | Description | Default Weight |
|--------|-------------|----------------|
| `ai` | Generated/validated by LLM | 1.0 |
| `statistical` | Generated by TF-IDF/word frequency | 0.9 |
| `manual` | Added by user | 1.2 |

Source weights are applied to confidence values (clamped to 0.0-1.0).

### Article Status Values

| Status | Description |
|--------|-------------|
| `concealed` | Unread article |
| `illuminated` | Read article |
| `golden_apple` | Favorited article |

### Bias Scores

| Field | Range | Description |
|-------|-------|-------------|
| `political_bias` | -2 to +2 | Political leaning (-2 = far left, +2 = far right) |
| `sachlichkeit` | 0 to 4 | Objectivity score (0 = emotional, 4 = factual) |

---

## Usage Example (TypeScript/Svelte)

```typescript
import { invoke } from '@tauri-apps/api/core';

// Get all feeds
const feeds = await invoke<Pentacle[]>('get_pentacles');

// Get articles with filter
const articles = await invoke<Fnord[]>('get_fnords', {
  filter: { pentacle_id: 1, status: 'concealed' }
});

// Process batch
const result = await invoke<BatchResult>('process_batch', { limit: 10 });

// Semantic search
const searchResults = await invoke<SemanticSearchResponse>('semantic_search', {
  query: 'artificial intelligence',
  limit: 20
});
```

---

*Last updated: February 2026*

*For development guidelines, database patterns, and project architecture, see [CLAUDE.md](../../CLAUDE.md).*
