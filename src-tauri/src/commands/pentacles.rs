use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Pentacle {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_url: Option<String>,
    pub icon_url: Option<String>,
    pub default_quality: i32,
    pub article_count: i64,
    pub unread_count: i64,
    pub illuminated_count: i64,
    pub golden_apple_count: i64,
}

#[tauri::command]
pub fn get_pentacles(state: State<AppState>) -> Result<Vec<Pentacle>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                p.id,
                p.url,
                p.title,
                p.description,
                p.site_url,
                p.icon_url,
                p.default_quality,
                COUNT(f.id) as article_count,
                COUNT(CASE WHEN f.status = 'concealed' THEN 1 END) as unread_count,
                COUNT(CASE WHEN f.status = 'illuminated' THEN 1 END) as illuminated_count,
                COUNT(CASE WHEN f.status = 'golden_apple' THEN 1 END) as golden_apple_count
            FROM pentacles p
            LEFT JOIN fnords f ON f.pentacle_id = p.id
            GROUP BY p.id
            ORDER BY p.title COLLATE NOCASE
            "#,
        )
        .map_err(|e| e.to_string())?;

    let pentacles = stmt
        .query_map([], |row| {
            Ok(Pentacle {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                site_url: row.get(4)?,
                icon_url: row.get(5)?,
                default_quality: row.get(6)?,
                article_count: row.get(7)?,
                unread_count: row.get(8)?,
                illuminated_count: row.get(9)?,
                golden_apple_count: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(pentacles)
}

#[tauri::command]
pub fn add_pentacle(state: State<AppState>, url: String, title: Option<String>) -> Result<Pentacle, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
            (&url, &title),
        )
        .map_err(|e| e.to_string())?;

    let id = db.conn().last_insert_rowid();

    Ok(Pentacle {
        id,
        url,
        title,
        description: None,
        site_url: None,
        icon_url: None,
        default_quality: 3,
        article_count: 0,
        unread_count: 0,
        illuminated_count: 0,
        golden_apple_count: 0,
    })
}

#[tauri::command]
pub fn delete_pentacle(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM pentacles WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    /// Helper to create a test database
    fn setup_test_db() -> Database {
        Database::new_in_memory().expect("Failed to create in-memory database")
    }

    // ============================================================
    // get_pentacles tests
    // ============================================================

    #[test]
    fn test_get_pentacles_empty_database() {
        let db = setup_test_db();

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM pentacles", [], |row| row.get(0))
            .expect("Failed to count pentacles");

        assert_eq!(count, 0, "Empty database should have no pentacles");
    }

    #[test]
    fn test_get_pentacles_returns_all_feeds() {
        let db = setup_test_db();

        // Insert multiple pentacles
        for i in 1..=3 {
            db.conn()
                .execute(
                    "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                    [format!("https://example{}.com/feed.xml", i), format!("Feed {}", i)],
                )
                .expect("Failed to insert pentacle");
        }

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM pentacles", [], |row| row.get(0))
            .expect("Failed to count pentacles");

        assert_eq!(count, 3, "Should have 3 pentacles");
    }

    #[test]
    fn test_get_pentacles_includes_article_counts() {
        let db = setup_test_db();

        // Insert a pentacle
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://example.com/feed.xml", "Test Feed"],
            )
            .expect("Failed to insert pentacle");

        let pentacle_id: i64 = db
            .conn()
            .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get pentacle id");

        // Insert fnords with different statuses
        for (i, status) in ["concealed", "concealed", "illuminated", "golden_apple"].iter().enumerate() {
            db.conn()
                .execute(
                    r#"INSERT INTO fnords (pentacle_id, guid, url, title, status)
                       VALUES (?1, ?2, ?3, ?4, ?5)"#,
                    rusqlite::params![pentacle_id, format!("guid-{}", i), format!("https://example.com/{}", i), format!("Article {}", i), status],
                )
                .expect("Failed to insert fnord");
        }

        // Query with counts (simulating the get_pentacles query)
        let result: (i64, i64, i64, i64) = db
            .conn()
            .query_row(
                r#"
                SELECT
                    COUNT(f.id) as article_count,
                    COUNT(CASE WHEN f.status = 'concealed' THEN 1 END) as unread_count,
                    COUNT(CASE WHEN f.status = 'illuminated' THEN 1 END) as illuminated_count,
                    COUNT(CASE WHEN f.status = 'golden_apple' THEN 1 END) as golden_apple_count
                FROM pentacles p
                LEFT JOIN fnords f ON f.pentacle_id = p.id
                WHERE p.id = ?1
                "#,
                [pentacle_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("Failed to query counts");

        let (article_count, unread_count, illuminated_count, golden_apple_count) = result;

        assert_eq!(article_count, 4, "Should have 4 articles");
        assert_eq!(unread_count, 2, "Should have 2 unread (concealed)");
        assert_eq!(illuminated_count, 1, "Should have 1 read (illuminated)");
        assert_eq!(golden_apple_count, 1, "Should have 1 favorite (golden_apple)");
    }

    #[test]
    fn test_get_pentacles_ordered_by_title() {
        let db = setup_test_db();

        // Insert pentacles in non-alphabetical order
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://z.com/feed.xml", "Zebra News"],
            )
            .expect("Failed to insert pentacle");

        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://a.com/feed.xml", "Alpha Feed"],
            )
            .expect("Failed to insert pentacle");

        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://m.com/feed.xml", "Middle Feed"],
            )
            .expect("Failed to insert pentacle");

        // Query ordered by title (case-insensitive)
        let mut stmt = db
            .conn()
            .prepare("SELECT title FROM pentacles ORDER BY title COLLATE NOCASE")
            .expect("Failed to prepare statement");

        let titles: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(titles[0], "Alpha Feed", "First should be Alpha Feed");
        assert_eq!(titles[1], "Middle Feed", "Second should be Middle Feed");
        assert_eq!(titles[2], "Zebra News", "Third should be Zebra News");
    }

    // ============================================================
    // add_pentacle tests
    // ============================================================

    #[test]
    fn test_add_pentacle_basic() {
        let db = setup_test_db();

        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://example.com/feed.xml", "Test Feed"],
            )
            .expect("Failed to insert pentacle");

        let id = db.conn().last_insert_rowid();

        let (url, title): (String, String) = db
            .conn()
            .query_row(
                "SELECT url, title FROM pentacles WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("Failed to query pentacle");

        assert_eq!(url, "https://example.com/feed.xml");
        assert_eq!(title, "Test Feed");
    }

    #[test]
    fn test_add_pentacle_with_null_title() {
        let db = setup_test_db();

        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                rusqlite::params!["https://example.com/feed.xml", None::<String>],
            )
            .expect("Failed to insert pentacle");

        let id = db.conn().last_insert_rowid();

        let title: Option<String> = db
            .conn()
            .query_row(
                "SELECT title FROM pentacles WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("Failed to query pentacle");

        assert!(title.is_none(), "Title should be NULL");
    }

    #[test]
    fn test_add_pentacle_returns_correct_id() {
        let db = setup_test_db();

        // Insert first pentacle
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://first.com/feed.xml", "First Feed"],
            )
            .expect("Failed to insert pentacle");
        let first_id = db.conn().last_insert_rowid();

        // Insert second pentacle
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://second.com/feed.xml", "Second Feed"],
            )
            .expect("Failed to insert pentacle");
        let second_id = db.conn().last_insert_rowid();

        assert_eq!(first_id, 1, "First pentacle should have id 1");
        assert_eq!(second_id, 2, "Second pentacle should have id 2");
    }

    #[test]
    fn test_add_pentacle_default_quality() {
        let db = setup_test_db();

        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://example.com/feed.xml", "Test Feed"],
            )
            .expect("Failed to insert pentacle");

        let id = db.conn().last_insert_rowid();

        let default_quality: i32 = db
            .conn()
            .query_row(
                "SELECT default_quality FROM pentacles WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("Failed to query default_quality");

        assert_eq!(default_quality, 3, "Default quality should be 3");
    }

    #[test]
    fn test_add_pentacle_duplicate_url_rejected() {
        let db = setup_test_db();
        let url = "https://example.com/feed.xml";

        // Insert first pentacle
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                [url, "Feed One"],
            )
            .expect("Failed to insert first pentacle");

        // Insert duplicate URL (should fail - UNIQUE constraint on url)
        let result = db.conn().execute(
            "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
            [url, "Feed Two"],
        );

        // The schema has a UNIQUE constraint on the url column
        assert!(result.is_err(), "Duplicate URL should be rejected (UNIQUE constraint)");
    }

    // ============================================================
    // delete_pentacle tests
    // ============================================================

    #[test]
    fn test_delete_pentacle_basic() {
        let db = setup_test_db();

        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://example.com/feed.xml", "Test Feed"],
            )
            .expect("Failed to insert pentacle");

        let id = db.conn().last_insert_rowid();

        // Verify it exists
        let count_before: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM pentacles WHERE id = ?1", [id], |row| row.get(0))
            .expect("Failed to count");
        assert_eq!(count_before, 1, "Pentacle should exist before delete");

        // Delete
        db.conn()
            .execute("DELETE FROM pentacles WHERE id = ?1", [id])
            .expect("Failed to delete pentacle");

        // Verify it's gone
        let count_after: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM pentacles WHERE id = ?1", [id], |row| row.get(0))
            .expect("Failed to count");
        assert_eq!(count_after, 0, "Pentacle should be deleted");
    }

    #[test]
    fn test_delete_pentacle_cascades_to_fnords() {
        let db = setup_test_db();

        // Insert pentacle
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://example.com/feed.xml", "Test Feed"],
            )
            .expect("Failed to insert pentacle");

        let pentacle_id = db.conn().last_insert_rowid();

        // Insert fnords
        for i in 0..3 {
            db.conn()
                .execute(
                    r#"INSERT INTO fnords (pentacle_id, guid, url, title, status)
                       VALUES (?1, ?2, ?3, ?4, ?5)"#,
                    rusqlite::params![pentacle_id, format!("guid-{}", i), format!("https://example.com/{}", i), format!("Article {}", i), "concealed"],
                )
                .expect("Failed to insert fnord");
        }

        // Verify fnords exist
        let fnord_count_before: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE pentacle_id = ?1",
                [pentacle_id],
                |row| row.get(0),
            )
            .expect("Failed to count fnords");
        assert_eq!(fnord_count_before, 3, "Should have 3 fnords before delete");

        // Delete pentacle (should cascade to fnords)
        db.conn()
            .execute("DELETE FROM pentacles WHERE id = ?1", [pentacle_id])
            .expect("Failed to delete pentacle");

        // Verify fnords are also deleted
        let fnord_count_after: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM fnords WHERE pentacle_id = ?1",
                [pentacle_id],
                |row| row.get(0),
            )
            .expect("Failed to count fnords");
        assert_eq!(fnord_count_after, 0, "Fnords should be deleted by cascade");
    }

    #[test]
    fn test_delete_pentacle_nonexistent() {
        let db = setup_test_db();

        // Delete non-existent pentacle (should succeed with 0 rows affected)
        let affected = db
            .conn()
            .execute("DELETE FROM pentacles WHERE id = 9999", [])
            .expect("Failed to execute delete");

        assert_eq!(affected, 0, "Deleting non-existent pentacle should affect 0 rows");
    }

    #[test]
    fn test_delete_pentacle_only_affects_target() {
        let db = setup_test_db();

        // Insert multiple pentacles
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://first.com/feed.xml", "First Feed"],
            )
            .expect("Failed to insert first pentacle");
        let first_id = db.conn().last_insert_rowid();

        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://second.com/feed.xml", "Second Feed"],
            )
            .expect("Failed to insert second pentacle");

        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
                ["https://third.com/feed.xml", "Third Feed"],
            )
            .expect("Failed to insert third pentacle");

        // Delete only the first
        db.conn()
            .execute("DELETE FROM pentacles WHERE id = ?1", [first_id])
            .expect("Failed to delete pentacle");

        // Verify count
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM pentacles", [], |row| row.get(0))
            .expect("Failed to count");

        assert_eq!(count, 2, "Should have 2 remaining pentacles");
    }

    // ============================================================
    // Pentacle struct tests
    // ============================================================

    #[test]
    fn test_pentacle_struct_serialization() {
        let pentacle = Pentacle {
            id: 1,
            url: "https://example.com/feed.xml".to_string(),
            title: Some("Test Feed".to_string()),
            description: Some("A test feed".to_string()),
            site_url: Some("https://example.com".to_string()),
            icon_url: Some("https://example.com/favicon.ico".to_string()),
            default_quality: 4,
            article_count: 100,
            unread_count: 25,
            illuminated_count: 70,
            golden_apple_count: 5,
        };

        let json = serde_json::to_string(&pentacle).expect("Serialization failed");
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"url\":\"https://example.com/feed.xml\""));
        assert!(json.contains("\"title\":\"Test Feed\""));
        assert!(json.contains("\"article_count\":100"));
        assert!(json.contains("\"unread_count\":25"));
    }

    #[test]
    fn test_pentacle_struct_deserialization() {
        let json = r#"{
            "id": 1,
            "url": "https://example.com/feed.xml",
            "title": "Test Feed",
            "description": null,
            "site_url": null,
            "icon_url": null,
            "default_quality": 3,
            "article_count": 50,
            "unread_count": 10,
            "illuminated_count": 35,
            "golden_apple_count": 5
        }"#;

        let pentacle: Pentacle = serde_json::from_str(json).expect("Deserialization failed");

        assert_eq!(pentacle.id, 1);
        assert_eq!(pentacle.url, "https://example.com/feed.xml");
        assert_eq!(pentacle.title, Some("Test Feed".to_string()));
        assert!(pentacle.description.is_none());
        assert_eq!(pentacle.article_count, 50);
    }

    #[test]
    fn test_pentacle_struct_with_optional_fields() {
        let pentacle = Pentacle {
            id: 1,
            url: "https://example.com/feed.xml".to_string(),
            title: None,
            description: None,
            site_url: None,
            icon_url: None,
            default_quality: 3,
            article_count: 0,
            unread_count: 0,
            illuminated_count: 0,
            golden_apple_count: 0,
        };

        assert!(pentacle.title.is_none());
        assert!(pentacle.description.is_none());
        assert!(pentacle.site_url.is_none());
        assert!(pentacle.icon_url.is_none());
    }
}
