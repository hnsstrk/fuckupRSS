//! Integration tests for batch processing with database operations

use crate::db::Database;

// ============================================================
// Database integration tests for batch processing
// ============================================================

#[test]
fn test_get_unprocessed_articles_query() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add a test feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test Feed')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add articles with different states
    let articles = vec![
        // (content_full, content_raw, processed_at)
        (Some("Full content 1"), None, None), // Unprocessed with content
        (None, Some("Raw content 2"), None),  // Unprocessed with raw
        (Some("Full content 3"), None, Some("2024-01-01")), // Processed
        (None, None, None),                   // Unprocessed no content
    ];

    for (i, (content_full, content_raw, processed_at)) in articles.iter().enumerate() {
        db.conn()
            .execute(
                r#"INSERT INTO fnords
                   (pentacle_id, guid, title, url, content_full, content_raw, processed_at, status)
                   VALUES (1, ?, ?, 'http://test.com/article', ?, ?, ?, 'concealed')"#,
                rusqlite::params![
                    format!("guid-{}", i),
                    format!("Article {}", i),
                    content_full,
                    content_raw,
                    processed_at,
                ],
            )
            .expect("Failed to insert fnord");
    }

    // Query: All unprocessed
    let total_unprocessed: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE processed_at IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");
    assert_eq!(total_unprocessed, 3);

    // Query: Unprocessed with content
    let with_content: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE processed_at IS NULL
               AND (content_full IS NOT NULL OR content_raw IS NOT NULL)"#,
            [],
            |row| row.get(0),
        )
        .expect("Query failed");
    assert_eq!(with_content, 2);
}

#[test]
fn test_batch_article_selection_order() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add a test feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test Feed')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add articles with different publish dates
    let articles = vec![
        ("guid-old", "Old Article", "2024-01-01"),
        ("guid-new", "New Article", "2024-01-03"),
        ("guid-mid", "Mid Article", "2024-01-02"),
    ];

    for (guid, title, date) in &articles {
        db.conn()
            .execute(
                r#"INSERT INTO fnords
                   (pentacle_id, guid, title, url, content_raw, published_at, status)
                   VALUES (1, ?, ?, 'http://test.com/a', 'content', ?, 'concealed')"#,
                rusqlite::params![guid, title, date],
            )
            .expect("Failed to insert fnord");
    }

    // Batch query should return newest first
    let mut stmt = db
        .conn()
        .prepare(
            r#"SELECT title FROM fnords
               WHERE processed_at IS NULL
               AND content_raw IS NOT NULL
               ORDER BY published_at DESC
               LIMIT 3"#,
        )
        .expect("Prepare failed");

    let titles: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .expect("Query failed")
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(titles.len(), 3);
    assert_eq!(titles[0], "New Article");
    assert_eq!(titles[1], "Mid Article");
    assert_eq!(titles[2], "Old Article");
}

#[test]
fn test_batch_limit_parameter() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add a test feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add 10 articles
    for i in 0..10 {
        db.conn()
            .execute(
                r#"INSERT INTO fnords
                   (pentacle_id, guid, title, url, content_raw, status)
                   VALUES (1, ?, ?, 'http://test.com/a', 'content', 'concealed')"#,
                rusqlite::params![format!("guid-{}", i), format!("Article {}", i)],
            )
            .expect("Failed to insert fnord");
    }

    // Test limit
    let limit = 5i64;
    let count: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM (
                   SELECT id FROM fnords
                   WHERE processed_at IS NULL
                   AND content_raw IS NOT NULL
                   LIMIT ?
               )"#,
            [limit],
            |row| row.get(0),
        )
        .expect("Query failed");

    assert_eq!(count, 5);
}

