//! Tests for Tauri commands, with focus on batch processing

use std::sync::atomic::{AtomicBool, Ordering};

// ============================================================
// BatchProgress struct tests
// ============================================================

#[test]
fn test_batch_progress_serialize() {
    use super::ollama::BatchProgress;

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
    use super::ollama::BatchProgress;

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
    use super::ollama::BatchProgress;

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
    use super::ollama::BatchProgress;

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
    use super::ollama::BatchResult;

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
    use super::ollama::BatchResult;

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
    use super::ollama::BatchResult;

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
    use super::ollama::UnprocessedCount;

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
    use super::ollama::UnprocessedCount;

    let count = UnprocessedCount {
        total: 100,
        with_content: 100,
    };

    assert_eq!(count.total, count.with_content);
}

#[test]
fn test_unprocessed_count_none_have_content() {
    use super::ollama::UnprocessedCount;

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
    use super::ollama::OllamaStatus;

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
    use super::ollama::OllamaStatus;

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
    use super::ollama::OllamaStatus;

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
    use super::ollama::SummaryResponse;

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
    use super::ollama::SummaryResponse;

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
    use super::ollama::AnalysisResponse;
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
    use super::ollama::DiscordianResponse;
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
    use super::ollama::DiscordianResponse;

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
    use super::ollama::PromptTemplates;

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
    use super::ollama::DefaultPrompts;

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
    use super::ollama::ModelPullResult;

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
    use super::ollama::ModelPullResult;

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
    use super::ollama::{BatchProgress, BatchResult};

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
    use super::ollama::BatchResult;

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
