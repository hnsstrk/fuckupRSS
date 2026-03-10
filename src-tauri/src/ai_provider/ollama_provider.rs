//! Ollama-based AI text provider
//!
//! Thin wrapper around OllamaClient for the AiTextProvider trait.
//! Automatically prepends `/no_think` to prompts (Ollama-specific optimization).

use async_trait::async_trait;

use crate::ollama::OllamaClient;

use super::{AiProviderError, AiTextProvider, EmbeddingProvider, GenerationResult};

/// Ollama text generation provider (local or remote)
pub struct OllamaTextProvider {
    client: OllamaClient,
    concurrency: usize,
}

impl OllamaTextProvider {
    pub fn new(base_url: &str, num_ctx: u32, concurrency: usize) -> Self {
        Self {
            client: OllamaClient::with_context(Some(base_url.to_string()), num_ctx),
            concurrency: concurrency.max(1),
        }
    }
}

#[async_trait]
impl AiTextProvider for OllamaTextProvider {
    async fn generate_text(
        &self,
        model: &str,
        prompt: &str,
        json_schema: Option<serde_json::Value>,
    ) -> Result<GenerationResult, AiProviderError> {
        // Prepend /no_think to the system message for Ollama models
        // (optimizes thinking-capable models)
        let no_think_system = "/no_think";

        let result = if let Some(schema) = json_schema {
            // JSON mode: pass schema as format, prompt as user message
            self.client
                .chat(model, Some(no_think_system), prompt, Some(schema))
                .await
        } else {
            // Freetext mode: no schema
            self.client
                .chat(model, Some(no_think_system), prompt, None)
                .await
        };

        match result {
            Ok(text) => Ok(GenerationResult {
                text,
                // Ollama doesn't return token counts in non-streaming mode
                input_tokens: None,
                output_tokens: None,
            }),
            Err(e) => Err(match e {
                crate::ollama::OllamaError::NotAvailable(msg) => {
                    AiProviderError::NotAvailable(msg)
                }
                crate::ollama::OllamaError::GenerationFailed(msg) => {
                    AiProviderError::GenerationFailed(msg)
                }
                crate::ollama::OllamaError::PullFailed(msg) => {
                    AiProviderError::GenerationFailed(msg)
                }
            }),
        }
    }

    async fn is_available(&self) -> bool {
        self.client.is_available().await
    }

    fn provider_name(&self) -> &str {
        "Ollama"
    }

    fn suggested_concurrency(&self) -> usize {
        self.concurrency
    }
}

/// Ollama-based embedding provider
///
/// Wraps `OllamaClient::generate_embedding()` to implement the `EmbeddingProvider` trait.
pub struct OllamaEmbeddingProvider {
    client: OllamaClient,
    model: String,
    dimensions: usize,
}

impl OllamaEmbeddingProvider {
    pub fn new(base_url: &str, model: &str, dimensions: usize) -> Self {
        Self {
            client: OllamaClient::new(Some(base_url.to_string())),
            model: model.to_string(),
            dimensions,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AiProviderError> {
        self.client
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

    async fn generate_embeddings_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, AiProviderError> {
        self.client
            .generate_embeddings_batch(&self.model, texts)
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
