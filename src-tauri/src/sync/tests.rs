//! Sync module tests

use super::*;
use crate::db::Database;

#[test]
fn test_compute_content_hash_basic() {
    let hash = compute_content_hash("Title", None, None, None);
    assert!(!hash.is_empty(), "Hash should not be empty");
    assert_eq!(hash.len(), 64, "SHA256 hash should be 64 hex characters");
}

#[test]
fn test_compute_content_hash_deterministic() {
    let hash1 = compute_content_hash("Title", Some("Author"), Some("Content"), None);
    let hash2 = compute_content_hash("Title", Some("Author"), Some("Content"), None);

    assert_eq!(hash1, hash2, "Same input should produce same hash");
}

#[test]
fn test_compute_content_hash_different_input() {
    let hash1 = compute_content_hash("Title", Some("Author"), Some("Content"), None);
    let hash2 = compute_content_hash("Title", Some("Author"), Some("Different Content"), None);

    assert_ne!(
        hash1, hash2,
        "Different input should produce different hash"
    );
}

#[test]
fn test_compute_content_hash_all_fields() {
    let hash = compute_content_hash("Title", Some("Author"), Some("Content"), Some("Summary"));

    assert!(!hash.is_empty(), "Hash should not be empty");
    assert_eq!(hash.len(), 64, "SHA256 hash should be 64 hex characters");
}

#[test]
fn test_compute_content_hash_none_fields() {
    let hash1 = compute_content_hash("Title", None, None, None);
    let hash2 = compute_content_hash("Title", Some(""), None, None);

    // Empty string author should produce different hash than None
    assert_ne!(
        hash1, hash2,
        "None and empty string should produce different hashes"
    );
}

#[test]
fn test_store_feed_new_articles() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert a pentacle first
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");

    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    // Create a fetched feed
    let feed = FetchedFeed {
        pentacle_id,
        title: Some("Test Feed Title".to_string()),
        description: Some("Test Description".to_string()),
        site_url: Some("https://example.com".to_string()),
        icon_url: None,
        resolved_url: None,
        entries: vec![
            FetchedEntry {
                guid: "guid-1".to_string(),
                url: "https://example.com/article-1".to_string(),
                title: "Article 1".to_string(),
                author: Some("Author 1".to_string()),
                content_raw: Some("Content 1".to_string()),
                summary: None,
                image_url: None,
                published_at: Some("2024-01-01T00:00:00Z".to_string()),
            },
            FetchedEntry {
                guid: "guid-2".to_string(),
                url: "https://example.com/article-2".to_string(),
                title: "Article 2".to_string(),
                author: None,
                content_raw: Some("Content 2".to_string()),
                summary: Some("Summary 2".to_string()),
                image_url: None,
                published_at: None,
            },
        ],
    };

    let result = FeedSyncer::store_feed(conn, feed).expect("Failed to store feed");

    assert_eq!(result.pentacle_id, pentacle_id);
    assert_eq!(result.new_articles, 2, "Should have 2 new articles");
    assert_eq!(result.updated_articles, 0, "Should have 0 updated articles");

    // Verify articles were inserted
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .expect("Failed to count fnords");

    assert_eq!(count, 2, "Should have 2 fnords in database");
}

#[test]
fn test_store_feed_no_update_on_same_content() {
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

    // First sync
    let feed1 = FetchedFeed {
        pentacle_id,
        title: None,
        description: None,
        site_url: None,
        icon_url: None,
        resolved_url: None,
        entries: vec![FetchedEntry {
            guid: "guid-1".to_string(),
            url: "https://example.com/article-1".to_string(),
            title: "Article 1".to_string(),
            author: None,
            content_raw: Some("Content".to_string()),
            summary: None,
            image_url: None,
            published_at: None,
        }],
    };

    let result1 = FeedSyncer::store_feed(conn, feed1).expect("Failed to store feed");
    assert_eq!(result1.new_articles, 1);
    assert_eq!(result1.updated_articles, 0);

    // Second sync with same content
    let feed2 = FetchedFeed {
        pentacle_id,
        title: None,
        description: None,
        site_url: None,
        icon_url: None,
        resolved_url: None,
        entries: vec![FetchedEntry {
            guid: "guid-1".to_string(),
            url: "https://example.com/article-1".to_string(),
            title: "Article 1".to_string(),
            author: None,
            content_raw: Some("Content".to_string()),
            summary: None,
            image_url: None,
            published_at: None,
        }],
    };

    let result2 = FeedSyncer::store_feed(conn, feed2).expect("Failed to store feed");
    assert_eq!(
        result2.new_articles, 0,
        "Should have 0 new articles on re-sync"
    );
    assert_eq!(
        result2.updated_articles, 0,
        "Should have 0 updated articles when content unchanged"
    );
}

