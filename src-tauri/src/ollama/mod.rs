use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[cfg(test)]
mod tests;

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
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
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
pub const RECOMMENDED_EMBEDDING_MODEL: &str = "nomic-embed-text";

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

IMPORTANT: Respond ONLY in the specified JSON format. Do not use any other format or add explanations.

Respond in the following JSON format (ONLY the JSON, no explanation):
{
  "political_bias": <-2 to 2, where -2=strong left, 0=neutral, 2=strong right>,
  "sachlichkeit": <0 to 4, where 0=highly emotional, 4=very objective>,
  "article_type": "<news|opinion|analysis|satire|ad|unknown>"
}

Title: {title}
Content: {content}

JSON:"#;

/// Combined prompt for full Discordian Analysis (summary + bias + categories + keywords)
pub const DEFAULT_DISCORDIAN_PROMPT: &str = r#"/no_think
Analyze this news article comprehensively. Respond in {language}.

IMPORTANT: Respond ONLY with the JSON object below. No explanation, no markdown.

{
  "summary": "<2-3 sentence summary in {language}>",
  "categories": ["<category1>", "<category2>"],
  "keywords": ["<keyword1>", "<keyword2>", "<keyword3>"],
  "political_bias": <-2 to 2>,
  "sachlichkeit": <0 to 4>,
  "article_type": "<type>"
}

Rules:
- summary: 2-3 factual sentences
- categories: 1-3 from ONLY these: Technik, Politik, Wirtschaft, Wissenschaft, Kultur, Sport, Gesellschaft, Umwelt, Sicherheit, Gesundheit, Verteidigung, Energie, Recht
- keywords: 3-7 specific terms (people, places, organizations, concepts) - NOT generic words
- political_bias: -2=strong left, -1=lean left, 0=neutral, 1=lean right, 2=strong right
- sachlichkeit: 0=highly emotional, 1=emotional, 2=mixed, 3=mostly objective, 4=objective
- article_type: news, opinion, analysis, satire, ad, or unknown

Title: {title}
Content: {content}

JSON:"#;

/// Get language name for prompt based on locale
pub fn get_language_for_locale(locale: &str) -> &'static str {
    match locale {
        "de" => "German",
        "en" => "English",
        _ => "German", // Default to German
    }
}

/// Extract and fix JSON object from LLM response, handling various formats
fn extract_json_from_response(response: &str) -> String {
    let trimmed = response.trim();

    // Remove markdown code blocks
    let without_markdown = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try to find JSON object by looking for { and }
    let json_str = if let Some(start) = without_markdown.find('{') {
        if let Some(end) = without_markdown.rfind('}') {
            if end > start {
                without_markdown[start..=end].to_string()
            } else {
                without_markdown.to_string()
            }
        } else {
            without_markdown.to_string()
        }
    } else {
        without_markdown.to_string()
    };

    // Fix common LLM JSON mistakes
    fix_json_string(&json_str)
}

/// Fix common JSON mistakes from LLM output
fn fix_json_string(json: &str) -> String {
    let mut result = json.to_string();

    // Replace single quotes with double quotes (common LLM mistake)
    // Be careful to only replace quotes that are likely JSON string delimiters
    result = fix_quotes(&result);

    // Remove trailing commas before } or ]
    result = result.replace(",}", "}").replace(",]", "]");

    result
}

/// Replace single quotes with double quotes in JSON-like strings
fn fix_quotes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '\'' {
            // Check if this looks like a JSON string delimiter
            // (preceded by : or , or [ or { with optional whitespace, or followed by similar)
            result.push('"');
        } else {
            result.push(c);
        }
        i += 1;
    }

    result
}

/// Ollama API client for local LLM inference
pub struct OllamaClient {
    base_url: String,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
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

    /// Generate embeddings for multiple texts (batch)
    #[allow(dead_code)]
    pub async fn generate_embeddings_batch(
        &self,
        model: &str,
        texts: &[String],
    ) -> Vec<Result<Vec<f32>, OllamaError>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.generate_embedding(model, text).await);
        }
        results
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
        self.generate(model, &prompt).await
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

        let response = self.generate(model, &prompt).await?;

        // Try to extract JSON from response - handle various LLM output formats
        let json_str = extract_json_from_response(&response);

        // Parse as RawBiasAnalysis (accepts floats) then convert to BiasAnalysis (integers)
        let raw: RawBiasAnalysis = serde_json::from_str(&json_str).map_err(|e| {
            eprintln!("JSON parse error: {}. Extracted JSON: {}", e, &json_str[..json_str.len().min(300)]);
            OllamaError::GenerationFailed(format!("Failed to parse bias analysis: {}", e))
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
        let language = get_language_for_locale(locale);
        let truncated_content = content.chars().take(6000).collect::<String>();

        let prompt = DEFAULT_DISCORDIAN_PROMPT
            .replace("{language}", language)
            .replace("{title}", title)
            .replace("{content}", &truncated_content);

        let response = self.generate(model, &prompt).await?;
        let json_str = extract_json_from_response(&response);

        let raw: RawDiscordianAnalysis = serde_json::from_str(&json_str).map_err(|e| {
            // Only log on actual parse failure (after extraction was attempted)
            eprintln!("JSON parse error: {}. Extracted JSON: {}", e, &json_str[..json_str.len().min(300)]);
            OllamaError::GenerationFailed(format!(
                "Failed to parse Discordian analysis: {}",
                e
            ))
        })?;

        Ok(raw.into())
    }

    /// Generate text with Ollama
    async fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaError> {
        let url = format!("{}/api/generate", self.base_url);
        let client = self.client();

        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            options: GenerateOptions {
                num_ctx: 8192, // 8K context is enough for article analysis
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
    summary: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
    political_bias: f64,
    sachlichkeit: f64,
    article_type: String,
}

/// Full Discordian analysis with all KI-extracted data
#[derive(Serialize, Debug, Clone)]
pub struct DiscordianAnalysis {
    pub summary: String,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub political_bias: i32,
    pub sachlichkeit: i32,
    pub article_type: String,
}

impl From<RawDiscordianAnalysis> for DiscordianAnalysis {
    fn from(raw: RawDiscordianAnalysis) -> Self {
        Self {
            summary: raw.summary,
            categories: raw.categories,
            keywords: raw.keywords,
            political_bias: raw.political_bias.round() as i32,
            sachlichkeit: raw.sachlichkeit.round() as i32,
            article_type: raw.article_type,
        }
    }
}