#[test]
fn test_update_article_after_processing() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add feed and article
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test')",
            [],
        )
        .expect("Failed to insert pentacle");

    db.conn()
        .execute(
            r#"INSERT INTO fnords
               (pentacle_id, guid, title, url, content_raw, status)
               VALUES (1, 'test-guid', 'Test Article', 'http://test.com/a', 'Test content', 'concealed')"#,
            [],
        )
        .expect("Failed to insert fnord");

    // Verify article is unprocessed
    let processed_at_before: Option<String> = db
        .conn()
        .query_row(
            "SELECT processed_at FROM fnords WHERE guid = 'test-guid'",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");
    assert!(processed_at_before.is_none());

    // Simulate batch processing update
    db.conn()
        .execute(
            r#"UPDATE fnords SET
                summary = 'Test summary',
                political_bias = 0,
                sachlichkeit = 3,
                processed_at = CURRENT_TIMESTAMP
            WHERE guid = 'test-guid'"#,
            [],
        )
        .expect("Update failed");

    // Verify update
    let (summary, bias, sach): (String, i32, i32) = db
        .conn()
        .query_row(
            "SELECT summary, political_bias, sachlichkeit FROM fnords WHERE guid = 'test-guid'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("Query failed");

    assert_eq!(summary, "Test summary");
    assert_eq!(bias, 0);
    assert_eq!(sach, 3);

    // Verify processed_at is now set
    let processed_at_after: Option<String> = db
        .conn()
        .query_row(
            "SELECT processed_at FROM fnords WHERE guid = 'test-guid'",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");
    assert!(processed_at_after.is_some());
}

#[test]
fn test_category_assignment_during_batch() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Setup feed and article
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test')",
            [],
        )
        .expect("Failed to insert pentacle");

    db.conn()
        .execute(
            r#"INSERT INTO fnords
               (pentacle_id, guid, title, url, content_raw, status)
               VALUES (1, 'test-guid', 'Test Article', 'http://test.com/a', 'content', 'concealed')"#,
            [],
        )
        .expect("Failed to insert fnord");

    let fnord_id: i64 = db
        .conn()
        .query_row(
            "SELECT id FROM fnords WHERE guid = 'test-guid'",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");

    // Get a category ID (Politik should exist from schema)
    let sephiroth_id: i64 = db
        .conn()
        .query_row(
            "SELECT id FROM sephiroth WHERE name = 'Politik'",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");

    // Assign category like batch processing would
    db.conn()
        .execute(
            r#"INSERT OR IGNORE INTO fnord_sephiroth
               (fnord_id, sephiroth_id, confidence, source, assigned_at)
               VALUES (?, ?, 1.0, 'ai', CURRENT_TIMESTAMP)"#,
            rusqlite::params![fnord_id, sephiroth_id],
        )
        .expect("Insert failed");

    // Verify assignment
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnord_sephiroth WHERE fnord_id = ?",
            [fnord_id],
            |row| row.get(0),
        )
        .expect("Query failed");

    assert_eq!(count, 1);
}

#[test]
fn test_keyword_assignment_during_batch() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Setup
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test')",
            [],
        )
        .expect("Failed to insert pentacle");

    db.conn()
        .execute(
            r#"INSERT INTO fnords
               (pentacle_id, guid, title, url, content_raw, status)
               VALUES (1, 'test-guid', 'Test Article', 'http://test.com/a', 'content', 'concealed')"#,
            [],
        )
        .expect("Failed to insert fnord");

    let fnord_id: i64 = db
        .conn()
        .query_row(
            "SELECT id FROM fnords WHERE guid = 'test-guid'",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");

    // Add keywords like batch processing would
    let keywords = vec!["Keyword1", "Keyword2", "Keyword3"];

    for keyword in &keywords {
        // Insert or update keyword
        db.conn()
            .execute(
                r#"INSERT INTO immanentize (name, count, article_count, first_seen, last_used)
                   VALUES (?1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                   ON CONFLICT(name) DO UPDATE SET
                       count = count + 1,
                       article_count = article_count + 1,
                       last_used = CURRENT_TIMESTAMP"#,
                [keyword],
            )
            .expect("Insert keyword failed");

        // Link to article
        let tag_id: i64 = db
            .conn()
            .query_row(
                "SELECT id FROM immanentize WHERE name = ?",
                [keyword],
                |row| row.get(0),
            )
            .expect("Query failed");

        db.conn()
            .execute(
                "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id) VALUES (?, ?)",
                rusqlite::params![fnord_id, tag_id],
            )
            .expect("Link failed");
    }

    // Verify
    let keyword_count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnord_immanentize WHERE fnord_id = ?",
            [fnord_id],
            |row| row.get(0),
        )
        .expect("Query failed");

    assert_eq!(keyword_count, 3);
}

