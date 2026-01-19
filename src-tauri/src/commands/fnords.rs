use crate::AppState;
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FnordCategoryInfo {
    pub color: Option<String>,
    pub icon: Option<String>,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Fnord {
    pub id: i64,
    pub pentacle_id: i64,
    pub pentacle_title: Option<String>,
    pub guid: String,
    pub url: String,
    pub title: String,
    pub author: Option<String>,
    pub content_raw: Option<String>,
    pub content_full: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub published_at: Option<String>,
    pub processed_at: Option<String>,
    pub status: String,
    pub political_bias: Option<i32>,
    pub sachlichkeit: Option<i32>,
    pub quality_score: Option<i32>,
    pub has_changes: bool,
    pub changed_at: Option<String>,
    pub revision_count: i32,
    pub categories: Vec<FnordCategoryInfo>,
    /// Error type from full-text fetch: NULL (no error), "404", "timeout", "parse_error", "blocked", etc.
    pub full_text_fetch_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FnordRevision {
    pub id: i64,
    pub fnord_id: i64,
    pub title: String,
    pub author: Option<String>,
    pub content_raw: Option<String>,
    pub content_full: Option<String>,
    pub summary: Option<String>,
    pub content_hash: String,
    pub revision_at: String,
}

#[derive(Debug, Deserialize)]
pub struct FnordFilter {
    pub pentacle_id: Option<i64>,
    pub sephiroth_id: Option<i64>,
    pub main_sephiroth_id: Option<i64>,  // Filter by main category (includes all subcategories)
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn fnord_from_row(row: &Row) -> Result<Fnord, rusqlite::Error> {
    Ok(Fnord {
        id: row.get(0)?,
        pentacle_id: row.get(1)?,
        pentacle_title: row.get(2)?,
        guid: row.get(3)?,
        url: row.get(4)?,
        title: row.get(5)?,
        author: row.get(6)?,
        content_raw: row.get(7)?,
        content_full: row.get(8)?,
        summary: row.get(9)?,
        image_url: row.get(10)?,
        published_at: row.get(11)?,
        processed_at: row.get(12)?,
        status: row.get(13)?,
        political_bias: row.get(14)?,
        sachlichkeit: row.get(15)?,
        quality_score: row.get(16)?,
        has_changes: row.get(17)?,
        changed_at: row.get(18)?,
        revision_count: row.get(19)?,
        full_text_fetch_error: row.get(20)?,
        categories: vec![],
    })
}

fn load_categories_for_fnords(
    conn: &rusqlite::Connection,
    fnord_ids: &[i64],
) -> std::collections::HashMap<i64, Vec<FnordCategoryInfo>> {
    if fnord_ids.is_empty() {
        return std::collections::HashMap::new();
    }

    let placeholders: String = fnord_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        r#"SELECT fs.fnord_id, s.name, s.color, s.icon
           FROM fnord_sephiroth fs
           JOIN sephiroth s ON s.id = fs.sephiroth_id
           WHERE fs.fnord_id IN ({})
           ORDER BY fs.confidence DESC"#,
        placeholders
    );

    let mut result: std::collections::HashMap<i64, Vec<FnordCategoryInfo>> =
        std::collections::HashMap::new();

    if let Ok(mut stmt) = conn.prepare(&sql) {
        let params: Vec<&dyn rusqlite::ToSql> = fnord_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        if let Ok(rows) = stmt.query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                FnordCategoryInfo {
                    name: row.get(1)?,
                    color: row.get(2)?,
                    icon: row.get(3)?,
                },
            ))
        }) {
            for row in rows.flatten() {
                result.entry(row.0).or_default().push(row.1);
            }
        }
    }

    result
}

const FNORD_SELECT_COLUMNS: &str = r#"
    f.id,
    f.pentacle_id,
    p.title as pentacle_title,
    f.guid,
    f.url,
    f.title,
    f.author,
    f.content_raw,
    f.content_full,
    f.summary,
    f.image_url,
    f.published_at,
    f.processed_at,
    f.status,
    f.political_bias,
    f.sachlichkeit,
    f.quality_score,
    COALESCE(f.has_changes, FALSE) as has_changes,
    f.changed_at,
    COALESCE(f.revision_count, 0) as revision_count,
    f.full_text_fetch_error
"#;

#[tauri::command]
pub fn get_fnords(
    state: State<AppState>,
    filter: Option<FnordFilter>,
) -> Result<Vec<Fnord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let filter = filter.unwrap_or(FnordFilter {
        pentacle_id: None,
        sephiroth_id: None,
        main_sephiroth_id: None,
        status: None,
        limit: Some(50),
        offset: None,
    });

    let mut sql = format!(
        "SELECT {} FROM fnords f LEFT JOIN pentacles p ON p.id = f.pentacle_id WHERE 1=1",
        FNORD_SELECT_COLUMNS
    );

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(pentacle_id) = filter.pentacle_id {
        sql.push_str(" AND f.pentacle_id = ?");
        params.push(Box::new(pentacle_id));
    }

    if let Some(sephiroth_id) = filter.sephiroth_id {
        sql.push_str(" AND f.id IN (SELECT fnord_id FROM fnord_sephiroth WHERE sephiroth_id = ?)");
        params.push(Box::new(sephiroth_id));
    }

    // Filter by main category (includes all subcategories)
    if let Some(main_sephiroth_id) = filter.main_sephiroth_id {
        sql.push_str(" AND f.id IN (SELECT fs.fnord_id FROM fnord_sephiroth fs JOIN sephiroth s ON s.id = fs.sephiroth_id WHERE s.parent_id = ?)");
        params.push(Box::new(main_sephiroth_id));
    }

    if let Some(status) = &filter.status {
        sql.push_str(" AND f.status = ?");
        params.push(Box::new(status.clone()));
    }

    sql.push_str(" ORDER BY f.published_at DESC");

    if let Some(limit) = filter.limit {
        sql.push_str(&format!(" LIMIT {}", limit));
    }

    if let Some(offset) = filter.offset {
        sql.push_str(&format!(" OFFSET {}", offset));
    }

    let mut stmt = db.conn().prepare(&sql).map_err(|e| e.to_string())?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let mut fnords: Vec<Fnord> = stmt
        .query_map(param_refs.as_slice(), fnord_from_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let fnord_ids: Vec<i64> = fnords.iter().map(|f| f.id).collect();
    let categories_map = load_categories_for_fnords(db.conn(), &fnord_ids);

    for fnord in &mut fnords {
        if let Some(cats) = categories_map.get(&fnord.id) {
            fnord.categories = cats.clone();
        }
    }

    Ok(fnords)
}

