use crate::retrieval::HagbardRetrieval;
use crate::sync::FeedSyncer;
use crate::AppState;
use futures::{stream, StreamExt};
use log::{info, warn};
use rusqlite::params;
use std::sync::Arc;
use tauri::State;

#[derive(serde::Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub results: Vec<SyncResultResponse>,
    pub total_new: usize,
    pub total_updated: usize,
}

#[derive(serde::Serialize)]
pub struct SyncResultResponse {
    pub pentacle_id: i64,
    pub pentacle_title: Option<String>,
    pub new_articles: usize,
    pub updated_articles: usize,
    pub full_text_fetched: usize,
    pub error: Option<String>,
}

/// Categorize a retrieval error into a simple error type string for tracking.
fn categorize_retrieval_error(error: &crate::retrieval::RetrievalError) -> String {
    use crate::retrieval::RetrievalError;

    let error_string = error.to_string().to_lowercase();

    match error {
        RetrievalError::Http(e) => {
            if let Some(status) = e.status() {
                return status.as_u16().to_string();
            }
            if e.is_timeout() {
                return "timeout".to_string();
            }
            if e.is_connect() {
                if error_string.contains("dns") || error_string.contains("name resolution") {
                    return "dns_error".to_string();
                }
                if error_string.contains("refused") {
                    return "connection_refused".to_string();
                }
                return "connection_error".to_string();
            }
            "http_error".to_string()
        }
        RetrievalError::UrlParse(_) => "invalid_url".to_string(),
        RetrievalError::Extraction(_) => {
            if error_string.contains("blocked") || error_string.contains("captcha") {
                return "blocked".to_string();
            }
            "parse_error".to_string()
        }
        RetrievalError::Db(_) => "db_error".to_string(),
        RetrievalError::Headless(e) => {
            let headless_str = e.to_string().to_lowercase();
            if headless_str.contains("timeout") {
                return "headless_timeout".to_string();
            }
            "headless_error".to_string()
        }
    }
}

