//! AI Provider abstraction for text generation
//!
//! Supports multiple backends:
//! - Ollama (local or remote)
//! - OpenAI-compatible APIs (OpenAI, Together.ai, Mistral, Groq, etc.)
//!
//! Embeddings always go through Ollama (via OllamaClient directly).

pub mod ollama_provider;
pub mod openai_provider;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Default model for OpenAI-compatible providers
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-5-nano";

/// Provider type enum for serialization/settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Ollama,
    OpenAiCompatible,
}

impl ProviderType {
    pub fn from_str_setting(s: &str) -> Self {
        match s {
            "openai_compatible" => ProviderType::OpenAiCompatible,
            _ => ProviderType::Ollama,
        }
    }

    pub fn to_setting_str(&self) -> &'static str {
        match self {
            ProviderType::Ollama => "ollama",
            ProviderType::OpenAiCompatible => "openai_compatible",
        }
    }
}

/// Configuration for creating a provider instance
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    /// Ollama base URL (used for OllamaProvider)
    pub ollama_url: String,
    /// Ollama model name (used for OllamaProvider)
    pub ollama_model: String,
    /// Ollama num_ctx setting
    pub ollama_num_ctx: u32,
    /// OpenAI-compatible API base URL
    pub openai_base_url: String,
    /// API key for OpenAI-compatible provider
    pub openai_api_key: String,
    /// Model name for OpenAI-compatible provider
    pub openai_model: String,
}

/// Errors from AI providers
#[derive(Error, Debug)]
pub enum AiProviderError {
    #[error("Provider not available: {0}")]
    NotAvailable(String),
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
    #[error("Rate limit exceeded")]
    RateLimited,
    #[error("Cost limit reached: {spent:.4}/{limit:.4} USD")]
    CostLimitReached { spent: f64, limit: f64 },
    #[error("JSON parse error: {message}")]
    JsonParseError {
        message: String,
        raw_response: String,
    },
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

/// Result of a text generation call, including token usage for cost tracking
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub text: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

/// Trait for AI text generation providers
///
/// Embeddings are NOT part of this trait - they always go through OllamaClient.
#[async_trait]
pub trait AiTextProvider: Send + Sync {
    /// Generate text from a prompt
    ///
    /// If `json_mode` is true, the provider should request structured JSON output.
    async fn generate_text(
        &self,
        model: &str,
        prompt: &str,
        json_mode: bool,
    ) -> Result<GenerationResult, AiProviderError>;

    /// Check if the provider is currently available
    async fn is_available(&self) -> bool;

    /// Human-readable provider name
    fn provider_name(&self) -> &str;
}

/// Resolves the effective model name based on provider type.
/// For Ollama: allows frontend model override.
/// For OpenAI-compatible: always uses the configured model.
pub fn resolve_effective_model(
    provider_name: &str,
    frontend_model: &str,
    config_model: &str,
) -> String {
    if provider_name == "Ollama" && !frontend_model.is_empty() {
        frontend_model.to_string()
    } else {
        config_model.to_string()
    }
}

/// Create a text provider based on configuration
pub fn create_provider(config: &ProviderConfig) -> Arc<dyn AiTextProvider> {
    match config.provider_type {
        ProviderType::Ollama => Arc::new(ollama_provider::OllamaTextProvider::new(
            &config.ollama_url,
            config.ollama_num_ctx,
        )),
        ProviderType::OpenAiCompatible => Arc::new(openai_provider::OpenAiCompatibleProvider::new(
            &config.openai_base_url,
            &config.openai_api_key,
        )),
    }
}

/// Result of a provider connection test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub models: Vec<String>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // ProviderType tests
    // ============================================================

    #[test]
    fn test_provider_type_from_str_ollama() {
        assert_eq!(ProviderType::from_str_setting("ollama"), ProviderType::Ollama);
    }

    #[test]
    fn test_provider_type_from_str_openai() {
        assert_eq!(
            ProviderType::from_str_setting("openai_compatible"),
            ProviderType::OpenAiCompatible
        );
    }

