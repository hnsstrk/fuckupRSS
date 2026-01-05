use readability::extractor;
use std::time::Duration;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum RetrievalError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest_new::Error),
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("Readability extraction failed: {0}")]
    Extraction(String),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
}

/// Extracted article content
#[derive(Debug, Clone)]
pub struct ExtractedArticle {
    #[allow(dead_code)]
    pub title: Option<String>,
    pub content: String,
    #[allow(dead_code)]
    pub text_content: String,
}

/// Hagbard's Retrieval - Full-text article fetcher
pub struct HagbardRetrieval {
    client: reqwest_new::Client,
}

impl HagbardRetrieval {
    pub fn new() -> Self {
        let client = reqwest_new::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (compatible; fuckupRSS/0.1; +https://github.com/fuckuprss)")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Fetch and extract full article content from URL
    pub async fn retrieve(&self, article_url: &str) -> Result<ExtractedArticle, RetrievalError> {
        let url = Url::parse(article_url)?;

        // Fetch the page
        let response: reqwest_new::Response = self.client.get(url.clone()).send().await?;
        let html: String = response.text().await?;

        // Extract article content using readability
        let extracted = extractor::extract(&mut html.as_bytes(), &url)
            .map_err(|e| RetrievalError::Extraction(e.to_string()))?;

        Ok(ExtractedArticle {
            title: Some(extracted.title),
            content: extracted.content,
            text_content: extracted.text,
        })
    }

    /// Check if content appears to be truncated (likely needs full-text fetch)
    pub fn is_truncated(content: &str) -> bool {
        // Heuristics for detecting truncated content:
        // 1. Very short content (< 500 chars)
        // 2. Ends with "..." or "Read more" patterns
        // 3. Contains typical truncation markers

        let content_trimmed = content.trim();
        let len = content_trimmed.len();

        // Too short - likely truncated
        if len < 500 {
            return true;
        }

        // Check for truncation patterns
        let lower = content_trimmed.to_lowercase();
        let truncation_patterns = [
            "...",
            "…",
            "read more",
            "weiterlesen",
            "continue reading",
            "[...]",
            "mehr lesen",
            "read the full",
        ];

        for pattern in &truncation_patterns {
            if lower.ends_with(pattern) || lower.contains(&format!("{}</", pattern)) {
                return true;
            }
        }

        false
    }
}

impl Default for HagbardRetrieval {
    fn default() -> Self {
        Self::new()
    }
}
