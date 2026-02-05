use rusqlite::Connection;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[allow(dead_code)] // Reserved for explicit transaction failure messages
    #[error("Transaction failed: {0}")]
    Failed(String),
}

/// Execute a function within a database transaction with automatic COMMIT/ROLLBACK handling.
///
/// This helper ensures proper transaction semantics:
/// - Begins transaction with BEGIN TRANSACTION
/// - Executes the provided function
/// - Commits on success
/// - Rolls back on error
///
/// # Example
/// ```rust
/// use crate::db::transaction::with_transaction;
///
/// let result = with_transaction(&conn, |conn| {
///     conn.execute("INSERT INTO fnords (title) VALUES (?1)", ["Test"])?;
///     conn.execute("UPDATE fnords SET status = ?1 WHERE id = ?2", ["revealed", 1])?;
///     Ok(())
/// })?;
/// ```
pub fn with_transaction<F, T>(conn: &Connection, f: F) -> Result<T, TransactionError>
where
    F: FnOnce(&Connection) -> Result<T, TransactionError>,
{
    // Begin transaction
    conn.execute("BEGIN TRANSACTION", [])?;

    // Execute the function
    let result = f(conn);

    // Handle commit or rollback
    match result {
        Ok(value) => {
            conn.execute("COMMIT", [])?;
            Ok(value)
        }
        Err(e) => {
            // Attempt rollback, but preserve original error
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}

/// Execute a function within a database transaction, converting errors to String for Tauri commands.
///
/// This is a convenience wrapper around `with_transaction` that converts errors to String,
/// which is commonly needed for Tauri commands that return `Result<T, String>`.
///
/// # Example
/// ```rust
/// use crate::db::transaction::with_transaction_result;
///
/// #[tauri::command]
/// pub fn update_article(
///     state: State<AppState>,
///     id: i64,
///     title: String,
/// ) -> Result<(), String> {
///     let db = state.db_conn()?;
///
///     with_transaction_result(db.conn(), |conn| {
///         conn.execute("UPDATE fnords SET title = ?1 WHERE id = ?2", [title, id])?;
///         Ok(())
///     })
/// }
/// ```
pub fn with_transaction_result<F, T>(conn: &Connection, f: F) -> Result<T, String>
where
    F: FnOnce(&Connection) -> Result<T, TransactionError>,
{
    with_transaction(conn, f).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_transaction_commits_on_success() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();

        let result = with_transaction(&conn, |conn| {
            conn.execute("INSERT INTO test (value) VALUES (?1)", ["test1"])?;
            conn.execute("INSERT INTO test (value) VALUES (?1)", ["test2"])?;
            Ok(())
        });

        assert!(result.is_ok());

        // Verify data was committed
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_transaction_rolls_back_on_error() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )
        .unwrap();

        let result = with_transaction(&conn, |conn| {
            conn.execute("INSERT INTO test (value) VALUES (?1)", ["test1"])?;
            // This should fail (NULL constraint)
            conn.execute("INSERT INTO test (value) VALUES (NULL)", [])?;
            Ok(())
        });

        assert!(result.is_err());

        // Verify rollback: no data should be in table
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_transaction_returns_value() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();

        let inserted_id = with_transaction(&conn, |conn| {
            conn.execute("INSERT INTO test (value) VALUES (?1)", ["test"])?;
            let id = conn.last_insert_rowid();
            Ok(id)
        })
        .unwrap();

        assert_eq!(inserted_id, 1);
    }

    #[test]
    fn test_transaction_result_converts_to_string() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )
        .unwrap();

        let result: Result<(), String> = with_transaction_result(&conn, |conn| {
            // Force an error
            conn.execute("INSERT INTO test (value) VALUES (NULL)", [])?;
            Ok(())
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("NOT NULL"));
    }
}
