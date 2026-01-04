mod schema;

use rusqlite::Connection;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use thiserror::Error;

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
        let db_path = Self::get_db_path(app)?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for better performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        // Initialize schema
        schema::init(&conn)?;

        Ok(Self { conn })
    }

    fn get_db_path(app: &AppHandle) -> Result<PathBuf, DbError> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|_| DbError::NoDataDir)?;
        Ok(data_dir.join("fuckup.db"))
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Seeds development data for testing
    #[cfg(debug_assertions)]
    pub fn seed_dev_data(&self) -> Result<(), DbError> {
        use chrono::Utc;

        // Check if we already have data
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM pentacles", [], |row| row.get(0))?;

        if count > 0 {
            return Ok(());
        }

        // Insert sample feeds (Pentacles)
        self.conn.execute(
            "INSERT INTO pentacles (url, title, description, site_url, default_quality) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                "https://www.heise.de/rss/heise-atom.xml",
                "heise online",
                "Nachrichten aus der Welt der IT",
                "https://www.heise.de",
                4,
            ),
        )?;

        self.conn.execute(
            "INSERT INTO pentacles (url, title, description, site_url, default_quality) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                "https://rss.golem.de/rss.php?feed=ATOM1.0",
                "Golem.de",
                "IT-News fuer Profis",
                "https://www.golem.de",
                4,
            ),
        )?;

        self.conn.execute(
            "INSERT INTO pentacles (url, title, description, site_url, default_quality) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                "https://www.tagesschau.de/index~rss2.xml",
                "tagesschau.de",
                "Die Nachrichten der ARD",
                "https://www.tagesschau.de",
                5,
            ),
        )?;

        // Insert sample articles (Fnords)
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, author, content_raw, published_at, status, article_type, political_bias, sachlichkeit, quality_score)
               VALUES (1, 'heise-001', 'https://heise.de/article/1', 'EU verabschiedet AI Act', 'Max Mustermann',
                       'Die Europäische Union hat heute den AI Act final verabschiedet. Das Gesetz sieht vor, dass Hochrisiko-KI-Anwendungen künftig strenge Auflagen erfüllen müssen. Verboten werden unter anderem Social-Scoring-Systeme und biometrische Echtzeit-Überwachung.',
                       ?1, 'fnord', 'news', 0, 4, 4)"#,
            [&now],
        )?;

        self.conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, author, content_raw, published_at, status, article_type, political_bias, sachlichkeit, quality_score)
               VALUES (1, 'heise-002', 'https://heise.de/article/2', 'Linux 6.12 mit wichtigen Neuerungen', 'Julia Schmidt',
                       'Der neue Linux-Kernel 6.12 bringt zahlreiche Verbesserungen mit. Besonders hervorzuheben sind die optimierte Speicherverwaltung und die erweiterte Hardware-Unterstützung für aktuelle Grafikkarten.',
                       ?1, 'fnord', 'news', 0, 4, 4)"#,
            [&now],
        )?;

        self.conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, author, content_raw, published_at, status, article_type, political_bias, sachlichkeit, quality_score)
               VALUES (2, 'golem-001', 'https://golem.de/article/1', 'Rust 2024 Edition angekündigt', 'Peter Meier',
                       'Die Rust Foundation hat die neue 2024 Edition der Programmiersprache angekündigt. Zu den Highlights gehören verbesserte async/await-Unterstützung und neue Compiler-Optimierungen.',
                       ?1, 'illuminated', 'news', 0, 4, 4)"#,
            [&now],
        )?;

        self.conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, author, content_raw, published_at, status, article_type, political_bias, sachlichkeit, quality_score)
               VALUES (3, 'tagesschau-001', 'https://tagesschau.de/article/1', 'Bundestagsdebatte zur Digitalisierung', NULL,
                       'Im Bundestag wurde heute über den Stand der Digitalisierung in Deutschland debattiert. Opposition kritisiert langsames Tempo, Regierung verteidigt bisherige Maßnahmen.',
                       ?1, 'fnord', 'news', 0, 3, 5)"#,
            [&now],
        )?;

        self.conn.execute(
            r#"INSERT INTO fnords (pentacle_id, guid, url, title, author, content_raw, published_at, status, article_type, political_bias, sachlichkeit, quality_score)
               VALUES (3, 'tagesschau-002', 'https://tagesschau.de/article/2', 'Kommentar: Warum die KI-Regulierung richtig ist', 'Anna Beispiel',
                       'Die EU hat mit dem AI Act ein wichtiges Signal gesendet. Es ist höchste Zeit, dass wir als Gesellschaft klare Regeln für den Einsatz von KI aufstellen. Dieser Kommentar erläutert die Gründe.',
                       ?1, 'golden_apple', 'opinion', -1, 2, 4)"#,
            [&now],
        )?;

        Ok(())
    }
}
