//! OpenAI-compatible API provider for text generation
//!
//! Supports OpenAI, Together.ai, Mistral API, Groq, and any other
//! provider that implements the OpenAI chat completions API.
//!
//! Features:
//! - Exponential backoff with jitter for rate limits (429) and server errors (5xx)
//! - Retry-After header support
//! - Differentiated error handling (auth, rate limit, server, timeout)

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use super::{AiProviderError, AiTextProvider, GenerationResult};

/// Maximum number of retries for retryable errors (429, 5xx)
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (1 second)
const BASE_RETRY_DELAY_MS: u64 = 1000;

/// Maximum completion tokens for responses.
/// Our JSON responses (discordian analysis) typically use 500-2000 tokens.
/// 4096 provides sufficient headroom while avoiding wasteful token reservation.
const MAX_COMPLETION_TOKENS: u32 = 4096;

/// OpenAI-compatible text generation provider
pub struct OpenAiCompatibleProvider {
    base_url: String,
    api_key: String,
    client: reqwest_new::Client,
    temperature: Option<f32>,
}

#[derive(Serialize, Clone)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
enum ResponseFormat {
    /// Simple format: `{"type": "json_object"}`
    Simple {
        #[serde(rename = "type")]
        format_type: String,
    },
    /// Schema-based format: `{"type": "json_schema", "json_schema": {...}}`
    JsonSchema {
        #[serde(rename = "type")]
        format_type: String,
        json_schema: JsonSchemaWrapper,
    },
}

#[derive(Serialize, Clone)]
struct JsonSchemaWrapper {
    name: String,
    schema: serde_json::Value,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<TokenUsage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

#[derive(Deserialize)]
struct TokenUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<ApiError>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ApiError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

/// Categorized HTTP error for retry decisions
enum HttpError {
    /// 401/403 - Authentication/authorization failure (not retryable)
    Auth(String),
    /// 429 - Rate limited (retryable with backoff)
    RateLimit { retry_after: Option<Duration> },
    /// 5xx - Server error (retryable with backoff)
    ServerError(u16, String),
    /// Other client errors 4xx (not retryable)
    ClientError(u16, String),
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: &str, api_key: &str, temperature: Option<f32>) -> Self {
        // Normalize base URL: remove trailing slash
        let base_url = base_url.trim_end_matches('/').to_string();
        let client = reqwest_new::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        Self {
            base_url,
            api_key: api_key.to_string(),
            client,
            temperature,
        }
    }

    fn endpoint_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    /// Classify an HTTP error response for retry decisions.
    fn classify_error(
        status: reqwest_new::StatusCode,
        body: &str,
        retry_after_header: Option<&str>,
    ) -> HttpError {
        let api_message = serde_json::from_str::<ErrorResponse>(body)
            .ok()
            .and_then(|r| r.error)
            .map(|e| e.message)
            .unwrap_or_else(|| body.chars().take(200).collect());

        match status.as_u16() {
            401 | 403 => HttpError::Auth(api_message),
            429 => {
                let retry_after = retry_after_header
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(Duration::from_secs);
                HttpError::RateLimit { retry_after }
            }
            500..=599 => HttpError::ServerError(status.as_u16(), api_message),
            _ => HttpError::ClientError(status.as_u16(), api_message),
        }
    }

    /// Calculate retry delay with exponential backoff and jitter.
    fn retry_delay(attempt: u32, retry_after: Option<Duration>) -> Duration {
        // If the server told us how long to wait, respect that
        if let Some(server_delay) = retry_after {
            return server_delay;
        }

        // Exponential backoff: 1s, 2s, 4s
        let base_ms = BASE_RETRY_DELAY_MS * 2u64.pow(attempt);

        // Add jitter: ±25% of base delay
        let jitter_range = base_ms / 4;
        let jitter = if jitter_range > 0 {
            // Simple deterministic jitter based on attempt number and current time
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

    /// Execute a single API request and return the response or a classified error.
    async fn execute_request(
        &self,
        url: &str,
        request: &ChatCompletionRequest,
    ) -> Result<(reqwest_new::StatusCode, bytes::Bytes), AiProviderError> {
        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiProviderError::NotAvailable(format!("Request timed out after 120s: {}", e))
                } else if e.is_connect() {
                    AiProviderError::NotAvailable(format!("Connection failed: {}", e))
                } else {
                    AiProviderError::NotAvailable(e.to_string())
                }
            })?;

        let status = resp.status();

