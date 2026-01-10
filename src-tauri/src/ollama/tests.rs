//! Ollama module tests

use super::*;

// ============================================================
// Language locale tests
// ============================================================

#[test]
fn test_get_language_for_locale_german() {
    assert_eq!(get_language_for_locale("de"), "German");
}

#[test]
fn test_get_language_for_locale_english() {
    assert_eq!(get_language_for_locale("en"), "English");
}

#[test]
fn test_get_language_for_locale_unknown() {
    // Unknown locale defaults to German
    assert_eq!(get_language_for_locale("fr"), "German");
    assert_eq!(get_language_for_locale(""), "German");
    assert_eq!(get_language_for_locale("xyz"), "German");
}

// ============================================================
// BiasAnalysis conversion tests
// ============================================================

#[test]
fn test_raw_bias_analysis_conversion() {
    let raw = RawBiasAnalysis {
        political_bias: 1.7,
        sachlichkeit: 3.2,
        article_type: "news".to_string(),
    };

    let analysis: BiasAnalysis = raw.into();

    assert_eq!(analysis.political_bias, 2); // 1.7 rounds to 2
    assert_eq!(analysis.sachlichkeit, 3); // 3.2 rounds to 3
    assert_eq!(analysis.article_type, "news");
}

#[test]
fn test_raw_bias_analysis_negative() {
    let raw = RawBiasAnalysis {
        political_bias: -1.6,
        sachlichkeit: 0.4,
        article_type: "opinion".to_string(),
    };

    let analysis: BiasAnalysis = raw.into();

    assert_eq!(analysis.political_bias, -2); // -1.6 rounds to -2
    assert_eq!(analysis.sachlichkeit, 0); // 0.4 rounds to 0
}

#[test]
fn test_raw_bias_analysis_exact_values() {
    let raw = RawBiasAnalysis {
        political_bias: 0.0,
        sachlichkeit: 4.0,
        article_type: "analysis".to_string(),
    };

    let analysis: BiasAnalysis = raw.into();

    assert_eq!(analysis.political_bias, 0);
    assert_eq!(analysis.sachlichkeit, 4);
}

// ============================================================
// DiscordianAnalysis conversion tests
// ============================================================

#[test]
fn test_raw_discordian_analysis_conversion() {
    let raw = RawDiscordianAnalysis {
        summary: "Test summary".to_string(),
        categories: vec!["Politik".to_string(), "Wirtschaft".to_string()],
        keywords: vec!["keyword1".to_string(), "keyword2".to_string()],
        political_bias: -0.8,
        sachlichkeit: 2.6,
        article_type: "news".to_string(),
    };

    let analysis: DiscordianAnalysis = raw.into();

    assert_eq!(analysis.summary, "Test summary");
    assert_eq!(analysis.categories.len(), 2);
    assert_eq!(analysis.keywords.len(), 2);
    assert_eq!(analysis.political_bias, -1); // -0.8 rounds to -1
    assert_eq!(analysis.sachlichkeit, 3); // 2.6 rounds to 3
    assert_eq!(analysis.article_type, "news");
}

#[test]
fn test_raw_discordian_analysis_empty_collections() {
    let raw = RawDiscordianAnalysis {
        summary: "Summary".to_string(),
        categories: vec![],
        keywords: vec![],
        political_bias: 0.0,
        sachlichkeit: 2.0,
        article_type: "unknown".to_string(),
    };

    let analysis: DiscordianAnalysis = raw.into();

    assert!(analysis.categories.is_empty());
    assert!(analysis.keywords.is_empty());
}

// ============================================================
// OllamaClient tests
// ============================================================

#[test]
fn test_ollama_client_creation() {
    let client = OllamaClient::new(None);
    assert_eq!(client.base_url, "http://localhost:11434");
}

#[test]
fn test_ollama_client_custom_url() {
    let client = OllamaClient::new(Some("http://custom:8080".to_string()));
    assert_eq!(client.base_url, "http://custom:8080");
}

#[test]
fn test_ollama_client_default() {
    let client = OllamaClient::default();
    assert_eq!(client.base_url, "http://localhost:11434");
}

