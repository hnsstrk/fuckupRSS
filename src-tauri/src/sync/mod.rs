use feed_rs::parser;
use rusqlite::Connection;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest_new::Error),
    #[error("Feed parsing error: {0}")]
    Parse(#[from] feed_rs::parser::ParseFeedError),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
}

/// Fetched feed data ready for database insertion
#[derive(Debug)]
pub struct FetchedFeed {
    pub pentacle_id: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_url: Option<String>,
    pub icon_url: Option<String>,
    pub entries: Vec<FetchedEntry>,
}

#[derive(Debug)]
pub struct FetchedEntry {
    pub guid: String,
    pub url: String,
    pub title: String,
    pub author: Option<String>,
    pub content_raw: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub published_at: Option<String>,
}

pub struct FeedSyncer {
    client: reqwest_new::Client,
}

impl FeedSyncer {
    pub fn new() -> Self {
        let client = reqwest_new::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("fuckupRSS/0.1 (RSS Reader)")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Fetch and parse a feed (async, no DB access)
    pub async fn fetch_feed(&self, pentacle_id: i64, url: &str) -> Result<FetchedFeed, SyncError> {
        // Fetch feed content
        let response: reqwest_new::Response = self.client.get(url).send().await?;
        let bytes: bytes::Bytes = response.bytes().await?;

        // Parse feed
        let feed = parser::parse(&bytes[..])?;

        let title = feed.title.as_ref().map(|t| t.content.clone());
        let description = feed.description.as_ref().map(|d| d.content.clone());
        let site_url = feed.links.first().map(|l| l.href.clone());
        let icon_url = feed.icon.as_ref().map(|i| i.uri.clone());

        let entries: Vec<FetchedEntry> = feed
            .entries
            .into_iter()
            .map(|entry| {
                let guid = entry.id.clone();
                let url = entry
                    .links
                    .first()
                    .map(|l| l.href.clone())
                    .unwrap_or_else(|| entry.id.clone());

                let title = entry
                    .title
                    .as_ref()
                    .map(|t| t.content.clone())
                    .unwrap_or_else(|| "Untitled".to_string());

                let author = entry.authors.first().map(|a| a.name.clone());

                let content_raw = entry
                    .content
                    .as_ref()
                    .and_then(|c| c.body.clone())
                    .or_else(|| entry.summary.as_ref().map(|s| s.content.clone()));

                let summary = entry.summary.as_ref().map(|s| s.content.clone());

                let image_url = entry
                    .media
                    .first()
                    .and_then(|m| m.content.first())
                    .and_then(|c| c.url.as_ref())
                    .map(|u| u.to_string());

                let published_at = entry.published.or(entry.updated).map(|dt| dt.to_rfc3339());

                FetchedEntry {
                    guid,
                    url,
                    title,
                    author,
                    content_raw,
                    summary,
                    image_url,
                    published_at,
                }
            })
            .collect();

        Ok(FetchedFeed {
            pentacle_id,
            title,
            description,
            site_url,
            icon_url,
            entries,
        })
    }

    /// Store fetched feed data in database (sync, called after async fetch)
    pub fn store_feed(conn: &Connection, feed: FetchedFeed) -> Result<SyncResult, SyncError> {
        let pentacle_id = feed.pentacle_id;
        let mut new_articles = 0;
        let mut updated_articles = 0;

        // Update pentacle metadata
        if let Some(title) = &feed.title {
            conn.execute(
                "UPDATE pentacles SET title = ?1 WHERE id = ?2 AND (title IS NULL OR title = '')",
                (title, pentacle_id),
            )?;
        }

        if let Some(description) = &feed.description {
            conn.execute(
                "UPDATE pentacles SET description = ?1 WHERE id = ?2 AND (description IS NULL OR description = '')",
                (description, pentacle_id),
            )?;
        }

        if let Some(site_url) = &feed.site_url {
            conn.execute(
                "UPDATE pentacles SET site_url = ?1 WHERE id = ?2 AND (site_url IS NULL OR site_url = '')",
                (site_url, pentacle_id),
            )?;
        }

        if let Some(icon_url) = &feed.icon_url {
            conn.execute(
                "UPDATE pentacles SET icon_url = ?1 WHERE id = ?2",
                (icon_url, pentacle_id),
            )?;
        }

        // Process entries
        for entry in feed.entries {
            // Check if article already exists
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM fnords WHERE pentacle_id = ?1 AND guid = ?2)",
                (&pentacle_id, &entry.guid),
                |row| row.get(0),
            )?;

            if exists {
                // Update existing article if content changed
                let rows = conn.execute(
                    r#"UPDATE fnords SET
                        title = ?3,
                        content_raw = COALESCE(?4, content_raw)
                    WHERE pentacle_id = ?1 AND guid = ?2 AND title != ?3"#,
                    (&pentacle_id, &entry.guid, &entry.title, &entry.content_raw),
                )?;
                if rows > 0 {
                    updated_articles += 1;
                }
            } else {
                // Insert new article
                conn.execute(
                    r#"INSERT INTO fnords (
                        pentacle_id, guid, url, title, author,
                        content_raw, summary, image_url, published_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
                    (
                        &pentacle_id,
                        &entry.guid,
                        &entry.url,
                        &entry.title,
                        &entry.author,
                        &entry.content_raw,
                        &entry.summary,
                        &entry.image_url,
                        &entry.published_at,
                    ),
                )?;
                new_articles += 1;
            }
        }

        // Update sync timestamp and article count
        conn.execute(
            r#"UPDATE pentacles SET
                last_sync = CURRENT_TIMESTAMP,
                article_count = (SELECT COUNT(*) FROM fnords WHERE pentacle_id = ?1),
                error_count = 0,
                last_error = NULL
            WHERE id = ?1"#,
            [pentacle_id],
        )?;

        Ok(SyncResult {
            pentacle_id,
            new_articles,
            updated_articles,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncResult {
    pub pentacle_id: i64,
    pub new_articles: usize,
    pub updated_articles: usize,
}
