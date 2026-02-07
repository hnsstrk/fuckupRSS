//! OpenAI-compatible embedding provider
//!
//! Supports OpenAI text-embedding-3-small/large and any compatible API.
//! Features:
//! - Configurable dimensions (OpenAI supports Matryoshka truncation)
//! - Exponential backoff with jitter for rate limits (429) and server errors (5xx)
//! - Cost tracking via token usage in response

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use super::{AiProviderError, EmbeddingProvider};

/// Maximum number of retries for retryable errors (429, 5xx)
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (1 second)
const BASE_RETRY_DELAY_MS: u64 = 1000;

/// OpenAI-compatible embedding provider
pub struct OpenAiEmbeddingProvider {
    base_url: String,
    api_key: String,
    model: String,
    dimensions: usize,
    client: reqwest_new::Client,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    #[serde(default)]
    usage: Option<EmbeddingUsage>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbeddingUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

impl OpenAiEmbeddingProvider {
    pub fn new(base_url: &str, api_key: &str, model: &str, dimensions: usize) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        let client = reqwest_new::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        Self {
            base_url,
            api_key: api_key.to_string(),
            model: model.to_string(),
            dimensions,
            client,
        }
    }

    fn endpoint_url(&self) -> String {
        format!("{}/v1/embeddings", self.base_url)
    }

