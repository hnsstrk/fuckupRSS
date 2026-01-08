use crate::retrieval::HagbardRetrieval;
use crate::AppState;
use tauri::State;

#[derive(serde::Serialize)]
pub struct RetrievalResponse {
    pub fnord_id: i64,
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
}

/// Fetch full article content for a single fnord
#[tauri::command]
pub async fn fetch_full_content(
    state: State<'_, AppState>,
    fnord_id: i64,
) -> Result<RetrievalResponse, String> {
    let retrieval = HagbardRetrieval::new();

    // Get article URL (sync)
    let url: String = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn()
            .query_row("SELECT url FROM fnords WHERE id = ?1", [fnord_id], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?
    };

    // Fetch full content (async)
    match retrieval.retrieve(&url).await {
        Ok(extracted) => {
            // Store in database (sync)
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.conn()
                .execute(
                    "UPDATE fnords SET content_full = ?1, full_text_fetched = TRUE WHERE id = ?2",
                    (&extracted.content, fnord_id),
                )
                .map_err(|e| e.to_string())?;

            Ok(RetrievalResponse {
                fnord_id,
                success: true,
                content: Some(extracted.content),
                error: None,
            })
        }
        Err(e) => Ok(RetrievalResponse {
            fnord_id,
            success: false,
            content: None,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub async fn fetch_truncated_articles(
    state: State<'_, AppState>,
    pentacle_id: Option<i64>,
) -> Result<Vec<RetrievalResponse>, String> {
    let retrieval = HagbardRetrieval::new();

    let articles: Vec<(i64, String, Option<String>)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let base_sql = "SELECT id, url, content_raw FROM fnords WHERE full_text_fetched = FALSE";
        let sql = match pentacle_id {
            Some(_) => format!("{} AND pentacle_id = ?1 ORDER BY published_at DESC LIMIT 50", base_sql),
            None => format!("{} ORDER BY published_at DESC LIMIT 50", base_sql),
        };

        let mut stmt = db.conn().prepare(&sql).map_err(|e| e.to_string())?;

        let row_mapper = |row: &rusqlite::Row| -> Result<(i64, String, Option<String>), rusqlite::Error> {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        };

        let rows: Vec<(i64, String, Option<String>)> = match pentacle_id {
            Some(pid) => stmt.query_map([pid], row_mapper),
            None => stmt.query_map([], row_mapper),
        }
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|(_, _, content)| content.as_ref().map(|c| HagbardRetrieval::is_truncated(c)).unwrap_or(true))
        .collect();

        rows
    };

    let mut results = Vec::new();

    // Fetch each article (async)
    for (id, url, _) in articles.into_iter() {
        match retrieval.retrieve(&url).await {
            Ok(extracted) => {
                // Store in database (sync)
                if let Ok(db) = state.db.lock() {
                    let _ = db.conn().execute(
                        "UPDATE fnords SET content_full = ?1, full_text_fetched = TRUE WHERE id = ?2",
                        (&extracted.content, id),
                    );
                }

                results.push(RetrievalResponse {
                    fnord_id: id,
                    success: true,
                    content: Some(extracted.content),
                    error: None,
                });
            }
            Err(e) => {
                // Mark as attempted to avoid retrying
                if let Ok(db) = state.db.lock() {
                    let _ = db.conn().execute(
                        "UPDATE fnords SET full_text_fetched = TRUE WHERE id = ?1",
                        [id],
                    );
                }

                results.push(RetrievalResponse {
                    fnord_id: id,
                    success: false,
                    content: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(results)
}
