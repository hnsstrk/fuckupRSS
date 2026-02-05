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
    let db = state.db_conn()?;

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
    let db = state.db_conn()?;

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
    let db = state.db_conn()?;

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
    let db = state.db_conn()?;

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
    let db = state.db_conn()?;

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
    let db = state.db_conn()?;

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
    let db = state.db_conn()?;

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

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    /// Helper to create a test database with seeded categories
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
    fn insert_test_fnord(conn: &rusqlite::Connection, pentacle_id: i64) -> i64 {
        conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, status)
               VALUES (?1, ?2, ?3, ?4, ?5)"#,
            rusqlite::params![pentacle_id, "guid-test", "https://example.com/article", "Test Article", "concealed"],
        )
        .expect("Failed to insert test fnord");
        conn.last_insert_rowid()
    }

    // ============================================================
    // get_all_categories tests
    // ============================================================

    #[test]
    fn test_get_all_categories_returns_seeded() {
        let db = setup_test_db();

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM sephiroth", [], |row| row.get(0))
            .expect("Failed to count categories");

        // Database is seeded with 6 main categories + 13 subcategories = 19 total
        assert_eq!(count, 19, "Should have 19 seeded categories (6 main + 13 sub)");
    }

    #[test]
    fn test_get_all_categories_has_hierarchical_structure() {
        let db = setup_test_db();

        // Check main categories (level 0)
        let main_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sephiroth WHERE level = 0",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count main categories");

        // Check subcategories (level 1)
        let sub_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sephiroth WHERE level = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count subcategories");

        assert_eq!(main_count, 6, "Should have 6 main categories");
        assert_eq!(sub_count, 13, "Should have 13 subcategories");
    }

    #[test]
    fn test_get_all_categories_subcategories_have_parent() {
        let db = setup_test_db();

        // All subcategories should have a valid parent_id
        let orphan_count: i64 = db
            .conn()
            .query_row(
                r#"SELECT COUNT(*) FROM sephiroth
                   WHERE level = 1 AND parent_id IS NULL"#,
                [],
                |row| row.get(0),
            )
            .expect("Failed to count orphan subcategories");

        assert_eq!(orphan_count, 0, "All subcategories should have a parent");
    }

    // ============================================================
    // get_main_categories tests
    // ============================================================

    #[test]
    fn test_get_main_categories_only_level_0() {
        let db = setup_test_db();

        let mut stmt = db
            .conn()
            .prepare("SELECT id, name, level FROM sephiroth WHERE level = 0")
            .expect("Failed to prepare statement");

        let main_cats: Vec<(i64, String, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(main_cats.len(), 6, "Should have 6 main categories");

        for (_, _, level) in &main_cats {
            assert_eq!(*level, 0, "All returned categories should be level 0");
        }
    }

    #[test]
    fn test_get_main_categories_expected_names() {
        let db = setup_test_db();

        let mut stmt = db
            .conn()
            .prepare("SELECT name FROM sephiroth WHERE level = 0 ORDER BY name")
            .expect("Failed to prepare statement");

        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        // Main categories as defined in schema.rs (in alphabetical order)
        let expected = [
            "Kultur & Leben",
            "Politik & Gesellschaft",
            "Sicherheit",
            "Umwelt & Gesundheit",
            "Wirtschaft",
            "Wissen & Technologie",
        ];

        for expected_name in expected {
            assert!(
                names.contains(&expected_name.to_string()),
                "Should contain main category '{}'",
                expected_name
            );
        }
    }

    // ============================================================
    // get_subcategories tests
    // ============================================================

    #[test]
    fn test_get_subcategories_only_level_1() {
        let db = setup_test_db();

        let mut stmt = db
            .conn()
            .prepare("SELECT level FROM sephiroth WHERE level = 1")
            .expect("Failed to prepare statement");

        let levels: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        for level in &levels {
            assert_eq!(*level, 1, "All subcategories should be level 1");
        }
    }

    #[test]
    fn test_get_subcategories_includes_expected_names() {
        let db = setup_test_db();

        // These subcategories should exist in the seeded data
        let expected_subcats = [
            "Technik",       // under Technik main
            "Politik",       // under Politik main
            "Wirtschaft",    // under Wirtschaft main
            "Wissenschaft",  // under Wissenschaft main
            "Kultur",
            "Sport",
            "Gesellschaft",
            "Umwelt",
            "Sicherheit",
            "Gesundheit",
            "Verteidigung",
            "Energie",
            "Recht",
        ];

        for name in expected_subcats {
            let exists: i64 = db
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM sephiroth WHERE name = ?1 AND level = 1",
                    [name],
                    |row| row.get(0),
                )
                .expect("Failed to query");

            assert!(exists >= 1, "Subcategory '{}' should exist", name);
        }
    }

    // ============================================================
    // get_article_categories tests
    // ============================================================

    #[test]
    fn test_get_article_categories_empty() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id);

        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnord_sephiroth WHERE fnord_id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to count categories");

        assert_eq!(count, 0, "New fnord should have no categories");
    }

    #[test]
    fn test_get_article_categories_returns_assigned() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id);

        // Get a subcategory ID
        let sephiroth_id: i64 = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE name = 'Politik' AND level = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to get sephiroth id");

        // Assign category to fnord
        db.conn()
            .execute(
                r#"INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source)
                   VALUES (?1, ?2, ?3, ?4)"#,
                rusqlite::params![fnord_id, sephiroth_id, 0.9, "ai"],
            )
            .expect("Failed to assign category");

        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnord_sephiroth WHERE fnord_id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to count categories");

        assert_eq!(count, 1, "Should have 1 assigned category");
    }

    #[test]
    fn test_get_article_categories_with_confidence() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id);

        // Get subcategory IDs
        let politik_id: i64 = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE name = 'Politik' AND level = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to get Politik id");

        let technik_id: i64 = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE name = 'Technik' AND level = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to get Technik id");

        // Assign categories with different confidence levels
        db.conn()
            .execute(
                r#"INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source)
                   VALUES (?1, ?2, ?3, ?4)"#,
                rusqlite::params![fnord_id, politik_id, 0.95, "ai"],
            )
            .expect("Failed to assign Politik");

        db.conn()
            .execute(
                r#"INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source)
                   VALUES (?1, ?2, ?3, ?4)"#,
                rusqlite::params![fnord_id, technik_id, 0.75, "ai"],
            )
            .expect("Failed to assign Technik");

        // Query ordered by confidence DESC (simulating the command)
        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT s.name, fs.confidence FROM fnord_sephiroth fs
                   JOIN sephiroth s ON s.id = fs.sephiroth_id
                   WHERE fs.fnord_id = ?1
                   ORDER BY fs.confidence DESC"#,
            )
            .expect("Failed to prepare statement");

        let results: Vec<(String, f64)> = stmt
            .query_map([fnord_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(results.len(), 2, "Should have 2 categories");
        assert_eq!(results[0].0, "Politik", "First should be Politik (higher confidence)");
        assert_eq!(results[0].1, 0.95, "Politik confidence should be 0.95");
        assert_eq!(results[1].0, "Technik", "Second should be Technik");
    }

    // ============================================================
    // set_article_categories tests
    // ============================================================

    #[test]
    fn test_set_article_categories_replaces_existing() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id);

        // Get subcategory IDs
        let politik_id: i64 = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE name = 'Politik' AND level = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to get Politik id");

        let technik_id: i64 = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE name = 'Technik' AND level = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to get Technik id");

        // Set initial category (Politik)
        db.conn()
            .execute(
                r#"INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source)
                   VALUES (?1, ?2, ?3, ?4)"#,
                rusqlite::params![fnord_id, politik_id, 0.9, "ai"],
            )
            .expect("Failed to assign initial category");

        // Verify initial state
        let initial_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnord_sephiroth WHERE fnord_id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to count");
        assert_eq!(initial_count, 1);

        // Simulate set_article_categories: delete existing, add new
        db.conn()
            .execute("DELETE FROM fnord_sephiroth WHERE fnord_id = ?1", [fnord_id])
            .expect("Failed to delete");

        db.conn()
            .execute(
                r#"INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source)
                   VALUES (?1, ?2, ?3, ?4)"#,
                rusqlite::params![fnord_id, technik_id, 1.0, "manual"],
            )
            .expect("Failed to add new category");

        // Verify only new category exists
        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT s.name FROM fnord_sephiroth fs
                   JOIN sephiroth s ON s.id = fs.sephiroth_id
                   WHERE fs.fnord_id = ?1"#,
            )
            .expect("Failed to prepare");

        let names: Vec<String> = stmt
            .query_map([fnord_id], |row| row.get(0))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(names.len(), 1, "Should have exactly 1 category");
        assert_eq!(names[0], "Technik", "Category should be Technik");
    }

    #[test]
    fn test_set_article_categories_manual_source() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id);

        let sephiroth_id: i64 = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE name = 'Politik' AND level = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to get sephiroth id");

        // Simulate manual assignment (as in set_article_categories)
        db.conn()
            .execute(
                r#"INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source, assigned_at)
                   VALUES (?1, ?2, 1.0, 'manual', CURRENT_TIMESTAMP)"#,
                rusqlite::params![fnord_id, sephiroth_id],
            )
            .expect("Failed to assign category");

        let source: String = db
            .conn()
            .query_row(
                "SELECT source FROM fnord_sephiroth WHERE fnord_id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .expect("Failed to get source");

        assert_eq!(source, "manual", "Source should be 'manual' for user assignments");
    }

    #[test]
    fn test_set_article_categories_updates_article_count() {
        let db = setup_test_db();
        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        let fnord_id = insert_test_fnord(db.conn(), pentacle_id);

        let sephiroth_id: i64 = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE name = 'Politik' AND level = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to get sephiroth id");

        // Assign category
        db.conn()
            .execute(
                r#"INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source)
                   VALUES (?1, ?2, 1.0, 'manual')"#,
                rusqlite::params![fnord_id, sephiroth_id],
            )
            .expect("Failed to assign category");

        // Update article_count (as done in set_article_categories)
        db.conn()
            .execute(
                r#"UPDATE sephiroth SET article_count =
                   (SELECT COUNT(*) FROM fnord_sephiroth WHERE sephiroth_id = ?1)
                   WHERE id = ?1"#,
                [sephiroth_id],
            )
            .expect("Failed to update article_count");

        let article_count: i64 = db
            .conn()
            .query_row(
                "SELECT article_count FROM sephiroth WHERE id = ?1",
                [sephiroth_id],
                |row| row.get(0),
            )
            .expect("Failed to get article_count");

        assert_eq!(article_count, 1, "Article count should be 1");
    }

    // ============================================================
    // get_subcategory_names tests
    // ============================================================

    #[test]
    fn test_get_subcategory_names_returns_all() {
        let db = setup_test_db();

        let mut stmt = db
            .conn()
            .prepare("SELECT name FROM sephiroth WHERE level = 1 ORDER BY name")
            .expect("Failed to prepare statement");

        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(names.len(), 13, "Should have 13 subcategory names");
    }

    #[test]
    fn test_get_subcategory_names_sorted_alphabetically() {
        let db = setup_test_db();

        let mut stmt = db
            .conn()
            .prepare("SELECT name FROM sephiroth WHERE level = 1 ORDER BY name")
            .expect("Failed to prepare statement");

        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        // Verify alphabetical order
        let mut sorted_names = names.clone();
        sorted_names.sort();

        assert_eq!(names, sorted_names, "Names should be in alphabetical order");
    }

    // ============================================================
    // Struct serialization tests
    // ============================================================

    #[test]
    fn test_sephiroth_struct_serialization() {
        let sephiroth = Sephiroth {
            id: 1,
            name: "Politik".to_string(),
            parent_id: Some(2),
            level: 1,
            description: Some("Political news".to_string()),
            color: Some("#ff0000".to_string()),
            icon: Some("fa-landmark".to_string()),
            article_count: 42,
        };

        let json = serde_json::to_string(&sephiroth).expect("Serialization failed");
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"name\":\"Politik\""));
        assert!(json.contains("\"parent_id\":2"));
        assert!(json.contains("\"level\":1"));
        assert!(json.contains("\"article_count\":42"));
    }

    #[test]
    fn test_article_category_struct_serialization() {
        let category = ArticleCategory {
            sephiroth_id: 1,
            name: "Politik".to_string(),
            icon: Some("fa-landmark".to_string()),
            color: Some("#ff0000".to_string()),
            confidence: 0.95,
            source: "ai".to_string(),
            assigned_at: Some("2024-01-01T10:00:00Z".to_string()),
            parent_id: Some(2),
            main_category_name: Some("Politik".to_string()),
            main_category_color: Some("#ff0000".to_string()),
        };

        let json = serde_json::to_string(&category).expect("Serialization failed");
        assert!(json.contains("\"sephiroth_id\":1"));
        assert!(json.contains("\"confidence\":0.95"));
        assert!(json.contains("\"source\":\"ai\""));
        assert!(json.contains("\"main_category_name\":\"Politik\""));
    }

    #[test]
    fn test_main_category_struct() {
        let main_cat = MainCategory {
            id: 1,
            name: "Politik".to_string(),
            icon: Some("fa-landmark".to_string()),
            color: Some("#ff0000".to_string()),
            article_count: 100,
            read_count: 75,
            percentage: 75.0,
            subcategories: vec![
                SubCategory {
                    id: 10,
                    name: "Innenpolitik".to_string(),
                    icon: Some("fa-home".to_string()),
                    parent_id: 1,
                    article_count: 50,
                    read_count: 40,
                    percentage: 80.0,
                },
            ],
        };

        assert_eq!(main_cat.article_count, 100);
        assert_eq!(main_cat.percentage, 75.0);
        assert_eq!(main_cat.subcategories.len(), 1);
        assert_eq!(main_cat.subcategories[0].name, "Innenpolitik");
    }

    // ============================================================
    // Category case-insensitive lookup tests
    // ============================================================

    #[test]
    fn test_category_lookup_case_insensitive() {
        let db = setup_test_db();

        // Test case-insensitive lookup (as used in set_article_categories)
        let id_lower: Option<i64> = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?1) AND level = 1",
                ["politik"],
                |row| row.get(0),
            )
            .ok();

        let id_upper: Option<i64> = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?1) AND level = 1",
                ["POLITIK"],
                |row| row.get(0),
            )
            .ok();

        let id_mixed: Option<i64> = db
            .conn()
            .query_row(
                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?1) AND level = 1",
                ["PoLiTiK"],
                |row| row.get(0),
            )
            .ok();

        assert!(id_lower.is_some(), "Should find 'politik'");
        assert!(id_upper.is_some(), "Should find 'POLITIK'");
        assert!(id_mixed.is_some(), "Should find 'PoLiTiK'");
        assert_eq!(id_lower, id_upper, "All variants should find same category");
        assert_eq!(id_lower, id_mixed, "All variants should find same category");
    }
}
