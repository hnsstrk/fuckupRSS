use log::{debug, warn};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use thiserror::Error;

#[cfg(test)]
mod tests;

/// Custom deserializers to handle LLM responses that may return objects instead of strings.
/// For example, the LLM might return {"name": "keyword"} instead of just "keyword".
mod flexible_deser {
    use super::*;

    /// Extract a string from a Value that could be:
    /// - A plain string
    /// - An object with "name", "text", "value", or "content" field
    fn extract_string_from_value(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Object(map) => {
                // Try common field names
                for key in &["name", "text", "value", "content", "keyword", "category"] {
                    if let Some(Value::String(s)) = map.get(*key) {
                        return Some(s.clone());
                    }
                }
                // Last resort: take the first string value in the object
                for val in map.values() {
                    if let Value::String(s) = val {
                        return Some(s.clone());
                    }
                }
                None
            }
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }

    /// Deserialize a string that might be an object with a text field.
    /// Returns an empty string for `null` values (LLMs sometimes return `"field": null`).
    pub fn flexible_string<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;
        if v.is_null() {
            return Ok(String::new());
        }
        extract_string_from_value(&v)
            .ok_or_else(|| serde::de::Error::custom(format!("cannot extract string from {:?}", v)))
    }

    /// Deserialize a Vec<String> where items might be objects with text fields
    pub fn flexible_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;
        match v {
            Value::Array(arr) => {
                let mut result = Vec::with_capacity(arr.len());
                for item in arr {
                    if let Some(s) = extract_string_from_value(&item) {
                        if !s.is_empty() {
                            result.push(s);
                        }
                    }
                }
                Ok(result)
            }
            Value::Null => Ok(Vec::new()),
            _ => Err(serde::de::Error::custom(format!(
                "expected array, got {:?}",
                v
            ))),
        }
    }
}

/// Safely truncate a string to at most `max_bytes` bytes at a character boundary
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Find the last character boundary at or before max_bytes
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[derive(Error, Debug)]
pub enum OllamaError {
    #[error("Ollama not available: {0}")]
    NotAvailable(String),
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
    #[error("Incomplete response: generation stopped before completion (done=false), output likely truncated")]
    IncompleteResponse,
    #[error("Model pull failed: {0}")]
    PullFailed(String),
}

#[derive(Serialize)]
struct GenerateOptions {
    num_ctx: u32,
    /// Maximum number of tokens to generate (output)
    /// -1 = infinite, 128 = default, we use 2048 for JSON analysis
    num_predict: i32,
}

// ============================================================
// Chat API structs (/api/chat)
// ============================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<Value>,
    options: GenerateOptions,
    keep_alive: String,
    /// Thinking/Reasoning aktivieren (None = Modell-Default, Some(true) = aktiviert)
    /// Aktuell immer None wegen Ollama Bug (≤0.18.2), /no_think System-Message wird stattdessen verwendet
    #[serde(skip_serializing_if = "Option::is_none")]
    think: Option<bool>,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
    /// Whether generation completed successfully
    #[serde(default)]
    done: bool,
    /// Reason for stopping: "stop" (normal) or "length" (truncated)
    #[serde(default)]
    pub done_reason: Option<String>,
    /// Number of tokens generated (for logging)
    #[serde(default)]
    pub eval_count: Option<u64>,
}

// ============================================================
// JSON Schema constants for structured outputs
// ============================================================

/// JSON Schema for DiscordianAnalysis (with rejections)
///
/// Ollama validates the response against this schema when passed as `format`.
/// Fields: political_bias, sachlichkeit, summary, keywords, categories,
///         rejected_keywords, rejected_categories
pub fn discordian_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "political_bias": { "type": "integer" },
            "sachlichkeit": { "type": "integer" },
            "summary": { "type": "string" },
            "keywords": {
                "type": "array",
                "items": { "type": "string" }
            },
            "categories": {
                "type": "array",
                "items": { "type": "string" }
            },
            "rejected_keywords": {
                "type": "array",
                "items": { "type": "string" }
            },
            "rejected_categories": {
                "type": "array",
                "items": { "type": "string" }
            },
            "article_type": {
                "type": "string",
                "enum": ["news", "analysis", "opinion", "satire", "ad", "unknown"]
            }
        },
        "required": [
            "political_bias", "sachlichkeit", "summary",
            "keywords", "categories",
            "rejected_keywords", "rejected_categories",
            "article_type"
        ]
    })
}

