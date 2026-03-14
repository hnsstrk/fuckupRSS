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
    AnalysisResponse,
    ArticleEmbeddingBatchResult,
    ArticleEmbeddingCount,
    ArticleEmbeddingProgress,
    BatchArticle,
    // Batch Processing Types
    BatchProgress,
    BatchResult,
    CategoryWithSource,
    DefaultPrompts,
    DiscordianResponse,
    FailedCount,
    HopelessCount,
    // Source Tracking Types
    KeywordWithMetadata,
    LoadedModel,
    LoadedModelsResponse,
    ModelPullResult,
    // Status & Model Types
    OllamaStatus,
    // Prompt Types
    PromptTemplates,
    ResetForReprocessingResult,
    SearchResult,
    SemanticSearchResponse,
    SimilarArticle,
    SimilarArticleCategory,
    // Similarity & Search Types
    SimilarArticleTag,
    SimilarArticlesResponse,
    // Article Processing Types
    SummaryResponse,
    UnprocessedCount,
};

// Re-export cluster batch types (only available with clustering feature)
#[cfg(feature = "clustering")]
#[allow(unused_imports)]
pub use batch_processor::{ClusterBatchConfig, ClusterBatchResult};
