//! Database maintenance commands

use crate::error::{CmdResult, FuckupError};
use crate::AppState;
use log::{info, warn};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
pub struct VacuumResult {
    pub size_before_mb: i64,
    pub size_after_mb: i64,
    pub freed_mb: i64,
    pub success: bool,
}

/// Perform database maintenance: checkpoint WAL, VACUUM, and ANALYZE
///
/// This command performs a full database optimization:
/// 1. PRAGMA wal_checkpoint(TRUNCATE) - Checkpoint and truncate WAL file
/// 2. VACUUM - Defragment and compact database
/// 3. ANALYZE - Update query planner statistics
///
/// **When to use:**
/// - After deleting feeds (many articles removed)
/// - After keyword pruning (1000+ keywords deleted)
/// - Periodically to reclaim free pages
///
/// **Note:** This operation can take 1-2 minutes for large databases (>100MB)
#[tauri::command]
pub async fn vacuum_database(state: State<'_, AppState>) -> Result<VacuumResult, String> {
    info!("Starting database VACUUM operation");

    let db = state.db_conn()?;
    let conn = db.conn();

    // Get size before VACUUM
    let size_before: i64 = conn
        .query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let size_before_mb = size_before / 1024 / 1024;

    info!("Database size before VACUUM: {} MB", size_before_mb);

    // Step 1: Checkpoint and truncate WAL file
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
        let busy: i32 = row.get(0)?;
        let log: i32 = row.get(1)?;
        let checkpointed: i32 = row.get(2)?;
        Ok((busy, log, checkpointed))
    })
    .map_err(|e| format!("WAL checkpoint failed: {}", e))?;

    info!("WAL checkpoint completed");

    // Step 2: VACUUM - defragment and compact database
    conn.execute("VACUUM", [])
        .map_err(|e| format!("VACUUM failed: {}", e))?;

    info!("VACUUM completed");

    // Step 3: ANALYZE - update query planner statistics
    conn.execute("ANALYZE", [])
        .map_err(|e| format!("ANALYZE failed: {}", e))?;

    info!("ANALYZE completed");

    // Get size after VACUUM
    let size_after: i64 = conn
        .query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let size_after_mb = size_after / 1024 / 1024;
    let freed_mb = size_before_mb - size_after_mb;

    info!(
        "Database VACUUM complete: {} MB -> {} MB (freed {} MB)",
        size_before_mb, size_after_mb, freed_mb
    );

    Ok(VacuumResult {
        size_before_mb,
        size_after_mb,
        freed_mb,
        success: true,
    })
}

#[derive(Debug, Serialize)]
pub struct OrphanedArticleStats {
    pub total: i64,
    pub favorites: i64,
}

#[tauri::command]
pub fn find_orphaned_articles(state: State<AppState>) -> CmdResult<OrphanedArticleStats> {
    let db = state.db_conn()?;

    let stats = db.conn().query_row(
        "SELECT COUNT(*) as total, COUNT(CASE WHEN status = 'golden_apple' THEN 1 END) as favorites FROM fnords WHERE pentacle_id NOT IN (SELECT id FROM pentacles)",
        [],
        |row| {
            Ok(OrphanedArticleStats {
                total: row.get(0)?,
                favorites: row.get(1)?,
            })
        },
    )?;

    Ok(stats)
}

#[tauri::command]
pub fn delete_orphaned_articles(state: State<AppState>, include_favorites: bool) -> CmdResult<i64> {
    let db = state.db_conn()?;
    let conn = db.conn();

    conn.execute("BEGIN", [])?;

    let result = if include_favorites {
        conn.execute(
            "DELETE FROM fnords WHERE pentacle_id NOT IN (SELECT id FROM pentacles)",
            [],
        )
    } else {
        conn.execute(
            "DELETE FROM fnords WHERE pentacle_id NOT IN (SELECT id FROM pentacles) AND status != 'golden_apple'",
            [],
        )
    };

    match result {
        Ok(_) => {
            let deleted = conn.changes() as i64;

            // Recalculate article_count for all keywords that may have stale counts
            // (covers data from before the trigger was added)
            let _ = conn.execute(
                r#"UPDATE immanentize SET article_count = (
                    SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = immanentize.id
                )"#,
                [],
            );

            conn.execute("COMMIT", [])?;
            info!(
                "Deleted {} orphaned articles (include_favorites={})",
                deleted, include_favorites
            );
            Ok(deleted)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e.into())
        }
    }
}