/// JSON Schema for BiasAnalysis
///
/// Fields: political_bias, sachlichkeit
pub fn bias_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "political_bias": { "type": "integer" },
            "sachlichkeit": { "type": "integer" }
        },
        "required": ["political_bias", "sachlichkeit"]
    })
}

/// JSON Schema for simple DiscordianAnalysis (without rejections, legacy)
pub fn discordian_simple_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "political_bias": { "type": "integer" },
            "sachlichkeit": { "type": "integer" },
            "summary": { "type": "string" },
            "keywords": {
                "type": "array",
                "items": { "type": "string" }
            },
            "categories": {
                "type": "array",
                "items": { "type": "string" }
            },
            "article_type": {
                "type": "string",
                "enum": ["news", "analysis", "opinion", "satire", "ad", "unknown"]
            }
        },
        "required": [
            "political_bias", "sachlichkeit", "summary",
            "keywords", "categories",
            "article_type"
        ]
    })
}

/// JSON Schema for synonym verification
pub fn synonym_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "is_synonym": { "type": "boolean" },
            "confidence": { "type": "number" },
            "explanation": { "type": "string" }
        },
        "required": ["is_synonym", "confidence"]
    })
}

/// JSON Schema for structured Briefing output
///
/// Fields: tldr (overview, trends, conclusion), topics (title, body, article_indices, keywords)
pub fn briefing_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "tldr": {
                "type": "object",
                "properties": {
                    "overview": { "type": "string" },
                    "trends": { "type": "string" },
                    "conclusion": { "type": "string" }
                },
                "required": ["overview", "trends", "conclusion"]
            },
            "topics": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "body": { "type": "string" },
                        "article_indices": {
                            "type": "array",
                            "items": { "type": "integer" }
                        },
                        "keywords": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["title", "body", "article_indices", "keywords"]
                }
            }
        },
        "required": ["tldr", "topics"]
    })
}

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ModelInfo {
    pub name: String,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Serialize)]
struct PullRequest {
    name: String,
    stream: bool,
}

#[derive(Deserialize)]
struct PullResponse {
    status: String,
}

// Embedding structs for Ollama /api/embed endpoint (batch-capable)
#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: Vec<String>,
    keep_alive: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

/// Recommended models for fuckupRSS
/// ministral-3:latest — schnell, gute Qualität für Analyse-Tasks
pub const RECOMMENDED_MAIN_MODEL: &str = "ministral-3:latest";
/// deepseek-r1:latest — Reasoning-Modell für Briefings und Perspektivenvergleich
pub const RECOMMENDED_REASONING_MODEL: &str = "deepseek-r1:latest";
/// snowflake-arctic-embed2: Multilingual (74 languages incl. German/English), 1024-dim
pub const RECOMMENDED_EMBEDDING_MODEL: &str = "snowflake-arctic-embed2:latest";

/// Default prompts (English prompts with {language} placeholder for output language)
///
/// Legacy combined prompt (kept for backward compatibility with custom prompts in DB).
/// Internally, the system/user split is used via DEFAULT_SUMMARY_SYSTEM/USER.
pub const DEFAULT_SUMMARY_PROMPT: &str = r#"You are a news article analyst. Create a brief, factual summary of the following article in 2-3 sentences.

IMPORTANT: Respond ONLY in {language}. Do not use any other language.
Respond ONLY with the summary, without introduction or explanation.

Article:
{content}

Summary:"#;

