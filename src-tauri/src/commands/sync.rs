use crate::retrieval::HagbardRetrieval;
use crate::sync::FeedSyncer;
use crate::AppState;
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

/// Fetch full content for articles that need it
async fn fetch_full_content_for_articles(
    state: &State<'_, AppState>,
    pentacle_id: i64,
) -> usize {
    let retrieval = HagbardRetrieval::new();
    let mut fetched_count = 0;

    // Get articles that need full text fetching
    let articles: Vec<(i64, String, Option<String>)> = {
        let db = match state.db.lock() {
            Ok(db) => db,
            Err(_) => return 0,
        };

        let mut stmt = match db.conn().prepare(
            r#"SELECT id, url, content_raw FROM fnords
               WHERE pentacle_id = ?1
               AND full_text_fetched = FALSE
               ORDER BY published_at DESC
               LIMIT 100"#,
        ) {
            Ok(stmt) => stmt,
            Err(_) => return 0,
        };

        stmt.query_map([pentacle_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }; // db lock released here

    // Fetch full content for each article
    for (id, url, content_raw) in articles {
        // Check if content is truncated or missing
        let needs_fetch = content_raw
            .as_ref()
            .map(|c| HagbardRetrieval::is_truncated(c))
            .unwrap_or(true);

        if !needs_fetch {
            // Content is not truncated, mark as fetched
            if let Ok(db) = state.db.lock() {
                let _ = db.conn().execute(
                    "UPDATE fnords SET full_text_fetched = TRUE WHERE id = ?1",
                    [id],
                );
            }
            continue;
        }

        // Fetch full content
        match retrieval.retrieve(&url).await {
            Ok(extracted) => {
                if let Ok(db) = state.db.lock() {
                    let _ = db.conn().execute(
                        "UPDATE fnords SET content_full = ?1, full_text_fetched = TRUE WHERE id = ?2",
                        (&extracted.content, id),
                    );
                    fetched_count += 1;
                }
            }
            Err(_) => {
                // Mark as attempted to avoid infinite retries
                if let Ok(db) = state.db.lock() {
                    let _ = db.conn().execute(
                        "UPDATE fnords SET full_text_fetched = TRUE WHERE id = ?1",
                        [id],
                    );
                }
            }
        }
    }

    fetched_count
}

/// Sync all feeds
#[tauri::command]
pub async fn sync_all_feeds(state: State<'_, AppState>) -> Result<SyncResponse, String> {
    let syncer = FeedSyncer::new();

    // Get all pentacles (sync)
    let pentacles: Vec<(i64, String, Option<String>)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
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
                    let db = state.db.lock().map_err(|e| e.to_string())?;
                    FeedSyncer::store_feed(db.conn(), fetched)
                }; // db lock released here

                match store_result {
                    Ok(sync_result) => {
                        total_new += sync_result.new_articles;
                        total_updated += sync_result.updated_articles;

                        // Auto-fetch full content for new/truncated articles
                        let fetched_count = fetch_full_content_for_articles(&state, id).await;

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

    Ok(SyncResponse {
        success: true,
        results,
        total_new,
        total_updated,
    })
}

/// Sync a single feed
#[tauri::command]
pub async fn sync_feed(state: State<'_, AppState>, pentacle_id: i64) -> Result<SyncResultResponse, String> {
    let syncer = FeedSyncer::new();

    // Get feed URL (sync)
    let (url, title): (String, Option<String>) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
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
        let db = state.db.lock().map_err(|e| e.to_string())?;
        FeedSyncer::store_feed(db.conn(), fetched)
    }; // db lock released here

    match store_result {
        Ok(result) => {
            // Auto-fetch full content for new/truncated articles
            let fetched_count = fetch_full_content_for_articles(&state, pentacle_id).await;

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
