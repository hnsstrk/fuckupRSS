//! Tests for Tauri commands, with focus on batch processing

use std::sync::atomic::{AtomicBool, Ordering};

// ============================================================
// BatchProgress struct tests
// ============================================================

#[test]
fn test_batch_progress_serialize() {
    use super::ai::BatchProgress;

    let progress = BatchProgress {
        current: 5,
        total: 10,
        fnord_id: 123,
        title: "Test Article".to_string(),
        success: true,
        error: None,
    };

    let json = serde_json::to_string(&progress).expect("Serialization failed");
    assert!(json.contains("\"current\":5"));
    assert!(json.contains("\"total\":10"));
    assert!(json.contains("\"fnord_id\":123"));
    assert!(json.contains("\"title\":\"Test Article\""));
    assert!(json.contains("\"success\":true"));
}

#[test]
fn test_batch_progress_with_error() {
    use super::ai::BatchProgress;

    let progress = BatchProgress {
        current: 3,
        total: 10,
        fnord_id: 456,
        title: "Failed Article".to_string(),
        success: false,
        error: Some("Connection timeout".to_string()),
    };

    let json = serde_json::to_string(&progress).expect("Serialization failed");
    assert!(json.contains("\"success\":false"));
    assert!(json.contains("Connection timeout"));
}

#[test]
fn test_batch_progress_initial_state() {
    use super::ai::BatchProgress;

    // Initial progress event (current=0)
    let progress = BatchProgress {
        current: 0,
        total: 50,
        fnord_id: 0,
        title: "Starting...".to_string(),
        success: true,
        error: None,
    };

    assert_eq!(progress.current, 0);
    assert_eq!(progress.total, 50);
}

#[test]
fn test_batch_progress_cancellation() {
    use super::ai::BatchProgress;

    let progress = BatchProgress {
        current: 25,
        total: 100,
        fnord_id: 0,
        title: "Cancelled".to_string(),
        success: false,
        error: Some("Batch cancelled by user".to_string()),
    };

    assert!(!progress.success);
    assert!(progress.error.as_ref().unwrap().contains("cancelled"));
}

// ============================================================
// BatchResult struct tests
// ============================================================

#[test]
fn test_batch_result_serialize() {
    use super::ai::BatchResult;

    let result = BatchResult {
        processed: 100,
        succeeded: 95,
        failed: 5,
    };

    let json = serde_json::to_string(&result).expect("Serialization failed");
    assert!(json.contains("\"processed\":100"));
    assert!(json.contains("\"succeeded\":95"));
    assert!(json.contains("\"failed\":5"));
}

#[test]
fn test_batch_result_all_success() {
    use super::ai::BatchResult;

    let result = BatchResult {
        processed: 50,
        succeeded: 50,
        failed: 0,
    };

    assert_eq!(result.processed, result.succeeded);
    assert_eq!(result.failed, 0);
}

#[test]
fn test_batch_result_all_failed() {
    use super::ai::BatchResult;

    let result = BatchResult {
        processed: 10,
        succeeded: 0,
        failed: 10,
    };

    assert_eq!(result.processed, result.failed);
    assert_eq!(result.succeeded, 0);
}

// ============================================================
// UnprocessedCount struct tests
// ============================================================

#[test]
fn test_unprocessed_count_serialize() {
    use super::ai::UnprocessedCount;

    let count = UnprocessedCount {
        total: 150,
        with_content: 120,
    };

    let json = serde_json::to_string(&count).expect("Serialization failed");
    assert!(json.contains("\"total\":150"));
    assert!(json.contains("\"with_content\":120"));
}

#[test]
fn test_unprocessed_count_all_have_content() {
    use super::ai::UnprocessedCount;

    let count = UnprocessedCount {
        total: 100,
        with_content: 100,
    };

    assert_eq!(count.total, count.with_content);
}

#[test]
fn test_unprocessed_count_none_have_content() {
    use super::ai::UnprocessedCount;

    let count = UnprocessedCount {
        total: 50,
        with_content: 0,
    };

    assert!(count.with_content < count.total);
}

// ============================================================
// OllamaStatus struct tests
// ============================================================

#[test]
fn test_ollama_status_available() {
    use super::ai::OllamaStatus;

    let status = OllamaStatus {
        available: true,
        models: vec!["qwen3-vl:8b".to_string(), "nomic-embed-text".to_string()],
        recommended_main: "qwen3-vl:8b".to_string(),
        recommended_embedding: "nomic-embed-text".to_string(),
        has_recommended_main: true,
        has_recommended_embedding: true,
    };

    assert!(status.available);
    assert_eq!(status.models.len(), 2);
    assert!(status.has_recommended_main);
    assert!(status.has_recommended_embedding);
}

#[test]
fn test_ollama_status_unavailable() {
    use super::ai::OllamaStatus;

    let status = OllamaStatus {
        available: false,
        models: vec![],
        recommended_main: "qwen3-vl:8b".to_string(),
        recommended_embedding: "nomic-embed-text".to_string(),
        has_recommended_main: false,
        has_recommended_embedding: false,
    };

    assert!(!status.available);
    assert!(status.models.is_empty());
    assert!(!status.has_recommended_main);
}

#[test]
fn test_ollama_status_partial_models() {
    use super::ai::OllamaStatus;

    let status = OllamaStatus {
        available: true,
        models: vec!["qwen3-vl:8b".to_string()],
        recommended_main: "qwen3-vl:8b".to_string(),
        recommended_embedding: "nomic-embed-text".to_string(),
        has_recommended_main: true,
        has_recommended_embedding: false, // Missing embedding model
    };

    assert!(status.available);
    assert!(status.has_recommended_main);
    assert!(!status.has_recommended_embedding);
}

// ============================================================
// Atomic cancellation flag tests
// ============================================================

#[test]
fn test_cancellation_flag_default() {
    let cancel_flag = AtomicBool::new(false);
    assert!(!cancel_flag.load(Ordering::SeqCst));
}