pub const DEFAULT_ANALYSIS_PROMPT: &str = r#"Analyze the following news article for political bias and objectivity.
Respond in the following JSON format:
{
  "political_bias": <-2 to 2, where -2=strong left, 0=neutral, 2=strong right>,
  "sachlichkeit": <0 to 4, where 0=highly emotional, 4=very objective>
}

Title: {title}
Content: {content}"#;

/// Combined prompt for full Discordian Analysis (summary + bias + categories + keywords)
/// NOTE: This is a legacy fallback. Prefer DEFAULT_DISCORDIAN_PROMPT_WITH_STATS.
/// Categories are now primarily derived from the keyword network (statistical).
pub const DEFAULT_DISCORDIAN_PROMPT: &str = r#"Analyze this news article. Respond in {language}. Return ONLY this JSON:

{
  "political_bias": <-2 to 2>,
  "sachlichkeit": <0 to 4>,
  "summary": "<2-3 sentences>",
  "keywords": ["<kw1>", "<kw2>", "<kw3>"],
  "categories": ["<cat1>"],
  "article_type": "<type>"
}

Rules:
- political_bias: -2=strong left, 0=neutral, 2=strong right (be precise!)
- sachlichkeit: 0=emotional/sensational, 2=mixed, 4=objective/factual (be precise!)
- summary: 2-3 factual sentences in {language}, capture the key information
- keywords: 3-5 short keywords (1-2 words each) - IMPORTANT for categorization
- categories: 0-1 from: Technik, Politik, Wirtschaft, Wissenschaft, Kultur, Sport, Gesellschaft, Umwelt, Sicherheit, Gesundheit, Verteidigung, Energie, Recht (optional, empty [] is fine)
- article_type: exactly one of: news, analysis, opinion, satire, ad, unknown

Title: {title}
Content: {content}"#;

/// Enhanced Discordian prompt with statistical pre-analysis for quality control
/// The LLM validates/corrects statistical suggestions rather than generating from scratch
/// NOTE: Categories are now primarily derived from the keyword network (statistical).
/// LLM categories serve only as optional validation/fallback.
///
/// OPTIMIZED: Reduced from ~37 lines to ~20 lines (~40% fewer tokens)
pub const DEFAULT_DISCORDIAN_PROMPT_WITH_STATS: &str = r#"Analyze this article. Statistical pre-analysis already computed keywords and categories.

PRE-COMPUTED: keywords={stat_keywords}, categories={stat_categories}

YOUR TASKS:
1. Write summary (2-3 factual sentences in {language})
2. Assess political_bias: -2=strong left, -1=left, 0=neutral, 1=right, 2=strong right
3. Assess sachlichkeit: 0=emotional/sensational, 2=mixed, 4=objective/factual
4. Review keywords: keep good ones, add max 2 important missing ones
5. Categories: only provide if pre-computed ones are clearly wrong (empty [] is fine)
6. Classify article_type: exactly one of: news, analysis, opinion, satire, ad, unknown

Return ONLY valid JSON:
{
  "political_bias": <-2 to 2>,
  "sachlichkeit": <0 to 4>,
  "summary": "<summary in {language}>",
  "keywords": ["kw1", "kw2", "..."],
  "categories": [],
  "rejected_keywords": [],
  "rejected_categories": [],
  "article_type": "<type>"
}

Title: {title}
Content: {content}"#;

// ============================================================
// System/User message split for /api/chat
// ============================================================

/// System message for summary generation
#[allow(dead_code)] // Available for future prompt customization
pub const DEFAULT_SUMMARY_SYSTEM: &str =
    "You are a news article analyst. Create brief, factual summaries \
     in 2-3 sentences. Respond ONLY in {language}. Do not use any other \
     language. Respond ONLY with the summary, without introduction or \
     explanation.";

/// User message template for summary generation
#[allow(dead_code)] // Available for future prompt customization
pub const DEFAULT_SUMMARY_USER: &str = "Article:\n{content}\n\nSummary:";