#[tauri::command]
pub fn get_fnord(state: State<AppState>, id: i64) -> Result<Fnord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let sql = format!(
        "SELECT {} FROM fnords f LEFT JOIN pentacles p ON p.id = f.pentacle_id WHERE f.id = ?1",
        FNORD_SELECT_COLUMNS
    );

    let mut fnord = db
        .conn()
        .query_row(&sql, [id], fnord_from_row)
        .map_err(|e| e.to_string())?;

    let categories_map = load_categories_for_fnords(db.conn(), &[id]);
    if let Some(cats) = categories_map.get(&id) {
        fnord.categories = cats.clone();
    }

    Ok(fnord)
}

#[tauri::command]
pub fn update_fnord_status(state: State<AppState>, id: i64, status: String) -> Result<(), String> {
    // Validate status
    if !["concealed", "illuminated", "golden_apple"].contains(&status.as_str()) {
        return Err(format!("Invalid status: {}", status));
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Only set read_at on first read (when transitioning from concealed to read status)
    // and preserve existing read_at value
    if status == "illuminated" || status == "golden_apple" {
        // Set read_at only if it's NULL (first time reading)
        db.conn()
            .execute(
                "UPDATE fnords SET status = ?1, read_at = COALESCE(read_at, ?2) WHERE id = ?3",
                (&status, chrono::Utc::now().to_rfc3339(), id),
            )
            .map_err(|e| e.to_string())?;
    } else {
        // Just update status, don't touch read_at (preserve reading history)
        db.conn()
            .execute("UPDATE fnords SET status = ?1 WHERE id = ?2", (&status, id))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_changed_fnords(state: State<AppState>) -> Result<Vec<Fnord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let sql = format!(
        "SELECT {} FROM fnords f LEFT JOIN pentacles p ON p.id = f.pentacle_id WHERE f.has_changes = TRUE ORDER BY f.changed_at DESC",
        FNORD_SELECT_COLUMNS
    );

    let mut stmt = db.conn().prepare(&sql).map_err(|e| e.to_string())?;

    let mut fnords: Vec<Fnord> = stmt
        .query_map([], fnord_from_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let fnord_ids: Vec<i64> = fnords.iter().map(|f| f.id).collect();
    let categories_map = load_categories_for_fnords(db.conn(), &fnord_ids);

    for fnord in &mut fnords {
        if let Some(cats) = categories_map.get(&fnord.id) {
            fnord.categories = cats.clone();
        }
    }

    Ok(fnords)
}

/// Acknowledge changes for an article (dismiss change notification)
#[tauri::command]
pub fn acknowledge_changes(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute("UPDATE fnords SET has_changes = FALSE WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get revision history for an article
#[tauri::command]
pub fn get_fnord_revisions(
    state: State<AppState>,
    fnord_id: i64,
) -> Result<Vec<FnordRevision>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, fnord_id, title, author, content_raw, content_full, summary, content_hash, revision_at
            FROM fnord_revisions
            WHERE fnord_id = ?1
            ORDER BY revision_at DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let revisions = stmt
        .query_map([fnord_id], |row| {
            Ok(FnordRevision {
                id: row.get(0)?,
                fnord_id: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                content_raw: row.get(4)?,
                content_full: row.get(5)?,
                summary: row.get(6)?,
                content_hash: row.get(7)?,
                revision_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(revisions)
}

/// Get total count of fnords matching a filter (for lazy loading)
#[tauri::command]
pub fn get_fnords_count(
    state: State<AppState>,
    filter: Option<FnordFilter>,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut sql = String::from("SELECT COUNT(*) FROM fnords f WHERE 1=1");
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref f) = filter {
        if let Some(pentacle_id) = f.pentacle_id {
            sql.push_str(" AND f.pentacle_id = ?");
            params.push(Box::new(pentacle_id));
        }

        if let Some(sephiroth_id) = f.sephiroth_id {
            sql.push_str(" AND f.id IN (SELECT fnord_id FROM fnord_sephiroth WHERE sephiroth_id = ?)");
            params.push(Box::new(sephiroth_id));
        }

        // Filter by main category (includes all subcategories)
        if let Some(main_sephiroth_id) = f.main_sephiroth_id {
            sql.push_str(" AND f.id IN (SELECT fs.fnord_id FROM fnord_sephiroth fs JOIN sephiroth s ON s.id = fs.sephiroth_id WHERE s.parent_id = ?)");
            params.push(Box::new(main_sephiroth_id));
        }

        if let Some(ref status) = f.status {
            sql.push_str(" AND f.status = ?");
            params.push(Box::new(status.clone()));
        }
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let count: i64 = db
        .conn()
        .query_row(&sql, param_refs.as_slice(), |row| row.get(0))
        .map_err(|e| e.to_string())?;

    Ok(count)
}

/// Get count of changed articles
#[tauri::command]
pub fn get_changed_count(state: State<AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE has_changes = TRUE",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(count)
}

/// Reset change flags for articles without actual revisions (false positives from migration)
#[tauri::command]
pub fn reset_all_changes(state: State<AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Only reset changes for articles that have no actual revisions
    // (these are false positives from the migration bug)
    let affected = db
        .conn()
        .execute(
            r#"UPDATE fnords SET has_changes = FALSE
               WHERE has_changes = TRUE
               AND id NOT IN (SELECT DISTINCT fnord_id FROM fnord_revisions)"#,
            [],
        )
        .map_err(|e| e.to_string())?;

    Ok(affected as i64)
}

// ============================================================
// Fnord Statistics (für Fnord-Tab)
// ============================================================

#[derive(Debug, Serialize)]
pub struct FnordStats {
    pub total_revisions: i64,
    pub articles_with_changes: i64,
    pub by_category: Vec<CategoryRevisionStats>,
    pub by_source: Vec<SourceRevisionStats>,
}

#[derive(Debug, Serialize)]
pub struct CategoryRevisionStats {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub revision_count: i64,
    pub article_count: i64,
}

#[derive(Debug, Serialize)]
pub struct SourceRevisionStats {
    pub pentacle_id: i64,
    pub title: Option<String>,
    pub revision_count: i64,
    pub article_count: i64,
}

/// Get Fnord statistics: revision counts by category and source
#[tauri::command]
pub fn get_fnord_stats(state: State<AppState>) -> Result<FnordStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Total revisions
    let total_revisions: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM fnord_revisions", [], |r| r.get(0))
        .unwrap_or(0);

    // Articles with changes (have at least one revision)
    let articles_with_changes: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(DISTINCT fnord_id) FROM fnord_revisions",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // By category - aggregate revisions by main category (level = 0)
    // Articles are assigned to subcategories (level 1), so we join through
    // subcategories to their parent main categories
    let mut stmt_cat = db
        .conn()
        .prepare(
            r#"
            SELECT m.id, m.name, m.icon, m.color,
                   COUNT(DISTINCT r.id) as revision_count,
                   COUNT(DISTINCT r.fnord_id) as article_count
            FROM sephiroth m
            LEFT JOIN sephiroth s ON s.parent_id = m.id
            LEFT JOIN fnord_sephiroth fs ON fs.sephiroth_id = s.id
            LEFT JOIN fnord_revisions r ON r.fnord_id = fs.fnord_id
            WHERE m.level = 0
            GROUP BY m.id
            ORDER BY revision_count DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let by_category = stmt_cat
        .query_map([], |row| {
            Ok(CategoryRevisionStats {
                sephiroth_id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                revision_count: row.get(4)?,
                article_count: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // By source - count revisions for articles from each pentacle
    let mut stmt_src = db
        .conn()
        .prepare(
            r#"
            SELECT p.id, p.title,
                   COUNT(DISTINCT r.id) as revision_count,
                   COUNT(DISTINCT r.fnord_id) as article_count
            FROM pentacles p
            LEFT JOIN fnords f ON f.pentacle_id = p.id
            LEFT JOIN fnord_revisions r ON r.fnord_id = f.id
            GROUP BY p.id
            ORDER BY revision_count DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let by_source = stmt_src
        .query_map([], |row| {
            Ok(SourceRevisionStats {
                pentacle_id: row.get(0)?,
                title: row.get(1)?,
                revision_count: row.get(2)?,
                article_count: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(FnordStats {
        total_revisions,
        articles_with_changes,
        by_category,
        by_source,
    })
}

/// Get subcategory stats for a main category
#[tauri::command]
pub fn get_subcategory_stats(
    state: State<AppState>,
    main_category_id: i64,
) -> Result<Vec<CategoryRevisionStats>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT s.id, s.name, s.icon, m.color,
                   COUNT(DISTINCT r.id) as revision_count,
                   COUNT(DISTINCT r.fnord_id) as article_count
            FROM sephiroth s
            JOIN sephiroth m ON m.id = s.parent_id
            LEFT JOIN fnord_sephiroth fs ON fs.sephiroth_id = s.id
            LEFT JOIN fnord_revisions r ON r.fnord_id = fs.fnord_id
            WHERE s.parent_id = ?1 AND s.level = 1
            GROUP BY s.id
            ORDER BY revision_count DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let subcategories = stmt
        .query_map([main_category_id], |row| {
            Ok(CategoryRevisionStats {
                sephiroth_id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                revision_count: row.get(4)?,
                article_count: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(subcategories)
}

// ============================================================
// Extended Fnord Statistics (Plan 4: Fnord-Statistiken Vollausbau)
// ============================================================

/// Timeline data point for article/revision activity
#[derive(Debug, Serialize)]
pub struct TimelineDataPoint {
    pub date: String,
    pub articles: i64,
    pub revisions: i64,
}

/// Article timeline for a given period
#[derive(Debug, Serialize)]
pub struct ArticleTimeline {
    pub data: Vec<TimelineDataPoint>,
    pub period_days: i64,
}

/// Greyface Index - measures bias across the corpus
#[derive(Debug, Serialize)]
pub struct GreyfaceIndex {
    /// Overall index (0-100, where 0 = perfectly balanced, 100 = extreme bias)
    pub index: f64,
    /// Average political bias (-2 to +2)
    pub avg_political_bias: f64,
    /// Average sachlichkeit (0-4, higher = more factual)
    pub avg_sachlichkeit: f64,
    /// Distribution counts
    pub bias_distribution: BiasDistribution,
    /// Number of articles with bias data
    pub articles_with_bias: i64,
    /// Total articles
    pub total_articles: i64,
}

#[derive(Debug, Serialize)]
pub struct BiasDistribution {
    pub left_extreme: i64,    // -2
    pub left_leaning: i64,    // -1
    pub neutral: i64,         // 0
    pub right_leaning: i64,   // +1
    pub right_extreme: i64,   // +2
}

/// Keyword statistics with trend
#[derive(Debug, Serialize)]
pub struct KeywordStats {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub trend: f64,  // Percentage change from previous period
    pub keyword_type: Option<String>,
}

/// Feed activity statistics
#[derive(Debug, Serialize)]
pub struct FeedActivity {
    pub pentacle_id: i64,
    pub title: Option<String>,
    pub articles_total: i64,
    pub articles_period: i64,
    pub revisions_period: i64,
    pub last_sync: Option<String>,
}

/// Bias heatmap entry (Feed x Bias)
#[derive(Debug, Serialize)]
pub struct BiasHeatmapEntry {
    pub pentacle_id: i64,
    pub pentacle_title: Option<String>,
    pub bias_minus_2: i64,
    pub bias_minus_1: i64,
    pub bias_0: i64,
    pub bias_plus_1: i64,
    pub bias_plus_2: i64,
    pub avg_bias: f64,
}

/// Keyword cloud entry
#[derive(Debug, Serialize)]
pub struct KeywordCloudEntry {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub weight: f64,  // Normalized weight for display (0.0-1.0)
    pub keyword_type: Option<String>,
}

/// Get article timeline for a period
#[tauri::command]
pub fn get_article_timeline(
    state: State<AppState>,
    days: i64,
) -> Result<ArticleTimeline, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // SQLite doesn't have generate_series by default, use a different approach
    let mut data = Vec::new();

    // Get articles by date
    let mut stmt_articles = db
        .conn()
        .prepare(
            r#"
            SELECT date(published_at) as pub_date, COUNT(*) as count
            FROM fnords
            WHERE published_at >= date('now', '-' || ?1 || ' days')
            GROUP BY date(published_at)
            "#,
        )
        .map_err(|e| e.to_string())?;

    let articles_by_date: std::collections::HashMap<String, i64> = stmt_articles
        .query_map([days], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get revisions by date
    let mut stmt_revisions = db
        .conn()
        .prepare(
            r#"
            SELECT date(revision_at) as rev_date, COUNT(*) as count
            FROM fnord_revisions
            WHERE revision_at >= date('now', '-' || ?1 || ' days')
            GROUP BY date(revision_at)
            "#,
        )
        .map_err(|e| e.to_string())?;

    let revisions_by_date: std::collections::HashMap<String, i64> = stmt_revisions
        .query_map([days], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Generate date range
    for i in 0..days {
        let date: String = db
            .conn()
            .query_row(
                "SELECT date('now', '-' || ?1 || ' days')",
                [days - 1 - i],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        data.push(TimelineDataPoint {
            date: date.clone(),
            articles: *articles_by_date.get(&date).unwrap_or(&0),
            revisions: *revisions_by_date.get(&date).unwrap_or(&0),
        });
    }

    Ok(ArticleTimeline {
        data,
        period_days: days,
    })
}

/// Get the Greyface Index (bias metrics)
#[tauri::command]
pub fn get_greyface_index(state: State<AppState>) -> Result<GreyfaceIndex, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get bias distribution
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                political_bias,
                COUNT(*) as count
            FROM fnords
            WHERE political_bias IS NOT NULL
            GROUP BY political_bias
            "#,
        )
        .map_err(|e| e.to_string())?;

    let bias_counts: std::collections::HashMap<i32, i64> = stmt
        .query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let distribution = BiasDistribution {
        left_extreme: *bias_counts.get(&-2).unwrap_or(&0),
        left_leaning: *bias_counts.get(&-1).unwrap_or(&0),
        neutral: *bias_counts.get(&0).unwrap_or(&0),
        right_leaning: *bias_counts.get(&1).unwrap_or(&0),
        right_extreme: *bias_counts.get(&2).unwrap_or(&0),
    };

    // Get averages
    let (avg_political_bias, avg_sachlichkeit, articles_with_bias): (f64, f64, i64) = db
        .conn()
        .query_row(
            r#"
            SELECT
                COALESCE(AVG(CAST(political_bias AS REAL)), 0.0),
                COALESCE(AVG(CAST(sachlichkeit AS REAL)), 0.0),
                COUNT(*)
            FROM fnords
            WHERE political_bias IS NOT NULL
            "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    let total_articles: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    // Calculate Greyface Index
    // Formula: combines bias imbalance and lack of neutrality
    // Lower sachlichkeit increases index, imbalance in left/right increases index
    let left_total = distribution.left_extreme as f64 * 2.0 + distribution.left_leaning as f64;
    let right_total = distribution.right_extreme as f64 * 2.0 + distribution.right_leaning as f64;
    let total = articles_with_bias.max(1) as f64;

    let imbalance = ((left_total - right_total).abs() / total) * 25.0; // 0-50 range
    let extremism = ((distribution.left_extreme + distribution.right_extreme) as f64 / total) * 25.0; // 0-25 range
    let unsachlichkeit = ((4.0 - avg_sachlichkeit) / 4.0) * 25.0; // 0-25 range

    let index = (imbalance + extremism + unsachlichkeit).min(100.0);

    Ok(GreyfaceIndex {
        index,
        avg_political_bias,
        avg_sachlichkeit,
        bias_distribution: distribution,
        articles_with_bias,
        total_articles,
    })
}

/// Get top keywords with trend for a period
#[tauri::command]
pub fn get_top_keywords_stats(
    state: State<AppState>,
    days: i64,
    limit: i64,
) -> Result<Vec<KeywordStats>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Current period counts
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                i.keyword_type,
                COUNT(fi.fnord_id) as count,
                (
                    SELECT COUNT(*)
                    FROM fnord_immanentize fi2
                    JOIN fnords f2 ON f2.id = fi2.fnord_id
                    WHERE fi2.immanentize_id = i.id
                    AND f2.published_at >= date('now', '-' || (?1 * 2) || ' days')
                    AND f2.published_at < date('now', '-' || ?1 || ' days')
                ) as prev_count
            FROM immanentize i
            JOIN fnord_immanentize fi ON fi.immanentize_id = i.id
            JOIN fnords f ON f.id = fi.fnord_id
            WHERE f.published_at >= date('now', '-' || ?1 || ' days')
            GROUP BY i.id
            ORDER BY count DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map([days, limit], |row| {
            let count: i64 = row.get(3)?;
            let prev_count: i64 = row.get(4)?;
            let trend = if prev_count > 0 {
                ((count - prev_count) as f64 / prev_count as f64) * 100.0
            } else if count > 0 {
                100.0 // New keyword
            } else {
                0.0
            };

            Ok(KeywordStats {
                id: row.get(0)?,
                name: row.get(1)?,
                keyword_type: row.get(2)?,
                count,
                trend,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

/// Get feed activity for a period
#[tauri::command]
pub fn get_feed_activity(
    state: State<AppState>,
    days: i64,
    limit: i64,
) -> Result<Vec<FeedActivity>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                p.id,
                p.title,
                (SELECT COUNT(*) FROM fnords WHERE pentacle_id = p.id) as articles_total,
                (
                    SELECT COUNT(*)
                    FROM fnords
                    WHERE pentacle_id = p.id
                    AND published_at >= date('now', '-' || ?1 || ' days')
                ) as articles_period,
                (
                    SELECT COUNT(*)
                    FROM fnord_revisions r
                    JOIN fnords f ON f.id = r.fnord_id
                    WHERE f.pentacle_id = p.id
                    AND r.revision_at >= date('now', '-' || ?1 || ' days')
                ) as revisions_period,
                p.last_sync
            FROM pentacles p
            ORDER BY articles_period DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;

    let feeds = stmt
        .query_map([days, limit], |row| {
            Ok(FeedActivity {
                pentacle_id: row.get(0)?,
                title: row.get(1)?,
                articles_total: row.get(2)?,
                articles_period: row.get(3)?,
                revisions_period: row.get(4)?,
                last_sync: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(feeds)
}

/// Get bias heatmap (feeds x bias levels)
#[tauri::command]
pub fn get_bias_heatmap(state: State<AppState>) -> Result<Vec<BiasHeatmapEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                p.id,
                p.title,
                SUM(CASE WHEN f.political_bias = -2 THEN 1 ELSE 0 END) as bias_m2,
                SUM(CASE WHEN f.political_bias = -1 THEN 1 ELSE 0 END) as bias_m1,
                SUM(CASE WHEN f.political_bias = 0 THEN 1 ELSE 0 END) as bias_0,
                SUM(CASE WHEN f.political_bias = 1 THEN 1 ELSE 0 END) as bias_p1,
                SUM(CASE WHEN f.political_bias = 2 THEN 1 ELSE 0 END) as bias_p2,
                COALESCE(AVG(CAST(f.political_bias AS REAL)), 0.0) as avg_bias
            FROM pentacles p
            LEFT JOIN fnords f ON f.pentacle_id = p.id AND f.political_bias IS NOT NULL
            GROUP BY p.id
            HAVING (bias_m2 + bias_m1 + bias_0 + bias_p1 + bias_p2) > 0
            ORDER BY avg_bias ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let heatmap = stmt
        .query_map([], |row| {
            Ok(BiasHeatmapEntry {
                pentacle_id: row.get(0)?,
                pentacle_title: row.get(1)?,
                bias_minus_2: row.get(2)?,
                bias_minus_1: row.get(3)?,
                bias_0: row.get(4)?,
                bias_plus_1: row.get(5)?,
                bias_plus_2: row.get(6)?,
                avg_bias: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(heatmap)
}

/// Get keyword cloud data
#[tauri::command]
pub fn get_keyword_cloud(
    state: State<AppState>,
    days: i64,
    limit: i64,
) -> Result<Vec<KeywordCloudEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                i.keyword_type,
                COUNT(fi.fnord_id) as count
            FROM immanentize i
            JOIN fnord_immanentize fi ON fi.immanentize_id = i.id
            JOIN fnords f ON f.id = fi.fnord_id
            WHERE f.published_at >= date('now', '-' || ?1 || ' days')
            GROUP BY i.id
            ORDER BY count DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords: Vec<(i64, String, Option<String>, i64)> = stmt
        .query_map([days, limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Calculate max for normalization
    let max_count = keywords.iter().map(|(_, _, _, c)| *c).max().unwrap_or(1) as f64;

    let cloud = keywords
        .into_iter()
        .map(|(id, name, keyword_type, count)| KeywordCloudEntry {
            id,
            name,
            keyword_type,
            count,
            weight: (count as f64 / max_count).powf(0.5), // Square root for better distribution
        })
        .collect();

    Ok(cloud)
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    /// Helper to create a test database with a pentacle and fnords
    fn setup_test_db() -> Database {
        let db = Database::new_in_memory().expect("Failed to create in-memory database");

        // Insert a test pentacle
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://example.com/feed.xml", "Test Feed"],
            )
            .expect("Failed to insert test pentacle");

        db
    }

    /// Helper to insert a test fnord
    fn insert_test_fnord(
        conn: &rusqlite::Connection,
        pentacle_id: i64,
        guid: &str,
        title: &str,
        status: &str,
    ) -> i64 {
        conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, status)
               VALUES (?1, ?2, ?3, ?4, ?5)"#,
            rusqlite::params![pentacle_id, guid, format!("https://example.com/{}", guid), title, status],
        )
        .expect("Failed to insert test fnord");
        conn.last_insert_rowid()
    }

    // ============================================================
    // get_fnords tests (via direct DB queries simulating the command)
    // ============================================================

    #[test]
    fn test_get_fnords_empty_database() {
        let db = setup_test_db();

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
            .expect("Failed to count fnords");

        assert_eq!(count, 0, "Empty database should have no fnords");
    }

    #[test]
    fn test_get_fnords_returns_all_fnords() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        // Insert multiple fnords
        insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Article 1", "concealed");
        insert_test_fnord(db.conn(), pentacle_id, "guid-2", "Article 2", "illuminated");
        insert_test_fnord(db.conn(), pentacle_id, "guid-3", "Article 3", "golden_apple");

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
            .expect("Failed to count fnords");

        assert_eq!(count, 3, "Should have 3 fnords");
    }

    #[test]
    fn test_get_fnords_filter_by_status() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        // Insert fnords with different statuses
        insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Unread 1", "concealed");
        insert_test_fnord(db.conn(), pentacle_id, "guid-2", "Unread 2", "concealed");
        insert_test_fnord(db.conn(), pentacle_id, "guid-3", "Read", "illuminated");

        let concealed_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE status = 'concealed'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count concealed fnords");

        assert_eq!(concealed_count, 2, "Should have 2 concealed fnords");
    }

    #[test]
    fn test_get_fnords_filter_by_pentacle() {
        let db = setup_test_db();

        // Insert second pentacle
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://other.com/feed.xml", "Other Feed"],
            )
            .expect("Failed to insert second pentacle");

        let pentacle1_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles WHERE title = 'Test Feed'", [], |row| row.get(0))
            .expect("Failed to get pentacle 1 id");

        let pentacle2_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles WHERE title = 'Other Feed'", [], |row| row.get(0))
            .expect("Failed to get pentacle 2 id");

        // Insert fnords in different pentacles
        insert_test_fnord(db.conn(), pentacle1_id, "guid-1", "Feed1 Article 1", "concealed");
        insert_test_fnord(db.conn(), pentacle1_id, "guid-2", "Feed1 Article 2", "concealed");
        insert_test_fnord(db.conn(), pentacle2_id, "guid-3", "Feed2 Article", "concealed");

        let feed1_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE pentacle_id = ?1",
                [pentacle1_id],
                |row| row.get(0),
            )
            .expect("Failed to count fnords for pentacle 1");

        assert_eq!(feed1_count, 2, "Pentacle 1 should have 2 fnords");
    }

    #[test]
    fn test_get_fnords_with_limit() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        // Insert 10 fnords
        for i in 0..10 {
            insert_test_fnord(
                db.conn(),
                pentacle_id,
                &format!("guid-{}", i),
                &format!("Article {}", i),
                "concealed",
            );
        }

        // Query with limit
        let mut stmt = db
            .conn()
            .prepare("SELECT id FROM fnords LIMIT 5")
            .expect("Failed to prepare statement");

        let fnords: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(fnords.len(), 5, "Should return only 5 fnords with limit");
    }

    // ============================================================
    // update_fnord_status tests
    // ============================================================

    #[test]
    fn test_update_fnord_status_concealed_to_illuminated() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Test Article", "concealed");

        // Update status
        db.conn()
            .execute(
                "UPDATE fnords SET status = ?1 WHERE id = ?2",
                ["illuminated", &fnord_id.to_string()],
            )
            .expect("Failed to update status");

        let status: String = db
            .conn()
            .query_row(
                "SELECT status FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to get status");

        assert_eq!(status, "illuminated", "Status should be updated to illuminated");
    }

    #[test]
    fn test_update_fnord_status_to_golden_apple() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Test Article", "concealed");

        // Update status to golden_apple
        db.conn()
            .execute(
                "UPDATE fnords SET status = ?1 WHERE id = ?2",
                ["golden_apple", &fnord_id.to_string()],
            )
            .expect("Failed to update status");

        let status: String = db
            .conn()
            .query_row(
                "SELECT status FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to get status");

        assert_eq!(status, "golden_apple", "Status should be updated to golden_apple");
    }

    #[test]
    fn test_update_fnord_status_sets_read_at_on_first_read() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Test Article", "concealed");

        // Verify read_at is NULL initially
        let read_at_before: Option<String> = db
            .conn()
            .query_row(
                "SELECT read_at FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to get read_at");

        assert!(read_at_before.is_none(), "read_at should be NULL before first read");

        // Update to illuminated with read_at (simulating the command logic)
        db.conn()
            .execute(
                "UPDATE fnords SET status = ?1, read_at = COALESCE(read_at, ?2) WHERE id = ?3",
                rusqlite::params!["illuminated", chrono::Utc::now().to_rfc3339(), fnord_id],
            )
            .expect("Failed to update status");

        let read_at_after: Option<String> = db
            .conn()
            .query_row(
                "SELECT read_at FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to get read_at");

        assert!(read_at_after.is_some(), "read_at should be set after first read");
    }

    #[test]
    fn test_update_fnord_status_preserves_read_at_on_subsequent_reads() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Test Article", "concealed");

        // Set initial read_at
        let first_read_time = "2024-01-01T10:00:00Z";
        db.conn()
            .execute(
                "UPDATE fnords SET status = 'illuminated', read_at = ?1 WHERE id = ?2",
                [first_read_time, &fnord_id.to_string()],
            )
            .expect("Failed to set initial read");

        // Update to golden_apple (should preserve read_at)
        db.conn()
            .execute(
                "UPDATE fnords SET status = ?1, read_at = COALESCE(read_at, ?2) WHERE id = ?3",
                rusqlite::params!["golden_apple", chrono::Utc::now().to_rfc3339(), fnord_id],
            )
            .expect("Failed to update status");

        let read_at: String = db
            .conn()
            .query_row(
                "SELECT read_at FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to get read_at");

        assert_eq!(read_at, first_read_time, "read_at should be preserved from first read");
    }

    #[test]
    fn test_update_fnord_status_invalid_status() {
        // Test that the validation logic works
        let invalid_statuses = ["unread", "read", "favorite", "CONCEALED", ""];
        let valid_statuses = ["concealed", "illuminated", "golden_apple"];

        for status in valid_statuses {
            assert!(
                ["concealed", "illuminated", "golden_apple"].contains(&status),
                "{} should be valid",
                status
            );
        }

        for status in invalid_statuses {
            assert!(
                !["concealed", "illuminated", "golden_apple"].contains(&status),
                "{} should be invalid",
                status
            );
        }
    }

    // ============================================================
    // acknowledge_changes tests
    // ============================================================

    #[test]
    fn test_acknowledge_changes_clears_flag() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Test Article", "concealed");

        // Set has_changes to TRUE
        db.conn()
            .execute(
                "UPDATE fnords SET has_changes = TRUE WHERE id = ?1",
                [fnord_id],
            )
            .expect("Failed to set has_changes");

        // Verify has_changes is TRUE
        let has_changes_before: bool = db
            .conn()
            .query_row(
                "SELECT has_changes FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to get has_changes");

        assert!(has_changes_before, "has_changes should be TRUE before acknowledge");

        // Acknowledge changes
        db.conn()
            .execute(
                "UPDATE fnords SET has_changes = FALSE WHERE id = ?1",
                [fnord_id],
            )
            .expect("Failed to acknowledge changes");

        let has_changes_after: bool = db
            .conn()
            .query_row(
                "SELECT has_changes FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to get has_changes");

        assert!(!has_changes_after, "has_changes should be FALSE after acknowledge");
    }

    // ============================================================
    // get_changed_fnords tests
    // ============================================================

    #[test]
    fn test_get_changed_fnords_empty() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        // Insert fnords without changes
        insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Article 1", "concealed");
        insert_test_fnord(db.conn(), pentacle_id, "guid-2", "Article 2", "illuminated");

        let changed_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE has_changes = TRUE",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count changed fnords");

        assert_eq!(changed_count, 0, "Should have no changed fnords");
    }

    #[test]
    fn test_get_changed_fnords_returns_only_changed() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord1_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Changed Article", "concealed");
        insert_test_fnord(db.conn(), pentacle_id, "guid-2", "Unchanged Article", "concealed");

        // Mark first fnord as changed
        db.conn()
            .execute(
                "UPDATE fnords SET has_changes = TRUE WHERE id = ?1",
                [fnord1_id],
            )
            .expect("Failed to set has_changes");

        let changed_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE has_changes = TRUE",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count changed fnords");

        assert_eq!(changed_count, 1, "Should have exactly 1 changed fnord");
    }

    // ============================================================
    // get_fnord_revisions tests
    // ============================================================

    #[test]
    fn test_get_fnord_revisions_empty() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Test Article", "concealed");

        let revision_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnord_revisions WHERE fnord_id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to count revisions");

        assert_eq!(revision_count, 0, "New fnord should have no revisions");
    }

    #[test]
    fn test_get_fnord_revisions_returns_all() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Test Article", "concealed");

        // Insert revisions
        for i in 0..3 {
            db.conn()
                .execute(
                    r#"INSERT INTO fnord_revisions (fnord_id, title, content_raw, content_hash)
                       VALUES (?1, ?2, ?3, ?4)"#,
                    rusqlite::params![fnord_id, format!("Title v{}", i), format!("Content v{}", i), format!("hash{}", i)],
                )
                .expect("Failed to insert revision");
        }

        let revision_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnord_revisions WHERE fnord_id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to count revisions");

        assert_eq!(revision_count, 3, "Should have 3 revisions");
    }

    // ============================================================
    // get_fnords_count tests
    // ============================================================

    #[test]
    fn test_get_fnords_count_empty() {
        let db = setup_test_db();

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
            .expect("Failed to count fnords");

        assert_eq!(count, 0, "Empty database should return 0");
    }

    #[test]
    fn test_get_fnords_count_with_status_filter() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Unread 1", "concealed");
        insert_test_fnord(db.conn(), pentacle_id, "guid-2", "Unread 2", "concealed");
        insert_test_fnord(db.conn(), pentacle_id, "guid-3", "Read", "illuminated");
        insert_test_fnord(db.conn(), pentacle_id, "guid-4", "Favorite", "golden_apple");

        let total: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
            .expect("Failed to count");

        let concealed: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE status = 'concealed'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count");

        assert_eq!(total, 4, "Total should be 4");
        assert_eq!(concealed, 2, "Concealed count should be 2");
    }

    // ============================================================
    // get_changed_count tests
    // ============================================================

    #[test]
    fn test_get_changed_count() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord1_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Article 1", "concealed");
        let fnord2_id = insert_test_fnord(db.conn(), pentacle_id, "guid-2", "Article 2", "concealed");
        insert_test_fnord(db.conn(), pentacle_id, "guid-3", "Article 3", "concealed");

        // Mark some as changed
        db.conn()
            .execute(
                "UPDATE fnords SET has_changes = TRUE WHERE id IN (?1, ?2)",
                rusqlite::params![fnord1_id, fnord2_id],
            )
            .expect("Failed to set has_changes");

        let changed_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE has_changes = TRUE",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count changed");

        assert_eq!(changed_count, 2, "Should have 2 changed fnords");
    }

    // ============================================================
    // reset_all_changes tests
    // ============================================================

    #[test]
    fn test_reset_all_changes_without_revisions() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord1_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Article 1", "concealed");
        let fnord2_id = insert_test_fnord(db.conn(), pentacle_id, "guid-2", "Article 2", "concealed");

        // Mark both as changed (false positives - no actual revisions)
        db.conn()
            .execute(
                "UPDATE fnords SET has_changes = TRUE",
                [],
            )
            .expect("Failed to set has_changes");

        // Reset changes for articles without revisions
        let affected = db
            .conn()
            .execute(
                r#"UPDATE fnords SET has_changes = FALSE
                   WHERE has_changes = TRUE
                   AND id NOT IN (SELECT DISTINCT fnord_id FROM fnord_revisions)"#,
                [],
            )
            .expect("Failed to reset changes");

        assert_eq!(affected, 2, "Should reset 2 false positives");

        let changed_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE has_changes = TRUE",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count changed");

        assert_eq!(changed_count, 0, "All changes should be reset");
    }

    #[test]
    fn test_reset_all_changes_preserves_real_changes() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord1_id = insert_test_fnord(db.conn(), pentacle_id, "guid-1", "Real Change", "concealed");
        let fnord2_id = insert_test_fnord(db.conn(), pentacle_id, "guid-2", "False Positive", "concealed");

        // Add a revision to fnord1 (real change)
        db.conn()
            .execute(
                r#"INSERT INTO fnord_revisions (fnord_id, title, content_hash)
                   VALUES (?1, 'Old Title', 'hash1')"#,
                [fnord1_id],
            )
            .expect("Failed to insert revision");

        // Mark both as changed
        db.conn()
            .execute("UPDATE fnords SET has_changes = TRUE", [])
            .expect("Failed to set has_changes");

        // Reset only false positives
        db.conn()
            .execute(
                r#"UPDATE fnords SET has_changes = FALSE
                   WHERE has_changes = TRUE
                   AND id NOT IN (SELECT DISTINCT fnord_id FROM fnord_revisions)"#,
                [],
            )
            .expect("Failed to reset changes");

        // Verify fnord1 (with revision) still has changes
        let fnord1_has_changes: bool = db
            .conn()
            .query_row(
                "SELECT has_changes FROM fnords WHERE id = ?1",
                [fnord1_id],
                |row| row.get(0),
            )
            .expect("Failed to get has_changes");

        // Verify fnord2 (without revision) was reset
        let fnord2_has_changes: bool = db
            .conn()
            .query_row(
                "SELECT has_changes FROM fnords WHERE id = ?1",
                [fnord2_id],
                |row| row.get(0),
            )
            .expect("Failed to get has_changes");

        assert!(fnord1_has_changes, "Real change should be preserved");
        assert!(!fnord2_has_changes, "False positive should be reset");
    }

    // ============================================================
    // Fnord struct tests
    // ============================================================

    #[test]
    fn test_fnord_struct_default_values() {
        let fnord = Fnord {
            id: 1,
            pentacle_id: 1,
            pentacle_title: Some("Test Feed".to_string()),
            guid: "guid-123".to_string(),
            url: "https://example.com/article".to_string(),
            title: "Test Article".to_string(),
            author: None,
            content_raw: None,
            content_full: None,
            summary: None,
            image_url: None,
            published_at: None,
            processed_at: None,
            status: "concealed".to_string(),
            political_bias: None,
            sachlichkeit: None,
            quality_score: None,
            has_changes: false,
            changed_at: None,
            revision_count: 0,
            categories: vec![],
            full_text_fetch_error: None,
        };

        assert_eq!(fnord.status, "concealed");
        assert!(!fnord.has_changes);
        assert_eq!(fnord.revision_count, 0);
        assert!(fnord.categories.is_empty());
        assert!(fnord.full_text_fetch_error.is_none());
    }

    #[test]
    fn test_fnord_serialization() {
        let fnord = Fnord {
            id: 1,
            pentacle_id: 1,
            pentacle_title: Some("Test Feed".to_string()),
            guid: "guid-123".to_string(),
            url: "https://example.com/article".to_string(),
            title: "Test Article".to_string(),
            author: Some("Author Name".to_string()),
            content_raw: Some("Raw content".to_string()),
            content_full: Some("Full content".to_string()),
            summary: Some("Summary".to_string()),
            image_url: None,
            published_at: Some("2024-01-01T10:00:00Z".to_string()),
            processed_at: Some("2024-01-01T11:00:00Z".to_string()),
            status: "illuminated".to_string(),
            political_bias: Some(0),
            sachlichkeit: Some(3),
            quality_score: Some(4),
            has_changes: true,
            changed_at: Some("2024-01-02T10:00:00Z".to_string()),
            revision_count: 2,
            categories: vec![FnordCategoryInfo {
                name: "Politik".to_string(),
                color: Some("#ff0000".to_string()),
                icon: Some("fa-landmark".to_string()),
            }],
            full_text_fetch_error: Some("404".to_string()),
        };

        let json = serde_json::to_string(&fnord).expect("Serialization failed");
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"status\":\"illuminated\""));
        assert!(json.contains("\"has_changes\":true"));
        assert!(json.contains("\"revision_count\":2"));
        assert!(json.contains("\"categories\""));
        assert!(json.contains("\"full_text_fetch_error\":\"404\""));
    }

    // ============================================================
    // FnordFilter struct tests
    // ============================================================

    #[test]
    fn test_fnord_filter_deserialization() {
        let json = r#"{
            "pentacle_id": 1,
            "status": "concealed",
            "limit": 50,
            "offset": 0
        }"#;

        let filter: FnordFilter = serde_json::from_str(json).expect("Deserialization failed");

        assert_eq!(filter.pentacle_id, Some(1));
        assert_eq!(filter.status, Some("concealed".to_string()));
        assert_eq!(filter.limit, Some(50));
        assert_eq!(filter.offset, Some(0));
        assert!(filter.sephiroth_id.is_none());
    }

    #[test]
    fn test_fnord_filter_empty() {
        let json = "{}";

        let filter: FnordFilter = serde_json::from_str(json).expect("Deserialization failed");

        assert!(filter.pentacle_id.is_none());
        assert!(filter.status.is_none());
        assert!(filter.limit.is_none());
        assert!(filter.offset.is_none());
    }
}