// ============================================================
// DB RESET (destructive — drops all article data, keeps config)
// ============================================================

/// Tables that are always preserved on reset.
/// KEEP IN SYNC with `TABLES_TO_CLEAR` below — every non-derived user-facing
/// table must appear in exactly one of the two lists.
pub const PRESERVED_TABLES: &[&str] = &[
    "pentacles",    // Feeds
    "stopwords",    // Custom stopwords
    "settings",     // API keys, provider config, locale, theme
    "sephiroth",    // Category definitions
    "bias_weights", // Learned bias weights
];

/// Tables that get cleared on reset. Order matters for FK cascades: parents
/// last so child rows don't trigger cascade on already-empty tables.
/// `preserved_compounds` and `compound_decisions` are FK-CASCADEd from
/// `immanentize`, so they drop automatically — listing them explicitly to
/// keep observability of what was touched.
pub const TABLES_TO_CLEAR: &[&str] = &[
    // Derived / reports
    "theme_reports", // cascade: theme_report_themes, theme_report_articles
    "briefings",
    "recommendation_feedback",
    "ai_cost_log",
    "analysis_cache",
    "embedding_queue",
    "keyword_type_prototypes",
    "dismissed_synonyms",
    // Keyword network (no FK dependency)
    "immanentize_daily",
    "immanentize_neighbors",
    "immanentize_clusters",
    // Articles (cascade: fnord_*)
    "fnords",
    // Keywords + entities (cascade: preserved_compounds, compound_decisions)
    "immanentize_sephiroth",
    "immanentize",
    "entities",
    // Corpus statistics
    "corpus_stats",
];

/// Shadow tables of the `vec0` virtual tables. We wipe these directly
/// because the vec0 module — required to operate on the virtual tables
/// themselves — is only loaded when the Rust app runs it, and even then
/// DROP TRIGGER dance would be required for foreign-key-less deletes to
/// compile. Emptying shadow tables keeps the index consistent with an
/// empty `fnords` / `immanentize` parent.
pub const VEC_SHADOW_TABLES: &[&str] = &[
    "vec_fnords_chunks",
    "vec_fnords_info",
    "vec_fnords_rowids",
    "vec_fnords_vector_chunks00",
    "vec_immanentize_chunks",
    "vec_immanentize_info",
    "vec_immanentize_rowids",
    "vec_immanentize_vector_chunks00",
];

/// Triggers that reference the `vec0` virtual table. Dropped before the
/// DELETE pass and recreated immediately after it (see `perform_db_reset`);
/// waiting for the next `schema::init()` would leave the running session
/// without vec cleanup on keyword deletions.
const VEC_TRIGGERS: &[&str] = &["immanentize_delete_vec"];

/// The token the frontend must send to confirm the destructive operation.
/// Typed by the user into a text field before the Reset button becomes active.
const RESET_CONFIRM_TOKEN: &str = "RESET";

/// Preview stats for the Settings UI — shown before the user confirms the reset.
#[derive(Debug, Serialize)]
pub struct DbResetPreview {
    pub articles: i64,
    pub keywords: i64,
    pub entities: i64,
    pub theme_reports: i64,
    pub briefings: i64,
    pub analysis_cache_entries: i64,
    pub db_size_bytes: i64,
    pub tables_to_clear: Vec<String>,
    pub tables_preserved: Vec<String>,
}

