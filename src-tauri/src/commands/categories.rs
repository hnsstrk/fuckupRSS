use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Sephiroth - Category from the Tree of Life (hierarchical)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sephiroth {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub level: i64,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub article_count: i64,
}

/// Main category (level 0) with aggregated stats
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MainCategory {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub article_count: i64,
    pub read_count: i64,
    pub percentage: f64,
    pub subcategories: Vec<SubCategory>,
}

/// Subcategory (level 1) with individual stats
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubCategory {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub parent_id: i64,
    pub article_count: i64,
    pub read_count: i64,
    pub percentage: f64,
}

/// Category assignment for an article with confidence score
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleCategory {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub confidence: f64,
    pub source: String,
    pub assigned_at: Option<String>,
    pub parent_id: Option<i64>,
    pub main_category_name: Option<String>,
    pub main_category_color: Option<String>,
}

/// Get all available categories (Sephiroth) - both main and sub
#[tauri::command]
pub fn get_all_categories(state: State<AppState>) -> Result<Vec<Sephiroth>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, name, parent_id, level, description, color, icon, article_count
            FROM sephiroth
            ORDER BY level, parent_id, name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([], |row| {
            Ok(Sephiroth {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                level: row.get(3)?,
                description: row.get(4)?,
                color: row.get(5)?,
                icon: row.get(6)?,
                article_count: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

/// Get main categories (level 0) for sidebar display
#[tauri::command]
pub fn get_main_categories(state: State<AppState>) -> Result<Vec<Sephiroth>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                m.id, m.name, m.parent_id, m.level, m.description, m.color, m.icon,
                COALESCE(SUM(s.article_count), 0) as article_count
            FROM sephiroth m
            LEFT JOIN sephiroth s ON s.parent_id = m.id
            WHERE m.level = 0
            GROUP BY m.id
            ORDER BY m.id
            "#,
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([], |row| {
            Ok(Sephiroth {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                level: row.get(3)?,
                description: row.get(4)?,
                color: row.get(5)?,
                icon: row.get(6)?,
                article_count: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

/// Get subcategories (level 1) for KI classification
#[tauri::command]
pub fn get_subcategories(state: State<AppState>) -> Result<Vec<Sephiroth>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, name, parent_id, level, description, color, icon, article_count
            FROM sephiroth
            WHERE level = 1
            ORDER BY parent_id, name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([], |row| {
            Ok(Sephiroth {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                level: row.get(3)?,
                description: row.get(4)?,
                color: row.get(5)?,
                icon: row.get(6)?,
                article_count: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

/// Get main categories with full hierarchy and stats for Operation Mindfuck
#[tauri::command]
pub fn get_categories_with_stats(state: State<AppState>) -> Result<Vec<MainCategory>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get main categories
    let mut main_stmt = db
        .conn()
        .prepare(
            r#"
            SELECT m.id, m.name, m.icon, m.color
            FROM sephiroth m
            WHERE m.level = 0
            ORDER BY m.id
            "#,
        )
        .map_err(|e| e.to_string())?;

    let main_cats: Vec<(i64, String, Option<String>, Option<String>)> = main_stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();

    for (main_id, main_name, main_icon, main_color) in main_cats {
        // Get subcategories with stats
        let mut sub_stmt = db
            .conn()
            .prepare(
                r#"
                SELECT
                    sub.id,
                    sub.name,
                    sub.icon,
                    sub.parent_id,
                    COUNT(DISTINCT fs.fnord_id) as article_count,
                    COUNT(DISTINCT CASE WHEN f.read_at IS NOT NULL THEN f.id END) as read_count
                FROM sephiroth sub
                LEFT JOIN fnord_sephiroth fs ON fs.sephiroth_id = sub.id
                LEFT JOIN fnords f ON f.id = fs.fnord_id
                WHERE sub.parent_id = ?
                GROUP BY sub.id
                ORDER BY sub.name
                "#,
            )
            .map_err(|e| e.to_string())?;

        let subcategories: Vec<SubCategory> = sub_stmt
            .query_map([main_id], |row| {
                let article_count: i64 = row.get(4)?;
                let read_count: i64 = row.get(5)?;
                let percentage = if article_count > 0 {
                    (read_count as f64 / article_count as f64) * 100.0
                } else {
                    0.0
                };
                Ok(SubCategory {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    parent_id: row.get(3)?,
                    article_count,
                    read_count,
                    percentage,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        // Aggregate stats for main category
        let total_articles: i64 = subcategories.iter().map(|s| s.article_count).sum();
        let total_read: i64 = subcategories.iter().map(|s| s.read_count).sum();
        let percentage = if total_articles > 0 {
            (total_read as f64 / total_articles as f64) * 100.0
        } else {
            0.0
        };

        result.push(MainCategory {
            id: main_id,
            name: main_name,
            icon: main_icon,
            color: main_color,
            article_count: total_articles,
            read_count: total_read,
            percentage,
            subcategories,
        });
    }

    Ok(result)
}

/// Get categories assigned to an article (returns subcategories with parent info)
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
            SELECT
                s.id, s.name, s.icon, s.color,
                fs.confidence, COALESCE(fs.source, 'ai') as source, fs.assigned_at,
                s.parent_id,
                m.name as main_name,
                m.color as main_color
            FROM fnord_sephiroth fs
            JOIN sephiroth s ON s.id = fs.sephiroth_id
            LEFT JOIN sephiroth m ON m.id = s.parent_id
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
                source: row.get(5)?,
                assigned_at: row.get(6)?,
                parent_id: row.get(7)?,
                main_category_name: row.get(8)?,
                main_category_color: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

/// Set categories for an article (replaces existing) - manual assignment
/// Now expects subcategory names (level 1)
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

    // Then insert new assignments with source='manual'
    for name in &category_names {
        // Find subcategory by name (case-insensitive, level 1 only)
        let sephiroth_id: Option<i64> = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?) AND level = 1",
                [name],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = sephiroth_id {
            db.conn()
                .execute(
                    r#"INSERT OR IGNORE INTO fnord_sephiroth
                       (fnord_id, sephiroth_id, confidence, source, assigned_at)
                       VALUES (?, ?, 1.0, 'manual', CURRENT_TIMESTAMP)"#,
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

/// Get subcategory names for KI prompt
#[tauri::command]
pub fn get_subcategory_names(state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare("SELECT name FROM sephiroth WHERE level = 1 ORDER BY name")
        .map_err(|e| e.to_string())?;

    let names = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(names)
}