/// System message for Discordian analysis with statistical pre-analysis
#[allow(dead_code)] // Available for future prompt customization
pub const DEFAULT_DISCORDIAN_SYSTEM: &str =
    "You are a professional media analyst. Analyze news articles for \
     political bias, objectivity, keywords, and categories. Statistical \
     pre-analysis has already computed keyword and category suggestions. \
     Validate and refine them. Respond ONLY in {language} for the summary. \
     Return ONLY valid JSON matching the specified schema.";

/// User message template for Discordian analysis with stats
#[allow(dead_code)] // Available for future prompt customization
pub const DEFAULT_DISCORDIAN_USER: &str = r#"PRE-COMPUTED: keywords={stat_keywords}, categories={stat_categories}

YOUR TASKS:
1. Write summary (2-3 factual sentences in {language})
2. Assess political_bias: -2=strong left, -1=left, 0=neutral, 1=right, 2=strong right
3. Assess sachlichkeit: 0=emotional/sensational, 2=mixed, 4=objective/factual
4. Review keywords: keep good ones, add max 2 important missing ones
5. Categories: only provide if pre-computed ones are clearly wrong (empty [] is fine)

Title: {title}
Content: {content}"#;

/// System message for bias analysis
#[allow(dead_code)] // Available for future prompt customization
pub const DEFAULT_BIAS_SYSTEM: &str =
    "You are a professional media analyst specialized in detecting \
     political bias and objectivity in news articles. Be precise with \
     your ratings. Return ONLY valid JSON matching the specified schema.";

/// User message template for bias analysis
#[allow(dead_code)] // Available for future prompt customization
pub const DEFAULT_BIAS_USER: &str = "Title: {title}\nContent: {content}";

/// Get language name for prompt based on locale
pub fn get_language_for_locale(locale: &str) -> &'static str {
    match locale {
        "de" => "German",
        "en" => "English",
        _ => "German", // Default to German
    }
}

/// Default context length - optimized for 12GB GPU (100% GPU, ~1.5s/article)
pub const DEFAULT_NUM_CTX: u32 = 8192;

/// Higher context for briefing generation (more articles in prompt)
pub const BRIEFING_NUM_CTX: u32 = 16384;

/// Ollama API client for local LLM inference
pub struct OllamaClient {
    base_url: String,
    num_ctx: u32,
    http_client: reqwest_new::Client,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            num_ctx: DEFAULT_NUM_CTX,
            http_client: reqwest_new::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create client with custom context length
    pub fn with_context(base_url: Option<String>, num_ctx: u32) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            num_ctx,
            http_client: reqwest_new::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    fn client(&self) -> &reqwest_new::Client {
        &self.http_client
    }

    /// Check if Ollama is available and return list of models
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        let client = self.client();

        let resp: reqwest_new::Response = client
            .get(&url)
            .send()
            .await
            .map_err(|e: reqwest_new::Error| OllamaError::NotAvailable(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(OllamaError::NotAvailable(format!(
                "Status: {}",
                resp.status()
            )));
        }

        let bytes: bytes::Bytes = resp
            .bytes()
            .await
            .map_err(|e: reqwest_new::Error| OllamaError::NotAvailable(e.to_string()))?;

        let models: ModelsResponse =
            serde_json::from_slice(&bytes).map_err(|e| OllamaError::NotAvailable(e.to_string()))?;

        Ok(models.models)
    }

    /// Check if Ollama is running
    pub async fn is_available(&self) -> bool {
        self.list_models().await.is_ok()
    }

