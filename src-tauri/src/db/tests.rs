//! Database module tests

use super::*;

#[test]
fn test_database_creation_in_memory() {
    let db = Database::new_in_memory();
    assert!(db.is_ok(), "Failed to create in-memory database");
}

#[test]
fn test_schema_tables_exist() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Check that all expected tables exist
    let tables = [
        "pentacles",
        "fnords",
        "fnord_revisions",
        "sephiroth",
        "fnord_sephiroth",
        "immanentize",
        "fnord_immanentize",
        "immanentize_neighbors",
        "immanentize_sephiroth",
        "settings",
    ];

    for table in tables {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap_or(0);

        assert_eq!(exists, 1, "Table '{}' should exist", table);
    }
}

#[test]
fn test_sephiroth_seeded_with_13_categories() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sephiroth", [], |row| row.get(0))
        .expect("Failed to count sephiroth");

    assert_eq!(
        count, 19,
        "Should have 19 default categories (6 main + 13 sub)"
    );
}

#[test]
fn test_sephiroth_categories_names() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    let expected_categories = [
        "Technik",
        "Politik",
        "Wirtschaft",
        "Wissenschaft",
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

    for category in expected_categories {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sephiroth WHERE name = ?1",
                [category],
                |row| row.get(0),
            )
            .expect("Failed to query sephiroth");

        // With hierarchical categories, some names may exist at multiple levels
        // (e.g., "Wirtschaft" as both main and sub, "Sicherheit" as both main and sub)
        assert!(
            exists >= 1,
            "Category '{}' should exist at least once",
            category
        );
    }
}

#[test]
fn test_insert_pentacle() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    let result = conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    );

    assert!(result.is_ok(), "Failed to insert pentacle");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM pentacles", [], |row| row.get(0))
        .expect("Failed to count pentacles");

    assert_eq!(count, 1, "Should have 1 pentacle");
}

#[test]
fn test_insert_fnord() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // First insert a pentacle (foreign key requirement)
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");

    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    // Insert a fnord
    let result = conn.execute(
        r#"INSERT INTO fnords (pentacle_id, guid, url, title, content_raw, status)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
        (
            pentacle_id,
            "guid-123",
            "https://example.com/article",
            "Test Article",
            "This is test content",
            "concealed",
        ),
    );

    assert!(result.is_ok(), "Failed to insert fnord");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .expect("Failed to count fnords");

    assert_eq!(count, 1, "Should have 1 fnord");
}

#[test]
fn test_fnord_status_enum() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert pentacle
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");

    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    // Test each valid status
    let valid_statuses = ["concealed", "illuminated", "golden_apple"];

    for (i, status) in valid_statuses.iter().enumerate() {
        let result = conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, status)
               VALUES (?1, ?2, ?3, ?4, ?5)"#,
            (
                pentacle_id,
                format!("guid-{}", i),
                format!("https://example.com/article-{}", i),
                format!("Article {}", i),
                status,
            ),
        );
        assert!(result.is_ok(), "Status '{}' should be valid", status);
    }
}

#[test]
fn test_fnord_sephiroth_association() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert pentacle and fnord
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");

    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    conn.execute(
        r#"INSERT INTO fnords (pentacle_id, guid, url, title, status)
           VALUES (?1, ?2, ?3, ?4, ?5)"#,
        (
            pentacle_id,
            "guid-123",
            "https://example.com/article",
            "Test Article",
            "concealed",
        ),
    )
    .expect("Failed to insert fnord");

    let fnord_id: i64 = conn
        .query_row("SELECT id FROM fnords LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get fnord id");

    // Get a sephiroth id
    let sephiroth_id: i64 = conn
        .query_row(
            "SELECT id FROM sephiroth WHERE name = 'Technik'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to get sephiroth id");

    // Associate fnord with sephiroth
    let result = conn.execute(
        r#"INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source)
           VALUES (?1, ?2, ?3, ?4)"#,
        (fnord_id, sephiroth_id, 0.95, "ai"),
    );

    assert!(result.is_ok(), "Failed to associate fnord with sephiroth");
}

#[test]
fn test_immanentize_keyword() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert a keyword (using unique name to avoid collision with seeded keywords)
    let result = conn.execute(
        r#"INSERT INTO immanentize (name, count, article_count)
           VALUES (?1, ?2, ?3)"#,
        ("TestOnlyKeyword_Cybersec", 5, 3),
    );

    assert!(result.is_ok(), "Failed to insert keyword");

    // Verify keyword exists
    let count: i64 = conn
        .query_row(
            "SELECT count FROM immanentize WHERE name = 'TestOnlyKeyword_Cybersec'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to query keyword");

    assert_eq!(count, 5, "Keyword count should be 5");
}

