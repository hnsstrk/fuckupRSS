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

    /// Deserialize a string that might be an object with a text field
    pub fn flexible_string<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;
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

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    options: GenerateOptions,
    keep_alive: String,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
    /// Whether generation completed successfully
    #[serde(default)]
    done: bool,
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

// Embedding structs for Ollama /api/embeddings endpoint
#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
    keep_alive: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

/// Recommended models for fuckupRSS
/// Note: qwen3-vl is a Vision-Language model (slow for text-only tasks)
/// ministral-3 is faster for pure text analysis
pub const RECOMMENDED_MAIN_MODEL: &str = "ministral-3:latest";
/// snowflake-arctic-embed2: Multilingual (74 languages incl. German/English), 1024-dim
pub const RECOMMENDED_EMBEDDING_MODEL: &str = "snowflake-arctic-embed2:latest";

/// Default prompts (English prompts with {language} placeholder for output language)
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
  "categories": ["<cat1>"]
}

Rules:
- political_bias: -2=strong left, 0=neutral, 2=strong right (be precise!)
- sachlichkeit: 0=emotional/sensational, 2=mixed, 4=objective/factual (be precise!)
- summary: 2-3 factual sentences in {language}, capture the key information
- keywords: 3-5 short keywords (1-2 words each) - IMPORTANT for categorization
- categories: 0-1 from: Technik, Politik, Wirtschaft, Wissenschaft, Kultur, Sport, Gesellschaft, Umwelt, Sicherheit, Gesundheit, Verteidigung, Energie, Recht (optional, empty [] is fine)

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

Return ONLY valid JSON:
{
  "political_bias": <-2 to 2>,
  "sachlichkeit": <0 to 4>,
  "summary": "<summary in {language}>",
  "keywords": ["kw1", "kw2", "..."],
  "categories": [],
  "rejected_keywords": [],
  "rejected_categories": []
}

Title: {title}
Content: {content}"#;

/// Get language name for prompt based on locale
pub fn get_language_for_locale(locale: &str) -> &'static str {
    match locale {
        "de" => "German",
        "en" => "English",
        _ => "German", // Default to German
    }
}

/// Default context length - optimized for 12GB GPU (100% GPU, ~1.5s/article)
pub const DEFAULT_NUM_CTX: u32 = 4096;

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

    /// Generate embedding vector for text using nomic-embed-text or similar
    pub async fn generate_embedding(
        &self,
        model: &str,
        text: &str,
    ) -> Result<Vec<f32>, OllamaError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let client = self.client();

        let request = EmbeddingRequest {
            model: model.to_string(),
            prompt: text.to_string(),
            keep_alive: "30m".to_string(),
        };

        let resp = client.post(&url).json(&request).send().await.map_err(|e| {
            OllamaError::GenerationFailed(format!("Embedding request failed: {}", e))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::GenerationFailed(format!(
                "Embedding failed with status {}: {}",
                status, body
            )));
        }

        let result: EmbeddingResponse = resp.json().await.map_err(|e| {
            OllamaError::GenerationFailed(format!("Failed to parse embedding: {}", e))
        })?;

        Ok(result.embedding)
    }

    /// Generate a summary with custom prompt template
    pub async fn summarize_with_prompt(
        &self,
        model: &str,
        content: &str,
        prompt_template: &str,
    ) -> Result<String, OllamaError> {
        let truncated_content = content.chars().take(8000).collect::<String>();
        let prompt = prompt_template.replace("{content}", &truncated_content);
        self.generate(model, &prompt, None).await
    }

    /// Simple text generation (public API for synonym verification etc.)
    pub async fn generate_simple(&self, model: &str, prompt: &str) -> Result<String, OllamaError> {
        self.generate(model, prompt, Some("json".to_string())).await
    }

    /// Generate text with Ollama
    async fn generate(
        &self,
        model: &str,
        prompt: &str,
        format: Option<String>,
    ) -> Result<String, OllamaError> {
        let url = format!("{}/api/generate", self.base_url);
        let client = self.client();

        let prompt_len = prompt.len();
        debug!(
            "[Ollama] Sending request to model '{}' (prompt: {} chars, num_ctx: {})",
            model, prompt_len, self.num_ctx
        );
        let request_start = Instant::now();

        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            format,
            options: GenerateOptions {
                num_ctx: self.num_ctx,
                // Ensure enough output tokens for JSON analysis
                // Default is 128 which is too small for structured output
                // 4096 allows for detailed summaries + full JSON structure
                num_predict: 4096,
            },
            keep_alive: "30m".to_string(),
        };

        let resp: reqwest_new::Response =
            client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|e: reqwest_new::Error| {
                    warn!(
                        "[Ollama] Request failed after {:.2}s: {}",
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
                "[Ollama] Request failed after {:.2}s with status {}: {}",
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

        let result: GenerateResponse = serde_json::from_slice(&bytes)
            .map_err(|e| OllamaError::GenerationFailed(e.to_string()))?;

        let duration = request_start.elapsed();
        let response_len = result.response.len();

        // Warn if generation was incomplete (output truncated)
        if !result.done {
            warn!("[Ollama] Generation incomplete (done=false) after {:.2}s - response may be truncated", duration.as_secs_f64());
        } else {
            debug!(
                "[Ollama] Request completed in {:.2}s (response: {} chars)",
                duration.as_secs_f64(),
                response_len
            );
        }

        Ok(result.response)
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
}

/// Full Discordian analysis with all KI-extracted data
#[derive(Serialize, Debug, Clone)]
pub struct DiscordianAnalysis {
    pub summary: String,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub political_bias: i32,
    pub sachlichkeit: i32,
}

impl From<RawDiscordianAnalysis> for DiscordianAnalysis {
    fn from(raw: RawDiscordianAnalysis) -> Self {
        Self {
            summary: raw.summary,
            categories: raw.categories,
            keywords: raw.keywords,
            political_bias: raw.political_bias.round() as i32,
            sachlichkeit: raw.sachlichkeit.round() as i32,
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
        }
    }
}
