//! Ollama-related commands for AI processing
//!
//! This module is organized into sub-modules:
//! - `types`: Shared data structures
//! - `helpers`: Utility functions for settings, prompts, merging
//! - `data_persistence`: Database operations for categories, keywords, embeddings
//! - `model_management`: Ollama model status, loading, unloading
//! - `article_processor`: Single article processing commands
//! - `batch_processor`: Batch processing commands
//! - `prompts`: Prompt template management
//! - `similarity`: Similar articles and semantic search

pub mod article_processor;
pub mod batch_processor;
pub mod data_persistence;
pub mod helpers;
pub mod model_management;
pub mod prompts;
pub mod similarity;
pub mod types;

// Re-export types for backward compatibility with existing tests and code
#[allow(unused_imports)]
pub use types::{
    // Status & Model Types
    OllamaStatus,
    LoadedModel,
    LoadedModelsResponse,
    ModelPullResult,
    // Prompt Types
    PromptTemplates,
    DefaultPrompts,
    // Article Processing Types
    SummaryResponse,
    AnalysisResponse,
    DiscordianResponse,
    // Source Tracking Types
    KeywordWithSource,
    CategoryWithSource,
    // Batch Processing Types
    BatchProgress,
    BatchResult,
    UnprocessedCount,
    HopelessCount,
    FailedCount,
    BatchArticle,
    ResetForReprocessingResult,
    // Similarity & Search Types
    SimilarArticleTag,
    SimilarArticleCategory,
    SimilarArticle,
    SimilarArticlesResponse,
    SearchResult,
    SemanticSearchResponse,
    ArticleEmbeddingCount,
    ArticleEmbeddingProgress,
    ArticleEmbeddingBatchResult,
};

// Re-export cluster batch types
#[allow(unused_imports)]
pub use batch_processor::{ClusterBatchConfig, ClusterBatchResult};
