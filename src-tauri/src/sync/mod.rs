use feed_rs::parser;
use log::{debug, info};
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::time::Duration;
use thiserror::Error;

/// Type alias for existing article row data used in sync
/// (id, title, author, content_raw, content_full, summary, content_hash)
type ExistingArticleRow = (
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[cfg(test)]
mod tests;

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
        debug!("Fetching feed: {} (pentacle_id: {})", url, pentacle_id);

        // Fetch feed content
        let response: reqwest_new::Response = self.client.get(url).send().await?;
        let bytes: bytes::Bytes = response.bytes().await?;
        debug!("Received {} bytes from {}", bytes.len(), url);

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

        info!(
            "Parsed feed '{}' with {} entries",
            title.as_deref().unwrap_or("Unknown"),
            entries.len()
        );

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
        let entry_count = feed.entries.len();
        debug!(
            "Storing {} entries for pentacle {}",
            entry_count, pentacle_id
        );

        // Begin transaction for atomic feed storage
        conn.execute("BEGIN TRANSACTION", [])?;

        let result = Self::store_feed_inner(conn, feed);

        match &result {
            Ok(_) => {
                conn.execute("COMMIT", [])?;
            }
            Err(_) => {
                let _ = conn.execute("ROLLBACK", []);
            }
        }

        result
    }

    /// Inner implementation of store_feed, separated for transaction handling
    fn store_feed_inner(conn: &Connection, feed: FetchedFeed) -> Result<SyncResult, SyncError> {
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
            // Compute hash for new content
            let new_hash = compute_content_hash(
                &entry.title,
                entry.author.as_deref(),
                entry.content_raw.as_deref(),
                entry.summary.as_deref(),
            );

            // Check if article already exists and get current data (including content_full for revision)
            let existing: Option<ExistingArticleRow> = conn
                .query_row(
                    r#"SELECT id, title, author, content_raw, content_full, summary, content_hash
                       FROM fnords WHERE pentacle_id = ?1 AND guid = ?2"#,
                    (&pentacle_id, &entry.guid),
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                        ))
                    },
                )
                .optional()?;

            if let Some((
                fnord_id,
                old_title,
                old_author,
                old_content,
                old_content_full,
                old_summary,
                old_hash,
            )) = existing
            {
                // Article exists - check if content changed via hash comparison
                // Only consider it a change if we had a previous hash AND it differs
                let content_changed = match &old_hash {
                    Some(existing_hash) => existing_hash != &new_hash,
                    None => false, // First time computing hash - not a real change
                };

                if content_changed {
                    // Save old version to revisions table (including content_full if available)
                    let old_hash_str = old_hash.as_ref().unwrap(); // Safe: content_changed is only true if old_hash is Some

                    conn.execute(
                        r#"INSERT INTO fnord_revisions (fnord_id, title, author, content_raw, content_full, summary, content_hash)
                           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
                        (&fnord_id, &old_title, &old_author, &old_content, &old_content_full, &old_summary, &old_hash_str),
                    )?;

                    // Update article with new content and mark as changed
                    conn.execute(
                        r#"UPDATE fnords SET
                            title = ?1,
                            author = COALESCE(?2, author),
                            content_raw = COALESCE(?3, content_raw),
                            summary = COALESCE(?4, summary),
                            content_hash = ?5,
                            has_changes = TRUE,
                            changed_at = CURRENT_TIMESTAMP,
                            revision_count = revision_count + 1
                        WHERE id = ?6"#,
                        (
                            &entry.title,
                            &entry.author,
                            &entry.content_raw,
                            &entry.summary,
                            &new_hash,
                            &fnord_id,
                        ),
                    )?;

                    updated_articles += 1;
                } else if old_hash.is_none() {
                    // First sync after migration - just set the hash without marking as changed
                    conn.execute(
                        "UPDATE fnords SET content_hash = ?1 WHERE id = ?2",
                        (&new_hash, &fnord_id),
                    )?;
                }
            } else {
                // Insert new article with hash
                conn.execute(
                    r#"INSERT INTO fnords (
                        pentacle_id, guid, url, title, author,
                        content_raw, summary, image_url, published_at, content_hash
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
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
                        &new_hash,
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

        if new_articles > 0 || updated_articles > 0 {
            info!(
                "Sync complete for pentacle {}: {} new, {} updated",
                pentacle_id, new_articles, updated_articles
            );
        } else {
            debug!("Sync complete for pentacle {}: no changes", pentacle_id);
        }

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

/// Compute SHA256 hash of article content for change detection
pub fn compute_content_hash(
    title: &str,
    author: Option<&str>,
    content: Option<&str>,
    summary: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    if let Some(a) = author {
        hasher.update(b"|author:");
        hasher.update(a.as_bytes());
    }
    if let Some(c) = content {
        hasher.update(b"|content:");
        hasher.update(c.as_bytes());
    }
    if let Some(s) = summary {
        hasher.update(b"|summary:");
        hasher.update(s.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