#[test]
fn test_cancellation_flag_set() {
    let cancel_flag = AtomicBool::new(false);
    cancel_flag.store(true, Ordering::SeqCst);
    assert!(cancel_flag.load(Ordering::SeqCst));
}

#[test]
fn test_cancellation_flag_reset() {
    let cancel_flag = AtomicBool::new(true);
    cancel_flag.store(false, Ordering::SeqCst);
    assert!(!cancel_flag.load(Ordering::SeqCst));
}

#[test]
fn test_cancellation_flag_multiple_toggles() {
    let cancel_flag = AtomicBool::new(false);

    // Simulate start of batch
    cancel_flag.store(false, Ordering::SeqCst);
    assert!(!cancel_flag.load(Ordering::SeqCst));

    // User cancels
    cancel_flag.store(true, Ordering::SeqCst);
    assert!(cancel_flag.load(Ordering::SeqCst));

    // New batch starts
    cancel_flag.store(false, Ordering::SeqCst);
    assert!(!cancel_flag.load(Ordering::SeqCst));
}

// ============================================================
// SummaryResponse struct tests
// ============================================================

#[test]
fn test_summary_response_success() {
    use super::ai::SummaryResponse;

    let response = SummaryResponse {
        fnord_id: 123,
        success: true,
        summary: Some("This is a test summary.".to_string()),
        error: None,
    };

    assert!(response.success);
    assert!(response.summary.is_some());
    assert!(response.error.is_none());
}

#[test]
fn test_summary_response_failure() {
    use super::ai::SummaryResponse;

    let response = SummaryResponse {
        fnord_id: 456,
        success: false,
        summary: None,
        error: Some("No content available".to_string()),
    };

    assert!(!response.success);
    assert!(response.summary.is_none());
    assert!(response.error.is_some());
}

// ============================================================
// AnalysisResponse struct tests
// ============================================================

#[test]
fn test_analysis_response_success() {
    use super::ai::AnalysisResponse;
    use crate::ollama::BiasAnalysis;

    let analysis = BiasAnalysis {
        political_bias: 0,
        sachlichkeit: 3,
    };

    let response = AnalysisResponse {
        fnord_id: 789,
        success: true,
        analysis: Some(analysis),
        error: None,
    };

    assert!(response.success);
    assert!(response.analysis.is_some());
    let a = response.analysis.unwrap();
    assert_eq!(a.political_bias, 0);
    assert_eq!(a.sachlichkeit, 3);
}

// ============================================================
// DiscordianResponse struct tests
// ============================================================

#[test]
fn test_discordian_response_success() {
    use super::ai::DiscordianResponse;
    use crate::ollama::DiscordianAnalysis;

    let analysis = DiscordianAnalysis {
        summary: "Test summary".to_string(),
        categories: vec!["Politik".to_string()],
        keywords: vec!["keyword1".to_string(), "keyword2".to_string()],
        political_bias: -1,
        sachlichkeit: 3,
    };

    let response = DiscordianResponse {
        fnord_id: 999,
        success: true,
        analysis: Some(analysis),
        categories_saved: vec!["Politik".to_string()],
        tags_saved: vec!["keyword1".to_string(), "keyword2".to_string()],
        error: None,
    };

    assert!(response.success);
    assert_eq!(response.categories_saved.len(), 1);
    assert_eq!(response.tags_saved.len(), 2);
}

#[test]
fn test_discordian_response_no_content() {
    use super::ai::DiscordianResponse;

    let response = DiscordianResponse {
        fnord_id: 111,
        success: false,
        analysis: None,
        categories_saved: vec![],
        tags_saved: vec![],
        error: Some("No content available".to_string()),
    };

    assert!(!response.success);
    assert!(response.categories_saved.is_empty());
    assert!(response.tags_saved.is_empty());
}

// ============================================================
// Prompt templates tests
// ============================================================

#[test]
fn test_prompt_templates_serialize() {
    use super::ai::PromptTemplates;

    let templates = PromptTemplates {
        summary_prompt: "Summarize: {content}".to_string(),
        analysis_prompt: "Analyze: {title} {content}".to_string(),
        discordian_prompt: "Discordian: {language} {title} {content} {stat_keywords} {stat_categories}".to_string(),
    };

    let json = serde_json::to_string(&templates).expect("Serialization failed");
    assert!(json.contains("summary_prompt"));
    assert!(json.contains("analysis_prompt"));
    assert!(json.contains("discordian_prompt"));
}

#[test]
fn test_default_prompts_serialize() {
    use super::ai::DefaultPrompts;

    let defaults = DefaultPrompts {
        summary_prompt: "Default summary prompt".to_string(),
        analysis_prompt: "Default analysis prompt".to_string(),
        discordian_prompt: "Default discordian prompt".to_string(),
    };

    let json = serde_json::to_string(&defaults).expect("Serialization failed");
    assert!(json.contains("Default summary"));
    assert!(json.contains("discordian_prompt"));
}

// ============================================================
// ModelPullResult struct tests
// ============================================================

#[test]
fn test_model_pull_result_success() {
    use super::ai::ModelPullResult;

    let result = ModelPullResult {
        success: true,
        model: "qwen3-vl:8b".to_string(),
        status: Some("completed".to_string()),
        error: None,
    };

    assert!(result.success);
    assert_eq!(result.model, "qwen3-vl:8b");
    assert!(result.status.is_some());
}

#[test]
fn test_model_pull_result_failure() {
    use super::ai::ModelPullResult;

    let result = ModelPullResult {
        success: false,
        model: "nonexistent-model".to_string(),
        status: None,
        error: Some("Model not found".to_string()),
    };

    assert!(!result.success);
    assert!(result.error.is_some());
}

// ============================================================
// Batch processing flow simulation tests
// ============================================================