    #[test]
    fn test_provider_type_from_str_unknown_defaults_to_ollama() {
        assert_eq!(ProviderType::from_str_setting("unknown"), ProviderType::Ollama);
        assert_eq!(ProviderType::from_str_setting(""), ProviderType::Ollama);
        assert_eq!(ProviderType::from_str_setting("openai"), ProviderType::Ollama);
    }

    #[test]
    fn test_provider_type_to_setting_str() {
        assert_eq!(ProviderType::Ollama.to_setting_str(), "ollama");
        assert_eq!(
            ProviderType::OpenAiCompatible.to_setting_str(),
            "openai_compatible"
        );
    }

    #[test]
    fn test_provider_type_roundtrip() {
        let ollama = ProviderType::Ollama;
        assert_eq!(ProviderType::from_str_setting(ollama.to_setting_str()), ollama);

        let openai = ProviderType::OpenAiCompatible;
        assert_eq!(ProviderType::from_str_setting(openai.to_setting_str()), openai);
    }

    #[test]
    fn test_provider_type_serde_serialize() {
        let json = serde_json::to_string(&ProviderType::Ollama).unwrap();
        assert_eq!(json, "\"ollama\"");

        let json = serde_json::to_string(&ProviderType::OpenAiCompatible).unwrap();
        assert_eq!(json, "\"open_ai_compatible\"");
    }

    #[test]
    fn test_provider_type_serde_deserialize() {
        let ollama: ProviderType = serde_json::from_str("\"ollama\"").unwrap();
        assert_eq!(ollama, ProviderType::Ollama);

        let openai: ProviderType = serde_json::from_str("\"open_ai_compatible\"").unwrap();
        assert_eq!(openai, ProviderType::OpenAiCompatible);
    }

    // ============================================================
    // ProviderConfig tests
    // ============================================================

    #[test]
    fn test_provider_config_defaults() {
        let config = ProviderConfig {
            provider_type: ProviderType::Ollama,
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "ministral-3:latest".to_string(),
            ollama_num_ctx: 4096,
            openai_base_url: "https://api.openai.com".to_string(),
            openai_api_key: "".to_string(),
            openai_model: "gpt-5-nano".to_string(),
        };

        assert_eq!(config.provider_type, ProviderType::Ollama);
        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert_eq!(config.ollama_num_ctx, 4096);
    }

    #[test]
    fn test_provider_config_clone() {
        let config = ProviderConfig {
            provider_type: ProviderType::OpenAiCompatible,
            ollama_url: "http://192.168.1.100:11434".to_string(),
            ollama_model: "test".to_string(),
            ollama_num_ctx: 8192,
            openai_base_url: "https://api.together.xyz".to_string(),
            openai_api_key: "sk-test-key".to_string(),
            openai_model: "meta-llama/Llama-3-70b".to_string(),
        };

        let cloned = config.clone();
        assert_eq!(cloned.provider_type, ProviderType::OpenAiCompatible);
        assert_eq!(cloned.openai_api_key, "sk-test-key");
        assert_eq!(cloned.openai_base_url, "https://api.together.xyz");
    }

    // ============================================================
    // Factory tests
    // ============================================================

    #[test]
    fn test_create_provider_ollama() {
        let config = ProviderConfig {
            provider_type: ProviderType::Ollama,
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "test".to_string(),
            ollama_num_ctx: 4096,
            openai_base_url: String::new(),
            openai_api_key: String::new(),
            openai_model: String::new(),
        };

        let provider = create_provider(&config);
        assert_eq!(provider.provider_name(), "Ollama");
    }

    #[test]
    fn test_create_provider_openai() {
        let config = ProviderConfig {
            provider_type: ProviderType::OpenAiCompatible,
            ollama_url: String::new(),
            ollama_model: String::new(),
            ollama_num_ctx: 4096,
            openai_base_url: "https://api.openai.com".to_string(),
            openai_api_key: "sk-test".to_string(),
            openai_model: "gpt-5-nano".to_string(),
        };

        let provider = create_provider(&config);
        assert_eq!(provider.provider_name(), "OpenAI-compatible");
    }

    // ============================================================
    // GenerationResult tests
    // ============================================================

