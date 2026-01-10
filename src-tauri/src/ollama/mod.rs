use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[cfg(test)]
mod tests;

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
    #[error("JSON parse error: {message}")]
    JsonParseError {
        message: String,
        raw_response: String,
    },
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

// Embedding structs for future nomic-embed-text integration
#[derive(Serialize)]
#[allow(dead_code)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

/// Recommended models for fuckupRSS
/// Note: qwen3-vl is a Vision-Language model (slow for text-only tasks)
/// ministral-3 is faster for pure text analysis
pub const RECOMMENDED_MAIN_MODEL: &str = "ministral-3:latest";
/// snowflake-arctic-embed2: Multilingual (74 languages incl. German/English), 1024-dim
pub const RECOMMENDED_EMBEDDING_MODEL: &str = "snowflake-arctic-embed2";

/// Default prompts (English prompts with {language} placeholder for output language)
pub const DEFAULT_SUMMARY_PROMPT: &str = r#"/no_think
You are a news article analyst. Create a brief, factual summary of the following article in 2-3 sentences.

IMPORTANT: Respond ONLY in {language}. Do not use any other language.
Respond ONLY with the summary, without introduction or explanation.

Article:
{content}

Summary:"#;

pub const DEFAULT_ANALYSIS_PROMPT: &str = r#"/no_think
Analyze the following news article for political bias and objectivity.
Respond in the following JSON format:
{
  "political_bias": <-2 to 2, where -2=strong left, 0=neutral, 2=strong right>,
  "sachlichkeit": <0 to 4, where 0=highly emotional, 4=very objective>,
  "article_type": "<news|opinion|analysis|satire|ad|unknown>"
}

Title: {title}
Content: {content}"#;

/// Combined prompt for full Discordian Analysis (summary + bias + categories + keywords)
pub const DEFAULT_DISCORDIAN_PROMPT: &str = r#"/no_think
Analyze this news article. Respond in {language}. Return ONLY this JSON:

{
  "political_bias": <-2 to 2>,
  "sachlichkeit": <0 to 4>,
  "summary": "<1-2 sentences>",
  "categories": ["<cat1>", "<cat2>"],
  "keywords": ["<kw1>", "<kw2>", "<kw3>"]
}

Rules:
- political_bias: -2=strong left, 0=neutral, 2=strong right
- sachlichkeit: 0=emotional, 2=mixed, 4=objective
- summary: 1-2 factual sentences in {language}
- categories: 1-2 from: Technik, Politik, Wirtschaft, Wissenschaft, Kultur, Sport, Gesellschaft, Umwelt, Sicherheit, Gesundheit, Verteidigung, Energie, Recht
- keywords: 3-5 short keywords (1-2 words each)

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
}

