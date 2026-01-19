//! Headless browser fetching module using chromiumoxide.
//!
//! This module provides a `HeadlessFetcher` for fetching web pages that require
//! JavaScript rendering. It uses a headless Chromium browser to render pages
//! and extract content.
//!
//! # Example
//!
//! ```no_run
//! use fuckuprss_lib::retrieval::headless::HeadlessFetcher;
//!
//! async fn example() {
//!     let fetcher = HeadlessFetcher::new();
//!
//!     // Get rendered HTML
//!     let html = fetcher.fetch("https://example.com").await.unwrap();
//!
//!     // Or extract just text content
//!     let text = fetcher.extract_text("https://example.com").await.unwrap();
//! }
//! ```

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::error::CdpError;
use chromiumoxide::Page;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::OnceCell;
use tokio::time::timeout;

/// Default timeout for page loading operations (30 seconds).
const DEFAULT_PAGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Default timeout for browser initialization (60 seconds).
const DEFAULT_BROWSER_INIT_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum wait time for network idle detection.
const NETWORK_IDLE_TIMEOUT: Duration = Duration::from_secs(10);

/// Errors that can occur during headless browser operations.
#[derive(Error, Debug)]
pub enum HeadlessError {
    /// Failed to initialize the browser.
    #[error("Browser initialization failed: {0}")]
    BrowserInit(String),

    /// Failed to create a new page/tab.
    #[error("Failed to create new page: {0}")]
    PageCreation(String),

    /// Navigation to the URL failed.
    #[error("Navigation failed for URL '{url}': {message}")]
    Navigation { url: String, message: String },

    /// Page loading timed out.
    #[error("Page load timeout after {timeout_secs}s for URL: {url}")]
    Timeout { url: String, timeout_secs: u64 },

    /// Failed to extract content from the page.
    #[error("Content extraction failed: {0}")]
    ContentExtraction(String),