#[test]
fn test_batch_processing_flow() {
    use super::ai::{BatchProgress, BatchResult};

    // Simulate a batch of 3 articles
    let total = 3i64;
    let mut succeeded = 0i64;
    let mut failed = 0i64;

    // Simulate processing
    let articles = vec![
        (1, "Article 1", true),
        (2, "Article 2", true),
        (3, "Article 3", false), // This one fails
    ];

    for (idx, (id, title, success)) in articles.iter().enumerate() {
        let current = (idx + 1) as i64;

        if *success {
            succeeded += 1;
        } else {
            failed += 1;
        }

        let progress = BatchProgress {
            current,
            total,
            fnord_id: *id,
            title: title.to_string(),
            success: *success,
            error: if *success { None } else { Some("Failed".to_string()) },
        };

        // Verify progress tracking
        assert_eq!(progress.current, current);
        assert_eq!(progress.total, total);
    }

    let result = BatchResult {
        processed: total,
        succeeded,
        failed,
    };

    assert_eq!(result.processed, 3);
    assert_eq!(result.succeeded, 2);
    assert_eq!(result.failed, 1);
}

#[test]
fn test_batch_processing_with_cancellation() {
    let cancel_flag = AtomicBool::new(false);
    let total = 100i64;
    let mut processed = 0i64;

    // Simulate processing with cancellation at 50%
    for idx in 0..total {
        if cancel_flag.load(Ordering::SeqCst) {
            break;
        }

        processed += 1;

        // User cancels at 50%
        if idx == 49 {
            cancel_flag.store(true, Ordering::SeqCst);
        }
    }

    // Should have stopped at 50
    assert_eq!(processed, 50);
    assert!(cancel_flag.load(Ordering::SeqCst));
}

#[test]
fn test_batch_empty_articles() {
    use super::ai::BatchResult;

    // No articles to process
    let result = BatchResult {
        processed: 0,
        succeeded: 0,
        failed: 0,
    };

    assert_eq!(result.processed, 0);
}

// ============================================================
// Database query simulation tests
// ============================================================

#[test]
fn test_unprocessed_articles_query_logic() {
    // Simulate the query logic for getting unprocessed articles
    #[derive(Clone)]
    struct MockArticle {
        id: i64,
        title: String,
        content_full: Option<String>,
        content_raw: Option<String>,
        processed_at: Option<String>,
    }

    let articles = vec![
        MockArticle {
            id: 1,
            title: "Processed".to_string(),
            content_full: Some("Content".to_string()),
            content_raw: None,
            processed_at: Some("2024-01-01".to_string()),
        },
        MockArticle {
            id: 2,
            title: "Unprocessed with full content".to_string(),
            content_full: Some("Full content".to_string()),
            content_raw: None,
            processed_at: None,
        },
        MockArticle {
            id: 3,
            title: "Unprocessed with raw content".to_string(),
            content_full: None,
            content_raw: Some("Raw content".to_string()),
            processed_at: None,
        },
        MockArticle {
            id: 4,
            title: "Unprocessed no content".to_string(),
            content_full: None,
            content_raw: None,
            processed_at: None,
        },
    ];

    // Filter: processed_at IS NULL
    let unprocessed: Vec<_> = articles
        .iter()
        .filter(|a| a.processed_at.is_none())
        .collect();
    assert_eq!(unprocessed.len(), 3);

    // Filter: processed_at IS NULL AND (content_full OR content_raw IS NOT NULL)
    let with_content: Vec<_> = articles
        .iter()
        .filter(|a| a.processed_at.is_none())
        .filter(|a| a.content_full.is_some() || a.content_raw.is_some())
        .collect();
    assert_eq!(with_content.len(), 2);
}

#[test]
fn test_content_coalesce_logic() {
    // Test the COALESCE(content_full, content_raw, '') logic
    fn coalesce_content(content_full: Option<&str>, content_raw: Option<&str>) -> String {
        content_full
            .or(content_raw)
            .unwrap_or("")
            .to_string()
    }

    // Full content takes priority
    assert_eq!(coalesce_content(Some("Full"), Some("Raw")), "Full");

    // Falls back to raw
    assert_eq!(coalesce_content(None, Some("Raw")), "Raw");

    // Empty string if neither
    assert_eq!(coalesce_content(None, None), "");

    // Full only
    assert_eq!(coalesce_content(Some("Full"), None), "Full");
}

// ============================================================
// Semantic search and similarity tests
// ============================================================

/// Helper function to convert distance to similarity (mirrors similarity.rs logic)
fn distance_to_similarity(distance: f64) -> f64 {
    1.0 - (distance / 2.0)
}

#[test]
fn test_similar_article_response_serialize() {
    use super::ai::types::{SimilarArticle, SimilarArticlesResponse, SimilarArticleTag, SimilarArticleCategory};

    let similar = SimilarArticle {
        fnord_id: 123,
        title: "Test Article".to_string(),
        pentacle_title: Some("Test Feed".to_string()),
        published_at: Some("2024-01-15".to_string()),
        similarity: 0.85,
        tags: vec![
            SimilarArticleTag { id: 1, name: "Tag1".to_string() },
            SimilarArticleTag { id: 2, name: "Tag2".to_string() },
        ],
        categories: vec![
            SimilarArticleCategory {
                id: 1,
                name: "Politik".to_string(),
                icon: Some("fa-landmark".to_string()),
                color: Some("#dc3545".to_string()),
            },
        ],
    };

    let response = SimilarArticlesResponse {
        fnord_id: 456,
        similar: vec![similar],
    };

    let json = serde_json::to_string(&response).expect("Serialization failed");
    assert!(json.contains("\"fnord_id\":456"));
    assert!(json.contains("\"similarity\":0.85"));
    assert!(json.contains("\"tags\""));
    assert!(json.contains("\"categories\""));
}

#[test]
fn test_similar_articles_empty_response() {
    use super::ai::types::SimilarArticlesResponse;

    // When article has no embedding, return empty similar list
    let response = SimilarArticlesResponse {
        fnord_id: 123,
        similar: vec![],
    };

    assert_eq!(response.fnord_id, 123);
    assert!(response.similar.is_empty());
}

