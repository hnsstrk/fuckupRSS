//! Database maintenance commands

use crate::error::CmdResult;
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