#[test]
fn test_immanentize_neighbors() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert two keywords (using unique names to avoid collision with seeded keywords)
    conn.execute(
        "INSERT INTO immanentize (name) VALUES (?1)",
        ["TestOnlyKeyword_AlphaNode"],
    )
    .expect("Failed to insert keyword 1");

    conn.execute(
        "INSERT INTO immanentize (name) VALUES (?1)",
        ["TestOnlyKeyword_BetaNode"],
    )
    .expect("Failed to insert keyword 2");

    let id_a: i64 = conn
        .query_row(
            "SELECT id FROM immanentize WHERE name = 'TestOnlyKeyword_AlphaNode'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to get id_a");

    let id_b: i64 = conn
        .query_row(
            "SELECT id FROM immanentize WHERE name = 'TestOnlyKeyword_BetaNode'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to get id_b");

    // Create neighbor relationship
    let result = conn.execute(
        r#"INSERT INTO immanentize_neighbors (immanentize_id_a, immanentize_id_b, cooccurrence)
           VALUES (?1, ?2, ?3)"#,
        (id_a, id_b, 10),
    );

    assert!(result.is_ok(), "Failed to create neighbor relationship");
}

#[test]
fn test_settings_crud() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert new setting (use a unique key to avoid conflicts with defaults)
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)",
        ["test_setting", "initial_value"],
    )
    .expect("Failed to insert setting");

    // Read setting
    let value: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'test_setting'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to read setting");

    assert_eq!(
        value, "initial_value",
        "Setting value should be 'initial_value'"
    );

    // Update setting
    conn.execute(
        "UPDATE settings SET value = ?1 WHERE key = ?2",
        ["updated_value", "test_setting"],
    )
    .expect("Failed to update setting");

    let updated_value: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'test_setting'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to read updated setting");

    assert_eq!(
        updated_value, "updated_value",
        "Setting value should be 'updated_value'"
    );

    // Test INSERT OR REPLACE for existing keys (like theme which has a default)
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        ["theme", "mocha"],
    )
    .expect("Failed to upsert setting");

    let theme_value: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'theme'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to read theme setting");

    assert_eq!(theme_value, "mocha", "Theme should be 'mocha'");
}

#[test]
fn test_fnord_revision_tracking() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert pentacle and fnord
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");

    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    conn.execute(
        r#"INSERT INTO fnords (pentacle_id, guid, url, title, content_raw, status)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
        (
            pentacle_id,
            "guid-123",
            "https://example.com/article",
            "Test Article",
            "Original content",
            "concealed",
        ),
    )
    .expect("Failed to insert fnord");

    let fnord_id: i64 = conn
        .query_row("SELECT id FROM fnords LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get fnord id");

    // Insert a revision
    conn.execute(
        r#"INSERT INTO fnord_revisions (fnord_id, title, content_raw, content_hash)
           VALUES (?1, ?2, ?3, ?4)"#,
        (fnord_id, "Test Article", "Original content", "hash123"),
    )
    .expect("Failed to insert revision");

    // Verify revision exists
    let revision_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fnord_revisions WHERE fnord_id = ?1",
            [fnord_id],
            |row| row.get(0),
        )
        .expect("Failed to count revisions");

    assert_eq!(revision_count, 1, "Should have 1 revision");
}

#[test]
fn test_database_with_tempfile() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new_at_path(&db_path).expect("Failed to create database at path");

    // Verify database file was created
    assert!(db_path.exists(), "Database file should exist");

    // Verify it works
    let count: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM sephiroth", [], |row| row.get(0))
        .expect("Failed to count sephiroth");

    assert_eq!(count, 19, "Should have 19 categories (6 main + 13 sub)");
}

#[test]
fn test_pentacle_cascade_delete() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert pentacle
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");

    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    // Insert fnord
    conn.execute(
        r#"INSERT INTO fnords (pentacle_id, guid, url, title, status)
           VALUES (?1, ?2, ?3, ?4, ?5)"#,
        (
            pentacle_id,
            "guid-123",
            "https://example.com/article",
            "Test Article",
            "concealed",
        ),
    )
    .expect("Failed to insert fnord");

    // Verify fnord exists
    let fnord_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .expect("Failed to count fnords");
    assert_eq!(fnord_count, 1, "Should have 1 fnord");

    // Delete pentacle (should cascade to fnords)
    conn.execute("DELETE FROM pentacles WHERE id = ?1", [pentacle_id])
        .expect("Failed to delete pentacle");

    // Verify fnord was deleted
    let fnord_count_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .expect("Failed to count fnords after delete");
    assert_eq!(fnord_count_after, 0, "Fnords should be deleted by cascade");
}