#[test]
fn test_search_result_serialize() {
    use super::ai::types::{SearchResult, SemanticSearchResponse};

    let result = SearchResult {
        fnord_id: 789,
        title: "Search Result Article".to_string(),
        pentacle_title: Some("News Feed".to_string()),
        published_at: Some("2024-01-10".to_string()),
        summary: Some("This is a summary of the article.".to_string()),
        similarity: 0.72,
    };

    let response = SemanticSearchResponse {
        query: "test query".to_string(),
        results: vec![result],
    };

    let json = serde_json::to_string(&response).expect("Serialization failed");
    assert!(json.contains("\"query\":\"test query\""));
    assert!(json.contains("\"similarity\":0.72"));
    assert!(json.contains("\"summary\""));
}

#[test]
fn test_semantic_search_empty_query() {
    use super::ai::types::SemanticSearchResponse;

    // Empty query should return empty results
    let response = SemanticSearchResponse {
        query: "".to_string(),
        results: vec![],
    };

    assert!(response.query.is_empty());
    assert!(response.results.is_empty());
}

#[test]
fn test_article_embedding_count_serialize() {
    use super::ai::types::ArticleEmbeddingCount;

    let count = ArticleEmbeddingCount {
        total_articles: 1000,
        with_embedding: 750,
        without_embedding: 250,
        processable: 200,
    };

    let json = serde_json::to_string(&count).expect("Serialization failed");
    assert!(json.contains("\"total_articles\":1000"));
    assert!(json.contains("\"with_embedding\":750"));
    assert!(json.contains("\"without_embedding\":250"));
    assert!(json.contains("\"processable\":200"));
}

#[test]
fn test_article_embedding_count_consistency() {
    use super::ai::types::ArticleEmbeddingCount;

    let count = ArticleEmbeddingCount {
        total_articles: 100,
        with_embedding: 60,
        without_embedding: 40,
        processable: 30,
    };

    // with_embedding + without_embedding should equal total_articles
    assert_eq!(count.with_embedding + count.without_embedding, count.total_articles);

    // processable should be <= without_embedding (subset of articles without embedding)
    assert!(count.processable <= count.without_embedding);
}

#[test]
fn test_article_embedding_batch_result_serialize() {
    use super::ai::types::ArticleEmbeddingBatchResult;

    let result = ArticleEmbeddingBatchResult {
        processed: 50,
        succeeded: 45,
        failed: 5,
    };

    let json = serde_json::to_string(&result).expect("Serialization failed");
    assert!(json.contains("\"processed\":50"));
    assert!(json.contains("\"succeeded\":45"));
    assert!(json.contains("\"failed\":5"));
}

#[test]
fn test_article_embedding_batch_result_consistency() {
    use super::ai::types::ArticleEmbeddingBatchResult;

    let result = ArticleEmbeddingBatchResult {
        processed: 100,
        succeeded: 95,
        failed: 5,
    };

    // succeeded + failed should equal processed
    assert_eq!(result.succeeded + result.failed, result.processed);
}

#[test]
fn test_article_embedding_progress_serialize() {
    use super::ai::types::ArticleEmbeddingProgress;

    let progress = ArticleEmbeddingProgress {
        current: 25,
        total: 100,
        fnord_id: 456,
        title: "Processing article...".to_string(),
        success: true,
        error: None,
    };

    let json = serde_json::to_string(&progress).expect("Serialization failed");
    assert!(json.contains("\"current\":25"));
    assert!(json.contains("\"total\":100"));
    assert!(json.contains("\"fnord_id\":456"));
    assert!(json.contains("\"success\":true"));
}

#[test]
fn test_article_embedding_progress_with_error() {
    use super::ai::types::ArticleEmbeddingProgress;

    let progress = ArticleEmbeddingProgress {
        current: 10,
        total: 50,
        fnord_id: 789,
        title: "Failed article".to_string(),
        success: false,
        error: Some("Ollama connection failed".to_string()),
    };

    assert!(!progress.success);
    assert!(progress.error.is_some());
    assert!(progress.error.as_ref().unwrap().contains("Ollama"));
}

// ============================================================
// Similarity threshold tests
// ============================================================

#[test]
fn test_distance_to_similarity_conversion() {
    // Distance 0 = identical = similarity 1.0
    assert!((distance_to_similarity(0.0) - 1.0).abs() < 1e-6);

    // Distance 2.0 (max for cosine) = similarity 0.0
    assert!((distance_to_similarity(2.0) - 0.0).abs() < 1e-6);

    // Distance 1.0 = similarity 0.5
    assert!((distance_to_similarity(1.0) - 0.5).abs() < 1e-6);

    // Distance 0.5 = similarity 0.75
    assert!((distance_to_similarity(0.5) - 0.75).abs() < 1e-6);
}

#[test]
fn test_similar_articles_threshold_filter() {
    // Threshold for similar articles is >= 0.5
    let threshold = 0.5;

    let test_cases = vec![
        (0.0, true),   // distance 0 → similarity 1.0 → pass
        (0.5, true),   // distance 0.5 → similarity 0.75 → pass
        (1.0, true),   // distance 1.0 → similarity 0.5 → pass (exactly at threshold)
        (1.01, false), // distance 1.01 → similarity 0.495 → fail
        (1.5, false),  // distance 1.5 → similarity 0.25 → fail
        (2.0, false),  // distance 2.0 → similarity 0.0 → fail
    ];

    for (distance, should_pass) in test_cases {
        let similarity = distance_to_similarity(distance);
        let passes = similarity >= threshold;
        assert_eq!(
            passes, should_pass,
            "Distance {} → similarity {} should {} pass threshold {}",
            distance,
            similarity,
            if should_pass { "" } else { "not " },
            threshold
        );
    }
}

#[test]
fn test_semantic_search_threshold_filter() {
    // Threshold for semantic search is >= 0.3
    let threshold = 0.3;

    let test_cases = vec![
        (0.0, true),   // distance 0 → similarity 1.0 → pass
        (1.0, true),   // distance 1.0 → similarity 0.5 → pass
        (1.4, true),   // distance 1.4 → similarity 0.3 → pass (exactly at threshold)
        (1.41, false), // distance 1.41 → similarity 0.295 → fail
        (1.8, false),  // distance 1.8 → similarity 0.1 → fail
        (2.0, false),  // distance 2.0 → similarity 0.0 → fail
    ];

    for (distance, should_pass) in test_cases {
        let similarity = distance_to_similarity(distance);
        let passes = similarity >= threshold;
        assert_eq!(
            passes, should_pass,
            "Distance {} → similarity {} should {} pass threshold {}",
            distance,
            similarity,
            if should_pass { "" } else { "not " },
            threshold
        );
    }
}