    /// CDP (Chrome DevTools Protocol) error.
    #[error("CDP error: {0}")]
    Cdp(#[from] CdpError),

    /// Browser connection was lost.
    #[error("Browser connection lost")]
    ConnectionLost,

    /// Invalid URL provided.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// Internal browser state managed by `HeadlessFetcher`.
struct BrowserState {
    browser: Browser,
    #[allow(dead_code)]
    handle: tokio::task::JoinHandle<()>,
}

/// A headless browser fetcher for rendering JavaScript-heavy pages.
///
/// The browser is lazily initialized on first use to avoid resource consumption
/// when not needed. It uses Chromium/Chrome in headless mode.
pub struct HeadlessFetcher {
    /// Lazily initialized browser instance.
    browser_state: Arc<OnceCell<BrowserState>>,
    /// Timeout for page loading.
    page_timeout: Duration,
}

impl HeadlessFetcher {
    /// Creates a new `HeadlessFetcher` with default settings.
    ///
    /// The browser is not started until the first fetch operation.
    /// This allows creating the fetcher cheaply without resource overhead.
    pub fn new() -> Self {
        Self {
            browser_state: Arc::new(OnceCell::new()),
            page_timeout: DEFAULT_PAGE_TIMEOUT,
        }
    }

    /// Creates a new `HeadlessFetcher` with a custom page timeout.
    ///
    /// # Arguments
    ///
    /// * `page_timeout` - Maximum time to wait for a page to load.
    pub fn with_timeout(page_timeout: Duration) -> Self {
        Self {
            browser_state: Arc::new(OnceCell::new()),
            page_timeout,
        }
    }

    /// Initializes the browser lazily.
    ///
    /// This method is called automatically on first fetch. It starts a headless
    /// Chromium browser instance.
    async fn ensure_browser(&self) -> Result<&BrowserState, HeadlessError> {
        self.browser_state
            .get_or_try_init(|| async {
                log::info!("Initializing headless browser...");

                // Note: chromiumoxide 0.7 runs headless by default.
                // Use .with_head() (no args) to enable headed mode.
                let browser_config = BrowserConfig::builder()
                    .no_sandbox() // Required for some environments
                    .viewport(None) // No viewport restriction
                    .request_timeout(DEFAULT_PAGE_TIMEOUT)
                    .build()
                    .map_err(|e| HeadlessError::BrowserInit(e.to_string()))?;

                // Launch browser with timeout
                let launch_result = timeout(
                    DEFAULT_BROWSER_INIT_TIMEOUT,
                    Browser::launch(browser_config),
                )
                .await
                .map_err(|_| {
                    HeadlessError::BrowserInit(
                        "Browser launch timed out. Is Chrome/Chromium installed?".to_string(),
                    )
                })?
                .map_err(|e| HeadlessError::BrowserInit(e.to_string()))?;

                let (browser, mut handler) = launch_result;

                // Spawn handler to process browser events
                let handle = tokio::spawn(async move {
                    while let Some(_event) = handler.next().await {
                        // Events are handled internally
                    }
                });

                log::info!("Headless browser initialized successfully");

                Ok(BrowserState { browser, handle })
            })
            .await
    }

    /// Creates a new page with standard configurations.
    async fn create_page(&self) -> Result<Page, HeadlessError> {
        let state = self.ensure_browser().await?;

        state
            .browser
            .new_page("about:blank")
            .await
            .map_err(|e| HeadlessError::PageCreation(e.to_string()))
    }

    /// Navigates to a URL and waits for the page to be ready.
    ///
    /// This method:
    /// 1. Navigates to the URL
    /// 2. Waits for DOMContentLoaded
    /// 3. Attempts to wait for network idle (with timeout fallback)
    async fn navigate_and_wait(&self, page: &Page, url: &str) -> Result<(), HeadlessError> {
        // Validate URL
        url::Url::parse(url).map_err(|e| HeadlessError::InvalidUrl(e.to_string()))?;

        let nav_params = NavigateParams::builder()
            .url(url)
            .build()
            .map_err(|e| HeadlessError::Navigation {
                url: url.to_string(),
                message: format!("Failed to build navigation params: {}", e),
            })?;

        // Navigate with timeout
        let nav_result = timeout(self.page_timeout, page.execute(nav_params))
            .await
            .map_err(|_| HeadlessError::Timeout {
                url: url.to_string(),
                timeout_secs: self.page_timeout.as_secs(),
            })?
            .map_err(|e| HeadlessError::Navigation {
                url: url.to_string(),
                message: e.to_string(),
            })?;

        // Check for navigation errors
        if let Some(ref error_text) = nav_result.error_text {
            return Err(HeadlessError::Navigation {
                url: url.to_string(),
                message: error_text.clone(),
            });
        }

        // Wait for DOMContentLoaded
        timeout(self.page_timeout, page.wait_for_navigation())
            .await
            .map_err(|_| HeadlessError::Timeout {
                url: url.to_string(),
                timeout_secs: self.page_timeout.as_secs(),
            })?
            .map_err(|e| HeadlessError::Navigation {
                url: url.to_string(),
                message: format!("Navigation wait failed: {}", e),
            })?;

        // Give JavaScript a moment to execute
        // This is a pragmatic approach since true "network idle" detection is complex
        tokio::time::sleep(NETWORK_IDLE_TIMEOUT.min(Duration::from_secs(3))).await;

        Ok(())
    }

    /// Fetches a URL and returns the rendered HTML content.
    ///
    /// This method launches a headless browser (if not already running),
    /// navigates to the URL, waits for JavaScript to execute, and returns
    /// the fully rendered HTML.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch.
    ///
    /// # Returns
    ///
    /// The rendered HTML content as a string.
    ///
    /// # Errors
    ///
    /// Returns `HeadlessError` if:
    /// - Browser initialization fails
    /// - Navigation fails
    /// - Page loading times out
    /// - Content extraction fails
    pub async fn fetch(&self, url: &str) -> Result<String, HeadlessError> {
        log::debug!("Fetching URL with headless browser: {}", url);

        let page = self.create_page().await?;
        self.navigate_and_wait(&page, url).await?;

        // Get the rendered HTML
        let html = page
            .content()
            .await
            .map_err(|e| HeadlessError::ContentExtraction(e.to_string()))?;

        // Close the page to free resources
        if let Err(e) = page.close().await {
            log::warn!("Failed to close page: {}", e);
        }

        log::debug!("Successfully fetched {} bytes from {}", html.len(), url);
        Ok(html)
    }

    /// Fetches a URL and extracts only the text content.
    ///
    /// This method is similar to `fetch()`, but instead of returning raw HTML,
    /// it extracts the visible text content from the page using JavaScript's
    /// `document.body.innerText`.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch.
    ///
    /// # Returns
    ///
    /// The visible text content of the page.
    ///
    /// # Errors
    ///
    /// Returns `HeadlessError` if:
    /// - Browser initialization fails
    /// - Navigation fails
    /// - Page loading times out
    /// - Text extraction fails
    pub async fn extract_text(&self, url: &str) -> Result<String, HeadlessError> {
        log::debug!("Extracting text from URL with headless browser: {}", url);

        let page = self.create_page().await?;
        self.navigate_and_wait(&page, url).await?;

        // Extract text content using JavaScript
        let text: String = page
            .evaluate(
                r#"
                (() => {
                    // Remove script and style elements to get cleaner text
                    const scripts = document.querySelectorAll('script, style, noscript');
                    scripts.forEach(el => el.remove());

                    // Get the body text content
                    return document.body ? document.body.innerText || '' : '';
                })()
                "#,
            )
            .await
            .map_err(|e| HeadlessError::ContentExtraction(format!("JavaScript evaluation failed: {}", e)))?
            .into_value()
            .map_err(|e| HeadlessError::ContentExtraction(format!("Failed to convert result: {}", e)))?;

        // Close the page to free resources
        if let Err(e) = page.close().await {
            log::warn!("Failed to close page: {}", e);
        }

        // Clean up the text (normalize whitespace)
        let cleaned_text = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<&str>>()
            .join("\n");

        log::debug!(
            "Successfully extracted {} chars of text from {}",
            cleaned_text.len(),
            url
        );
        Ok(cleaned_text)
    }

    /// Checks if the browser is currently initialized.
    ///
    /// This can be useful for monitoring or deciding whether to use
    /// headless fetching vs. regular HTTP fetching.
    pub fn is_initialized(&self) -> bool {
        self.browser_state.initialized()
    }

    /// Returns the configured page timeout.
    pub fn page_timeout(&self) -> Duration {
        self.page_timeout
    }
}

impl Default for HeadlessFetcher {
    fn default() -> Self {
        Self::new()
    }
}

// Note: We don't implement Drop with explicit browser shutdown because:
// 1. The browser process is tied to the handler task
// 2. When HeadlessFetcher is dropped, the BrowserState is dropped
// 3. This drops the Browser, which cleans up the connection
// 4. The handler task will then complete when it receives no more events

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_fetcher_creation() {
        let fetcher = HeadlessFetcher::new();
        assert!(!fetcher.is_initialized());
        assert_eq!(fetcher.page_timeout(), DEFAULT_PAGE_TIMEOUT);
    }

    #[test]
    fn test_headless_fetcher_with_custom_timeout() {
        let custom_timeout = Duration::from_secs(60);
        let fetcher = HeadlessFetcher::with_timeout(custom_timeout);
        assert_eq!(fetcher.page_timeout(), custom_timeout);
    }

    #[test]
    fn test_headless_error_display() {
        let err = HeadlessError::Timeout {
            url: "https://example.com".to_string(),
            timeout_secs: 30,
        };
        assert!(err.to_string().contains("30s"));
        assert!(err.to_string().contains("example.com"));
    }

    #[test]
    fn test_invalid_url_error() {
        let err = HeadlessError::InvalidUrl("not a valid url".to_string());
        assert!(err.to_string().contains("Invalid URL"));
    }
}