        if !status.is_success() {
            // Extract Retry-After header before consuming the response
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let body = resp.text().await.unwrap_or_default();

            match Self::classify_error(status, &body, retry_after.as_deref()) {
                HttpError::Auth(msg) => {
                    return Err(AiProviderError::AuthenticationFailed(msg));
                }
                HttpError::RateLimit { retry_after } => {
                    return Err(AiProviderError::GenerationFailed(format!(
                        "RETRYABLE:429:{}",
                        retry_after.map(|d| d.as_secs()).unwrap_or(0)
                    )));
                }
                HttpError::ServerError(code, msg) => {
                    return Err(AiProviderError::GenerationFailed(format!(
                        "RETRYABLE:{}:{}",
                        code, msg
                    )));
                }
                HttpError::ClientError(code, msg) => {
                    return Err(AiProviderError::GenerationFailed(format!(
                        "API error ({}): {}",
                        code, msg
                    )));
                }
            }
        }

        let body = resp.bytes().await.map_err(|e| {
            AiProviderError::GenerationFailed(format!("Failed to read response body: {}", e))
        })?;

        Ok((status, body))
    }
}

#[async_trait]
impl AiTextProvider for OpenAiCompatibleProvider {
    async fn generate_text(
        &self,
        model: &str,
        prompt: &str,
        json_schema: Option<serde_json::Value>,
    ) -> Result<GenerationResult, AiProviderError> {
        let url = self.endpoint_url();
        let json_mode = json_schema.is_some();

        let mut messages = Vec::new();

        // Add system message with role-specific instructions
        if json_mode {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: "You are a professional news article analyst. \
                    Always respond with valid JSON matching the exact \
                    schema specified in the user message. Be concise \
                    and factual."
                    .to_string(),
            });
        } else {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: "You are a professional news article analyst. \
                    Be concise and factual."
                    .to_string(),
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        // Build response_format based on schema
        let response_format = match json_schema {
            Some(schema) if schema.is_object() => {
                // Full JSON Schema provided - use structured output
                Some(ResponseFormat::JsonSchema {
                    format_type: "json_schema".to_string(),
                    json_schema: JsonSchemaWrapper {
                        name: "response".to_string(),
                        schema,
                    },
                })
            }
            Some(_) => {
                // Schema is a string like "json" - fall back to
                // json_object
                Some(ResponseFormat::Simple {
                    format_type: "json_object".to_string(),
                })
            }
            None => None,
        };

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            response_format,
            max_completion_tokens: Some(MAX_COMPLETION_TOKENS),
            temperature: self.temperature,
        };

        let prompt_len = prompt.len();
        debug!(
            "[OpenAI] Sending request to '{}' model '{}' \
             (prompt: {} chars, json_mode: {}, max_tokens: {})",
            self.base_url, model, prompt_len, json_mode, MAX_COMPLETION_TOKENS
        );
        let request_start = Instant::now();

        // Retry loop with exponential backoff for 429 and 5xx errors
        let mut last_error: Option<AiProviderError> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                // This is a retry - calculate and apply delay
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
                    "[OpenAI] Retry {}/{} after {:.1}s delay (elapsed: {:.1}s)",
                    attempt,
                    MAX_RETRIES,
                    delay.as_secs_f64(),
                    request_start.elapsed().as_secs_f64()
                );
                tokio::time::sleep(delay).await;
            }

            match self.execute_request(&url, &request).await {
                Ok((_status, body)) => {
                    // Successful response - parse it
                    let response: ChatCompletionResponse =
                        serde_json::from_slice(&body).map_err(|e| {
                            AiProviderError::GenerationFailed(format!(
                                "Failed to parse response: {}",
                                e
                            ))
                        })?;

                    let duration = request_start.elapsed();

                    // Check for empty response (no choices)
                    if response.choices.is_empty() {
                        return Err(AiProviderError::GenerationFailed(
                            "API returned empty response (no choices)".to_string(),
                        ));
                    }

                    let first_choice = &response.choices[0];

                    // Check for truncation via finish_reason
                    if first_choice.finish_reason.as_deref() == Some("length") {
                        warn!(
                            "[OpenAI] Response was truncated (finish_reason: length, max_completion_tokens: {})",
                            MAX_COMPLETION_TOKENS
                        );
                    }

                    let text = first_choice.message.content.clone().unwrap_or_default();

                    // Check for null/empty content
                    if text.is_empty() {
                        return Err(AiProviderError::GenerationFailed(
                            "API returned empty content (null or empty string)".to_string(),
                        ));
                    }

                    let (input_tokens, output_tokens) = response
                        .usage
                        .map(|u| (Some(u.prompt_tokens), Some(u.completion_tokens)))
                        .unwrap_or((None, None));

                    if attempt > 0 {
                        info!(
                            "[OpenAI] Request succeeded after {} retries in {:.2}s (response: {} chars, tokens: {:?}/{:?})",
                            attempt, duration.as_secs_f64(), text.len(), input_tokens, output_tokens
                        );
                    } else {
                        debug!(
                            "[OpenAI] Request completed in {:.2}s (response: {} chars, tokens: {:?}/{:?})",
                            duration.as_secs_f64(), text.len(), input_tokens, output_tokens
                        );
                    }

                    return Ok(GenerationResult {
                        text,
                        input_tokens,
                        output_tokens,
                    });
                }
                Err(e) => {
                    // Check if this error is retryable
                    let is_retryable = matches!(&e, AiProviderError::GenerationFailed(msg) if msg.starts_with("RETRYABLE:"));

                    if !is_retryable || attempt == MAX_RETRIES {
                        // Non-retryable error or final retry exhausted
                        let duration = request_start.elapsed();

                        // Convert internal RETRYABLE markers to proper error types
                        let final_error =
                            match &e {
                                AiProviderError::GenerationFailed(msg)
                                    if msg.starts_with("RETRYABLE:429:") =>
                                {
                                    warn!(
                                        "[OpenAI] Rate limited after {} retries ({:.2}s total)",
                                        attempt,
                                        duration.as_secs_f64()
                                    );
                                    AiProviderError::RateLimited
                                }
                                AiProviderError::GenerationFailed(msg)
                                    if msg.starts_with("RETRYABLE:") =>
                                {
                                    // Extract the actual error message from "RETRYABLE:5xx:message"
                                    let parts: Vec<&str> = msg.splitn(3, ':').collect();
                                    let code = parts.get(1).unwrap_or(&"5xx");
                                    let message = parts.get(2).unwrap_or(&"Server error");
                                    warn!(
                                    "[OpenAI] Server error {} after {} retries ({:.2}s total): {}",
                                    code, attempt, duration.as_secs_f64(), message
                                );
                                    AiProviderError::GenerationFailed(format!(
                                        "Server error ({}) after {} retries: {}",
                                        code, attempt, message
                                    ))
                                }
                                _ => {
                                    warn!(
                                        "[OpenAI] Request failed after {:.2}s: {}",
                                        duration.as_secs_f64(),
                                        e
                                    );
                                    e
                                }
                            };

                        return Err(final_error);
                    }

                    // Retryable error - store and continue loop
                    warn!("[OpenAI] Retryable error on attempt {}: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        // Should never reach here due to the loop logic, but just in case
        Err(last_error
            .unwrap_or_else(|| AiProviderError::GenerationFailed("Unknown error".to_string())))
    }

    async fn is_available(&self) -> bool {
        let client = &self.client;

        // Try to list models as a health check
        let url = format!("{}/v1/models", self.base_url);
        match client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    fn provider_name(&self) -> &str {
        "OpenAI-compatible"
    }

    fn suggested_concurrency(&self) -> usize {
        // Conservative default to avoid rate limiting.
        // With 5 concurrent requests at ~2s each, we stay well under
        // typical RPM limits (500 RPM for Tier 2+).
        // Users can override via settings (openai_concurrency).
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delay_exponential() {
        // attempt 0: ~1000ms base
        let d0 = OpenAiCompatibleProvider::retry_delay(0, None);
        assert!(
            d0.as_millis() >= 750 && d0.as_millis() <= 1250,
            "Attempt 0 delay should be ~1000ms, got {}ms",
            d0.as_millis()
        );

        // attempt 1: ~2000ms base
        let d1 = OpenAiCompatibleProvider::retry_delay(1, None);
        assert!(
            d1.as_millis() >= 1500 && d1.as_millis() <= 2500,
            "Attempt 1 delay should be ~2000ms, got {}ms",
            d1.as_millis()
        );

        // attempt 2: ~4000ms base
        let d2 = OpenAiCompatibleProvider::retry_delay(2, None);
        assert!(
            d2.as_millis() >= 3000 && d2.as_millis() <= 5000,
            "Attempt 2 delay should be ~4000ms, got {}ms",
            d2.as_millis()
        );
    }

    #[test]
    fn test_retry_delay_respects_retry_after() {
        let delay = OpenAiCompatibleProvider::retry_delay(0, Some(Duration::from_secs(10)));
        assert_eq!(delay, Duration::from_secs(10));
    }

    #[test]
    fn test_classify_error_auth_401() {
        let body = r#"{"error": {"message": "Invalid API key", "type": "auth_error"}}"#;
        let result = OpenAiCompatibleProvider::classify_error(
            reqwest_new::StatusCode::UNAUTHORIZED,
            body,
            None,
        );
        assert!(matches!(result, HttpError::Auth(msg) if msg.contains("Invalid API key")));
    }

    #[test]
    fn test_classify_error_auth_403() {
        let body = r#"{"error": {"message": "Access denied", "type": "permission_error"}}"#;
        let result = OpenAiCompatibleProvider::classify_error(
            reqwest_new::StatusCode::FORBIDDEN,
            body,
            None,
        );
        assert!(matches!(result, HttpError::Auth(msg) if msg.contains("Access denied")));
    }

    #[test]
    fn test_classify_error_rate_limit() {
        let body = r#"{"error": {"message": "Rate limit exceeded", "type": "rate_limit_error"}}"#;
        let result = OpenAiCompatibleProvider::classify_error(
            reqwest_new::StatusCode::TOO_MANY_REQUESTS,
            body,
            Some("5"),
        );
        assert!(
            matches!(result, HttpError::RateLimit { retry_after: Some(d) } if d == Duration::from_secs(5))
        );
    }

    #[test]
    fn test_classify_error_rate_limit_no_header() {
        let body = r#"{"error": {"message": "Rate limit exceeded", "type": "rate_limit_error"}}"#;
        let result = OpenAiCompatibleProvider::classify_error(
            reqwest_new::StatusCode::TOO_MANY_REQUESTS,
            body,
            None,
        );
        assert!(matches!(result, HttpError::RateLimit { retry_after: None }));
    }

    #[test]
    fn test_classify_error_server_error() {
        let body = r#"{"error": {"message": "Internal server error", "type": "server_error"}}"#;
        let result = OpenAiCompatibleProvider::classify_error(
            reqwest_new::StatusCode::INTERNAL_SERVER_ERROR,
            body,
            None,
        );
        assert!(matches!(result, HttpError::ServerError(500, _)));
    }

    #[test]
    fn test_classify_error_bad_request() {
        let body = r#"{"error": {"message": "Invalid model", "type": "invalid_request_error"}}"#;
        let result = OpenAiCompatibleProvider::classify_error(
            reqwest_new::StatusCode::BAD_REQUEST,
            body,
            None,
        );
        assert!(matches!(result, HttpError::ClientError(400, _)));
    }

    #[test]
    fn test_classify_error_malformed_body() {
        let body = "not json at all";
        let result = OpenAiCompatibleProvider::classify_error(
            reqwest_new::StatusCode::INTERNAL_SERVER_ERROR,
            body,
            None,
        );
        assert!(matches!(result, HttpError::ServerError(500, msg) if msg == "not json at all"));
    }

    #[test]
    fn test_max_completion_tokens_constant() {
        assert_eq!(MAX_COMPLETION_TOKENS, 4096);
    }

    #[test]
    fn test_provider_name() {
        let provider = OpenAiCompatibleProvider::new("https://api.openai.com", "sk-test", None);
        assert_eq!(provider.provider_name(), "OpenAI-compatible");
    }

    #[test]
    fn test_endpoint_url() {
        let provider = OpenAiCompatibleProvider::new("https://api.openai.com", "sk-test", None);
        assert_eq!(
            provider.endpoint_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_endpoint_url_trailing_slash() {
        let provider = OpenAiCompatibleProvider::new("https://api.openai.com/", "sk-test", None);
        assert_eq!(
            provider.endpoint_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_request_serialization_json_mode_simple() {
        let request = ChatCompletionRequest {
            model: "gpt-5-nano".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "Test".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
            ],
            response_format: Some(ResponseFormat::Simple {
                format_type: "json_object".to_string(),
            }),
            max_completion_tokens: Some(MAX_COMPLETION_TOKENS),
            temperature: Some(0.3),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"max_completion_tokens\":4096"));
        assert!(json.contains("\"json_object\""));
        assert!(json.contains("\"temperature\":0.3"));
    }

    #[test]
    fn test_request_serialization_json_schema_mode() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "political_bias": { "type": "integer" }
            },
            "required": ["political_bias"]
        });

        let request = ChatCompletionRequest {
            model: "gpt-5-nano".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Analyze this".to_string(),
            }],
            response_format: Some(ResponseFormat::JsonSchema {
                format_type: "json_schema".to_string(),
                json_schema: JsonSchemaWrapper {
                    name: "response".to_string(),
                    schema,
                },
            }),
            max_completion_tokens: Some(MAX_COMPLETION_TOKENS),
            temperature: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"json_schema\""));
        assert!(json.contains("\"name\":\"response\""));
        assert!(json.contains("\"political_bias\""));
    }

    #[test]
    fn test_request_serialization_no_json_mode() {
        let request = ChatCompletionRequest {
            model: "gpt-5-nano".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            response_format: None,
            max_completion_tokens: Some(MAX_COMPLETION_TOKENS),
            temperature: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("response_format"));
        assert!(!json.contains("temperature"));
        assert!(json.contains("\"max_completion_tokens\":4096"));
    }
}