#[test]
fn test_store_feed_detects_content_change() {
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

    // First sync
    let feed1 = FetchedFeed {
        pentacle_id,
        title: None,
        description: None,
        site_url: None,
        icon_url: None,
        resolved_url: None,
        entries: vec![FetchedEntry {
            guid: "guid-1".to_string(),
            url: "https://example.com/article-1".to_string(),
            title: "Original Title".to_string(),
            author: None,
            content_raw: Some("Original Content".to_string()),
            summary: None,
            image_url: None,
            published_at: None,
        }],
    };

    FeedSyncer::store_feed(conn, feed1).expect("Failed to store feed");

    // Second sync with changed content
    let feed2 = FetchedFeed {
        pentacle_id,
        title: None,
        description: None,
        site_url: None,
        icon_url: None,
        resolved_url: None,
        entries: vec![FetchedEntry {
            guid: "guid-1".to_string(),
            url: "https://example.com/article-1".to_string(),
            title: "Updated Title".to_string(),
            author: None,
            content_raw: Some("Updated Content".to_string()),
            summary: None,
            image_url: None,
            published_at: None,
        }],
    };

    let result2 = FeedSyncer::store_feed(conn, feed2).expect("Failed to store feed");
    assert_eq!(result2.new_articles, 0, "Should have 0 new articles");
    assert_eq!(result2.updated_articles, 1, "Should detect content change");

    // Verify has_changes flag
    let has_changes: bool = conn
        .query_row(
            "SELECT has_changes FROM fnords WHERE guid = 'guid-1'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to query has_changes");

    assert!(has_changes, "Article should be marked as changed");

    // Verify revision was created
    let revision_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM fnord_revisions", [], |row| row.get(0))
        .expect("Failed to count revisions");

    assert_eq!(revision_count, 1, "Should have 1 revision saved");
}

