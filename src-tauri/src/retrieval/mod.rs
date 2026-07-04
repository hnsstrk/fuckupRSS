use readability::extractor;
use regex::Regex;
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use url::Url;
use uuid::Uuid;

pub mod headless;

use crate::retrieval::headless::{HeadlessError, HeadlessFetcher};

/// Minimum extracted content length (chars) before the headless fallback is
/// skipped in `retrieve_with_fallback`. Below this, the regular HTTP + readability
/// path is treated as insufficient and the JavaScript-rendering fallback kicks in.
///
/// Why 1500: RSS-feed snippets at BBC, Le Monde and similar outlets are typically
/// 500–1200 chars (the BBC sample that triggered this tuning was 662 chars and
/// was wrongly accepted). Real extracted article bodies sit well above 1500
/// chars, so this threshold separates "got only the feed teaser" from "got the
/// actual article". Some fast-news sources (TASS, wire services) can publish
/// genuine articles below 1500 chars — those will now go through the headless
/// fallback, which is acceptable as long as headless throughput holds up.
const MIN_CONTENT_CHARS: usize = 1500;

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
    Headless(Box<HeadlessError>),
    #[error("URL blocked: {0}")]
    BlockedUrl(String),
}

impl From<HeadlessError> for RetrievalError {
    fn from(err: HeadlessError) -> Self {
        RetrievalError::Headless(Box::new(err))
    }
}

