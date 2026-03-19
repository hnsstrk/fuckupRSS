//! Ollama-based AI text provider
//!
//! Thin wrapper around OllamaClient for the AiTextProvider trait.
//! Automatically prepends `/no_think` to prompts (Ollama-specific optimization).
//!
//! Includes retry logic with exponential backoff for transient errors
//! (connection failures, timeouts). Does NOT retry on content errors,
//! JSON parse failures, or "model not found" errors.

use async_trait::async_trait;
use log::warn;
use std::time::Duration;

use crate::ollama::{OllamaClient, OllamaError};

use super::{AiProviderError, AiTextProvider, EmbeddingProvider, GenerationResult};

/// Maximum number of retries for transient errors (connection, timeout)
const MAX_RETRIES: u32 = 2;

/// Retry delays for exponential backoff: 2s, then 4s
const RETRY_DELAYS_MS: [u64; 2] = [2000, 4000];

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

    /// Check if an OllamaError is transient and worth retrying.
    ///
    /// Retryable: connection failures, timeouts, server errors (5xx).
    /// NOT retryable: model not found (404), JSON parse errors,
    /// incomplete responses, pull failures.
    fn is_retryable_error(error: &OllamaError) -> bool {
        match error {
            // Connection failures and timeouts are always retryable
            OllamaError::NotAvailable(_) => true,
            // Server errors (5xx) are retryable, client errors (4xx) are not
            OllamaError::GenerationFailed(msg) => {
                // The OllamaClient formats HTTP errors as "Status <code>: <body>"
                if let Some(rest) = msg.strip_prefix("Status ") {
                    if let Some(code_str) = rest.split(':').next() {
                        if let Ok(code) = code_str.trim().parse::<u16>() {
                            return (500..600).contains(&code);
                        }
                    }
                }
                false
            }
            // Incomplete response (done=false) — not retryable, would give same result
            OllamaError::IncompleteResponse => false,
            // Pull failures — not retryable
            OllamaError::PullFailed(_) => false,
        }
    }

    /// Convert an OllamaError into an AiProviderError.
    fn convert_error(error: OllamaError) -> AiProviderError {
        match error {
            OllamaError::NotAvailable(msg) => AiProviderError::NotAvailable(msg),
            OllamaError::GenerationFailed(msg) => AiProviderError::GenerationFailed(msg),
            OllamaError::IncompleteResponse => AiProviderError::GenerationFailed(
                "Incomplete response: generation stopped before completion (done=false)"
                    .to_string(),
            ),
            OllamaError::PullFailed(msg) => AiProviderError::GenerationFailed(msg),
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

        let mut last_error: Option<AiProviderError> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                // Apply exponential backoff delay before retry
                let delay_ms = RETRY_DELAYS_MS[(attempt - 1) as usize];
                let delay = Duration::from_millis(delay_ms);
                warn!(
                    "[Ollama] Retry {}/{} after {:.1}s delay (error: {})",
                    attempt,
                    MAX_RETRIES,
                    delay.as_secs_f64(),
                    last_error
                        .as_ref()
                        .map(|e| e.to_string())
                        .unwrap_or_default()
                );
                tokio::time::sleep(delay).await;
            }

            let result = if let Some(ref schema) = json_schema {
                self.client
                    .chat(model, Some(no_think_system), prompt, Some(schema.clone()))
                    .await
            } else {
                self.client
                    .chat(model, Some(no_think_system), prompt, None)
                    .await
            };

            match result {
                Ok(text) => {
                    if attempt > 0 {
                        warn!("[Ollama] Request succeeded after {} retries", attempt);
                    }
                    return Ok(GenerationResult {
                        text,
                        // Ollama doesn't return token counts in non-streaming mode
                        input_tokens: None,
                        output_tokens: None,
                    });
                }
                Err(e) => {
                    let is_retryable = Self::is_retryable_error(&e);
                    let provider_error = Self::convert_error(e);

                    if !is_retryable || attempt == MAX_RETRIES {
                        if attempt > 0 {
                            warn!(
                                "[Ollama] Request failed after {} retries: {}",
                                attempt, provider_error
                            );
                        }
                        return Err(provider_error);
                    }

                    last_error = Some(provider_error);
                }
            }
        }

        // Should never reach here due to loop logic
        Err(last_error
            .unwrap_or_else(|| AiProviderError::GenerationFailed("Unknown error".to_string())))
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
                OllamaError::NotAvailable(msg) => AiProviderError::NotAvailable(msg),
                OllamaError::GenerationFailed(msg) => AiProviderError::GenerationFailed(msg),
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
                OllamaError::NotAvailable(msg) => AiProviderError::NotAvailable(msg),
                OllamaError::GenerationFailed(msg) => AiProviderError::GenerationFailed(msg),
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