#[test]
fn test_search_threshold_more_permissive_than_similar() {
    // Search threshold (0.3) should be more permissive than similar articles (0.5)
    let similar_threshold = 0.5;
    let search_threshold = 0.3;

    assert!(search_threshold < similar_threshold);

    // A result that passes similar articles should always pass search
    let high_similarity = 0.75;
    assert!(high_similarity >= similar_threshold);
    assert!(high_similarity >= search_threshold);

    // A result between thresholds should pass search but not similar
    let mid_similarity = 0.4;
    assert!(mid_similarity < similar_threshold);
    assert!(mid_similarity >= search_threshold);
}

// ============================================================
// Empty embedding edge cases
// ============================================================

#[test]
fn test_empty_embedding_returns_empty_similar() {
    // Simulating the logic: if embedding is None or empty, return empty results
    let embedding: Option<Vec<u8>> = None;

    let should_search = match embedding {
        Some(e) if !e.is_empty() => true,
        _ => false,
    };

    assert!(!should_search);
}

#[test]
fn test_empty_vec_embedding_returns_empty_similar() {
    // Edge case: embedding exists but is empty vec
    let embedding: Option<Vec<u8>> = Some(vec![]);

    let should_search = match embedding {
        Some(e) if !e.is_empty() => true,
        _ => false,
    };

    assert!(!should_search);
}

#[test]
fn test_valid_embedding_allows_search() {
    // Valid embedding should allow search
    let embedding: Option<Vec<u8>> = Some(vec![1, 2, 3, 4]); // 1 f32 value

    let should_search = match embedding {
        Some(e) if !e.is_empty() => true,
        _ => false,
    };

    assert!(should_search);
}

// ============================================================
// Article embedding stats tests
// ============================================================

#[test]
fn test_embedding_stats_all_embedded() {
    use super::ai::types::ArticleEmbeddingCount;

    // All articles have embeddings
    let count = ArticleEmbeddingCount {
        total_articles: 500,
        with_embedding: 500,
        without_embedding: 0,
        processable: 0,
    };

    assert_eq!(count.with_embedding, count.total_articles);
    assert_eq!(count.without_embedding, 0);
    assert_eq!(count.processable, 0);
}

#[test]
fn test_embedding_stats_none_embedded() {
    use super::ai::types::ArticleEmbeddingCount;

    // No articles have embeddings yet
    let count = ArticleEmbeddingCount {
        total_articles: 100,
        with_embedding: 0,
        without_embedding: 100,
        processable: 80, // Only 80 are processable (have content_full)
    };

    assert_eq!(count.with_embedding, 0);
    assert_eq!(count.without_embedding, count.total_articles);
    assert!(count.processable <= count.without_embedding);
}

#[test]
fn test_embedding_stats_processable_constraint() {
    // Processable articles must:
    // 1. Have embedding IS NULL
    // 2. Have processed_at IS NOT NULL (already analyzed)
    // 3. Have content_full IS NOT NULL
    // 4. Have LENGTH(content_full) >= 100

    #[derive(Clone)]
    struct MockArticle {
        embedding: Option<Vec<u8>>,
        processed_at: Option<String>,
        content_full: Option<String>,
    }

    let articles = vec![
        MockArticle {
            embedding: Some(vec![1, 2, 3, 4]),
            processed_at: Some("2024-01-01".to_string()),
            content_full: Some("x".repeat(200)),
        }, // Has embedding - not processable
        MockArticle {
            embedding: None,
            processed_at: None,
            content_full: Some("x".repeat(200)),
        }, // Not processed - not processable
        MockArticle {
            embedding: None,
            processed_at: Some("2024-01-01".to_string()),
            content_full: None,
        }, // No content - not processable
        MockArticle {
            embedding: None,
            processed_at: Some("2024-01-01".to_string()),
            content_full: Some("short".to_string()),
        }, // Content too short - not processable
        MockArticle {
            embedding: None,
            processed_at: Some("2024-01-01".to_string()),
            content_full: Some("x".repeat(200)),
        }, // Processable!
    ];

    let processable: Vec<_> = articles
        .iter()
        .filter(|a| a.embedding.is_none())
        .filter(|a| a.processed_at.is_some())
        .filter(|a| a.content_full.as_ref().map(|c| c.len() >= 100).unwrap_or(false))
        .collect();

    assert_eq!(processable.len(), 1);
}

// ============================================================
// Batch embedding progress tracking tests
// ============================================================

#[test]
fn test_batch_embedding_flow() {
    use super::ai::types::{ArticleEmbeddingProgress, ArticleEmbeddingBatchResult};

    // Simulate a batch of 5 articles
    let total = 5i64;
    let mut succeeded = 0i64;
    let mut failed = 0i64;

    let articles = vec![
        (1, "Article 1", true),
        (2, "Article 2", true),
        (3, "Article 3", false), // Fails
        (4, "Article 4", true),
        (5, "Article 5", true),
    ];

    for (idx, (id, title, success)) in articles.iter().enumerate() {
        let current = (idx + 1) as i64;

        if *success {
            succeeded += 1;
        } else {
            failed += 1;
        }

        let progress = ArticleEmbeddingProgress {
            current,
            total,
            fnord_id: *id,
            title: title.to_string(),
            success: *success,
            error: if *success { None } else { Some("Embedding failed".to_string()) },
        };

        assert_eq!(progress.current, current);
        assert_eq!(progress.total, total);
    }

    let result = ArticleEmbeddingBatchResult {
        processed: total,
        succeeded,
        failed,
    };

    assert_eq!(result.processed, 5);
    assert_eq!(result.succeeded, 4);
    assert_eq!(result.failed, 1);
}