#[test]
fn test_store_feed_updates_pentacle_metadata() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert pentacle with minimal data
    conn.execute(
        "INSERT INTO pentacles (url) VALUES (?1)",
        ["https://example.com/feed.xml"],
    )
    .expect("Failed to insert pentacle");

    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    // Sync with metadata
    let feed = FetchedFeed {
        pentacle_id,
        title: Some("Feed Title".to_string()),
        description: Some("Feed Description".to_string()),
        site_url: Some("https://example.com".to_string()),
        icon_url: Some("https://example.com/icon.png".to_string()),
        resolved_url: None,
        entries: vec![],
    };

    FeedSyncer::store_feed(conn, feed).expect("Failed to store feed");

    // Verify pentacle was updated
    let (title, description, site_url, icon_url): (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT title, description, site_url, icon_url FROM pentacles WHERE id = ?1",
            [pentacle_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("Failed to query pentacle");

    assert_eq!(title.as_deref(), Some("Feed Title"));
    assert_eq!(description.as_deref(), Some("Feed Description"));
    assert_eq!(site_url.as_deref(), Some("https://example.com"));
    assert_eq!(icon_url.as_deref(), Some("https://example.com/icon.png"));
}

#[test]
fn test_store_feed_preserves_existing_metadata() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();

    // Insert pentacle with existing data
    conn.execute(
        "INSERT INTO pentacles (url, title, description) VALUES (?1, ?2, ?3)",
        [
            "https://example.com/feed.xml",
            "Existing Title",
            "Existing Description",
        ],
    )
    .expect("Failed to insert pentacle");

    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    // Sync with different metadata
    let feed = FetchedFeed {
        pentacle_id,
        title: Some("New Title".to_string()),
        description: Some("New Description".to_string()),
        site_url: None,
        icon_url: None,
        resolved_url: None,
        entries: vec![],
    };

    FeedSyncer::store_feed(conn, feed).expect("Failed to store feed");

    // Verify existing data was NOT overwritten
    let (title, description): (String, String) = conn
        .query_row(
            "SELECT title, description FROM pentacles WHERE id = ?1",
            [pentacle_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("Failed to query pentacle");

    assert_eq!(
        title, "Existing Title",
        "Title should not be overwritten if already set"
    );
    assert_eq!(
        description, "Existing Description",
        "Description should not be overwritten if already set"
    );
}

#[test]
fn test_feed_syncer_creation() {
    let syncer = FeedSyncer::new();
    // Just verify it can be created without panicking
    drop(syncer);
}

#[test]
fn test_fetched_entry_struct() {
    let entry = FetchedEntry {
        guid: "test-guid".to_string(),
        url: "https://example.com".to_string(),
        title: "Test".to_string(),
        author: None,
        content_raw: None,
        summary: None,
        image_url: None,
        published_at: None,
    };

    assert_eq!(entry.guid, "test-guid");
    assert_eq!(entry.title, "Test");
}

#[test]
fn test_fetched_feed_struct() {
    let feed = FetchedFeed {
        pentacle_id: 1,
        title: Some("Feed".to_string()),
        description: None,
        site_url: None,
        icon_url: None,
        resolved_url: None,
        entries: vec![],
    };

    assert_eq!(feed.pentacle_id, 1);
    assert!(feed.entries.is_empty());
}

#[test]
fn test_sync_result_struct() {
    let result = SyncResult {
        pentacle_id: 1,
        new_articles: 5,
        updated_articles: 2,
    };

    assert_eq!(result.pentacle_id, 1);
    assert_eq!(result.new_articles, 5);
    assert_eq!(result.updated_articles, 2);
}

fn make_entry(guid: &str, url: &str, title: &str) -> FetchedEntry {
    FetchedEntry {
        guid: guid.to_string(),
        url: url.to_string(),
        title: title.to_string(),
        author: None,
        content_raw: Some("Content".to_string()),
        summary: None,
        image_url: None,
        published_at: None,
    }
}

fn make_feed(pentacle_id: i64, entries: Vec<FetchedEntry>) -> FetchedFeed {
    FetchedFeed {
        pentacle_id,
        title: None,
        description: None,
        site_url: None,
        icon_url: None,
        resolved_url: None,
        entries,
    }
}

#[test]
fn test_store_feed_updates_url_on_redirect() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/old-feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");
    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    let mut feed = make_feed(pentacle_id, vec![]);
    feed.resolved_url = Some("https://example.com/new-feed.xml".to_string());
    FeedSyncer::store_feed(conn, feed).expect("Failed to store feed");

    let url: String = conn
        .query_row(
            "SELECT url FROM pentacles WHERE id = ?1",
            [pentacle_id],
            |row| row.get(0),
        )
        .expect("Failed to read url");
    assert_eq!(
        url, "https://example.com/new-feed.xml",
        "Redirected feed URL must be written back to pentacles.url"
    );
}

#[test]
fn test_store_feed_url_fallback_dedup_with_percent_in_url() {
    // URL-fallback dedup (GUID changed, URL identical up to fragment) must
    // still match when the URL contains literal '%' from percent-encoding.
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");
    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    let url = "https://example.com/a%20b%20c";
    let first = make_feed(pentacle_id, vec![make_entry("guid-old", url, "Article")]);
    FeedSyncer::store_feed(conn, first).expect("Failed to store feed");

    // Same article: new GUID, same URL plus fragment
    let refetched_url = format!("{}#comments", url);
    let second = make_feed(
        pentacle_id,
        vec![make_entry("guid-new", &refetched_url, "Article")],
    );
    let result = FeedSyncer::store_feed(conn, second).expect("Failed to store feed");

    assert_eq!(
        result.new_articles, 0,
        "URL-fallback must dedup despite '%' in URL"
    );
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .expect("Failed to count fnords");
    assert_eq!(count, 1, "No duplicate article may be created");
    let guid: String = conn
        .query_row("SELECT guid FROM fnords LIMIT 1", [], |row| row.get(0))
        .expect("Failed to read guid");
    assert_eq!(guid, "guid-new", "GUID must be updated on URL match");
}

#[test]
fn test_store_feed_url_fallback_no_wildcard_false_match() {
    // A literal '%' in the incoming URL must NOT act as a LIKE wildcard and
    // dedup against an unrelated article.
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");
    let pentacle_id: i64 = conn
        .query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id");

    let first = make_feed(
        pentacle_id,
        vec![make_entry(
            "guid-a",
            "https://example.com/foo-unrelated-end#sec",
            "Article A",
        )],
    );
    FeedSyncer::store_feed(conn, first).expect("Failed to store feed");

    // Unescaped, 'foo-%-end' would LIKE-match 'foo-unrelated-end#sec'
    let second = make_feed(
        pentacle_id,
        vec![make_entry(
            "guid-b",
            "https://example.com/foo-%-end",
            "Article B",
        )],
    );
    let result = FeedSyncer::store_feed(conn, second).expect("Failed to store feed");

    assert_eq!(
        result.new_articles, 1,
        "Literal '%' must not wildcard-match an unrelated article"
    );
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .expect("Failed to count fnords");
    assert_eq!(count, 2, "Both distinct articles must exist");
}

fn insert_test_pentacle(conn: &rusqlite::Connection) -> i64 {
    conn.execute(
        "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
        ["https://example.com/feed.xml", "Test Feed"],
    )
    .expect("Failed to insert pentacle");
    conn.query_row("SELECT id FROM pentacles LIMIT 1", [], |row| row.get(0))
        .expect("Failed to get pentacle id")
}

fn health_of(conn: &rusqlite::Connection, id: i64) -> (String, i64, i64) {
    conn.query_row(
        "SELECT health_status, error_count, consecutive_empty_syncs FROM pentacles WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .expect("Failed to read health")
}

#[test]
fn test_feed_health_broken_after_consecutive_errors() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();
    let id = insert_test_pentacle(conn);

    record_sync_error(conn, id, "timeout");
    record_sync_error(conn, id, "timeout");
    let (status, errors, _) = health_of(conn, id);
    assert_eq!(status, "ok", "below threshold must stay ok");
    assert_eq!(errors, 2);

    record_sync_error(conn, id, "timeout");
    let (status, errors, _) = health_of(conn, id);
    assert_eq!(
        status, "broken",
        "3rd consecutive error must flip to broken"
    );
    assert_eq!(errors, 3);
}

#[test]
fn test_feed_health_recovers_on_successful_sync() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();
    let id = insert_test_pentacle(conn);

    for _ in 0..3 {
        record_sync_error(conn, id, "dns");
    }
    assert_eq!(health_of(conn, id).0, "broken");

    let feed = make_feed(id, vec![make_entry("g1", "https://example.com/a1", "A1")]);
    FeedSyncer::store_feed(conn, feed).expect("Failed to store feed");

    let (status, errors, empty) = health_of(conn, id);
    assert_eq!(
        status, "ok",
        "successful sync with new article must recover"
    );
    assert_eq!(errors, 0);
    assert_eq!(empty, 0);
}

#[test]
fn test_feed_health_counts_empty_syncs_and_goes_stale() {
    let db = Database::new_in_memory().expect("Failed to create database");
    let conn = db.conn();
    let id = insert_test_pentacle(conn);

    // One article, then age it beyond the stale threshold
    let feed = make_feed(id, vec![make_entry("g1", "https://example.com/a1", "A1")]);
    FeedSyncer::store_feed(conn, feed).expect("Failed to store feed");
    assert_eq!(health_of(conn, id).0, "ok");

    conn.execute(
        "UPDATE fnords SET fetched_at = datetime('now', '-10 days')",
        [],
    )
    .expect("Failed to age article");

    // Empty sync: fetch works, but nothing new for 10 days -> stale
    let empty_feed = make_feed(id, vec![]);
    FeedSyncer::store_feed(conn, empty_feed).expect("Failed to store feed");

    let (status, _, empty) = health_of(conn, id);
    assert_eq!(status, "stale", "no new article for 10 days must be stale");
    assert_eq!(empty, 1, "empty sync must increment the counter");
}
