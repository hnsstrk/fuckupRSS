use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OllamaError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Ollama not available: {0}")]
    NotAvailable(String),
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
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
    pub async fn is_available(&self) -> bool {
        self.list_models().await.is_ok()
    }

    /// Generate a summary for article content
    pub async fn summarize(&self, model: &str, content: &str) -> Result<String, OllamaError> {
        let prompt = format!(
            r#"Du bist ein Analyst für Nachrichtenartikel. Erstelle eine kurze, sachliche Zusammenfassung des folgenden Artikels in 2-3 Sätzen. Antworte NUR mit der Zusammenfassung, ohne Einleitung oder Erklärung.

Artikel:
{}

Zusammenfassung:"#,
            content.chars().take(8000).collect::<String>()
        );

        self.generate(model, &prompt).await
    }

    /// Analyze article for bias and objectivity
    pub async fn analyze_bias(
        &self,
        model: &str,
        title: &str,
        content: &str,
    ) -> Result<BiasAnalysis, OllamaError> {
        let prompt = format!(
            r#"Analysiere den folgenden Nachrichtenartikel auf politische Tendenz und Sachlichkeit.

Antworte im folgenden JSON-Format (NUR das JSON, keine Erklärung):
{{
  "political_bias": <-2 bis 2, wobei -2=stark links, 0=neutral, 2=stark rechts>,
  "sachlichkeit": <0 bis 4, wobei 0=stark emotional, 4=sehr sachlich>,
  "article_type": "<news|opinion|analysis|satire|ad|unknown>"
}}

Titel: {}
Inhalt: {}

JSON:"#,
            title,
            content.chars().take(4000).collect::<String>()
        );

        let response = self.generate(model, &prompt).await?;

        // Try to parse JSON from response
        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(json_str).map_err(|e| {
            OllamaError::GenerationFailed(format!("Failed to parse bias analysis: {}", e))
        })
    }

    /// Generate text with Ollama
    async fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaError> {
        let url = format!("{}/api/generate", self.base_url);
        let client = self.client();

        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
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

#[derive(Deserialize, Serialize, Debug)]
pub struct BiasAnalysis {
    pub political_bias: i32,
    pub sachlichkeit: i32,
    pub article_type: String,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new(None)
    }
}
