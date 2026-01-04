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
    pub error: Option<String>,
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
    };

    let mut results = Vec::new();
    let mut total_new = 0;
    let mut total_updated = 0;

    // Fetch all feeds (async) and store
    for (id, url, title) in pentacles {
        let fetch_result = syncer.fetch_feed(id, &url).await;

        match fetch_result {
            Ok(fetched) => {
                // Store in database (sync)
                let db = state.db.lock().map_err(|e| e.to_string())?;
                match FeedSyncer::store_feed(db.conn(), fetched) {
                    Ok(sync_result) => {
                        total_new += sync_result.new_articles;
                        total_updated += sync_result.updated_articles;
                        results.push(SyncResultResponse {
                            pentacle_id: id,
                            pentacle_title: title,
                            new_articles: sync_result.new_articles,
                            updated_articles: sync_result.updated_articles,
                            error: None,
                        });
                    }
                    Err(e) => {
                        // Update error in database
                        let _ = db.conn().execute(
                            "UPDATE pentacles SET error_count = error_count + 1, last_error = ?1 WHERE id = ?2",
                            (&e.to_string(), id),
                        );
                        results.push(SyncResultResponse {
                            pentacle_id: id,
                            pentacle_title: title,
                            new_articles: 0,
                            updated_articles: 0,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                // Update error in database
                let db = state.db.lock().map_err(|e| e.to_string())?;
                let _ = db.conn().execute(
                    "UPDATE pentacles SET error_count = error_count + 1, last_error = ?1 WHERE id = ?2",
                    (&e.to_string(), id),
                );
                results.push(SyncResultResponse {
                    pentacle_id: id,
                    pentacle_title: title,
                    new_articles: 0,
                    updated_articles: 0,
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
    };

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

    // Store in database (sync)
    let db = state.db.lock().map_err(|e| e.to_string())?;
    match FeedSyncer::store_feed(db.conn(), fetched) {
        Ok(result) => Ok(SyncResultResponse {
            pentacle_id,
            pentacle_title: title,
            new_articles: result.new_articles,
            updated_articles: result.updated_articles,
            error: None,
        }),
        Err(e) => {
            let _ = db.conn().execute(
                "UPDATE pentacles SET error_count = error_count + 1, last_error = ?1 WHERE id = ?2",
                (&e.to_string(), pentacle_id),
            );
            Err(e.to_string())
        }
    }
}
