//! Database maintenance commands

use crate::AppState;
use log::info;
use serde::Serialize;
use tauri::State;

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

    let db = state.db.lock().map_err(|e| e.to_string())?;
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
    conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", [])
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
}
