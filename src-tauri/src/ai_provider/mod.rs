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
