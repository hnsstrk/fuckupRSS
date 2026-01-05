use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Sephiroth - Category from the Tree of Life
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sephiroth {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub article_count: i64,
}

/// Category assignment for an article with confidence score
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleCategory {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub confidence: f64,
}

/// Get all available categories (Sephiroth)
#[tauri::command]
pub fn get_all_categories(state: State<AppState>) -> Result<Vec<Sephiroth>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, name, description, color, icon, article_count
            FROM sephiroth
            ORDER BY name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([], |row| {
            Ok(Sephiroth {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                color: row.get(3)?,
                icon: row.get(4)?,
                article_count: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

/// Get categories assigned to an article
#[tauri::command]
pub fn get_article_categories(
    state: State<AppState>,
    fnord_id: i64,
) -> Result<Vec<ArticleCategory>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT s.id, s.name, s.icon, s.color, fs.confidence
            FROM fnord_sephiroth fs
            JOIN sephiroth s ON s.id = fs.sephiroth_id
            WHERE fs.fnord_id = ?
            ORDER BY fs.confidence DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([fnord_id], |row| {
            Ok(ArticleCategory {
                sephiroth_id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                confidence: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

/// Set categories for an article (replaces existing)
#[tauri::command]
pub fn set_article_categories(
    state: State<AppState>,
    fnord_id: i64,
    category_names: Vec<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // First, delete existing category assignments
    db.conn()
        .execute("DELETE FROM fnord_sephiroth WHERE fnord_id = ?", [fnord_id])
        .map_err(|e| e.to_string())?;

    // Then insert new assignments
    for name in &category_names {
        // Find category by name (case-insensitive)
        let sephiroth_id: Option<i64> = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?)",
                [name],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = sephiroth_id {
            db.conn()
                .execute(
                    "INSERT OR IGNORE INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence) VALUES (?, ?, 1.0)",
                    rusqlite::params![fnord_id, id],
                )
                .map_err(|e| e.to_string())?;

            // Update article count
            db.conn()
                .execute(
                    "UPDATE sephiroth SET article_count = (SELECT COUNT(*) FROM fnord_sephiroth WHERE sephiroth_id = ?) WHERE id = ?",
                    rusqlite::params![id, id],
                )
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
