use crate::retrieval::headless::HeadlessFetcher;
use crate::retrieval::HagbardRetrieval;
use crate::AppState;
use once_cell::sync::OnceCell;
use tauri::State;

/// Global HeadlessFetcher instance, lazily initialized on first use.
/// The browser is not started until the first fetch operation that needs it.
static HEADLESS_FETCHER: OnceCell<HeadlessFetcher> = OnceCell::new();

/// Get or initialize the global HeadlessFetcher.
fn get_headless_fetcher() -> &'static HeadlessFetcher {
    HEADLESS_FETCHER.get_or_init(HeadlessFetcher::new)
}

/// Check if headless browser is enabled in settings.
fn is_headless_enabled(db: &crate::db::Database) -> bool {
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'enable_headless_browser'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v == "true")
        .unwrap_or(false)
}

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

    // Get article URL and headless setting (sync)
    let (url, use_headless): (String, bool) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let url = db
            .conn()
            .query_row("SELECT url FROM fnords WHERE id = ?1", [fnord_id], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;
        let headless = is_headless_enabled(&db);
        (url, headless)
    };

    // Get headless fetcher reference if enabled
    let headless_fetcher = if use_headless {
        Some(get_headless_fetcher())
    } else {
        None
    };

    // Fetch full content with optional headless fallback (async)
    match retrieval
        .retrieve_with_fallback(&url, use_headless, headless_fetcher)
        .await
    {
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

    // Get articles and headless setting (sync)
    let (articles, use_headless): (Vec<(i64, String, Option<String>)>, bool) = {
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

        let headless = is_headless_enabled(&db);
        (rows, headless)
    };

    // Get headless fetcher reference if enabled
    let headless_fetcher = if use_headless {
        Some(get_headless_fetcher())
    } else {
        None
    };

    let mut results = Vec::new();

    // Fetch each article (async)
    for (id, url, _) in articles.into_iter() {
        match retrieval
            .retrieve_with_fallback(&url, use_headless, headless_fetcher)
            .await
        {
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