/// Fetch full content for articles in parallel (much faster than sequential)
/// This is called automatically after feed sync.
async fn fetch_full_content_for_articles_parallel(
    state: &State<'_, AppState>,
    pentacle_id: i64,
) -> usize {
    const PARALLEL_REQUESTS: usize = 10;
    const MAX_ARTICLES: i64 = 100;

    // Get articles that need full text fetching
    let articles: Vec<(i64, String)> = {
        let db = match state.db.lock() {
            Ok(db) => db,
            Err(_) => return 0,
        };

        let mut stmt = match db.conn().prepare(
            r#"SELECT id, url FROM fnords
               WHERE pentacle_id = ?1
               AND full_text_fetched = FALSE
               ORDER BY published_at DESC
               LIMIT ?2"#,
        ) {
            Ok(stmt) => stmt,
            Err(_) => return 0,
        };

        stmt.query_map(params![pentacle_id, MAX_ARTICLES], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }; // db lock released here

    let total = articles.len();
    if total == 0 {
        return 0;
    }

    info!(
        "Starting parallel fulltext fetch for {} articles (pentacle {}) with {} concurrent requests",
        total, pentacle_id, PARALLEL_REQUESTS
    );

    // Create shared retrieval client
    let retrieval = match HagbardRetrieval::new() {
        Ok(r) => Arc::new(r),
        Err(e) => {
            warn!("Failed to create HagbardRetrieval: {}", e);
            return 0;
        }
    };
    let db_arc = state.db.clone();

    // Process articles in parallel using futures::stream
    let results: Vec<(i64, bool)> = stream::iter(articles)
        .map(|(fnord_id, url)| {
            let retrieval = Arc::clone(&retrieval);
            let db = db_arc.clone();
            async move {
                // Fetch fulltext
                match retrieval.retrieve(&url).await {
                    Ok(extracted) => {
                        // Save to database
                        if let Ok(db_guard) = db.lock() {
                            let _ = db_guard.conn().execute(
                                "UPDATE fnords SET content_full = ?1, full_text_fetched = TRUE, full_text_fetch_error = NULL WHERE id = ?2",
                                params![&extracted.content, fnord_id],
                            );
                        }
                        (fnord_id, true)
                    }
                    Err(e) => {
                        // Save error to database
                        let error_type = categorize_retrieval_error(&e);
                        if let Ok(db_guard) = db.lock() {
                            let _ = db_guard.conn().execute(
                                "UPDATE fnords SET full_text_fetched = TRUE, full_text_fetch_error = ?1 WHERE id = ?2",
                                params![&error_type, fnord_id],
                            );
                        }
                        (fnord_id, false)
                    }
                }
            }
        })
        .buffer_unordered(PARALLEL_REQUESTS)
        .collect()
        .await;

    // Count successes
    let successful = results.iter().filter(|(_, ok)| *ok).count();
    let failed = results.len() - successful;

    info!(
        "Parallel fulltext fetch complete for pentacle {}: {} successful, {} failed",
        pentacle_id, successful, failed
    );

    successful
}

/// Sync all feeds
#[tauri::command]
pub async fn sync_all_feeds(state: State<'_, AppState>) -> Result<SyncResponse, String> {
    let syncer = FeedSyncer::new();

    // Get all pentacles (sync)
    let pentacles: Vec<(i64, String, Option<String>)> = {
        let db = state.db_conn()?;
        let mut stmt = db
            .conn()
            .prepare("SELECT id, url, title FROM pentacles")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| e.to_string())?;

        rows.filter_map(|r| r.ok()).collect()
    }; // db lock released here

    let mut results = Vec::new();
    let mut total_new = 0;
    let mut total_updated = 0;

    // Fetch all feeds (async) and store
    for (id, url, title) in pentacles {
        let fetch_result = syncer.fetch_feed(id, &url).await;

        match fetch_result {
            Ok(fetched) => {
                // Store in database (sync) - use scope to release lock before async
                let store_result = {
                    let db = state.db_conn()?;
                    FeedSyncer::store_feed(db.conn(), fetched)
                }; // db lock released here

                match store_result {
                    Ok(sync_result) => {
                        total_new += sync_result.new_articles;
                        total_updated += sync_result.updated_articles;

                        // Auto-fetch full content for new articles (parallel for speed)
                        let fetched_count =
                            fetch_full_content_for_articles_parallel(&state, id).await;

                        results.push(SyncResultResponse {
                            pentacle_id: id,
                            pentacle_title: title,
                            new_articles: sync_result.new_articles,
                            updated_articles: sync_result.updated_articles,
                            full_text_fetched: fetched_count,
                            error: None,
                        });
                    }
                    Err(e) => {
                        // Update error in database
                        if let Ok(db) = state.db.lock() {
                            let _ = db.conn().execute(
                                "UPDATE pentacles SET error_count = error_count + 1, last_error = ?1 WHERE id = ?2",
                                (&e.to_string(), id),
                            );
                        }
                        results.push(SyncResultResponse {
                            pentacle_id: id,
                            pentacle_title: title,
                            new_articles: 0,
                            updated_articles: 0,
                            full_text_fetched: 0,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                // Update error in database
                if let Ok(db) = state.db.lock() {
                    let _ = db.conn().execute(
                        "UPDATE pentacles SET error_count = error_count + 1, last_error = ?1 WHERE id = ?2",
                        (&e.to_string(), id),
                    );
                }
                results.push(SyncResultResponse {
                    pentacle_id: id,
                    pentacle_title: title,
                    new_articles: 0,
                    updated_articles: 0,
                    full_text_fetched: 0,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Trigger WAL checkpoint if we synced a significant number of articles
    if total_new + total_updated >= 100 {
        if let Ok(db) = state.db.lock() {
            match db.conn().query_row(
                "PRAGMA wal_checkpoint(PASSIVE)",
                [],
                |row| {
                    let busy: i32 = row.get(0)?;
                    let log: i32 = row.get(1)?;
                    let checkpointed: i32 = row.get(2)?;
                    Ok((busy, log, checkpointed))
                },
            ) {
                Ok((busy, log, checkpointed)) => {
                    info!(
                        "WAL checkpoint after syncing {} articles: busy={}, log={}, checkpointed={}",
                        total_new + total_updated, busy, log, checkpointed
                    );
                }
                Err(e) => {
                    warn!("WAL checkpoint failed after sync: {}", e);
                }
            }
        }
    }

    Ok(SyncResponse {
        success: true,
        results,
        total_new,
        total_updated,
    })
}

/// Sync a single feed
#[tauri::command]
pub async fn sync_feed(
    state: State<'_, AppState>,
    pentacle_id: i64,
) -> Result<SyncResultResponse, String> {
    let syncer = FeedSyncer::new();

    // Get feed URL (sync)
    let (url, title): (String, Option<String>) = {
        let db = state.db_conn()?;
        db.conn()
            .query_row(
                "SELECT url, title FROM pentacles WHERE id = ?1",
                [pentacle_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?
    }; // db lock released here

    // Fetch feed (async)
    let fetched = match syncer.fetch_feed(pentacle_id, &url).await {
        Ok(f) => f,
        Err(e) => {
            // Update error in database
            if let Ok(db) = state.db.lock() {
                let _ = db.conn().execute(
                    "UPDATE pentacles SET error_count = error_count + 1, last_error = ?1 WHERE id = ?2",
                    (&e.to_string(), pentacle_id),
                );
            }
            return Err(e.to_string());
        }
    };

    // Store in database (sync) - use scope to release lock before async
    let store_result = {
        let db = state.db_conn()?;
        FeedSyncer::store_feed(db.conn(), fetched)
    }; // db lock released here

    match store_result {
        Ok(result) => {
            // Auto-fetch full content for new articles (parallel for speed)
            let fetched_count = fetch_full_content_for_articles_parallel(&state, pentacle_id).await;

            Ok(SyncResultResponse {
                pentacle_id,
                pentacle_title: title,
                new_articles: result.new_articles,
                updated_articles: result.updated_articles,
                full_text_fetched: fetched_count,
                error: None,
            })
        }
        Err(e) => {
            if let Ok(db) = state.db.lock() {
                let _ = db.conn().execute(
                    "UPDATE pentacles SET error_count = error_count + 1, last_error = ?1 WHERE id = ?2",
                    (&e.to_string(), pentacle_id),
                );
            }
            Err(e.to_string())
        }
    }
}
