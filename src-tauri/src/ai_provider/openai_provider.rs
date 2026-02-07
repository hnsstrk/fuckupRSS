//! OpenAI-compatible API provider for text generation
//!
//! Supports OpenAI, Together.ai, Mistral API, Groq, and any other
//! provider that implements the OpenAI chat completions API.

use async_trait::async_trait;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::{AiProviderError, AiTextProvider, GenerationResult};

/// OpenAI-compatible text generation provider
pub struct OpenAiCompatibleProvider {
    base_url: String,
    api_key: String,
    client: reqwest_new::Client,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
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
struct ApiError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        // Normalize base URL: remove trailing slash
        let base_url = base_url.trim_end_matches('/').to_string();
        let client = reqwest_new::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        Self {
            base_url,
            api_key: api_key.to_string(),
            client,
        }
    }

    fn endpoint_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }
}

#[async_trait]
impl AiTextProvider for OpenAiCompatibleProvider {
    async fn generate_text(
        &self,
        model: &str,
        prompt: &str,
        json_mode: bool,
    ) -> Result<GenerationResult, AiProviderError> {
        let client = &self.client;
        let url = self.endpoint_url();

        let mut messages = Vec::new();

        // Add system message with role-specific instructions
        if json_mode {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: "You are a professional news article analyst. Always respond with valid JSON matching the exact schema specified in the user message. Be concise and factual.".to_string(),
            });
        } else {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: "You are a professional news article analyst. Be concise and factual.".to_string(),
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            response_format: if json_mode {
                Some(ResponseFormat {
                    format_type: "json_object".to_string(),
                })
            } else {
                None
            },
            max_completion_tokens: Some(4096),
            temperature: Some(0.3),
        };

        let prompt_len = prompt.len();
        debug!(
            "[OpenAI] Sending request to '{}' model '{}' (prompt: {} chars, json_mode: {})",
            self.base_url, model, prompt_len, json_mode
        );
        let request_start = Instant::now();

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                warn!(
                    "[OpenAI] Request failed after {:.2}s: {}",
                    request_start.elapsed().as_secs_f64(),
                    e
                );
                AiProviderError::NotAvailable(e.to_string())
            })?;

        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let duration = request_start.elapsed();
            warn!(
                "[OpenAI] Request failed after {:.2}s with status {}: {}",
                duration.as_secs_f64(),
                status,
                &body[..body.len().min(300)]
            );

            // Try to parse error response
            if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(&body) {
                if let Some(api_error) = error_resp.error {
                    // Check for auth errors
                    if status.as_u16() == 401
                        || api_error
                            .error_type
                            .as_deref()
                            .is_some_and(|t| t.contains("auth"))
                    {
                        return Err(AiProviderError::AuthenticationFailed(api_error.message));
                    }
                    // Check for rate limiting
                    if status.as_u16() == 429 {
                        return Err(AiProviderError::RateLimited);
                    }
                    return Err(AiProviderError::GenerationFailed(format!(
                        "API error ({}): {}",
                        status, api_error.message
                    )));
                }
            }

            return Err(AiProviderError::GenerationFailed(format!(
                "Status {}: {}",
                status,
                &body[..body.len().min(200)]
            )));
        }

        let body = resp.bytes().await.map_err(|e| {
            AiProviderError::GenerationFailed(format!("Failed to read response: {}", e))
        })?;

        let response: ChatCompletionResponse = serde_json::from_slice(&body).map_err(|e| {
            AiProviderError::GenerationFailed(format!("Failed to parse response: {}", e))
        })?;

        let duration = request_start.elapsed();

        // Check for truncation via finish_reason
        if let Some(first_choice) = response.choices.first() {
            if first_choice.finish_reason.as_deref() == Some("length") {
                warn!("[OpenAI] Response was truncated (finish_reason: length)");
            }
        }

        let text = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let (input_tokens, output_tokens) = response
            .usage
            .map(|u| (Some(u.prompt_tokens), Some(u.completion_tokens)))
            .unwrap_or((None, None));

        debug!(
            "[OpenAI] Request completed in {:.2}s (response: {} chars, tokens: {:?}/{:?})",
            duration.as_secs_f64(),
            text.len(),
            input_tokens,
            output_tokens
        );

        Ok(GenerationResult {
            text,
            input_tokens,
            output_tokens,
        })
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
}