/// Result of the reset operation.
#[derive(Debug, Serialize, Clone)]
pub struct DbResetResult {
    pub backup_path: String,
    pub size_before_bytes: i64,
    pub size_after_bytes: i64,
    pub bytes_freed: i64,
    pub tables_cleared: Vec<ClearedTable>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClearedTable {
    pub name: String,
    pub rows_before: i64,
}

/// Progress event payload emitted during `reset_articles_data`.
/// Event name: `"db-reset-progress"`.
#[derive(Debug, Serialize, Clone)]
pub struct DbResetProgress {
    pub phase: String, // "backup" | "clearing" | "vacuum" | "done"
    pub current_table: Option<String>,
    pub tables_done: usize,
    pub tables_total: usize,
}

/// Return a preview of what the reset will do, without touching anything.
/// Safe to call as often as the UI needs.
#[tauri::command]
pub async fn get_db_reset_preview(state: State<'_, AppState>) -> CmdResult<DbResetPreview> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let articles: i64 = conn
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .unwrap_or(0);
    let keywords: i64 = conn
        .query_row("SELECT COUNT(*) FROM immanentize", [], |row| row.get(0))
        .unwrap_or(0);
    let entities: i64 = conn
        .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
        .unwrap_or(0);
    let theme_reports: i64 = conn
        .query_row("SELECT COUNT(*) FROM theme_reports", [], |row| row.get(0))
        .unwrap_or(0);
    let briefings: i64 = conn
        .query_row("SELECT COUNT(*) FROM briefings", [], |row| row.get(0))
        .unwrap_or(0);
    let analysis_cache_entries: i64 = conn
        .query_row("SELECT COUNT(*) FROM analysis_cache", [], |row| row.get(0))
        .unwrap_or(0);
    let db_size_bytes: i64 = conn
        .query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(DbResetPreview {
        articles,
        keywords,
        entities,
        theme_reports,
        briefings,
        analysis_cache_entries,
        db_size_bytes,
        tables_to_clear: TABLES_TO_CLEAR.iter().map(|s| s.to_string()).collect(),
        tables_preserved: PRESERVED_TABLES.iter().map(|s| s.to_string()).collect(),
    })
}

/// Resolve the on-disk path of the main database.
/// Uses `PRAGMA database_list` so we don't hard-code the project-relative
/// `data/fuckup.db` path and stay correct if the app is ever moved.
fn get_main_db_path(conn: &rusqlite::Connection) -> Result<PathBuf, rusqlite::Error> {
    let path: String = conn.query_row(
        "SELECT file FROM pragma_database_list() WHERE name = 'main'",
        [],
        |row| row.get(0),
    )?;
    Ok(PathBuf::from(path))
}

/// Shared reset implementation — exercised both by the Tauri command and
/// by the unit tests. Takes a connection rather than State so tests can
/// drive it against an in-memory DB without standing up a Tauri harness.
///
/// `backup_path`: path for `VACUUM INTO`. Pass `None` to skip (tests).
/// Progress callback is invoked for each cleared table so the Tauri
/// command can emit events while the tests can assert the order.
pub fn perform_db_reset(
    conn: &rusqlite::Connection,
    backup_path: Option<&std::path::Path>,
    mut progress: impl FnMut(&str, Option<&str>, usize, usize),
) -> Result<DbResetResult, rusqlite::Error> {
    let size_before: i64 = conn.query_row(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
        [],
        |row| row.get(0),
    )?;

    // 1) Backup via VACUUM INTO — atomic, works with WAL, identical contents.
    let backup_path_str = if let Some(p) = backup_path {
        let s = p.to_string_lossy().to_string();
        progress("backup", None, 0, TABLES_TO_CLEAR.len());
        info!("DB reset: creating backup at {}", s);
        // rusqlite does not bind PATH in VACUUM INTO, so format safely.
        // Path is controlled by the backend (not user input) → no injection.
        let escaped = s.replace('\'', "''");
        conn.execute_batch(&format!("VACUUM INTO '{}';", escaped))?;
        s
    } else {
        String::new()
    };

    // 2) Drop vec0-dependent triggers — schema::init recreates them next app start.
    for trig in VEC_TRIGGERS {
        let sql = format!("DROP TRIGGER IF EXISTS {};", trig);
        if let Err(e) = conn.execute_batch(&sql) {
            warn!("DB reset: DROP TRIGGER {} failed: {}", trig, e);
        }
    }

    // 3) Snapshot row counts per target table (before).
    let mut cleared: Vec<ClearedTable> = Vec::with_capacity(TABLES_TO_CLEAR.len());

    // 4) DELETE in a transaction with FK-cascade enabled.
    conn.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;

    for (idx, table) in TABLES_TO_CLEAR.iter().enumerate() {
        progress("clearing", Some(table), idx, TABLES_TO_CLEAR.len());

        let rows_before: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let affected = conn.execute(&format!("DELETE FROM {}", table), [])?;
        info!(
            "DB reset: {} rows deleted from {} (was {}, actually deleted {})",
            rows_before, table, rows_before, affected
        );

        cleared.push(ClearedTable {
            name: table.to_string(),
            rows_before,
        });
    }

    // 5) Clear the vec0 virtual tables through the module first — the same
    //    proven path as data_persistence.rs/similarity.rs — then wipe the
    //    known shadow tables as a belt-and-braces pass. If a future
    //    sqlite-vec version adds shadow tables this list doesn't know, the
    //    virtual-table DELETE above still covers them.
    for vec_table in ["vec_fnords", "vec_immanentize"] {
        if let Err(e) = conn.execute(&format!("DELETE FROM {}", vec_table), []) {
            warn!("DB reset: DELETE FROM {} failed: {}", vec_table, e);
        }
    }
    for shadow in VEC_SHADOW_TABLES {
        let _ = conn.execute(&format!("DELETE FROM {}", shadow), []);
    }

    conn.execute_batch("COMMIT;")?;

    // 5b) Recreate the vec cleanup trigger right away — the running session
    //     keeps deleting keywords (dedup/synonyms) and must not leave stale
    //     vectors behind until the next app start.
    if let Err(e) = conn.execute_batch(crate::db::IMMANENTIZE_DELETE_VEC_TRIGGER) {
        warn!("DB reset: failed to recreate vec cleanup trigger: {}", e);
    }

    // 6) VACUUM to reclaim space (outside transaction — VACUUM cannot run in one).
    progress("vacuum", None, TABLES_TO_CLEAR.len(), TABLES_TO_CLEAR.len());
    conn.execute_batch("VACUUM;")?;

    let size_after: i64 = conn.query_row(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
        [],
        |row| row.get(0),
    )?;

    progress("done", None, TABLES_TO_CLEAR.len(), TABLES_TO_CLEAR.len());

    Ok(DbResetResult {
        backup_path: backup_path_str,
        size_before_bytes: size_before,
        size_after_bytes: size_after,
        bytes_freed: size_before - size_after,
        tables_cleared: cleared,
    })
}

/// Reset all article data. **Destructive**. Requires `confirm_token == "RESET"`.
/// Refuses to run while a batch job is active.
///
/// Creates a timestamped backup via `VACUUM INTO` (atomic, WAL-safe) before
/// touching anything. Emits `"db-reset-progress"` events throughout.
#[tauri::command]
pub async fn reset_articles_data(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    confirm_token: String,
) -> CmdResult<DbResetResult> {
    if confirm_token != RESET_CONFIRM_TOKEN {
        return Err(FuckupError::Validation(format!(
            "Invalid confirm token. Type '{}' exactly to confirm.",
            RESET_CONFIRM_TOKEN
        )));
    }

    // Check BOTH flags: batch_running is released before the post-batch
    // embedding phase (model-swapping optimization), but that phase still
    // writes to fnords/vec_fnords and must not race a reset.
    if state.batch_running.load(Ordering::SeqCst) || state.embedding_running.load(Ordering::SeqCst)
    {
        return Err(FuckupError::Validation(
            "A batch job is currently running. Please cancel it before resetting the database."
                .to_string(),
        ));
    }

    // Compute backup path from the main DB path.
    let backup_path: PathBuf = {
        let db = state.db_conn()?;
        let main_path = get_main_db_path(db.conn()).map_err(FuckupError::from)?;
        let parent = main_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
        parent.join(format!("fuckup.db.backup-pre-reset-{}", ts))
    };

    info!(
        "reset_articles_data: starting (backup={})",
        backup_path.display()
    );

    // Acquire the DB lock for the full reset.
    let db = state.db_conn()?;
    let conn = db.conn();

    // Emit-closure capturing the Tauri handle — passed into perform_db_reset.
    let app = app_handle.clone();
    let result = perform_db_reset(
        conn,
        Some(&backup_path),
        |phase, current_table, done, total| {
            let _ = app.emit(
                "db-reset-progress",
                DbResetProgress {
                    phase: phase.to_string(),
                    current_table: current_table.map(|s| s.to_string()),
                    tables_done: done,
                    tables_total: total,
                },
            );
        },
    )
    .map_err(FuckupError::from)?;

    info!(
        "reset_articles_data: complete — {} MB -> {} MB (freed {} MB), backup: {}",
        result.size_before_bytes / 1_048_576,
        result.size_after_bytes / 1_048_576,
        result.bytes_freed / 1_048_576,
        result.backup_path
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_vacuum_in_memory() {
        let db = Database::new_in_memory().unwrap();

        // Insert some test data
        db.conn()
            .execute(
                "INSERT INTO pentacles (url, title) VALUES ('https://test.com/feed', 'Test Feed')",
                [],
            )
            .unwrap();

        // Insert and delete to create fragmentation
        for i in 0..100 {
            db.conn()
                .execute(
                    "INSERT INTO immanentize (name) VALUES (?)",
                    [format!("keyword_{}", i)],
                )
                .unwrap();
        }

        db.conn()
            .execute("DELETE FROM immanentize WHERE id > 50", [])
            .unwrap();

        // VACUUM should succeed
        db.conn().execute("VACUUM", []).unwrap();
        db.conn().execute("ANALYZE", []).unwrap();

        // Verify database is still functional
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM pentacles", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 1);
    }

    // ============================================================
    // DB RESET TESTS
    // ============================================================

    /// Seeds an in-memory DB with a representative row in every preserved and
    /// every cleared table so we can verify the reset touches exactly what it
    /// promises and nothing else.
    fn seed_reset_fixtures(db: &Database) {
        let conn = db.conn();

        // PRESERVED ----------------------------------------------------------
        conn.execute(
            "INSERT INTO pentacles (url, title) VALUES ('https://a.b/feed', 'A')",
            [],
        )
        .unwrap();
        let pentacle_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT OR IGNORE INTO stopwords (word, language) VALUES ('xxtest_stopword_xx', 'de')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('test_key', 'test_val')",
            [],
        )
        .unwrap();
        // sephiroth is seeded by schema::init, just verify it survives
        conn.execute(
            "INSERT OR IGNORE INTO bias_weights (weight_type, context_key, term, weight)
             VALUES ('keyword_boost', 'test_keyword', NULL, 1.5)",
            [],
        )
        .unwrap();

        // CLEARED ------------------------------------------------------------
        // Article with full relation graph — let IDs autoincrement to avoid
        // collisions with schema-seeded rows.
        conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, content_full, processed_at)
               VALUES (?, 'guid-1', 'https://a.b/1', 'Test Article', 'body', CURRENT_TIMESTAMP)"#,
            [pentacle_id],
        )
        .unwrap();
        let fnord_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO immanentize (name) VALUES ('xxTestKeyword_reset_xx')",
            [],
        )
        .unwrap();
        let imm_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO fnord_immanentize (fnord_id, immanentize_id) VALUES (?, ?)",
            [fnord_id, imm_id],
        )
        .unwrap();

        // Use first seeded sephiroth id so FK constraint holds
        let seph_id: i64 = conn
            .query_row("SELECT id FROM sephiroth ORDER BY id LIMIT 1", [], |r| {
                r.get(0)
            })
            .expect("sephiroth should be seeded by schema::init");
        conn.execute(
            "INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id) VALUES (?, ?)",
            [fnord_id, seph_id],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO entities (name, entity_type, normalized_name) VALUES ('Merkel', 'person', 'merkel')",
            [],
        )
        .unwrap();
        let entity_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO fnord_entities (fnord_id, entity_id, mention_count) VALUES (?, ?, 1)",
            [fnord_id, entity_id],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO theme_reports (period_start, period_end) VALUES (datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        let report_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO theme_report_themes (report_id, label) VALUES (?, 'Test')",
            [report_id],
        )
        .unwrap();
        let theme_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO theme_report_articles (theme_id, fnord_id, topic_score) VALUES (?, ?, 0.5)",
            [theme_id, fnord_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO briefings (period_type, period_start, period_end, content)
             VALUES ('daily', datetime('now'), datetime('now'), 'test briefing')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO analysis_cache (content_hash, summary, categories, keywords, political_bias, sachlichkeit, article_type)
             VALUES ('h1', 's', '[]', '[]', 0, 2, 'news')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO corpus_stats (term, document_count) VALUES ('merkel', 5)",
            [],
        )
        .unwrap();
    }

    /// Helper: row count, or 0 if table is empty/missing.
    fn count(db: &Database, table: &str) -> i64 {
        db.conn()
            .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))
            .unwrap_or(0)
    }

    #[test]
    fn test_reset_clears_all_article_tables() {
        let db = Database::new_in_memory().unwrap();
        seed_reset_fixtures(&db);

        // Sanity check — fixture actually populated cleared tables
        assert!(count(&db, "fnords") > 0, "fixture should insert a fnord");
        assert!(
            count(&db, "theme_reports") > 0,
            "fixture should insert a theme_report"
        );

        let result = perform_db_reset(db.conn(), None, |_, _, _, _| {}).unwrap();

        for table in TABLES_TO_CLEAR {
            assert_eq!(
                count(&db, table),
                0,
                "table {} should be empty after reset",
                table
            );
        }

        // Verify cascade cleared child tables too
        assert_eq!(count(&db, "fnord_immanentize"), 0);
        assert_eq!(count(&db, "fnord_sephiroth"), 0);
        assert_eq!(count(&db, "fnord_entities"), 0);
        assert_eq!(count(&db, "theme_report_themes"), 0);
        assert_eq!(count(&db, "theme_report_articles"), 0);

        // Result shape
        assert_eq!(result.tables_cleared.len(), TABLES_TO_CLEAR.len());
        assert!(
            result.tables_cleared.iter().any(|t| t.name == "fnords"),
            "result must list fnords"
        );
    }

    #[test]
    fn test_reset_preserves_configuration() {
        let db = Database::new_in_memory().unwrap();
        seed_reset_fixtures(&db);

        let pentacles_before = count(&db, "pentacles");
        let stopwords_before = count(&db, "stopwords");
        let settings_before = count(&db, "settings");
        let sephiroth_before = count(&db, "sephiroth");
        let bias_before = count(&db, "bias_weights");

        // Guard against accidentally seeding 0 rows — test would trivially pass.
        assert!(pentacles_before > 0);
        assert!(stopwords_before > 0);
        assert!(settings_before > 0);
        assert!(sephiroth_before > 0, "schema::init should seed categories");
        assert!(bias_before > 0);

        perform_db_reset(db.conn(), None, |_, _, _, _| {}).unwrap();

        assert_eq!(count(&db, "pentacles"), pentacles_before);
        assert_eq!(count(&db, "stopwords"), stopwords_before);
        assert_eq!(count(&db, "settings"), settings_before);
        assert_eq!(count(&db, "sephiroth"), sephiroth_before);
        assert_eq!(count(&db, "bias_weights"), bias_before);

        // Verify settings value survived byte-identical
        let val: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = 'test_key'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(val, "test_val");
    }

    #[test]
    fn test_reset_emits_progress_events_in_order() {
        let db = Database::new_in_memory().unwrap();
        seed_reset_fixtures(&db);

        let mut events: Vec<(String, Option<String>)> = Vec::new();
        perform_db_reset(db.conn(), None, |phase, table, _, _| {
            events.push((phase.to_string(), table.map(|s| s.to_string())));
        })
        .unwrap();

        // First event: a "clearing" for the first table (backup is skipped in
        // tests because backup_path is None).
        assert_eq!(events.first().map(|e| e.0.as_str()), Some("clearing"));
        // Last event: "done"
        assert_eq!(events.last().map(|e| e.0.as_str()), Some("done"));
        // Must include "vacuum" phase
        assert!(
            events.iter().any(|(p, _)| p == "vacuum"),
            "must emit a vacuum phase event"
        );
        // Must have one clearing event per table-to-clear
        let clearing_count = events.iter().filter(|(p, _)| p == "clearing").count();
        assert_eq!(clearing_count, TABLES_TO_CLEAR.len());
    }

    #[test]
    fn test_reset_shadow_tables_empty() {
        let db = Database::new_in_memory().unwrap();
        seed_reset_fixtures(&db);

        perform_db_reset(db.conn(), None, |_, _, _, _| {}).unwrap();

        for shadow in VEC_SHADOW_TABLES {
            assert_eq!(
                count(&db, shadow),
                0,
                "shadow table {} must be empty after reset",
                shadow
            );
        }
    }

    #[test]
    fn test_reset_reduces_db_size() {
        let db = Database::new_in_memory().unwrap();
        seed_reset_fixtures(&db);

        // Stuff in some bulk so VACUUM has something to reclaim
        for i in 0..500 {
            db.conn()
                .execute(
                    "INSERT INTO immanentize (name) VALUES (?)",
                    [format!("kw_{}", i)],
                )
                .unwrap();
        }

        let result = perform_db_reset(db.conn(), None, |_, _, _, _| {}).unwrap();
        assert!(
            result.size_after_bytes <= result.size_before_bytes,
            "size must not grow: before {} after {}",
            result.size_before_bytes,
            result.size_after_bytes
        );
    }

    #[test]
    fn test_reset_writes_backup_file() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = Database::new_at_path(&db_path).unwrap();
        seed_reset_fixtures(&db);

        let backup_path = tmp.path().join("test.db.backup");
        perform_db_reset(db.conn(), Some(&backup_path), |_, _, _, _| {}).unwrap();

        assert!(
            backup_path.exists(),
            "backup file {} should exist",
            backup_path.display()
        );
        // Backup must be non-empty and contain the fnords row that was still
        // present at backup time.
        let backup_conn = rusqlite::Connection::open(&backup_path).unwrap();
        let backup_fnords: i64 = backup_conn
            .query_row("SELECT COUNT(*) FROM fnords", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            backup_fnords, 1,
            "backup must capture state BEFORE the delete"
        );
    }

    #[test]
    fn test_reset_idempotent() {
        // Running reset twice on an already-empty DB must succeed without panic
        // or residual rows — important for the UI "reset again" case.
        let db = Database::new_in_memory().unwrap();
        seed_reset_fixtures(&db);

        perform_db_reset(db.conn(), None, |_, _, _, _| {}).unwrap();
        perform_db_reset(db.conn(), None, |_, _, _, _| {}).unwrap();

        for table in TABLES_TO_CLEAR {
            assert_eq!(count(&db, table), 0);
        }
    }
}