#[test]
fn test_cooccurrence_network_update() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add three keywords
    for keyword in &["KeywordA", "KeywordB", "KeywordC"] {
        db.conn()
            .execute(
                r#"INSERT INTO immanentize (name, count, article_count, first_seen, last_used)
                   VALUES (?1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"#,
                [keyword],
            )
            .expect("Insert keyword failed");
    }

    let tag_ids: Vec<i64> = vec!["KeywordA", "KeywordB", "KeywordC"]
        .iter()
        .map(|kw| {
            db.conn()
                .query_row("SELECT id FROM immanentize WHERE name = ?", [kw], |row| {
                    row.get(0)
                })
                .expect("Query failed")
        })
        .collect();

    // Create cooccurrence links like batch processing would
    for i in 0..tag_ids.len() {
        for j in (i + 1)..tag_ids.len() {
            let (id_a, id_b) = if tag_ids[i] < tag_ids[j] {
                (tag_ids[i], tag_ids[j])
            } else {
                (tag_ids[j], tag_ids[i])
            };

            db.conn()
                .execute(
                    r#"INSERT INTO immanentize_neighbors
                       (immanentize_id_a, immanentize_id_b, cooccurrence, first_seen, last_seen)
                       VALUES (?1, ?2, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                       ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
                           cooccurrence = cooccurrence + 1,
                           last_seen = CURRENT_TIMESTAMP"#,
                    rusqlite::params![id_a, id_b],
                )
                .expect("Insert cooccurrence failed");
        }
    }

    // Verify: 3 keywords should create 3 pairs (A-B, A-C, B-C)
    let pair_count: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM immanentize_neighbors", [], |row| {
            row.get(0)
        })
        .expect("Query failed");

    assert_eq!(pair_count, 3);
}

#[test]
fn test_batch_skips_empty_content() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add article with empty content
    db.conn()
        .execute(
            r#"INSERT INTO fnords
               (pentacle_id, guid, title, url, content_raw, status)
               VALUES (1, 'empty-guid', 'Empty Article', 'http://test.com/a', '', 'concealed')"#,
            [],
        )
        .expect("Failed to insert fnord");

    // Query for articles with actual content (simulating COALESCE check)
    let articles: Vec<(i64, String, String)> = {
        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT id, title, COALESCE(content_full, content_raw, '') as content
                   FROM fnords
                   WHERE processed_at IS NULL"#,
            )
            .expect("Prepare failed");

        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("Query failed")
            .filter_map(|r| r.ok())
            .collect()
    };

    // Article exists but content is empty
    assert_eq!(articles.len(), 1);
    assert!(articles[0].2.is_empty()); // content should be empty string
}

#[test]
fn test_processed_articles_excluded_from_batch() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add processed article
    db.conn()
        .execute(
            r#"INSERT INTO fnords
               (pentacle_id, guid, title, url, content_raw, processed_at, status)
               VALUES (1, 'processed-guid', 'Processed', 'http://test.com/a', 'content', CURRENT_TIMESTAMP, 'concealed')"#,
            [],
        )
        .expect("Failed to insert fnord");

    // Add unprocessed article
    db.conn()
        .execute(
            r#"INSERT INTO fnords
               (pentacle_id, guid, title, url, content_raw, status)
               VALUES (1, 'unprocessed-guid', 'Unprocessed', 'http://test.com/b', 'content', 'concealed')"#,
            [],
        )
        .expect("Failed to insert fnord");

    // Batch query should only return unprocessed
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE processed_at IS NULL AND content_raw IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");

    assert_eq!(count, 1);
}

// ============================================================
// Article embedding database integration tests
// ============================================================