#[test]
fn test_batch_embedding_empty() {
    use super::ai::types::ArticleEmbeddingBatchResult;

    // No articles to process
    let result = ArticleEmbeddingBatchResult {
        processed: 0,
        succeeded: 0,
        failed: 0,
    };

    assert_eq!(result.processed, 0);
    assert_eq!(result.succeeded, 0);
    assert_eq!(result.failed, 0);
}

#[test]
fn test_batch_embedding_initial_progress() {
    use super::ai::types::ArticleEmbeddingProgress;

    // Initial progress event (before processing starts)
    let progress = ArticleEmbeddingProgress {
        current: 0,
        total: 100,
        fnord_id: 0,
        title: "Starting...".to_string(),
        success: true,
        error: None,
    };

    assert_eq!(progress.current, 0);
    assert_eq!(progress.total, 100);
    assert_eq!(progress.title, "Starting...");
}

// ============================================================
// SimilarArticleTag and SimilarArticleCategory tests
// ============================================================

#[test]
fn test_similar_article_tag_serialize() {
    use super::ai::types::SimilarArticleTag;

    let tag = SimilarArticleTag {
        id: 42,
        name: "Künstliche Intelligenz".to_string(),
    };

    let json = serde_json::to_string(&tag).expect("Serialization failed");
    assert!(json.contains("\"id\":42"));
    assert!(json.contains("Künstliche Intelligenz"));
}

#[test]
fn test_similar_article_category_serialize() {
    use super::ai::types::SimilarArticleCategory;

    let category = SimilarArticleCategory {
        id: 1,
        name: "Wissen & Technologie".to_string(),
        icon: Some("fa-microchip".to_string()),
        color: Some("#007bff".to_string()),
    };

    let json = serde_json::to_string(&category).expect("Serialization failed");
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("Wissen & Technologie"));
    assert!(json.contains("fa-microchip"));
    assert!(json.contains("#007bff"));
}

#[test]
fn test_similar_article_category_optional_fields() {
    use super::ai::types::SimilarArticleCategory;

    // Category without icon and color
    let category = SimilarArticleCategory {
        id: 5,
        name: "Uncategorized".to_string(),
        icon: None,
        color: None,
    };

    let json = serde_json::to_string(&category).expect("Serialization failed");
    assert!(json.contains("\"icon\":null"));
    assert!(json.contains("\"color\":null"));
}

// ============================================================
// Vector distance calculation verification tests
// ============================================================

#[test]
fn test_cosine_distance_bounds() {
    // Cosine distance ranges from 0 (identical) to 2 (opposite)
    // Similarity = 1 - (distance / 2)

    // Identical vectors: distance = 0, similarity = 1.0
    let sim_identical = distance_to_similarity(0.0);
    assert!((sim_identical - 1.0).abs() < 1e-6);

    // Orthogonal vectors: distance = 1, similarity = 0.5
    let sim_orthogonal = distance_to_similarity(1.0);
    assert!((sim_orthogonal - 0.5).abs() < 1e-6);

    // Opposite vectors: distance = 2, similarity = 0.0
    let sim_opposite = distance_to_similarity(2.0);
    assert!((sim_opposite - 0.0).abs() < 1e-6);
}

#[test]
fn test_similarity_is_monotonic() {
    // As distance increases, similarity should decrease
    let distances = vec![0.0, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];

    let similarities: Vec<f64> = distances.iter().map(|d| distance_to_similarity(*d)).collect();

    for i in 1..similarities.len() {
        assert!(
            similarities[i] <= similarities[i - 1],
            "Similarity should decrease as distance increases: {} > {} at distances {}, {}",
            similarities[i],
            similarities[i - 1],
            distances[i],
            distances[i - 1]
        );
    }
}

#[test]
fn test_similarity_range() {
    // All similarities should be in [0, 1] for valid distances [0, 2]
    for distance in (0..=200).map(|i| i as f64 / 100.0) {
        let similarity = distance_to_similarity(distance);
        assert!(
            similarity >= 0.0 && similarity <= 1.0,
            "Similarity {} out of range for distance {}",
            similarity,
            distance
        );
    }
}

// ============================================================
// Keyword context extraction tests (get_keyword_context helpers)
// ============================================================

#[test]
fn test_strip_html_tags_basic() {
    use super::immanentize::strip_html_tags;

    let html = "<p>Hello <strong>World</strong>!</p>";
    let result = strip_html_tags(html);
    assert_eq!(result, "Hello World!");
}

#[test]
fn test_strip_html_tags_nested() {
    use super::immanentize::strip_html_tags;

    let html = "<div><section><p>Text inside <em>nested</em> tags</p></section></div>";
    let result = strip_html_tags(html);
    assert_eq!(result, "Text inside nested tags");
}

#[test]
fn test_strip_html_tags_preserves_text() {
    use super::immanentize::strip_html_tags;

    let text = "Plain text without any HTML tags";
    let result = strip_html_tags(text);
    assert_eq!(result, text);
}

#[test]
fn test_strip_html_tags_normalizes_whitespace() {
    use super::immanentize::strip_html_tags;

    let html = "<p>Text   with    multiple   spaces</p>";
    let result = strip_html_tags(html);
    assert_eq!(result, "Text with multiple spaces");
}

#[test]
fn test_strip_html_tags_empty_content() {
    use super::immanentize::strip_html_tags;

    let html = "<div></div><p></p>";
    let result = strip_html_tags(html);
    assert_eq!(result, "");
}

#[test]
fn test_find_sentence_start_at_period() {
    use super::immanentize::find_sentence_start;

    let text = "First sentence. Second sentence with keyword.";
    // Position 16 is 'S' of "Second"
    let start = find_sentence_start(text, 16);
    assert_eq!(start, 16); // Should start at 'S' after skipping whitespace
}

#[test]
fn test_find_sentence_start_at_beginning() {
    use super::immanentize::find_sentence_start;

    let text = "First sentence with keyword.";
    let start = find_sentence_start(text, 6);
    assert_eq!(start, 0); // Should return 0 if no sentence ender before
}

