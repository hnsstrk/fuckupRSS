//! Ollama-based AI text provider
//!
//! Thin wrapper around OllamaClient for the AiTextProvider trait.
//! Automatically prepends `/no_think` to prompts (Ollama-specific optimization).

use async_trait::async_trait;

use crate::ollama::OllamaClient;

use super::{AiProviderError, AiTextProvider, EmbeddingProvider, GenerationResult};

/// Ollama text generation provider (local or remote)
pub struct OllamaTextProvider {
    base_url: String,
    num_ctx: u32,
}

impl OllamaTextProvider {
    pub fn new(base_url: &str, num_ctx: u32) -> Self {
        Self {
            base_url: base_url.to_string(),
            num_ctx,
        }
    }

    fn client(&self) -> OllamaClient {
        OllamaClient::with_context(Some(self.base_url.clone()), self.num_ctx)
    }
}

#[async_trait]
impl AiTextProvider for OllamaTextProvider {
    async fn generate_text(
        &self,
        model: &str,
        prompt: &str,
        json_mode: bool,
    ) -> Result<GenerationResult, AiProviderError> {
        let client = self.client();

        // Prepend /no_think for Ollama models (optimizes thinking-capable models)
        let full_prompt = if prompt.starts_with("/no_think") {
            prompt.to_string()
        } else {
            format!("/no_think\n{}", prompt)
        };

        let result = if json_mode {
            client.generate_simple(model, &full_prompt).await
        } else {
            client.summarize_with_prompt(model, "", &full_prompt).await
        };

        match result {
            Ok(text) => Ok(GenerationResult {
                text,
                // Ollama doesn't return token counts in non-streaming mode
                input_tokens: None,
                output_tokens: None,
            }),
            Err(e) => Err(match e {
                crate::ollama::OllamaError::NotAvailable(msg) => AiProviderError::NotAvailable(msg),
                crate::ollama::OllamaError::GenerationFailed(msg) => {
                    AiProviderError::GenerationFailed(msg)
                }
                crate::ollama::OllamaError::JsonParseError {
                    message,
                    raw_response,
                } => AiProviderError::JsonParseError {
                    message,
                    raw_response,
                },
                crate::ollama::OllamaError::PullFailed(msg) => {
                    AiProviderError::GenerationFailed(msg)
                }
            }),
        }
    }

    async fn is_available(&self) -> bool {
        let client = self.client();
        client.is_available().await
    }

    fn provider_name(&self) -> &str {
        "Ollama"
    }

    fn suggested_concurrency(&self) -> usize {
        1
    }
}

/// Ollama-based embedding provider
///
/// Wraps `OllamaClient::generate_embedding()` to implement the `EmbeddingProvider` trait.
pub struct OllamaEmbeddingProvider {
    base_url: String,
    model: String,
    dimensions: usize,
}

impl OllamaEmbeddingProvider {
    pub fn new(base_url: &str, model: &str, dimensions: usize) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
            dimensions,
        }
    }

    fn client(&self) -> OllamaClient {
        OllamaClient::new(Some(self.base_url.clone()))
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AiProviderError> {
        let client = self.client();
        client
            .generate_embedding(&self.model, text)
            .await
            .map_err(|e| match e {
                crate::ollama::OllamaError::NotAvailable(msg) => AiProviderError::NotAvailable(msg),
                crate::ollama::OllamaError::GenerationFailed(msg) => {
                    AiProviderError::GenerationFailed(msg)
                }
                other => AiProviderError::GenerationFailed(other.to_string()),
            })
    }

    fn embedding_dimensions(&self) -> usize {
        self.dimensions
    }

    fn provider_name(&self) -> &str {
        "Ollama Embedding"
    }
}
