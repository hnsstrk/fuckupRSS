use regex::Regex;
use std::collections::HashMap;
use uuid::Uuid;
use readability::extractor;
use std::time::Duration;
use thiserror::Error;
use url::Url;


pub mod headless;

use crate::retrieval::headless::{HeadlessError, HeadlessFetcher};




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
    #[error("Headless browser error: {0}")]
    Headless(#[from] HeadlessError),
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

        // Preprocess to mask media elements
        let (masked_html, replacements) = preprocess_media_tags(&html);

        // Extract article content using readability
        let extracted = extractor::extract(&mut masked_html.as_bytes(), &url)
            .map_err(|e| RetrievalError::Extraction(e.to_string()))?;

        // Restore media elements
        let content = postprocess_media_tags(&extracted.content, &replacements);

        Ok(ExtractedArticle {
            title: Some(extracted.title),
            content,
            text_content: extracted.text,
        })
    }

    /// Fetch and extract article content with optional headless browser fallback.
    ///
    /// This method first attempts regular HTTP fetch with readability extraction.
    /// If the extracted content is too short (< 500 chars) and `use_headless` is true,
    /// it falls back to using the headless browser to render JavaScript and then
    /// re-extracts the content.
    ///
    /// # Arguments
    ///
    /// * `article_url` - The URL of the article to fetch.
    /// * `use_headless` - Whether to enable headless browser fallback.
    /// * `headless_fetcher` - Optional reference to a HeadlessFetcher instance.
    ///   Required when `use_headless` is true.
    ///
    /// # Returns
    ///
    /// The extracted article content.
    ///
    /// # Errors
    ///
    /// Returns `RetrievalError` if fetching or extraction fails.
    pub async fn retrieve_with_fallback(
        &self,
        article_url: &str,
        use_headless: bool,
        headless_fetcher: Option<&HeadlessFetcher>,
    ) -> Result<ExtractedArticle, RetrievalError> {
        let url = Url::parse(article_url)?;

        // First attempt: regular HTTP fetch with readability
        let response: reqwest_new::Response = self.client.get(url.clone()).send().await?;
        let html: String = response.text().await?;

        // Preprocess masked HTML
        let (masked_html, replacements) = preprocess_media_tags(&html);

        let extracted = extractor::extract(&mut masked_html.as_bytes(), &url)
            .map_err(|e| RetrievalError::Extraction(e.to_string()))?;

        let content = postprocess_media_tags(&extracted.content, &replacements);

        let result = ExtractedArticle {
            title: Some(extracted.title),
            content,
            text_content: extracted.text,
        };

        // Check if content is sufficient or if fallback is disabled/unavailable
        if result.content.len() >= 500 || !use_headless {
            log::debug!(
                "Regular extraction successful for {} ({} chars)",
                article_url,
                result.content.len()
            );
            return Ok(result);
        }

        // Fallback: use headless browser if available
        let Some(fetcher) = headless_fetcher else {
            log::warn!(
                "Content too short ({} chars) for {} but no headless fetcher provided",
                result.content.len(),
                article_url
            );
            return Ok(result);
        };

        log::info!(
            "Content too short ({} chars) for {}, attempting headless fallback",
            result.content.len(),
            article_url
        );

        // Fetch with headless browser (renders JavaScript)
        let rendered_html = fetcher.fetch(article_url).await?;

        // Preprocess rendered HTML too
        let (masked_rendered, rendered_replacements) = preprocess_media_tags(&rendered_html);

        // Re-extract content from the rendered HTML
        let extracted_from_rendered = extractor::extract(&mut masked_rendered.as_bytes(), &url)
            .map_err(|e| RetrievalError::Extraction(format!("Headless extraction failed: {}", e)))?;

        let rendered_content = postprocess_media_tags(&extracted_from_rendered.content, &rendered_replacements);

        let headless_result = ExtractedArticle {
            title: Some(extracted_from_rendered.title),
            content: rendered_content,
            text_content: extracted_from_rendered.text,
        };

        log::info!(
            "Headless fallback successful for {} ({} chars vs {} chars original)",
            article_url,
            headless_result.content.len(),
            result.content.len()
        );

        // Return the better result (headless or original)
        if headless_result.content.len() > result.content.len() {
            Ok(headless_result)
        } else {
            log::warn!(
                "Headless result not better for {}, using original",
                article_url
            );
            Ok(result)
        }
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

/// Preprocess HTML to mask media elements so they don't get stripped by readability
fn preprocess_media_tags(html: &str) -> (String, HashMap<String, String>) {
    let mut masked_html = html.to_string();
    let mut replacements = HashMap::new();

    // Patterns to match iframes and videos
    // Note: Regex for HTML is brittle, but sufficient for this specific masking task
    // We use non-greedy matching for content and ignore case
    let patterns = [
        ("iframe", Regex::new(r#"(?is)<iframe[^>]*>.*?</iframe>"#).unwrap()),
        ("video", Regex::new(r#"(?is)<video[^>]*>.*?</video>"#).unwrap()),
        ("object", Regex::new(r#"(?is)<object[^>]*>.*?</object>"#).unwrap()),
        ("embed", Regex::new(r#"(?is)<embed[^>]*>.*?</embed>"#).unwrap()),
    ];

    for (tag_type, regex) in patterns.iter() {
        masked_html = regex.replace_all(&masked_html, |caps: &regex::Captures| {
            let original = caps[0].to_string();
            let uuid = Uuid::new_v4().to_string();
            let key = format!("MEDIA_MASK_{}_{}", tag_type, uuid);
            
            // Use a p tag with sufficient text length to avoid being classified as "short content" or "empty"
            let replacement = format!(
                r#"<p id="{}" class="fuckuprss-media-placeholder">MEDIA_MASK_{} - This is a placeholder for embedded media content that must be preserved by the extraction algorithm.</p>"#, 
                key, uuid
            );
            
            replacements.insert(key, original);
            replacement
        }).to_string();
    }

    (masked_html, replacements)
}

/// Postprocess extracted content to restore masked media
fn postprocess_media_tags(content: &str, replacements: &HashMap<String, String>) -> String {
    let mut processed = content.to_string();
    
    for (key, original) in replacements {
        // Reconstruct the placeholder text used in preprocess
        // It matches the format: "MEDIA_MASK_... - This is a placeholder..."
        // We accept that "uuid" is part of "key"
        // key format: MEDIA_MASK_{tag}_{uuid}
        // placeholder format: {key} - This is a placeholder for embedded media content that must be preserved by the extraction algorithm.
        let placeholder_text = format!("{} - This is a placeholder for embedded media content that must be preserved by the extraction algorithm.", key);
        
        // Strategy 1: Find the element with the ID and replace the WHOLE element
        // This is cleanest as it removes the wrapper <p>
        let id_pattern = format!(r#"<[^>]+id="{}"[^>]*>.*?</[^>]+>"#, key);
        if let Ok(re) = Regex::new(&id_pattern) {
            if re.is_match(&processed) {
                processed = re.replace_all(&processed, original.as_str()).to_string();
                continue;
            }
        }
        
        // Strategy 2: Find the placeholder text and replace it
        // This handles cases where the ID was stripped but text preserved
        if processed.contains(&placeholder_text) {
            processed = processed.replace(&placeholder_text, original);
        } else if processed.contains(key) {
             // Strategy 3: Just the key (fallback if readability truncated our long text)
             processed = processed.replace(key, original);
        }
    }
    
    processed
}
