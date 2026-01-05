use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Immanentize - Keyword/Tag
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub last_used: Option<String>,
}

/// Get all tags, optionally limited to top N by count
#[tauri::command]
pub fn get_all_tags(state: State<AppState>, limit: Option<i64>) -> Result<Vec<Tag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let limit = limit.unwrap_or(100);

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, name, count, last_used
            FROM immanentize
            ORDER BY count DESC, name ASC
            LIMIT ?
            "#,
        )
        .map_err(|e| e.to_string())?;

    let tags = stmt
        .query_map([limit], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get(2)?,
                last_used: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(tags)
}

/// Get tags for a specific article
#[tauri::command]
pub fn get_article_tags(state: State<AppState>, fnord_id: i64) -> Result<Vec<Tag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT i.id, i.name, i.count, i.last_used
            FROM fnord_immanentize fi
            JOIN immanentize i ON i.id = fi.immanentize_id
            WHERE fi.fnord_id = ?
            ORDER BY i.name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let tags = stmt
        .query_map([fnord_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get(2)?,
                last_used: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(tags)
}

/// Add a tag to an article (creates tag if it doesn't exist)
#[tauri::command]
pub fn add_article_tag(
    state: State<AppState>,
    fnord_id: i64,
    tag_name: String,
) -> Result<Tag, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let tag_name = tag_name.trim().to_string();
    if tag_name.is_empty() {
        return Err("Tag name cannot be empty".to_string());
    }

    // Upsert the tag
    db.conn()
        .execute(
            r#"
        INSERT INTO immanentize (name, count, last_used)
        VALUES (?, 1, CURRENT_TIMESTAMP)
        ON CONFLICT(name) DO UPDATE SET
            count = count + 1,
            last_used = CURRENT_TIMESTAMP
        "#,
            [&tag_name],
        )
        .map_err(|e| e.to_string())?;

    // Get the tag ID
    let tag: Tag = db
        .conn()
        .query_row(
            "SELECT id, name, count, last_used FROM immanentize WHERE name = ?",
            [&tag_name],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    count: row.get(2)?,
                    last_used: row.get(3)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    // Link tag to article
    db.conn()
        .execute(
            "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id) VALUES (?, ?)",
            rusqlite::params![fnord_id, tag.id],
        )
        .map_err(|e| e.to_string())?;

    Ok(tag)
}

/// Remove a tag from an article
#[tauri::command]
pub fn remove_article_tag(
    state: State<AppState>,
    fnord_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Remove the link
    let deleted = db
        .conn()
        .execute(
            "DELETE FROM fnord_immanentize WHERE fnord_id = ? AND immanentize_id = ?",
            rusqlite::params![fnord_id, tag_id],
        )
        .map_err(|e| e.to_string())?;

    if deleted > 0 {
        // Decrement the count
        db.conn()
            .execute(
                "UPDATE immanentize SET count = MAX(0, count - 1) WHERE id = ?",
                [tag_id],
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Set multiple tags for an article (replaces existing)
#[tauri::command]
pub fn set_article_tags(
    state: State<AppState>,
    fnord_id: i64,
    tag_names: Vec<String>,
) -> Result<Vec<Tag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get existing tags to decrement their counts
    let existing_tag_ids: Vec<i64> = {
        let mut stmt = db
            .conn()
            .prepare("SELECT immanentize_id FROM fnord_immanentize WHERE fnord_id = ?")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([fnord_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    // Decrement counts for removed tags
    for tag_id in existing_tag_ids {
        db.conn()
            .execute(
                "UPDATE immanentize SET count = MAX(0, count - 1) WHERE id = ?",
                [tag_id],
            )
            .map_err(|e| e.to_string())?;
    }

    // Delete existing links
    db.conn()
        .execute(
            "DELETE FROM fnord_immanentize WHERE fnord_id = ?",
            [fnord_id],
        )
        .map_err(|e| e.to_string())?;

    // Add new tags
    let mut result_tags = Vec::new();
    for name in tag_names {
        let tag_name = name.trim().to_string();
        if tag_name.is_empty() {
            continue;
        }

        // Upsert the tag
        db.conn()
            .execute(
                r#"
            INSERT INTO immanentize (name, count, last_used)
            VALUES (?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(name) DO UPDATE SET
                count = count + 1,
                last_used = CURRENT_TIMESTAMP
            "#,
                [&tag_name],
            )
            .map_err(|e| e.to_string())?;

        // Get the tag
        let tag: Tag = db
            .conn()
            .query_row(
                "SELECT id, name, count, last_used FROM immanentize WHERE name = ?",
                [&tag_name],
                |row| {
                    Ok(Tag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        count: row.get(2)?,
                        last_used: row.get(3)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;

        // Link to article
        db.conn()
            .execute(
                "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id) VALUES (?, ?)",
                rusqlite::params![fnord_id, tag.id],
            )
            .map_err(|e| e.to_string())?;

        result_tags.push(tag);
    }

    Ok(result_tags)
}
