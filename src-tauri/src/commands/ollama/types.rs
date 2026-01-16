//! Shared types for the Ollama commands module

use crate::ollama::BiasAnalysis;
use crate::ollama::DiscordianAnalysis;

// ============================================================
// STATUS & MODEL TYPES
// ============================================================

#[derive(serde::Serialize)]
pub struct OllamaStatus {
    pub available: bool,
    pub models: Vec<String>,
    pub recommended_main: String,
    pub recommended_embedding: String,
    pub has_recommended_main: bool,
    pub has_recommended_embedding: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LoadedModel {
    pub name: String,
    pub size: u64,
    pub size_vram: u64,
    pub parameter_size: String,
}

#[derive(serde::Serialize)]
pub struct LoadedModelsResponse {
    pub models: Vec<LoadedModel>,
}

#[derive(serde::Serialize)]
pub struct ModelPullResult {
    pub success: bool,
    pub model: String,
    pub status: Option<String>,
    pub error: Option<String>,
}

// ============================================================
// PROMPT TYPES
// ============================================================

#[derive(serde::Serialize)]
pub struct PromptTemplates {
    pub summary_prompt: String,
    pub analysis_prompt: String,
}

#[derive(serde::Serialize)]
pub struct DefaultPrompts {
    pub summary_prompt: String,
    pub analysis_prompt: String,
}

// ============================================================
// ARTICLE PROCESSING TYPES
// ============================================================

#[derive(serde::Serialize)]
pub struct SummaryResponse {
    pub fnord_id: i64,
    pub success: bool,
    pub summary: Option<String>,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct AnalysisResponse {
    pub fnord_id: i64,
    pub success: bool,
    pub analysis: Option<BiasAnalysis>,
    pub error: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct DiscordianResponse {
    pub fnord_id: i64,
    pub success: bool,
    pub analysis: Option<DiscordianAnalysis>,
    pub categories_saved: Vec<String>,
    pub tags_saved: Vec<String>,
    pub error: Option<String>,
}

// ============================================================
// SOURCE TRACKING TYPES
// ============================================================

// Re-export from central types for backward compatibility
pub use crate::keywords::types::KeywordWithMetadata;

/// Keyword with source tracking (statistical vs ai)
/// DEPRECATED: Use KeywordWithMetadata from keywords::types instead
pub type KeywordWithSource = KeywordWithMetadata;

/// Category with source tracking (statistical vs ai)
#[derive(Debug, Clone)]
pub struct CategoryWithSource {
    pub name: String,
    pub source: String, // 'ai', 'statistical'
    pub confidence: f64,
}

// ============================================================
// BATCH PROCESSING TYPES
// ============================================================

#[derive(serde::Serialize, Clone)]
pub struct BatchProgress {
    pub current: i64,
    pub total: i64,
    pub fnord_id: i64,
    pub title: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct BatchResult {
    pub processed: i64,
    pub succeeded: i64,
    pub failed: i64,
}

#[derive(serde::Serialize)]
pub struct UnprocessedCount {
    pub total: i64,
    pub with_content: i64,
}

#[derive(serde::Serialize)]
pub struct HopelessCount {
    pub count: i64,
}

#[derive(serde::Serialize)]
pub struct FailedCount {
    pub count: i64,
}

#[derive(Clone)]
pub struct BatchArticle {
    pub fnord_id: i64,
    pub title: String,
    pub content: String,
    pub article_date: Option<String>,
    pub attempts: i64,
    pub previous_error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ResetForReprocessingResult {
    pub reset_count: i64,
}

// ============================================================
// SIMILARITY & SEARCH TYPES
// ============================================================

#[derive(serde::Serialize, Clone)]
pub struct SimilarArticleTag {
    pub id: i64,
    pub name: String,
}

#[derive(serde::Serialize, Clone)]
pub struct SimilarArticleCategory {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct SimilarArticle {
    pub fnord_id: i64,
    pub title: String,
    pub pentacle_title: Option<String>,
    pub published_at: Option<String>,
    pub similarity: f64,
    pub tags: Vec<SimilarArticleTag>,
    pub categories: Vec<SimilarArticleCategory>,
}

#[derive(serde::Serialize)]
pub struct SimilarArticlesResponse {
    pub fnord_id: i64,
    pub similar: Vec<SimilarArticle>,
}

#[derive(serde::Serialize, Clone)]
pub struct SearchResult {
    pub fnord_id: i64,
    pub title: String,
    pub pentacle_title: Option<String>,
    pub published_at: Option<String>,
    pub summary: Option<String>,
    pub similarity: f64,
}

#[derive(serde::Serialize)]
pub struct SemanticSearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(serde::Serialize)]
pub struct ArticleEmbeddingCount {
    pub total_articles: i64,
    pub with_embedding: i64,
    pub without_embedding: i64,
    pub processable: i64,
}

#[derive(serde::Serialize, Clone)]
pub struct ArticleEmbeddingProgress {
    pub current: i64,
    pub total: i64,
    pub fnord_id: i64,
    pub title: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ArticleEmbeddingBatchResult {
    pub processed: i64,
    pub succeeded: i64,
    pub failed: i64,
}