    #[test]
    fn test_generation_result_with_tokens() {
        let result = GenerationResult {
            text: "Hello world".to_string(),
            input_tokens: Some(10),
            output_tokens: Some(5),
        };

        assert_eq!(result.text, "Hello world");
        assert_eq!(result.input_tokens, Some(10));
        assert_eq!(result.output_tokens, Some(5));
    }

    #[test]
    fn test_generation_result_without_tokens() {
        let result = GenerationResult {
            text: "Response".to_string(),
            input_tokens: None,
            output_tokens: None,
        };

        assert!(result.input_tokens.is_none());
        assert!(result.output_tokens.is_none());
    }

    #[test]
    fn test_generation_result_clone() {
        let result = GenerationResult {
            text: "Test".to_string(),
            input_tokens: Some(100),
            output_tokens: Some(50),
        };

        let cloned = result.clone();
        assert_eq!(cloned.text, result.text);
        assert_eq!(cloned.input_tokens, result.input_tokens);
    }

    // ============================================================
    // AiProviderError tests
    // ============================================================

    #[test]
    fn test_error_not_available() {
        let err = AiProviderError::NotAvailable("Server down".to_string());
        assert!(err.to_string().contains("Server down"));
    }

    #[test]
    fn test_error_generation_failed() {
        let err = AiProviderError::GenerationFailed("Timeout".to_string());
        assert!(err.to_string().contains("Timeout"));
    }

    #[test]
    fn test_error_rate_limited() {
        let err = AiProviderError::RateLimited;
        assert!(err.to_string().contains("Rate limit"));
    }

    #[test]
    fn test_error_cost_limit_reached() {
        let err = AiProviderError::CostLimitReached {
            spent: 5.1234,
            limit: 5.0,
        };
        let msg = err.to_string();
        assert!(msg.contains("5.1234"));
        assert!(msg.contains("5.0000"));
    }

    #[test]
    fn test_error_json_parse() {
        let err = AiProviderError::JsonParseError {
            message: "invalid JSON".to_string(),
            raw_response: "{bad}".to_string(),
        };
        assert!(err.to_string().contains("invalid JSON"));
    }

    #[test]
    fn test_error_authentication_failed() {
        let err = AiProviderError::AuthenticationFailed("Invalid key".to_string());
        assert!(err.to_string().contains("Invalid key"));
    }

    // ============================================================
    // ProviderTestResult tests
    // ============================================================

    #[test]
    fn test_provider_test_result_success() {
        let result = ProviderTestResult {
            success: true,
            latency_ms: 42,
            models: vec!["gpt-4".to_string(), "gpt-3.5".to_string()],
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"latency_ms\":42"));
        assert!(json.contains("\"gpt-4\""));
    }

    #[test]
    fn test_provider_test_result_failure() {
        let result = ProviderTestResult {
            success: false,
            latency_ms: 0,
            models: vec![],
            error: Some("Connection refused".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("Connection refused"));
    }

    #[test]
    fn test_provider_test_result_deserialize() {
        let json = r#"{"success":true,"latency_ms":100,"models":["test-model"],"error":null}"#;
        let result: ProviderTestResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.latency_ms, 100);
        assert_eq!(result.models.len(), 1);
        assert!(result.error.is_none());
    }

    // ============================================================
    // resolve_effective_model tests
    // ============================================================

    #[test]
    fn test_resolve_effective_model_ollama_with_frontend_model() {
        let result = resolve_effective_model("Ollama", "ministral-3:latest", "default-model");
        assert_eq!(result, "ministral-3:latest");
    }

    #[test]
    fn test_resolve_effective_model_ollama_empty_frontend() {
        let result = resolve_effective_model("Ollama", "", "default-model");
        assert_eq!(result, "default-model");
    }

    #[test]
    fn test_resolve_effective_model_openai_ignores_frontend() {
        let result = resolve_effective_model("OpenAI-compatible", "ministral-3:latest", "gpt-5-nano");
        assert_eq!(result, "gpt-5-nano");
    }

    #[test]
    fn test_resolve_effective_model_openai_empty_frontend() {
        let result = resolve_effective_model("OpenAI-compatible", "", "gpt-5-nano");
        assert_eq!(result, "gpt-5-nano");
    }
}