    /// Pull/download a model from Ollama
    pub async fn pull_model(&self, model_name: &str) -> Result<String, OllamaError> {
        let url = format!("{}/api/pull", self.base_url);
        let client = reqwest_new::Client::builder()
            .timeout(Duration::from_secs(3600)) // 1 hour timeout for large models
            .build()
            .map_err(|e| OllamaError::PullFailed(format!("Failed to create HTTP client: {}", e)))?;

        let request = PullRequest {
            name: model_name.to_string(),
            stream: false,
        };

        let resp = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| OllamaError::PullFailed(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::PullFailed(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let result: PullResponse = resp
            .json()
            .await
            .map_err(|e| OllamaError::PullFailed(e.to_string()))?;

        Ok(result.status)
    }

    /// Generate embedding vector for a single text
    pub async fn generate_embedding(
        &self,
        model: &str,
        text: &str,
    ) -> Result<Vec<f32>, OllamaError> {
        let result = self
            .generate_embeddings_batch(model, &[text.to_string()])
            .await?;
        result
            .into_iter()
            .next()
            .ok_or_else(|| OllamaError::GenerationFailed("Empty embedding response".to_string()))
    }

    /// Generate embedding vectors for multiple texts in a single request
    pub async fn generate_embeddings_batch(
        &self,
        model: &str,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, OllamaError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/api/embed", self.base_url);

        let request = EmbedRequest {
            model: model.to_string(),
            input: texts.to_vec(),
            keep_alive: "5m".to_string(),
        };

        debug!(
            "[Ollama] Batch embedding request: {} texts to model '{}'",
            texts.len(),
            model
        );
        let request_start = Instant::now();

        let resp = self
            .client()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                OllamaError::GenerationFailed(format!("Batch embedding request failed: {}", e))
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::GenerationFailed(format!(
                "Batch embedding failed with status {}: {}",
                status, body
            )));
        }

        let result: EmbedResponse = resp.json().await.map_err(|e| {
            OllamaError::GenerationFailed(format!("Failed to parse batch embedding: {}", e))
        })?;

        let duration = request_start.elapsed();
        debug!(
            "[Ollama] Batch embedding completed in {:.2}s ({} embeddings)",
            duration.as_secs_f64(),
            result.embeddings.len()
        );

        Ok(result.embeddings)
    }

    /// Generate a summary with custom prompt template (no JSON schema)
    #[allow(dead_code)] // Public API for external callers
    pub async fn summarize_with_prompt(
        &self,
        model: &str,
        content: &str,
        prompt_template: &str,
    ) -> Result<String, OllamaError> {
        let truncated_content = content.chars().take(8000).collect::<String>();
        let prompt = prompt_template.replace("{content}", &truncated_content);
        // Freetext: use chat with no schema, prompt as user message
        self.chat(model, None, &prompt, None).await
    }

    /// Simple text generation with JSON schema
    /// (public API for synonym verification etc.)
    #[allow(dead_code)] // Public API for external callers
    pub async fn generate_simple(
        &self,
        model: &str,
        prompt: &str,
        json_schema: Option<Value>,
    ) -> Result<String, OllamaError> {
        let format = json_schema.or_else(|| Some(Value::String("json".to_string())));
        self.chat(model, None, prompt, format).await
    }

    /// Chat-based generation via /api/chat endpoint
    ///
    /// Uses system + user message split for better prompt control.
    /// If `format` is Some, Ollama validates the response against it
    /// (JSON schema or `"json"` string for plain JSON mode).
    pub async fn chat(
        &self,
        model: &str,
        system_message: Option<&str>,
        user_message: &str,
        format: Option<Value>,
    ) -> Result<String, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let client = self.client();

        let mut messages = Vec::new();
        if let Some(sys) = system_message {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: sys.to_string(),
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        let total_len: usize = messages.iter().map(|m| m.content.len()).sum();
        debug!(
            "[Ollama] Sending chat request to model '{}' \
             ({} messages, {} chars, num_ctx: {})",
            model,
            messages.len(),
            total_len,
            self.num_ctx
        );
        let request_start = Instant::now();

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
            format,
            options: GenerateOptions {
                num_ctx: self.num_ctx,
                // Ensure enough output tokens for JSON analysis
                // Default is 128 which is too small for structured output
                // 4096 allows for detailed summaries + full JSON structure
                num_predict: 4096,
            },
            keep_alive: "5m".to_string(),
            think: None,
        };