#[test]
fn test_find_sentence_end_at_period() {
    use super::immanentize::find_sentence_end;

    let text = "This is a sentence. Another sentence.";
    // Position 0 is 'T' of "This"
    let end = find_sentence_end(text, 0);
    assert_eq!(end, 19); // Position after the period
}

#[test]
fn test_find_sentence_end_question_mark() {
    use super::immanentize::find_sentence_end;

    let text = "Is this a question? Yes it is.";
    let end = find_sentence_end(text, 0);
    assert_eq!(end, 19); // Position after '?'
}

#[test]
fn test_find_sentence_end_exclamation() {
    use super::immanentize::find_sentence_end;

    let text = "This is exciting! More text.";
    let end = find_sentence_end(text, 0);
    assert_eq!(end, 17); // Position after '!'
}

#[test]
fn test_find_sentence_end_no_ender() {
    use super::immanentize::find_sentence_end;

    let text = "Text without sentence ender";
    let end = find_sentence_end(text, 0);
    assert_eq!(end, text.len()); // Should return text length
}

#[test]
fn test_extract_sentence_with_keyword_basic() {
    use super::immanentize::extract_sentence_with_keyword;

    let text = "First sentence. The Ukraine conflict continues. Third sentence.";
    let result = extract_sentence_with_keyword(text, "Ukraine");

    assert!(result.is_some());
    let sentence = result.unwrap();
    assert!(sentence.contains("Ukraine"));
    assert_eq!(sentence, "The Ukraine conflict continues.");
}

#[test]
fn test_extract_sentence_with_keyword_case_insensitive() {
    use super::immanentize::extract_sentence_with_keyword;

    let text = "The UKRAINE conflict is ongoing.";
    let result = extract_sentence_with_keyword(text, "ukraine");

    assert!(result.is_some());
    assert!(result.unwrap().contains("UKRAINE"));
}

#[test]
fn test_extract_sentence_with_keyword_not_found() {
    use super::immanentize::extract_sentence_with_keyword;

    let text = "This text does not contain the keyword.";
    let result = extract_sentence_with_keyword(text, "nonexistent");

    assert!(result.is_none());
}

#[test]
fn test_extract_sentence_with_keyword_html() {
    use super::immanentize::extract_sentence_with_keyword;

    let text = "<p>First paragraph.</p><p>The <strong>Ukraine</strong> crisis deepens.</p>";
    let result = extract_sentence_with_keyword(text, "Ukraine");

    assert!(result.is_some());
    let sentence = result.unwrap();
    assert!(sentence.contains("Ukraine"));
    assert!(!sentence.contains("<")); // No HTML tags
}

#[test]
fn test_extract_sentence_with_keyword_truncation() {
    use super::immanentize::extract_sentence_with_keyword;

    // Create a very long sentence (> 200 chars)
    let long_text = format!(
        "This is a very long sentence about Ukraine that contains {} padding text to make it exceed the maximum length limit.",
        "lots of extra words and ".repeat(10)
    );

    let result = extract_sentence_with_keyword(&long_text, "Ukraine");

    assert!(result.is_some());
    let sentence = result.unwrap();
    assert!(sentence.ends_with("..."));
    assert!(sentence.len() <= 203); // 200 + "..."
}

#[test]
fn test_truncate_at_word_boundary_short_text() {
    use super::immanentize::truncate_at_word_boundary;

    let text = "Short text";
    let result = truncate_at_word_boundary(text, 100);
    assert_eq!(result, text);
}

#[test]
fn test_truncate_at_word_boundary_at_space() {
    use super::immanentize::truncate_at_word_boundary;

    let text = "This is a longer sentence that needs truncation";
    let result = truncate_at_word_boundary(text, 20);
    // Should truncate at last space before position 20
    assert!(!result.ends_with(' '));
    assert!(result.len() <= 20);
}

#[test]
fn test_truncate_at_word_boundary_no_space() {
    use super::immanentize::truncate_at_word_boundary;

    let text = "Supercalifragilisticexpialidocious";
    let result = truncate_at_word_boundary(text, 10);
    assert_eq!(result.len(), 10);
}

// ============================================================
// Synonym assignment validation tests (logic simulation)
// ============================================================

#[test]
fn test_assign_synonym_self_reference_prevented() {
    // Test that assigning a keyword as its own synonym is prevented
    let synonym_id: i64 = 42;
    let canonical_id: i64 = 42;

    let result = if synonym_id == canonical_id {
        Err("Synonym und Canonical duerfen nicht gleich sein".to_string())
    } else {
        Ok(())
    };

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("nicht gleich"));
}

#[test]
fn test_assign_synonym_different_ids_allowed() {
    // Test that different IDs pass the self-reference check
    let synonym_id: i64 = 42;
    let canonical_id: i64 = 100;

    let passes_check = synonym_id != canonical_id;
    assert!(passes_check);
}

#[test]
fn test_assign_synonym_chain_prevention_logic() {
    // Simulate the chain prevention logic
    // If canonical already has a parent (canonical_id), reject

    struct MockKeyword {
        id: i64,
        canonical_id: Option<i64>,
    }

    let keywords = vec![
        MockKeyword { id: 1, canonical_id: None },      // Root keyword
        MockKeyword { id: 2, canonical_id: Some(1) },   // Synonym of 1
        MockKeyword { id: 3, canonical_id: None },      // Independent
    ];

    // Try to assign 3 as synonym of 2 (2 already has canonical_id = 1)
    let canonical = keywords.iter().find(|k| k.id == 2).unwrap();

    let chain_would_form = canonical.canonical_id.is_some();
    assert!(chain_would_form, "Should detect that canonical already has a parent");

    // Try to assign 2 as synonym of 1 (1 has no parent) - should be allowed
    let canonical = keywords.iter().find(|k| k.id == 1).unwrap();
    let chain_would_form = canonical.canonical_id.is_some();
    assert!(!chain_would_form, "Should allow since 1 has no parent");
}

