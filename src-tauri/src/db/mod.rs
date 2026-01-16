mod schema;
#[cfg(test)]
mod tests;

// Re-export stopword functions for use in commands
pub use schema::{reset_stopwords_to_default, restore_default_stopwords};

use log::info;
use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;
use std::path::PathBuf;
use std::sync::Once;
use tauri::AppHandle;
use thiserror::Error;

// Ensure sqlite-vec is registered exactly once
static SQLITE_VEC_INIT: Once = Once::new();

/// Register the bundled sqlite-vec extension.
/// This must be called before opening any database connections.
fn register_sqlite_vec() {
    SQLITE_VEC_INIT.call_once(|| {
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite3_vec_init as *const (),
            )));
        }
        info!("sqlite-vec extension registered successfully");
    });
}

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to determine data directory")]
    NoDataDir,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(app: &AppHandle) -> Result<Self, DbError> {
        // Register bundled sqlite-vec extension (must happen before opening connection)
        register_sqlite_vec();

        let db_path = Self::get_db_path(app)?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for better performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        // Initialize schema (includes vec0 virtual table)
        schema::init(&conn)?;

        Ok(Self { conn })
    }

    /// Create an in-memory database for testing
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, DbError> {
        register_sqlite_vec();
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        schema::init(&conn)?;
        Ok(Self { conn })
    }

    /// Create a database at a specific path (for testing)
    #[cfg(test)]
    pub fn new_at_path(path: &std::path::Path) -> Result<Self, DbError> {
        register_sqlite_vec();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        schema::init(&conn)?;
        Ok(Self { conn })
    }

    fn get_db_path(_app: &AppHandle) -> Result<PathBuf, DbError> {
        // Use project directory for database storage
        let cwd = std::env::current_dir().map_err(|_| DbError::NoDataDir)?;
        let data_dir = cwd.join("data");
        Ok(data_dir.join("fuckup.db"))
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Seeds development data for testing
    #[cfg(debug_assertions)]
    pub fn seed_dev_data(&self) -> Result<(), DbError> {
        // Check if we already have data
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM pentacles", [], |row| row.get(0))?;

        if count > 0 {
            return Ok(());
        }

        // Insert feeds from FEEDS.md
        let feeds = [
            ("https://augengeradeaus.net/feed/", "Augen geradeaus!", "Sicherheitspolitik und Bundeswehr", "https://augengeradeaus.net", 4),
            ("https://www.tagesschau.de/infoservices/alle-meldungen-100~rss2.xml", "tagesschau.de", "Alle Meldungen der ARD", "https://www.tagesschau.de", 5),
            ("https://rss.dw.com/xml/rss-de-all", "Deutsche Welle", "Nachrichten aus aller Welt", "https://www.dw.com", 4),
            ("http://feeds.bbci.co.uk/news/rss.xml", "BBC News", "Breaking news from the BBC", "https://www.bbc.co.uk/news", 5),
            ("https://www.bundeswehr.de/service/rss/de/517054/feed", "Bundeswehr", "Offizielle Nachrichten der Bundeswehr", "https://www.bundeswehr.de", 4),
            ("https://www.deutschlandfunk.de/nachrichten-100.rss", "Deutschlandfunk Nachrichten", "Aktuelle Nachrichten", "https://www.deutschlandfunk.de", 5),
            ("https://www.deutschlandfunk.de/politikportal-100.rss", "Deutschlandfunk Politik", "Politik-Nachrichten", "https://www.deutschlandfunk.de", 4),
            ("https://netzpolitik.org/feed/", "netzpolitik.org", "Plattform fuer digitale Freiheitsrechte", "https://netzpolitik.org", 4),
            ("https://linuxnews.de/feed/", "LinuxNews.de", "Linux und Open Source Nachrichten", "https://linuxnews.de", 3),
        ];

        for (url, title, description, site_url, quality) in feeds {
            self.conn.execute(
                "INSERT INTO pentacles (url, title, description, site_url, default_quality) VALUES (?1, ?2, ?3, ?4, ?5)",
                (url, title, description, site_url, quality),
            )?;
        }

        // Note: No sample articles - feeds will be synced in Phase 2

        Ok(())
    }
}