        let resp: reqwest_new::Response =
            client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|e: reqwest_new::Error| {
                    warn!(
                        "[Ollama] Chat request failed after {:.2}s: {}",
                        request_start.elapsed().as_secs_f64(),
                        e
                    );
                    OllamaError::NotAvailable(e.to_string())
                })?;

        let status = resp.status();
        if !status.is_success() {
            let body: String = resp
                .text()
                .await
                .unwrap_or_else(|_: reqwest_new::Error| "Unknown error".to_string());
            warn!(
                "[Ollama] Chat request failed after {:.2}s with status {}: {}",
                request_start.elapsed().as_secs_f64(),
                status,
                truncate_str(&body, 200)
            );
            return Err(OllamaError::GenerationFailed(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let bytes: bytes::Bytes = resp
            .bytes()
            .await
            .map_err(|e: reqwest_new::Error| OllamaError::GenerationFailed(e.to_string()))?;

        let result: ChatResponse = serde_json::from_slice(&bytes)
            .map_err(|e| OllamaError::GenerationFailed(e.to_string()))?;

        let duration = request_start.elapsed();
        let response_len = result.message.content.len();

        // Return error if generation was incomplete (output truncated)
        if !result.done {
            warn!(
                "[Ollama] Chat generation incomplete (done=false) after \
                 {:.2}s - response truncated ({} chars), likely context overflow",
                duration.as_secs_f64(),
                response_len
            );
            return Err(OllamaError::IncompleteResponse);
        }

        // With stream: false, Ollama always returns done=true.
        // Truncation is signaled via done_reason="length" instead.
        if result.done_reason.as_deref() == Some("length") {
            warn!(
                "[Ollama] Chat generation truncated (done_reason=length). \
                 Output likely contains incomplete JSON. \
                 num_ctx={}, eval_count={:?}",
                self.num_ctx, result.eval_count
            );
            return Err(OllamaError::IncompleteResponse);
        }

        debug!(
            "[Ollama] Chat request completed in {:.2}s \
             (response: {} chars)",
            duration.as_secs_f64(),
            response_len
        );

        Ok(result.message.content)
    }

    /// Unload a model from VRAM by sending a chat request with keep_alive: "0"
    pub async fn unload_model(&self, model: &str) -> Result<(), OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![],
            stream: false,
            format: None,
            options: GenerateOptions {
                num_ctx: self.num_ctx,
                num_predict: 1,
            },
            keep_alive: "0".to_string(),
            think: None,
        };

        let resp = self
            .client()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| OllamaError::GenerationFailed(format!("Unload request failed: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            warn!("[Ollama] Unload model '{}' failed: {}", model, body);
        } else {
            debug!("[Ollama] Model '{}' unloaded from VRAM", model);
        }

        Ok(())
    }
}

/// Raw bias analysis from LLM (accepts floats)
#[derive(Deserialize, Debug)]
pub struct RawBiasAnalysis {
    pub political_bias: f64,
    pub sachlichkeit: f64,
}

/// Bias analysis with integer values (for storage and display)
#[derive(Serialize, Debug, Clone)]
pub struct BiasAnalysis {
    pub political_bias: i32,
    pub sachlichkeit: i32,
}