impl OllamaClient {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            num_ctx: DEFAULT_NUM_CTX,
        }
    }

    /// Create client with custom context length
    pub fn with_context(base_url: Option<String>, num_ctx: u32) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            num_ctx,
        }
    }

    fn client(&self) -> reqwest_new::Client {
        reqwest_new::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client")
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

        let models: ModelsResponse = serde_json::from_slice(&bytes)
            .map_err(|e| OllamaError::NotAvailable(e.to_string()))?;

        Ok(models.models)
    }

    /// Check if Ollama is running
    #[allow(dead_code)]
    pub async fn is_available(&self) -> bool {
        self.list_models().await.is_ok()
    }

    /// Pull/download a model from Ollama
    pub async fn pull_model(&self, model_name: &str) -> Result<String, OllamaError> {
        let url = format!("{}/api/pull", self.base_url);
        let client = reqwest_new::Client::builder()
            .timeout(Duration::from_secs(3600)) // 1 hour timeout for large models
            .build()
            .expect("Failed to create HTTP client");

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
            return Err(OllamaError::PullFailed(format!("Status {}: {}", status, body)));
        }

        let result: PullResponse = resp
            .json()
            .await
            .map_err(|e| OllamaError::PullFailed(e.to_string()))?;

        Ok(result.status)
    }

    /// Check if a specific model is installed
    #[allow(dead_code)]
    pub async fn has_model(&self, model_name: &str) -> bool {
        match self.list_models().await {
            Ok(models) => models.iter().any(|m| {
                m.name == model_name || m.name.starts_with(&format!("{}:", model_name))
            }),
            Err(_) => false,
        }
    }

    /// Generate embedding vector for text using nomic-embed-text or similar
    #[allow(dead_code)]
    pub async fn generate_embedding(&self, model: &str, text: &str) -> Result<Vec<f32>, OllamaError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let client = self.client();

        let request = EmbeddingRequest {
            model: model.to_string(),
            prompt: text.to_string(),
        };

        let resp = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| OllamaError::GenerationFailed(format!("Embedding request failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::GenerationFailed(format!(
                "Embedding failed with status {}: {}",
                status, body
            )));
        }

        let result: EmbeddingResponse = resp
            .json()
            .await
            .map_err(|e| OllamaError::GenerationFailed(format!("Failed to parse embedding: {}", e)))?;

        Ok(result.embedding)
    }

    /// Generate embeddings for multiple texts (batch) parallelly
    #[allow(dead_code)]
    pub async fn generate_embeddings_batch(
        &self,
        model: &str,
        texts: &[String],
    ) -> Vec<Result<Vec<f32>, OllamaError>> {
        let futures = texts.iter().map(|text| self.generate_embedding(model, text));
        futures::future::join_all(futures).await
    }

    /// Generate a summary for article content
    #[allow(dead_code)]
    pub async fn summarize(&self, model: &str, content: &str) -> Result<String, OllamaError> {
        self.summarize_with_prompt(model, content, DEFAULT_SUMMARY_PROMPT).await
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

    /// Analyze article for bias and objectivity
    #[allow(dead_code)]
    pub async fn analyze_bias(
        &self,
        model: &str,
        title: &str,
        content: &str,
    ) -> Result<BiasAnalysis, OllamaError> {
        self.analyze_bias_with_prompt(model, title, content, DEFAULT_ANALYSIS_PROMPT).await
    }

    /// Analyze article with custom prompt template
    pub async fn analyze_bias_with_prompt(
        &self,
        model: &str,
        title: &str,
        content: &str,
        prompt_template: &str,
    ) -> Result<BiasAnalysis, OllamaError> {
        let truncated_content = content.chars().take(4000).collect::<String>();
        let prompt = prompt_template
            .replace("{title}", title)
            .replace("{content}", &truncated_content);

        // Use JSON mode
        let response = self.generate(model, &prompt, Some("json".to_string())).await?;

        // Parse directly
        let raw: RawBiasAnalysis = serde_json::from_str(&response).map_err(|e| {
            warn!("JSON parse error: {}. Response: {}", e, truncate_str(&response, 300));
            OllamaError::JsonParseError {
                message: e.to_string(),
                raw_response: response.chars().take(500).collect(),
            }
        })?;

        Ok(raw.into())
    }

    /// Full Discordian Analysis: Summary + Bias + Categories + Keywords in one call
    pub async fn discordian_analysis(
        &self,
        model: &str,
        title: &str,
        content: &str,
        locale: &str,
    ) -> Result<DiscordianAnalysis, OllamaError> {
        self.discordian_analysis_with_retry(model, title, content, locale, None).await
    }

    /// Discordian Analysis with optional retry feedback
    /// If previous_error is provided, sends a correction request to the LLM
    pub async fn discordian_analysis_with_retry(
        &self,
        model: &str,
        title: &str,
        content: &str,
        locale: &str,
        previous_error: Option<&str>,
    ) -> Result<DiscordianAnalysis, OllamaError> {
        debug!("Starting Discordian analysis for: {}", truncate_str(title, 60));
        let language = get_language_for_locale(locale);
        let truncated_content = content.chars().take(6000).collect::<String>();

        let prompt = if let Some(error) = previous_error {
            // Retry prompt with error feedback
            format!(
                r#"Your previous response could not be parsed. Error: {}
Return ONLY valid JSON:
{{
  "political_bias": 0,
  "sachlichkeit": 2,
  "summary": "...",
  "categories": ["..."],
  "keywords": ["..."]
}}

Title: {}
Content: {}"#,
                error, title, truncated_content
            )
        } else {
            // Normal prompt
            DEFAULT_DISCORDIAN_PROMPT
                .replace("{language}", language)
                .replace("{title}", title)
                .replace("{content}", &truncated_content)
        };

        // Use JSON mode
        let response = self.generate(model, &prompt, Some("json".to_string())).await?;

        let raw: RawDiscordianAnalysis = serde_json::from_str(&response).map_err(|e| {
            warn!("JSON parse error: {}. Response: {}", e, truncate_str(&response, 300));
            OllamaError::JsonParseError {
                message: e.to_string(),
                raw_response: response.chars().take(500).collect(),
            }
        })?;

        debug!(
            "Analysis complete: {} categories, {} keywords",
            raw.categories.len(),
            raw.keywords.len()
        );

        Ok(raw.into())
    }

    /// Generate text with Ollama
    async fn generate(&self, model: &str, prompt: &str, format: Option<String>) -> Result<String, OllamaError> {
        let url = format!("{}/api/generate", self.base_url);
        let client = self.client();

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
        };

        let resp: reqwest_new::Response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e: reqwest_new::Error| OllamaError::NotAvailable(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body: String = resp
                .text()
                .await
                .unwrap_or_else(|_: reqwest_new::Error| "Unknown error".to_string());
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

        // Warn if generation was incomplete (output truncated)
        if !result.done {
            warn!("Generation incomplete (done=false) - response may be truncated");
        }

        Ok(result.response)
    }
}

/// Raw bias analysis from LLM (accepts floats)
#[derive(Deserialize, Debug)]
struct RawBiasAnalysis {
    political_bias: f64,
    sachlichkeit: f64,
    article_type: String,
}

/// Bias analysis with integer values (for storage and display)
#[derive(Serialize, Debug, Clone)]
pub struct BiasAnalysis {
    pub political_bias: i32,
    pub sachlichkeit: i32,
    pub article_type: String,
}

impl From<RawBiasAnalysis> for BiasAnalysis {
    fn from(raw: RawBiasAnalysis) -> Self {
        Self {
            political_bias: raw.political_bias.round() as i32,
            sachlichkeit: raw.sachlichkeit.round() as i32,
            article_type: raw.article_type,
        }
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Raw Discordian analysis from LLM (accepts floats)
#[derive(Deserialize, Debug)]
struct RawDiscordianAnalysis {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
    political_bias: f64,
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
