use crate::retrieval::headless::HeadlessFetcher;
use crate::retrieval::HagbardRetrieval;
use crate::AppState;
use log::info;
use once_cell::sync::OnceCell;
use rusqlite::params;
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

/// Response for a single article refetch result
#[derive(serde::Serialize)]
pub struct RefetchResult {
    pub fnord_id: i64,
    pub title: String,
    pub old_length: i64,
    pub new_length: i64,
    pub improved: bool,
}

/// Response for batch refetch operation
#[derive(serde::Serialize)]
pub struct RefetchResponse {
    pub total_found: i64,
    pub processed: i64,
    pub improved: i64,
    pub failed: i64,
    pub results: Vec<RefetchResult>,
}

/// Re-fetch articles with content_full that is too short.
/// This is useful for articles where the initial fetch may have failed
/// or returned incomplete content.
#[tauri::command]
pub async fn refetch_short_articles(
    state: State<'_, AppState>,
    min_content_length: Option<i64>,
    limit: Option<i64>,
) -> Result<RefetchResponse, String> {
    let min_length = min_content_length.unwrap_or(500);
    let max_articles = limit.unwrap_or(50);

    let retrieval = HagbardRetrieval::new();

    // Get articles with short content and headless setting (sync)
    let (articles, use_headless): (Vec<(i64, String, String, i64)>, bool) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let sql = "
            SELECT id, url, title, COALESCE(LENGTH(content_full), 0) as content_length
            FROM fnords
            WHERE full_text_fetched = TRUE
              AND (content_full IS NULL OR LENGTH(content_full) < ?1)
            ORDER BY published_at DESC
            LIMIT ?2
        ";

        let mut stmt = db.conn().prepare(sql).map_err(|e| e.to_string())?;

        let rows: Vec<(i64, String, String, i64)> = stmt
            .query_map(params![min_length, max_articles], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let headless = is_headless_enabled(&db);
        (rows, headless)
    };

    let total_found = articles.len() as i64;
    info!(
        "Found {} articles with content_full < {} characters",
        total_found, min_length
    );

    if total_found == 0 {
        return Ok(RefetchResponse {
            total_found: 0,
            processed: 0,
            improved: 0,
            failed: 0,
            results: Vec::new(),
        });
    }

    // Get headless fetcher reference if enabled
    let headless_fetcher = if use_headless {
        Some(get_headless_fetcher())
    } else {
        None
    };

    let mut results = Vec::new();
    let mut processed = 0i64;
    let mut improved = 0i64;
    let mut failed = 0i64;

    // Fetch each article (async)
    for (id, url, title, old_length) in articles.into_iter() {
        processed += 1;

        match retrieval
            .retrieve_with_fallback(&url, use_headless, headless_fetcher)
            .await
        {
            Ok(extracted) => {
                let new_length = extracted.content.len() as i64;
                let is_improved = new_length > old_length;

                if is_improved {
                    improved += 1;
                }

                // Update in database (sync) - short lock
                {
                    let db = state.db.lock().map_err(|e| e.to_string())?;
                    db.conn()
                        .execute(
                            "UPDATE fnords SET content_full = ?1 WHERE id = ?2",
                            params![&extracted.content, id],
                        )
                        .map_err(|e| e.to_string())?;
                } // Lock released

                // Yield for other tasks
                tokio::task::yield_now().await;

                results.push(RefetchResult {
                    fnord_id: id,
                    title,
                    old_length,
                    new_length,
                    improved: is_improved,
                });

                info!(
                    "Refetched article {}: {} -> {} chars ({})",
                    id,
                    old_length,
                    new_length,
                    if is_improved { "improved" } else { "no change" }
                );
            }
            Err(e) => {
                failed += 1;

                results.push(RefetchResult {
                    fnord_id: id,
                    title,
                    old_length,
                    new_length: old_length, // unchanged
                    improved: false,
                });

                info!("Failed to refetch article {}: {}", id, e);
            }
        }
    }

    info!(
        "Refetch complete: {} processed, {} improved, {} failed",
        processed, improved, failed
    );

    Ok(RefetchResponse {
        total_found,
        processed,
        improved,
        failed,
        results,
    })
}

/// Statistics about short content articles
#[derive(serde::Serialize)]
pub struct ShortContentStats {
    /// Total articles with full_text_fetched = TRUE
    pub total_fetched: i64,
    /// Articles where content_full IS NULL OR length = 0
    pub content_null_or_empty: i64,
    /// Articles with content_full length < 200
    pub content_under_200: i64,
    /// Articles with content_full length 200-500
    pub content_200_to_500: i64,
    /// Articles with content_full length >= 500 (OK)
    pub content_over_500: i64,
    /// Statistics per feed
    pub by_feed: Vec<FeedStats>,
}

/// Per-feed statistics for short content
#[derive(serde::Serialize)]
pub struct FeedStats {
    pub pentacle_id: i64,
    pub pentacle_title: String,
    /// Articles with content_full < 500 characters
    pub short_articles: i64,
}

/// Get statistics about articles with short or missing full text content
#[tauri::command]
pub async fn get_short_content_stats(
    state: State<'_, AppState>,
) -> Result<ShortContentStats, String> {
    info!("Getting short content statistics");

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

    // Total fetched articles
    let total_fetched: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE full_text_fetched = TRUE",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Content NULL or empty
    let content_null_or_empty: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fnords
             WHERE full_text_fetched = TRUE
             AND (content_full IS NULL OR LENGTH(content_full) = 0)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Content under 200 characters (but not NULL/empty)
    let content_under_200: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fnords
             WHERE full_text_fetched = TRUE
             AND content_full IS NOT NULL
             AND LENGTH(content_full) > 0
             AND LENGTH(content_full) < 200",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Content 200-500 characters
    let content_200_to_500: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fnords
             WHERE full_text_fetched = TRUE
             AND content_full IS NOT NULL
             AND LENGTH(content_full) >= 200
             AND LENGTH(content_full) < 500",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Content >= 500 characters (OK)
    let content_over_500: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fnords
             WHERE full_text_fetched = TRUE
             AND content_full IS NOT NULL
             AND LENGTH(content_full) >= 500",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Per-feed statistics: count articles with short content (< 500 chars)
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.title, COUNT(f.id) as short_count
             FROM pentacles p
             LEFT JOIN fnords f ON f.pentacle_id = p.id
                 AND f.full_text_fetched = TRUE
                 AND (f.content_full IS NULL
                      OR LENGTH(f.content_full) < 500)
             GROUP BY p.id, p.title
             HAVING short_count > 0
             ORDER BY short_count DESC",
        )
        .map_err(|e| e.to_string())?;

    let by_feed: Vec<FeedStats> = stmt
        .query_map(params![], |row| {
            Ok(FeedStats {
                pentacle_id: row.get(0)?,
                pentacle_title: row.get(1)?,
                short_articles: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    info!(
        "Short content stats: total_fetched={}, null_or_empty={}, under_200={}, 200_to_500={}, over_500={}, feeds_with_short={}",
        total_fetched, content_null_or_empty, content_under_200, content_200_to_500, content_over_500, by_feed.len()
    );

    Ok(ShortContentStats {
        total_fetched,
        content_null_or_empty,
        content_under_200,
        content_200_to_500,
        content_over_500,
        by_feed,
    })
}