#[test]
fn test_assign_synonym_updates_correct_fields() {
    // Verify the expected field updates when assigning a synonym
    struct MockKeyword {
        id: i64,
        canonical_id: Option<i64>,
        is_canonical: bool,
    }

    let mut synonym = MockKeyword {
        id: 5,
        canonical_id: None,
        is_canonical: true,
    };

    let canonical_id: i64 = 10;

    // Simulate the assignment
    synonym.canonical_id = Some(canonical_id);
    synonym.is_canonical = false;

    assert_eq!(synonym.canonical_id, Some(10));
    assert!(!synonym.is_canonical);
}

#[test]
fn test_keyword_context_struct_fields() {
    use super::immanentize::KeywordContext;

    // Test KeywordContext struct can be created with all fields
    let context = KeywordContext {
        sentence: Some("This is a test sentence about Ukraine.".to_string()),
        article_title: Some("Test Article Title".to_string()),
        article_date: Some("2024-01-15".to_string()),
    };

    assert!(context.sentence.is_some());
    assert!(context.article_title.is_some());
    assert!(context.article_date.is_some());
}

#[test]
fn test_keyword_context_empty_response() {
    use super::immanentize::KeywordContext;

    // When no article is found, all fields should be None
    let context = KeywordContext {
        sentence: None,
        article_title: None,
        article_date: None,
    };

    assert!(context.sentence.is_none());
    assert!(context.article_title.is_none());
    assert!(context.article_date.is_none());
}

#[test]
fn test_keyword_context_serialize() {
    use super::immanentize::KeywordContext;

    let context = KeywordContext {
        sentence: Some("Test sentence.".to_string()),
        article_title: Some("Article".to_string()),
        article_date: Some("2024-01-01".to_string()),
    };

    let json = serde_json::to_string(&context).expect("Serialization failed");
    assert!(json.contains("\"sentence\""));
    assert!(json.contains("\"article_title\""));
    assert!(json.contains("\"article_date\""));
}

// ============================================================
// AI Provider Cost Tracking struct tests
// ============================================================

#[test]
fn test_monthly_cost_serialize() {
    use super::ai::model_management::MonthlyCost;

    let cost = MonthlyCost {
        spent: 1.23,
        limit: 5.0,
        remaining: 3.77,
        percentage: 24.6,
    };

    let json = serde_json::to_string(&cost).expect("Serialization failed");
    assert!(json.contains("\"spent\":1.23"));
    assert!(json.contains("\"limit\":5.0"));
    assert!(json.contains("\"remaining\":3.77"));
    assert!(json.contains("\"percentage\":24.6"));
}

#[test]
fn test_monthly_cost_zero_spend() {
    use super::ai::model_management::MonthlyCost;

    let cost = MonthlyCost {
        spent: 0.0,
        limit: 5.0,
        remaining: 5.0,
        percentage: 0.0,
    };

    assert_eq!(cost.spent, 0.0);
    assert_eq!(cost.remaining, cost.limit);
    assert_eq!(cost.percentage, 0.0);
}

#[test]
fn test_monthly_cost_at_limit() {
    use super::ai::model_management::MonthlyCost;

    let cost = MonthlyCost {
        spent: 5.0,
        limit: 5.0,
        remaining: 0.0,
        percentage: 100.0,
    };

    assert_eq!(cost.spent, cost.limit);
    assert_eq!(cost.remaining, 0.0);
    assert_eq!(cost.percentage, 100.0);
}

#[test]
fn test_cost_entry_serialize() {
    use super::ai::model_management::CostEntry;

    let entry = CostEntry {
        id: 1,
        provider: "openai_compatible".to_string(),
        model: "gpt-4.1-nano".to_string(),
        input_tokens: 1500,
        output_tokens: 500,
        estimated_cost_usd: 0.0023,
        created_at: "2026-02-06 10:00:00".to_string(),
    };

    let json = serde_json::to_string(&entry).expect("Serialization failed");
    assert!(json.contains("\"provider\":\"openai_compatible\""));
    assert!(json.contains("\"model\":\"gpt-4.1-nano\""));
    assert!(json.contains("\"input_tokens\":1500"));
    assert!(json.contains("\"output_tokens\":500"));
    assert!(json.contains("\"estimated_cost_usd\":0.0023"));
}

#[test]
fn test_cost_entry_ollama_provider() {
    use super::ai::model_management::CostEntry;

    let entry = CostEntry {
        id: 42,
        provider: "ollama".to_string(),
        model: "ministral-3:latest".to_string(),
        input_tokens: 0,
        output_tokens: 0,
        estimated_cost_usd: 0.0,
        created_at: "2026-01-01 00:00:00".to_string(),
    };

    assert_eq!(entry.provider, "ollama");
    assert_eq!(entry.estimated_cost_usd, 0.0);
}

// ============================================================
// ProviderTestResult struct tests
// ============================================================

#[test]
fn test_provider_test_result_success_serialize() {
    use crate::ai_provider::ProviderTestResult;

    let result = ProviderTestResult {
        success: true,
        latency_ms: 150,
        models: vec!["model-a".to_string(), "model-b".to_string()],
        error: None,
    };

    let json = serde_json::to_string(&result).expect("Serialization failed");
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"latency_ms\":150"));
    assert!(json.contains("\"model-a\""));
    assert!(json.contains("\"model-b\""));
    assert!(json.contains("\"error\":null"));
}

#[test]
fn test_provider_test_result_failure_serialize() {
    use crate::ai_provider::ProviderTestResult;

    let result = ProviderTestResult {
        success: false,
        latency_ms: 5000,
        models: vec![],
        error: Some("Connection timeout".to_string()),
    };

    let json = serde_json::to_string(&result).expect("Serialization failed");
    assert!(json.contains("\"success\":false"));
    assert!(json.contains("Connection timeout"));
}

#[test]
fn test_provider_test_result_auth_error() {
    use crate::ai_provider::ProviderTestResult;

    let result = ProviderTestResult {
        success: false,
        latency_ms: 200,
        models: vec![],
        error: Some("Authentication failed - check API key".to_string()),
    };

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Authentication"));
}