/// Returns `false` for IPs an article URL must never point to: loopback
/// (127/8, ::1), link-local (169.254/16 incl. cloud metadata 169.254.169.254,
/// fe80::/10), unspecified and multicast/broadcast. Private LAN ranges
/// (RFC 1918 / ULA) stay ALLOWED on purpose — this is a desktop app and
/// users legitimately subscribe to self-hosted feeds on their LAN.
fn is_ip_allowed(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            !(v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast())
        }
        std::net::IpAddr::V6(v6) => {
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // fe80::/10 link-local (is_unicast_link_local is unstable)
                || (v6.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

/// SSRF guard for article URLs coming from feed content (attacker-controlled
/// if a feed is compromised). Rejects non-HTTP schemes, localhost names and
/// blocked IP ranges — for hostnames, all resolved addresses must pass.
async fn check_url_allowed(url: &Url) -> Result<(), RetrievalError> {
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(RetrievalError::BlockedUrl(format!(
                "scheme '{}' not allowed",
                other
            )))
        }
    }

    let Some(host) = url.host() else {
        return Err(RetrievalError::BlockedUrl("URL has no host".to_string()));
    };

    match host {
        url::Host::Ipv4(ip) => {
            if !is_ip_allowed(&std::net::IpAddr::V4(ip)) {
                return Err(RetrievalError::BlockedUrl(format!(
                    "IP {} is in a blocked range",
                    ip
                )));
            }
        }
        url::Host::Ipv6(ip) => {
            if !is_ip_allowed(&std::net::IpAddr::V6(ip)) {
                return Err(RetrievalError::BlockedUrl(format!(
                    "IP {} is in a blocked range",
                    ip
                )));
            }
        }
        url::Host::Domain(domain) => {
            let lower = domain.to_lowercase();
            if lower == "localhost" || lower.ends_with(".localhost") {
                return Err(RetrievalError::BlockedUrl(
                    "localhost is not allowed".to_string(),
                ));
            }
            let port = url.port_or_known_default().unwrap_or(80);
            let addrs = tokio::net::lookup_host((lower.as_str(), port))
                .await
                .map_err(|e| {
                    RetrievalError::BlockedUrl(format!("DNS lookup failed for {}: {}", domain, e))
                })?;
            for addr in addrs {
                if !is_ip_allowed(&addr.ip()) {
                    return Err(RetrievalError::BlockedUrl(format!(
                        "{} resolves to blocked IP {}",
                        domain,
                        addr.ip()
                    )));
                }
            }
        }
    }

    Ok(())
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
    pub fn new() -> Result<Self, RetrievalError> {
        // Custom redirect policy: an outward-looking article link must not be
        // able to 301/302 onto localhost or another blocked address. Only
        // literal IPs / localhost names can be checked here (the policy is
        // synchronous, no DNS) — the initial target is DNS-checked upfront
        // in check_url_allowed().
        let redirect_policy = reqwest_new::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error("too many redirects");
            }
            let blocked = match attempt.url().host() {
                Some(url::Host::Ipv4(ip)) => !is_ip_allowed(&std::net::IpAddr::V4(ip)),
                Some(url::Host::Ipv6(ip)) => !is_ip_allowed(&std::net::IpAddr::V6(ip)),
                Some(url::Host::Domain(d)) => {
                    let l = d.to_lowercase();
                    l == "localhost" || l.ends_with(".localhost")
                }
                None => true,
            };
            if blocked {
                attempt.error("redirect to blocked address")
            } else {
                attempt.follow()
            }
        });

        let client = reqwest_new::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(redirect_policy)
            .user_agent("Mozilla/5.0 (compatible; fuckupRSS/0.1; +https://github.com/fuckuprss)")
            .build()?;

        Ok(Self { client })
    }

    /// Fetch and extract full article content from URL
    pub async fn retrieve(&self, article_url: &str) -> Result<ExtractedArticle, RetrievalError> {
        let url = Url::parse(article_url)?;
        check_url_allowed(&url).await?;

        // Fetch the page. Non-success statuses (402/403 bot blocks, 404, 5xx)
        // must surface as errors — their bodies are block/error pages, not articles.
        let response: reqwest_new::Response = self
            .client
            .get(url.clone())
            .send()
            .await?
            .error_for_status()?;
        let html: String = response.text().await?;

        // Preprocess to mask media elements
        let (masked_html, replacements) = preprocess_media_tags(&html);

        // Extract article content using readability
        let extracted = extractor::extract(&mut masked_html.as_bytes(), &url)
            .map_err(|e| RetrievalError::Extraction(e.to_string()))?;

        // Restore media elements
        let content = postprocess_media_tags(&extracted.content, &replacements);

        // Sanitize HTML to prevent XSS
        let content = sanitize_html(&content);

        Ok(ExtractedArticle {
            title: Some(extracted.title),
            content,
            text_content: extracted.text,
        })
    }

    /// Fetch and extract article content with optional headless browser fallback.
    ///
    /// This method first attempts regular HTTP fetch with readability extraction.
    /// If the extracted content is shorter than `MIN_CONTENT_CHARS` and `use_headless`
    /// is true, it falls back to using the headless browser to render JavaScript
    /// and then re-extracts the content.
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
        check_url_allowed(&url).await?;

        // First attempt: regular HTTP fetch with readability
        let response: reqwest_new::Response = self.client.get(url.clone()).send().await?;

        // Non-success responses (402/403 bot blocks, 404, 5xx) carry block/error
        // pages, not articles. Bot blocks often pass with a real browser
        // fingerprint, so try headless directly; otherwise surface the status.
        if let Err(status_err) = response.error_for_status_ref() {
            if use_headless {
                if let Some(fetcher) = headless_fetcher {
                    log::info!(
                        "HTTP {} for {}, attempting headless fallback",
                        response.status(),
                        article_url
                    );
                    return self.extract_via_headless(fetcher, article_url, &url).await;
                }
            }
            return Err(RetrievalError::Http(status_err));
        }

        let html: String = response.text().await?;

        // Preprocess masked HTML
        let (masked_html, replacements) = preprocess_media_tags(&html);

        let extracted = extractor::extract(&mut masked_html.as_bytes(), &url)
            .map_err(|e| RetrievalError::Extraction(e.to_string()))?;

        let content = postprocess_media_tags(&extracted.content, &replacements);

        // Sanitize HTML to prevent XSS
        let content = sanitize_html(&content);

        let result = ExtractedArticle {
            title: Some(extracted.title),
            content,
            text_content: extracted.text,
        };

        // Check if content is sufficient or if fallback is disabled/unavailable
        if result.content.len() >= MIN_CONTENT_CHARS || !use_headless {
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
                "Content too short ({} chars, threshold {}) for {} but no headless fetcher provided",
                result.content.len(),
                MIN_CONTENT_CHARS,
                article_url
            );
            return Ok(result);
        };

        log::info!(
            "Content too short ({} chars, threshold {}) for {}, attempting headless fallback",
            result.content.len(),
            MIN_CONTENT_CHARS,
            article_url
        );

        // Fetch with headless browser (renders JavaScript)
        let headless_result = self
            .extract_via_headless(fetcher, article_url, &url)
            .await?;

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

    /// Fetch a page via the headless browser and run readability extraction
    /// on the rendered HTML.
    async fn extract_via_headless(
        &self,
        fetcher: &HeadlessFetcher,
        article_url: &str,
        url: &Url,
    ) -> Result<ExtractedArticle, RetrievalError> {
        let rendered_html = fetcher.fetch(article_url).await?;

        let (masked_rendered, rendered_replacements) = preprocess_media_tags(&rendered_html);

        let extracted_from_rendered = extractor::extract(&mut masked_rendered.as_bytes(), url)
            .map_err(|e| {
                RetrievalError::Extraction(format!("Headless extraction failed: {}", e))
            })?;

        let rendered_content =
            postprocess_media_tags(&extracted_from_rendered.content, &rendered_replacements);

        // Sanitize HTML to prevent XSS
        let rendered_content = sanitize_html(&rendered_content);

        Ok(ExtractedArticle {
            title: Some(extracted_from_rendered.title),
            content: rendered_content,
            text_content: extracted_from_rendered.text,
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
        Self::new().expect("Failed to create HTTP client for HagbardRetrieval")
    }
}

/// Sanitize HTML content to prevent XSS attacks.
/// Allows safe formatting tags, links, images, and embedded media (iframe, video).
fn sanitize_html(html: &str) -> String {
    use std::collections::HashSet;

    let mut builder = ammonia::Builder::new();

    // Allow standard formatting and structural tags
    let mut tags: HashSet<&str> = HashSet::new();
    for tag in &[
        "p",
        "br",
        "div",
        "span",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "a",
        "img",
        "ul",
        "ol",
        "li",
        "blockquote",
        "pre",
        "code",
        "em",
        "strong",
        "b",
        "i",
        "u",
        "sub",
        "sup",
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        "figure",
        "figcaption",
        "picture",
        "source",
        // Media tags that the preprocess/postprocess pipeline preserves
        "iframe",
        "video",
        "audio",
        "object",
        "embed",
    ] {
        tags.insert(tag);
    }
    builder.tags(tags);

    // Allow safe attributes for links, images, and media
    let mut tag_attrs: HashMap<&str, HashSet<&str>> = HashMap::new();

    let mut a_attrs = HashSet::new();
    a_attrs.insert("href");
    a_attrs.insert("title");
    // NOTE: "rel" must NOT be in tag_attributes for <a> when link_rel() is used,
    // otherwise ammonia panics (assertion in Builder::clean).
    tag_attrs.insert("a", a_attrs);

    let mut img_attrs = HashSet::new();
    img_attrs.insert("src");
    img_attrs.insert("alt");
    img_attrs.insert("title");
    img_attrs.insert("width");
    img_attrs.insert("height");
    img_attrs.insert("loading");
    tag_attrs.insert("img", img_attrs);

    let mut iframe_attrs = HashSet::new();
    iframe_attrs.insert("src");
    iframe_attrs.insert("width");
    iframe_attrs.insert("height");
    iframe_attrs.insert("frameborder");
    iframe_attrs.insert("allowfullscreen");
    iframe_attrs.insert("allow");
    iframe_attrs.insert("title");
    tag_attrs.insert("iframe", iframe_attrs);

    let mut video_attrs = HashSet::new();
    video_attrs.insert("src");
    video_attrs.insert("width");
    video_attrs.insert("height");
    video_attrs.insert("controls");
    video_attrs.insert("poster");
    tag_attrs.insert("video", video_attrs);

    let mut audio_attrs = HashSet::new();
    audio_attrs.insert("src");
    audio_attrs.insert("controls");
    tag_attrs.insert("audio", audio_attrs);

    let mut source_attrs = HashSet::new();
    source_attrs.insert("src");
    source_attrs.insert("srcset");
    source_attrs.insert("type");
    source_attrs.insert("media");
    tag_attrs.insert("source", source_attrs);

    let mut td_attrs = HashSet::new();
    td_attrs.insert("colspan");
    td_attrs.insert("rowspan");
    tag_attrs.insert("td", td_attrs.clone());
    tag_attrs.insert("th", td_attrs);

    builder.tag_attributes(tag_attrs);

    // Allow https and data URIs for images
    let mut url_schemes: HashSet<&str> = HashSet::new();
    url_schemes.insert("http");
    url_schemes.insert("https");
    url_schemes.insert("data");
    builder.url_schemes(url_schemes);

    // Force rel="noopener noreferrer" on links
    builder.link_rel(Some("noopener noreferrer"));

    builder.clean(html).to_string()
}

/// Preprocess HTML to mask media elements so they don't get stripped by readability
fn preprocess_media_tags(html: &str) -> (String, HashMap<String, String>) {
    let mut masked_html = html.to_string();
    let mut replacements = HashMap::new();

    // Patterns to match iframes and videos
    // Note: Regex for HTML is brittle, but sufficient for this specific masking task
    // We use non-greedy matching for content and ignore case
    let patterns = [
        (
            "iframe",
            Regex::new(r#"(?is)<iframe[^>]*>.*?</iframe>"#).unwrap(),
        ),
        (
            "video",
            Regex::new(r#"(?is)<video[^>]*>.*?</video>"#).unwrap(),
        ),
        (
            "object",
            Regex::new(r#"(?is)<object[^>]*>.*?</object>"#).unwrap(),
        ),
        (
            "embed",
            Regex::new(r#"(?is)<embed[^>]*>.*?</embed>"#).unwrap(),
        ),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn check(url: &str) -> Result<(), RetrievalError> {
        let parsed = Url::parse(url).expect("test URL must parse");
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
            .block_on(check_url_allowed(&parsed))
    }

    #[test]
    fn test_url_guard_blocks_loopback_and_link_local() {
        assert!(matches!(
            check("http://127.0.0.1/feed"),
            Err(RetrievalError::BlockedUrl(_))
        ));
        assert!(matches!(
            check("http://localhost:8080/article"),
            Err(RetrievalError::BlockedUrl(_))
        ));
        // Cloud metadata endpoint (link-local)
        assert!(matches!(
            check("http://169.254.169.254/latest/meta-data/"),
            Err(RetrievalError::BlockedUrl(_))
        ));
        assert!(matches!(
            check("http://[::1]/article"),
            Err(RetrievalError::BlockedUrl(_))
        ));
    }

    #[test]
    fn test_url_guard_blocks_non_http_schemes() {
        assert!(matches!(
            check("file:///etc/passwd"),
            Err(RetrievalError::BlockedUrl(_))
        ));
        assert!(matches!(
            check("ftp://example.com/x"),
            Err(RetrievalError::BlockedUrl(_))
        ));
    }

    #[test]
    fn test_url_guard_allows_private_lan_ip_literals() {
        // Desktop app: self-hosted LAN feeds are a legitimate use case.
        assert!(check("http://192.168.177.11:3000/rss").is_ok());
        assert!(check("http://10.0.0.5/feed.xml").is_ok());
    }

    #[test]
    fn test_is_ip_allowed_classification() {
        use std::net::IpAddr;
        let blocked: &[&str] = &["127.0.0.1", "0.0.0.0", "169.254.169.254", "::1", "fe80::1"];
        for ip in blocked {
            assert!(
                !is_ip_allowed(&ip.parse::<IpAddr>().unwrap()),
                "{} must be blocked",
                ip
            );
        }
        let allowed: &[&str] = &[
            "93.184.216.34",
            "192.168.1.1",
            "10.1.2.3",
            "2606:2800:220:1::1",
        ];
        for ip in allowed {
            assert!(
                is_ip_allowed(&ip.parse::<IpAddr>().unwrap()),
                "{} must be allowed",
                ip
            );
        }
    }
}