// ============================================================
// Constants tests
// ============================================================

#[test]
fn test_recommended_models() {
    assert!(!RECOMMENDED_MAIN_MODEL.is_empty());
    assert!(!RECOMMENDED_EMBEDDING_MODEL.is_empty());
    assert!(RECOMMENDED_MAIN_MODEL.contains("ministral") || RECOMMENDED_MAIN_MODEL.contains("qwen"));
    // snowflake-arctic-embed2 for multilingual (DE/EN) embeddings
    assert!(RECOMMENDED_EMBEDDING_MODEL.contains("snowflake") || RECOMMENDED_EMBEDDING_MODEL.contains("arctic"));
}

#[test]
fn test_default_summary_prompt() {
    assert!(DEFAULT_SUMMARY_PROMPT.contains("{language}"));
    assert!(DEFAULT_SUMMARY_PROMPT.contains("{content}"));
    assert!(DEFAULT_SUMMARY_PROMPT.contains("summary"));
}

#[test]
fn test_default_analysis_prompt() {
    assert!(DEFAULT_ANALYSIS_PROMPT.contains("{title}"));
    assert!(DEFAULT_ANALYSIS_PROMPT.contains("{content}"));
    assert!(DEFAULT_ANALYSIS_PROMPT.contains("political_bias"));
    assert!(DEFAULT_ANALYSIS_PROMPT.contains("sachlichkeit"));
    assert!(DEFAULT_ANALYSIS_PROMPT.contains("article_type"));
}

#[test]
fn test_default_discordian_prompt() {
    assert!(DEFAULT_DISCORDIAN_PROMPT.contains("{language}"));
    assert!(DEFAULT_DISCORDIAN_PROMPT.contains("{title}"));
    assert!(DEFAULT_DISCORDIAN_PROMPT.contains("{content}"));
    assert!(DEFAULT_DISCORDIAN_PROMPT.contains("summary"));
    assert!(DEFAULT_DISCORDIAN_PROMPT.contains("categories"));
    assert!(DEFAULT_DISCORDIAN_PROMPT.contains("keywords"));
}

// ============================================================
// Error type tests
// ============================================================

#[test]
fn test_ollama_error_display() {
    let error = OllamaError::NotAvailable("test".to_string());
    assert!(format!("{}", error).contains("test"));

    let error = OllamaError::GenerationFailed("gen error".to_string());
    assert!(format!("{}", error).contains("gen error"));

    let error = OllamaError::PullFailed("pull error".to_string());
    assert!(format!("{}", error).contains("pull error"));
}

// ============================================================
// Struct serialization tests
// ============================================================

#[test]
fn test_bias_analysis_serialize() {
    let analysis = BiasAnalysis {
        political_bias: -1,
        sachlichkeit: 3,
        article_type: "news".to_string(),
    };

    let json = serde_json::to_string(&analysis).expect("Serialization failed");
    assert!(json.contains("political_bias"));
    assert!(json.contains("-1"));
}

#[test]
fn test_discordian_analysis_serialize() {
    let analysis = DiscordianAnalysis {
        summary: "Test".to_string(),
        categories: vec!["Politik".to_string()],
        keywords: vec!["keyword".to_string()],
        political_bias: 0,
        sachlichkeit: 2,
        article_type: "analysis".to_string(),
    };

    let json = serde_json::to_string(&analysis).expect("Serialization failed");
    assert!(json.contains("summary"));
    assert!(json.contains("Politik"));
    assert!(json.contains("keyword"));
}

#[test]
fn test_model_info_deserialize() {
    let json = r#"{"name": "test-model", "size": 1000000}"#;
    let info: ModelInfo = serde_json::from_str(json).expect("Deserialization failed");
    assert_eq!(info.name, "test-model");
    assert_eq!(info.size, Some(1000000));
}

#[test]
fn test_model_info_deserialize_no_size() {
    let json = r#"{"name": "test-model"}"#;
    let info: ModelInfo = serde_json::from_str(json).expect("Deserialization failed");
    assert_eq!(info.name, "test-model");
    assert_eq!(info.size, None);
}