#[test]
fn test_article_embedding_stats_query() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add test feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test Feed')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add articles with different embedding/processing states
    let articles = vec![
        // (guid, content_full, processed_at, embedding)
        (
            "guid-1",
            Some("x".repeat(200)),
            Some("2024-01-01"),
            Some(vec![0u8; 4096]),
        ), // Has embedding
        ("guid-2", Some("x".repeat(200)), Some("2024-01-01"), None), // Processable
        ("guid-3", Some("x".repeat(200)), None, None),               // Not processed yet
        ("guid-4", None, Some("2024-01-01"), None),                  // No content
        (
            "guid-5",
            Some("short".to_string()),
            Some("2024-01-01"),
            None,
        ), // Content too short
    ];

    for (guid, content_full, processed_at, embedding) in &articles {
        db.conn()
            .execute(
                r#"INSERT INTO fnords
                   (pentacle_id, guid, title, url, content_full, processed_at, embedding, status)
                   VALUES (1, ?, 'Test', 'http://test.com/a', ?, ?, ?, 'concealed')"#,
                rusqlite::params![guid, content_full, processed_at, embedding],
            )
            .expect("Failed to insert fnord");
    }

    // Query: Total articles
    let total: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .expect("Query failed");
    assert_eq!(total, 5);

    // Query: With embedding
    let with_embedding: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");
    assert_eq!(with_embedding, 1);

    // Query: Processable (no embedding, processed, has long content)
    let processable: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE embedding IS NULL
               AND processed_at IS NOT NULL
               AND content_full IS NOT NULL
               AND LENGTH(content_full) >= 100"#,
            [],
            |row| row.get(0),
        )
        .expect("Query failed");
    assert_eq!(processable, 1); // Only guid-2
}

#[test]
fn test_article_embedding_batch_selection() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add test feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test Feed')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add processable articles with different processed_at dates
    let articles = vec![
        ("guid-old", "2024-01-01"),
        ("guid-new", "2024-01-03"),
        ("guid-mid", "2024-01-02"),
    ];

    for (guid, processed_at) in &articles {
        db.conn()
            .execute(
                r#"INSERT INTO fnords
                   (pentacle_id, guid, title, url, content_full, processed_at, status)
                   VALUES (1, ?, 'Test', 'http://test.com/a', ?, ?, 'concealed')"#,
                rusqlite::params![guid, "x".repeat(200), processed_at],
            )
            .expect("Failed to insert fnord");
    }

    // Batch selection should return newest processed first
    let mut stmt = db
        .conn()
        .prepare(
            r#"SELECT guid FROM fnords
               WHERE embedding IS NULL
               AND processed_at IS NOT NULL
               AND content_full IS NOT NULL
               AND LENGTH(content_full) >= 100
               ORDER BY processed_at DESC
               LIMIT 3"#,
        )
        .expect("Prepare failed");

    let guids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .expect("Query failed")
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(guids.len(), 3);
    assert_eq!(guids[0], "guid-new");
    assert_eq!(guids[1], "guid-mid");
    assert_eq!(guids[2], "guid-old");
}

