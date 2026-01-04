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

/// Batch fetch full content for all truncated articles in a pentacle
#[tauri::command]
pub async fn fetch_truncated_articles(
    state: State<'_, AppState>,
    pentacle_id: Option<i64>,
) -> Result<Vec<RetrievalResponse>, String> {
    let retrieval = HagbardRetrieval::new();

    // Get articles that need fetching (sync)
    let articles: Vec<(i64, String, Option<String>)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let mut articles = Vec::new();

        if let Some(pid) = pentacle_id {
            let mut stmt = db
                .conn()
                .prepare(
                    "SELECT id, url, content_raw FROM fnords
                     WHERE pentacle_id = ?1 AND full_text_fetched = FALSE
                     ORDER BY published_at DESC LIMIT 50",
                )
                .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([pid], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
                })
                .map_err(|e| e.to_string())?;

            for row in rows.flatten() {
                if row.2.as_ref().map(|c| HagbardRetrieval::is_truncated(c)).unwrap_or(true) {
                    articles.push(row);
                }
            }
        } else {
            let mut stmt = db
                .conn()
                .prepare(
                    "SELECT id, url, content_raw FROM fnords
                     WHERE full_text_fetched = FALSE
                     ORDER BY published_at DESC LIMIT 50",
                )
                .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
                })
                .map_err(|e| e.to_string())?;

            for row in rows.flatten() {
                if row.2.as_ref().map(|c| HagbardRetrieval::is_truncated(c)).unwrap_or(true) {
                    articles.push(row);
                }
            }
        }

        articles
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