impl From<RawBiasAnalysis> for BiasAnalysis {
    fn from(raw: RawBiasAnalysis) -> Self {
        Self {
            political_bias: raw.political_bias.round() as i32,
            sachlichkeit: raw.sachlichkeit.round() as i32,
        }
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Raw Discordian analysis from LLM (accepts floats)
/// Uses flexible deserializers to handle LLM responses that return objects instead of strings
#[derive(Deserialize, Debug)]
pub struct RawDiscordianAnalysis {
    #[serde(default, deserialize_with = "flexible_deser::flexible_string")]
    summary: String,
    #[serde(default, deserialize_with = "flexible_deser::flexible_string_vec")]
    categories: Vec<String>,
    #[serde(default, deserialize_with = "flexible_deser::flexible_string_vec")]
    keywords: Vec<String>,
    #[serde(default)]
    political_bias: f64,
    #[serde(default)]
    sachlichkeit: f64,
    /// Article type: news/analysis/opinion/satire/ad/unknown
    #[serde(default = "default_article_type")]
    article_type: String,
}

fn default_article_type() -> String {
    "unknown".to_string()
}

/// Full Discordian analysis with all KI-extracted data
#[derive(Serialize, Debug, Clone)]
pub struct DiscordianAnalysis {
    pub summary: String,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub political_bias: i32,
    pub sachlichkeit: i32,
    /// Article type: news/analysis/opinion/satire/ad/unknown
    pub article_type: String,
}

/// Validate and normalize article_type to known values
fn normalize_article_type(raw: &str) -> String {
    match raw.to_lowercase().trim() {
        "news" => "news",
        "analysis" => "analysis",
        "opinion" => "opinion",
        "satire" => "satire",
        "ad" | "advertisement" => "ad",
        _ => "unknown",
    }
    .to_string()
}

impl From<RawDiscordianAnalysis> for DiscordianAnalysis {
    fn from(raw: RawDiscordianAnalysis) -> Self {
        Self {
            summary: raw.summary,
            categories: raw.categories,
            keywords: raw.keywords,
            political_bias: raw.political_bias.round() as i32,
            sachlichkeit: raw.sachlichkeit.round() as i32,
            article_type: normalize_article_type(&raw.article_type),
        }
    }
}

/// Raw Discordian analysis with rejections from LLM (for statistical validation workflow)
/// Uses flexible deserializers to handle LLM responses that return objects instead of strings
#[derive(Deserialize, Debug)]
pub struct RawDiscordianAnalysisWithRejections {
    #[serde(default, deserialize_with = "flexible_deser::flexible_string")]
    summary: String,
    #[serde(default, deserialize_with = "flexible_deser::flexible_string_vec")]
    categories: Vec<String>,
    #[serde(default, deserialize_with = "flexible_deser::flexible_string_vec")]
    keywords: Vec<String>,
    #[serde(default, deserialize_with = "flexible_deser::flexible_string_vec")]
    rejected_keywords: Vec<String>,
    #[serde(default, deserialize_with = "flexible_deser::flexible_string_vec")]
    rejected_categories: Vec<String>,
    #[serde(default)]
    political_bias: f64,
    #[serde(default)]
    sachlichkeit: f64,
    /// Article type: news/analysis/opinion/satire/ad/unknown
    #[serde(default = "default_article_type")]
    article_type: String,
}

/// Full Discordian analysis with rejection info for bias learning
#[derive(Serialize, Debug, Clone)]
pub struct DiscordianAnalysisWithRejections {
    pub summary: String,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    /// Keywords from statistical analysis that LLM rejected (for bias learning)
    pub rejected_keywords: Vec<String>,
    /// Categories from statistical analysis that LLM rejected (for bias learning)
    pub rejected_categories: Vec<String>,
    pub political_bias: i32,
    pub sachlichkeit: i32,
    /// Article type: news/analysis/opinion/satire/ad/unknown
    pub article_type: String,
}

impl From<RawDiscordianAnalysisWithRejections> for DiscordianAnalysisWithRejections {
    fn from(raw: RawDiscordianAnalysisWithRejections) -> Self {
        Self {
            summary: raw.summary,
            categories: raw.categories,
            keywords: raw.keywords,
            rejected_keywords: raw.rejected_keywords,
            rejected_categories: raw.rejected_categories,
            political_bias: raw.political_bias.round() as i32,
            sachlichkeit: raw.sachlichkeit.round() as i32,
            article_type: normalize_article_type(&raw.article_type),
        }
    }
}

impl From<DiscordianAnalysisWithRejections> for DiscordianAnalysis {
    fn from(raw: DiscordianAnalysisWithRejections) -> Self {
        Self {
            summary: raw.summary,
            categories: raw.categories,
            keywords: raw.keywords,
            political_bias: raw.political_bias,
            sachlichkeit: raw.sachlichkeit,
            article_type: raw.article_type,
        }
    }
}