#[test]
fn test_save_article_embedding() {
    use crate::embeddings::embedding_to_blob;

    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add test feed and article
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test Feed')",
            [],
        )
        .expect("Failed to insert pentacle");

    db.conn()
        .execute(
            r#"INSERT INTO fnords
               (pentacle_id, guid, title, url, content_full, processed_at, status)
               VALUES (1, 'test-guid', 'Test Article', 'http://test.com/a', 'Some content...', '2024-01-01', 'concealed')"#,
            [],
        )
        .expect("Failed to insert fnord");

    let fnord_id: i64 = db
        .conn()
        .query_row(
            "SELECT id FROM fnords WHERE guid = 'test-guid'",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");

    // Create and save a mock embedding
    let embedding: Vec<f32> = (0..1024).map(|i| (i as f32) / 1024.0).collect();
    let blob = embedding_to_blob(&embedding);

    db.conn()
        .execute(
            "UPDATE fnords SET embedding = ?1, embedding_at = datetime('now') WHERE id = ?2",
            rusqlite::params![blob, fnord_id],
        )
        .expect("Update failed");

    // Verify embedding was saved
    let (saved_embedding, embedding_at): (Vec<u8>, String) = db
        .conn()
        .query_row(
            "SELECT embedding, embedding_at FROM fnords WHERE id = ?",
            [fnord_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("Query failed");

    assert_eq!(saved_embedding.len(), 1024 * 4); // 1024 f32 values * 4 bytes each
    assert!(!embedding_at.is_empty());
}

#[test]
fn test_article_embedding_content_text_generation() {
    // Test the logic for generating embedding text from title + content preview
    let title = "Test Article Title";
    let content = "x".repeat(1000); // Long content

    // Take first 500 chars of content
    let content_preview: String = content.chars().take(500).collect();
    let embedding_text = format!("{}\n\n{}", title, content_preview);

    assert!(embedding_text.starts_with("Test Article Title"));
    assert_eq!(content_preview.len(), 500);
    assert!(embedding_text.len() > title.len());
}

#[test]
fn test_article_embedding_short_content() {
    // Test with content shorter than 500 chars
    let title = "Short Article";
    let content = "This is a short content.";

    let content_preview: String = content.chars().take(500).collect();
    let embedding_text = format!("{}\n\n{}", title, content_preview);

    assert_eq!(content_preview, content);
    assert!(embedding_text.contains(content));
}

#[test]
fn test_embedding_stats_without_embedding_calculation() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add test feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test Feed')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add 10 articles
    for i in 0..10 {
        let has_embedding = i < 3; // First 3 have embeddings
        let is_processed = i < 7; // First 7 are processed
        let has_content = i != 8; // All except 8th have content

        let embedding: Option<Vec<u8>> = if has_embedding {
            Some(vec![0u8; 4096])
        } else {
            None
        };
        let processed_at: Option<&str> = if is_processed {
            Some("2024-01-01")
        } else {
            None
        };
        let content_full: Option<String> = if has_content {
            Some("x".repeat(200))
        } else {
            None
        };

        db.conn()
            .execute(
                r#"INSERT INTO fnords
                   (pentacle_id, guid, title, url, content_full, processed_at, embedding, status)
                   VALUES (1, ?, 'Test', 'http://test.com/a', ?, ?, ?, 'concealed')"#,
                rusqlite::params![format!("guid-{}", i), content_full, processed_at, embedding],
            )
            .expect("Failed to insert fnord");
    }

    // Calculate stats
    let total: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .expect("Query failed");

    let with_embedding: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("Query failed");

    let without_embedding = total - with_embedding;

    // Verify stats
    assert_eq!(total, 10);
    assert_eq!(with_embedding, 3);
    assert_eq!(without_embedding, 7);
}

#[test]
fn test_find_articles_for_embedding_limit() {
    let db = Database::new_in_memory().expect("Failed to create in-memory database");

    // Add test feed
    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES ('http://test.com', 'Test Feed')",
            [],
        )
        .expect("Failed to insert pentacle");

    // Add 20 processable articles
    for i in 0..20 {
        db.conn()
            .execute(
                r#"INSERT INTO fnords
                   (pentacle_id, guid, title, url, content_full, processed_at, status)
                   VALUES (1, ?, 'Test', 'http://test.com/a', ?, '2024-01-01', 'concealed')"#,
                rusqlite::params![format!("guid-{}", i), "x".repeat(200)],
            )
            .expect("Failed to insert fnord");
    }

    // Test limit parameter
    let limit = 10i64;
    let articles: Vec<(i64, String, String)> = {
        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT id, title, content_full
                   FROM fnords
                   WHERE embedding IS NULL
                   AND processed_at IS NOT NULL
                   AND content_full IS NOT NULL
                   AND LENGTH(content_full) >= 100
                   ORDER BY processed_at DESC
                   LIMIT ?"#,
            )
            .expect("Prepare failed");

        stmt.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("Query failed")
            .filter_map(|r| r.ok())
            .collect()
    };

    assert_eq!(articles.len(), 10);
}
