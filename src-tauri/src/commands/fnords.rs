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
    COALESCE(f.revision_count, 0) as revision_count
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

    // By category - count revisions for articles in each subcategory (level 1)
    // Articles are only assigned to subcategories, so we filter by level = 1
    // and get the color from the parent (main) category
    let mut stmt_cat = db
        .conn()
        .prepare(
            r#"
            SELECT s.id, s.name, s.icon, COALESCE(m.color, s.color),
                   COUNT(DISTINCT r.id) as revision_count,
                   COUNT(DISTINCT r.fnord_id) as article_count
            FROM sephiroth s
            LEFT JOIN sephiroth m ON m.id = s.parent_id
            LEFT JOIN fnord_sephiroth fs ON fs.sephiroth_id = s.id
            LEFT JOIN fnord_revisions r ON r.fnord_id = fs.fnord_id
            WHERE s.level = 1
            GROUP BY s.id
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