    /// Calculate retry delay with exponential backoff and jitter.
    fn retry_delay(attempt: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(server_delay) = retry_after {
            return server_delay;
        }

        let base_ms = BASE_RETRY_DELAY_MS * 2u64.pow(attempt);
        let jitter_range = base_ms / 4;
        let jitter = if jitter_range > 0 {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos() as u64;
            seed % (jitter_range * 2)
        } else {
            0
        };
        let delay_ms = base_ms.saturating_sub(jitter_range).saturating_add(jitter);
        Duration::from_millis(delay_ms)
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbeddingProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AiProviderError> {
        let url = self.endpoint_url();

        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: text.to_string(),
            dimensions: Some(self.dimensions),
        };

        debug!(
            "[OpenAI Embedding] Requesting embedding from '{}' model '{}' (text: {} chars, dims: {})",
            self.base_url,
            self.model,
            text.len(),
            self.dimensions
        );
        let request_start = Instant::now();

        let mut last_error: Option<AiProviderError> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let retry_after = last_error.as_ref().and_then(|e| {
                    if let AiProviderError::GenerationFailed(msg) = e {
                        if msg.starts_with("RETRYABLE:429:") {
                            let secs_str = msg.strip_prefix("RETRYABLE:429:")?;
                            let secs = secs_str.parse::<u64>().ok()?;
                            if secs > 0 {
                                return Some(Duration::from_secs(secs));
                            }
                        }
                    }
                    None
                });

                let delay = Self::retry_delay(attempt - 1, retry_after);
                info!(
                    "[OpenAI Embedding] Retry {}/{} after {:.1}s delay",
                    attempt,
                    MAX_RETRIES,
                    delay.as_secs_f64()
                );
                tokio::time::sleep(delay).await;
            }

            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await;

            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    if e.is_timeout() {
                        return Err(AiProviderError::NotAvailable(format!(
                            "Embedding request timed out: {}",
                            e
                        )));
                    }
                    if e.is_connect() {
                        return Err(AiProviderError::NotAvailable(format!(
                            "Connection failed: {}",
                            e
                        )));
                    }
                    return Err(AiProviderError::NotAvailable(e.to_string()));
                }
            };

            let status = resp.status();

            if !status.is_success() {
                let retry_after_header = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                let body = resp.text().await.unwrap_or_default();
                let api_message = serde_json::from_str::<ErrorResponse>(&body)
                    .ok()
                    .and_then(|r| r.error)
                    .map(|e| e.message)
                    .unwrap_or_else(|| body.chars().take(200).collect());

                match status.as_u16() {
                    401 | 403 => {
                        return Err(AiProviderError::AuthenticationFailed(api_message));
                    }
                    429 => {
                        let retry_secs = retry_after_header
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0);
                        let err = AiProviderError::GenerationFailed(format!(
                            "RETRYABLE:429:{}",
                            retry_secs
                        ));
                        if attempt == MAX_RETRIES {
                            return Err(AiProviderError::RateLimited);
                        }
                        warn!(
                            "[OpenAI Embedding] Rate limited on attempt {}",
                            attempt + 1
                        );
                        last_error = Some(err);
                        continue;
                    }
                    500..=599 => {
                        let err = AiProviderError::GenerationFailed(format!(
                            "RETRYABLE:{}:{}",
                            status.as_u16(),
                            api_message
                        ));
                        if attempt == MAX_RETRIES {
                            return Err(AiProviderError::GenerationFailed(format!(
                                "Server error ({}) after {} retries: {}",
                                status.as_u16(),
                                MAX_RETRIES,
                                api_message
                            )));
                        }
                        warn!(
                            "[OpenAI Embedding] Server error {} on attempt {}",
                            status.as_u16(),
                            attempt + 1
                        );
                        last_error = Some(err);
                        continue;
                    }
                    _ => {
                        return Err(AiProviderError::GenerationFailed(format!(
                            "Embedding API error ({}): {}",
                            status.as_u16(),
                            api_message
                        )));
                    }
                }
            }

            // Success - parse response
            let body = resp.bytes().await.map_err(|e| {
                AiProviderError::GenerationFailed(format!("Failed to read response: {}", e))
            })?;

            let response: EmbeddingResponse = serde_json::from_slice(&body).map_err(|e| {
                AiProviderError::GenerationFailed(format!("Failed to parse embedding response: {}", e))
            })?;

            let duration = request_start.elapsed();

            if let Some(usage) = &response.usage {
                debug!(
                    "[OpenAI Embedding] Completed in {:.2}s (tokens: {}/{})",
                    duration.as_secs_f64(),
                    usage.prompt_tokens,
                    usage.total_tokens
                );
            } else {
                debug!(
                    "[OpenAI Embedding] Completed in {:.2}s",
                    duration.as_secs_f64()
                );
            }

            let embedding = response
                .data
                .into_iter()
                .next()
                .map(|d| d.embedding)
                .ok_or_else(|| {
                    AiProviderError::GenerationFailed(
                        "Empty embedding response: no data returned".to_string(),
                    )
                })?;

            return Ok(embedding);
        }

        Err(last_error
            .unwrap_or_else(|| AiProviderError::GenerationFailed("Unknown error".to_string())))
    }

    fn embedding_dimensions(&self) -> usize {
        self.dimensions
    }

    fn provider_name(&self) -> &str {
        "OpenAI Embedding"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider =
            OpenAiEmbeddingProvider::new("https://api.openai.com", "sk-test", "text-embedding-3-small", 1024);
        assert_eq!(provider.provider_name(), "OpenAI Embedding");
    }

    #[test]
    fn test_embedding_dimensions() {
        let provider =
            OpenAiEmbeddingProvider::new("https://api.openai.com", "sk-test", "text-embedding-3-small", 1024);
        assert_eq!(provider.embedding_dimensions(), 1024);
    }

    #[test]
    fn test_embedding_dimensions_custom() {
        let provider =
            OpenAiEmbeddingProvider::new("https://api.openai.com", "sk-test", "text-embedding-3-large", 3072);
        assert_eq!(provider.embedding_dimensions(), 3072);
    }

    #[test]
    fn test_endpoint_url() {
        let provider =
            OpenAiEmbeddingProvider::new("https://api.openai.com", "sk-test", "text-embedding-3-small", 1024);
        assert_eq!(
            provider.endpoint_url(),
            "https://api.openai.com/v1/embeddings"
        );
    }

    #[test]
    fn test_endpoint_url_trailing_slash() {
        let provider =
            OpenAiEmbeddingProvider::new("https://api.openai.com/", "sk-test", "text-embedding-3-small", 1024);
        assert_eq!(
            provider.endpoint_url(),
            "https://api.openai.com/v1/embeddings"
        );
    }

    #[test]
    fn test_request_serialization() {
        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: "Hello world".to_string(),
            dimensions: Some(1024),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"text-embedding-3-small\""));
        assert!(json.contains("\"input\":\"Hello world\""));
        assert!(json.contains("\"dimensions\":1024"));
    }

    #[test]
    fn test_request_serialization_no_dimensions() {
        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: "Hello world".to_string(),
            dimensions: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("dimensions"));
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{
            "object": "list",
            "data": [{"object": "embedding", "embedding": [0.1, 0.2, 0.3], "index": 0}],
            "model": "text-embedding-3-small",
            "usage": {"prompt_tokens": 5, "total_tokens": 5}
        }"#;

        let response: EmbeddingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].embedding, vec![0.1, 0.2, 0.3]);
        assert_eq!(response.usage.unwrap().prompt_tokens, 5);
    }

    #[test]
    fn test_response_deserialization_no_usage() {
        let json = r#"{
            "object": "list",
            "data": [{"object": "embedding", "embedding": [0.5], "index": 0}],
            "model": "test"
        }"#;

        let response: EmbeddingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert!(response.usage.is_none());
    }

    #[test]
    fn test_retry_delay_exponential() {
        let d0 = OpenAiEmbeddingProvider::retry_delay(0, None);
        assert!(
            d0.as_millis() >= 750 && d0.as_millis() <= 1250,
            "Attempt 0 delay should be ~1000ms, got {}ms",
            d0.as_millis()
        );

        let d1 = OpenAiEmbeddingProvider::retry_delay(1, None);
        assert!(
            d1.as_millis() >= 1500 && d1.as_millis() <= 2500,
            "Attempt 1 delay should be ~2000ms, got {}ms",
            d1.as_millis()
        );
    }

    #[test]
    fn test_retry_delay_respects_retry_after() {
        let delay = OpenAiEmbeddingProvider::retry_delay(0, Some(Duration::from_secs(10)));
        assert_eq!(delay, Duration::from_secs(10));
    }
}
