use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

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
    pub article_type: Option<String>,
    pub has_changes: bool,
    pub changed_at: Option<String>,
    pub revision_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FnordRevision {
    pub id: i64,
    pub fnord_id: i64,
    pub title: String,
    pub author: Option<String>,
    pub content_raw: Option<String>,
    pub summary: Option<String>,
    pub content_hash: String,
    pub revision_at: String,
}

#[derive(Debug, Deserialize)]
pub struct FnordFilter {
    pub pentacle_id: Option<i64>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[tauri::command]
pub fn get_fnords(state: State<AppState>, filter: Option<FnordFilter>) -> Result<Vec<Fnord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let filter = filter.unwrap_or(FnordFilter {
        pentacle_id: None,
        status: None,
        limit: Some(100),
    });

    let mut sql = String::from(
        r#"
        SELECT
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
            f.article_type,
            COALESCE(f.has_changes, FALSE) as has_changes,
            f.changed_at,
            COALESCE(f.revision_count, 0) as revision_count
        FROM fnords f
        LEFT JOIN pentacles p ON p.id = f.pentacle_id
        WHERE 1=1
        "#,
    );

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(pentacle_id) = filter.pentacle_id {
        sql.push_str(" AND f.pentacle_id = ?");
        params.push(Box::new(pentacle_id));
    }

    if let Some(status) = &filter.status {
        sql.push_str(" AND f.status = ?");
        params.push(Box::new(status.clone()));
    }

    sql.push_str(" ORDER BY f.published_at DESC");

    if let Some(limit) = filter.limit {
        sql.push_str(&format!(" LIMIT {}", limit));
    }

    let mut stmt = db.conn().prepare(&sql).map_err(|e| e.to_string())?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let fnords = stmt
        .query_map(param_refs.as_slice(), |row| {
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
                article_type: row.get(17)?,
                has_changes: row.get(18)?,
                changed_at: row.get(19)?,
                revision_count: row.get(20)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(fnords)
}

#[tauri::command]
pub fn get_fnord(state: State<AppState>, id: i64) -> Result<Fnord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let fnord = db
        .conn()
        .query_row(
            r#"
            SELECT
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
                f.article_type,
                COALESCE(f.has_changes, FALSE) as has_changes,
                f.changed_at,
                COALESCE(f.revision_count, 0) as revision_count
            FROM fnords f
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.id = ?1
            "#,
            [id],
            |row| {
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
                    article_type: row.get(17)?,
                    has_changes: row.get(18)?,
                    changed_at: row.get(19)?,
                    revision_count: row.get(20)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(fnord)
}

#[tauri::command]
pub fn update_fnord_status(state: State<AppState>, id: i64, status: String) -> Result<(), String> {
    // Validate status
    if !["concealed", "illuminated", "golden_apple"].contains(&status.as_str()) {
        return Err(format!("Invalid status: {}", status));
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;

    let read_at = if status == "illuminated" || status == "golden_apple" {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        None
    };

    db.conn()
        .execute(
            "UPDATE fnords SET status = ?1, read_at = ?2 WHERE id = ?3",
            (&status, &read_at, id),
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get all articles with pending changes (for Fnord view)
#[tauri::command]
pub fn get_changed_fnords(state: State<AppState>) -> Result<Vec<Fnord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
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
                f.article_type,
                f.has_changes,
                f.changed_at,
                COALESCE(f.revision_count, 0) as revision_count
            FROM fnords f
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.has_changes = TRUE
            ORDER BY f.changed_at DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let fnords = stmt
        .query_map([], |row| {
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
                article_type: row.get(17)?,
                has_changes: row.get(18)?,
                changed_at: row.get(19)?,
                revision_count: row.get(20)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(fnords)
}

/// Acknowledge changes for an article (dismiss change notification)
#[tauri::command]
pub fn acknowledge_changes(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "UPDATE fnords SET has_changes = FALSE WHERE id = ?1",
            [id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get revision history for an article
#[tauri::command]
pub fn get_fnord_revisions(state: State<AppState>, fnord_id: i64) -> Result<Vec<FnordRevision>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, fnord_id, title, author, content_raw, summary, content_hash, revision_at
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
                summary: row.get(5)?,
                content_hash: row.get(6)?,
                revision_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(revisions)
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
