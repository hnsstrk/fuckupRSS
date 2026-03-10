use crate::text_analysis::keyword_seeds::{seed_known_keywords, update_types_from_seeds};
use log::info;
use rusqlite::Connection;

/// Default stopwords embedded at compile time from txt files
const STOPWORDS_DE: &str = include_str!("../../resources/stopwords/de.txt");
const STOPWORDS_EN: &str = include_str!("../../resources/stopwords/en.txt");
const STOPWORDS_TECHNICAL: &str = include_str!("../../resources/stopwords/technical.txt");
const STOPWORDS_NEWS: &str = include_str!("../../resources/stopwords/news.txt");

/// Parse a stopword file (lines starting with # are comments, empty lines are skipped)
fn parse_stopword_file(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_lowercase())
        .collect()
}

/// Restore default stopwords from embedded txt files
///
/// This function is called during database initialization when the stopwords table is empty.
/// It ensures that a curated list of stopwords is always available, even when the database is
/// newly created or re-initialized.
///
/// Returns the number of stopwords that were restored.
pub fn restore_default_stopwords(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO stopwords (word, source, language) VALUES (?1, 'system', ?2)",
    )?;
    let mut count = 0;

    // Load German stopwords
    for word in parse_stopword_file(STOPWORDS_DE) {
        if stmt.execute(rusqlite::params![&word, "de"])? > 0 {
            count += 1;
        }
    }

    // Load English stopwords
    for word in parse_stopword_file(STOPWORDS_EN) {
        if stmt.execute(rusqlite::params![&word, "en"])? > 0 {
            count += 1;
        }
    }

    // Load technical stopwords
    for word in parse_stopword_file(STOPWORDS_TECHNICAL) {
        if stmt.execute(rusqlite::params![&word, "technical"])? > 0 {
            count += 1;
        }
    }

    // Load news stopwords
    for word in parse_stopword_file(STOPWORDS_NEWS) {
        if stmt.execute(rusqlite::params![&word, "news"])? > 0 {
            count += 1;
        }
    }

    if count > 0 {
        info!(
            "Restored {} default stopwords from embedded txt files",
            count
        );
    }

    Ok(count)
}

/// Reset stopwords to default (removes all user stopwords and re-seeds system stopwords)
pub fn reset_stopwords_to_default(conn: &Connection) -> Result<usize, rusqlite::Error> {
    // Delete all stopwords
    conn.execute("DELETE FROM stopwords", [])?;
    // Re-seed with defaults
    restore_default_stopwords(conn)
}

/// Run migrations for existing databases
fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Check if content_hash column exists
    let has_content_hash: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('fnords') WHERE name = 'content_hash'")?
        .query_row([], |row| row.get(0))?;

    if !has_content_hash {
        // Add new columns for change detection
        conn.execute_batch(
            r#"
            ALTER TABLE fnords ADD COLUMN content_hash TEXT;
            ALTER TABLE fnords ADD COLUMN has_changes BOOLEAN DEFAULT FALSE;
            ALTER TABLE fnords ADD COLUMN changed_at DATETIME;
            ALTER TABLE fnords ADD COLUMN revision_count INTEGER DEFAULT 0;
            "#,
        )?;
    }

    // Create index for has_changes (after migration ensures column exists)
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_fnords_has_changes ON fnords(has_changes);",
    )?;

    // Rename "Tech" to "Technik" if exists AND "Technik" doesn't exist yet
    // This prevents UNIQUE constraint violation if both exist
    let technik_exists: bool = conn
        .prepare("SELECT COUNT(*) FROM sephiroth WHERE name = 'Technik'")?
        .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

    if !technik_exists {
        conn.execute(
            "UPDATE sephiroth SET name = 'Technik' WHERE name = 'Tech'",
            [],
        )?;
    } else {
        // If "Technik" already exists, just delete the old "Tech" entry (if any)
        conn.execute("DELETE FROM sephiroth WHERE name = 'Tech'", [])?;
    }

    // Add source and assigned_at columns to fnord_sephiroth if missing
    let has_source: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('fnord_sephiroth') WHERE name = 'source'")?
        .query_row([], |row| row.get(0))?;

    if !has_source {
        conn.execute_batch(
            r#"
            ALTER TABLE fnord_sephiroth ADD COLUMN source TEXT DEFAULT 'ai' CHECK(source IN ('ai', 'statistical', 'manual'));
            ALTER TABLE fnord_sephiroth ADD COLUMN assigned_at DATETIME DEFAULT CURRENT_TIMESTAMP;
            "#,
        )?;
    }

    // Create indexes for source and assigned_at (after migration ensures columns exist)
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_fnord_sephiroth_assigned ON fnord_sephiroth(assigned_at DESC);
        CREATE INDEX IF NOT EXISTS idx_fnord_sephiroth_source ON fnord_sephiroth(source);
        "#,
    )?;

    // Migration: Add 'statistical' to fnord_sephiroth source CHECK constraint
    // SQLite requires table recreation to modify CHECK constraints
    let needs_statistical_source: bool = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='fnord_sephiroth'",
            [],
            |row| {
                let sql: String = row.get(0)?;
                // Check if 'statistical' is NOT in the CHECK constraint
                Ok(!sql.contains("statistical"))
            },
        )
        .unwrap_or(false);

    if needs_statistical_source {
        conn.execute_batch(
            r#"
            -- Recreate fnord_sephiroth with updated CHECK constraint
            CREATE TABLE fnord_sephiroth_new (
                fnord_id INTEGER NOT NULL,
                sephiroth_id INTEGER NOT NULL,
                confidence REAL DEFAULT 1.0,
                source TEXT DEFAULT 'ai' CHECK(source IN ('ai', 'statistical', 'manual')),
                assigned_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (fnord_id, sephiroth_id),
                FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE,
                FOREIGN KEY (sephiroth_id) REFERENCES sephiroth(id) ON DELETE CASCADE
            );
            INSERT INTO fnord_sephiroth_new SELECT * FROM fnord_sephiroth;
            DROP TABLE fnord_sephiroth;
            ALTER TABLE fnord_sephiroth_new RENAME TO fnord_sephiroth;
            CREATE INDEX IF NOT EXISTS idx_fnord_sephiroth_assigned ON fnord_sephiroth(assigned_at DESC);
            CREATE INDEX IF NOT EXISTS idx_fnord_sephiroth_source ON fnord_sephiroth(source);
            "#,
        )?;
    }

    // Add confidence column to fnord_sephiroth if missing (for consistency with fnord_immanentize)
    let has_fs_confidence: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('fnord_sephiroth') WHERE name = 'confidence'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_fs_confidence {
        conn.execute_batch(
            r#"
            ALTER TABLE fnord_sephiroth ADD COLUMN confidence REAL DEFAULT 1.0;
            "#,
        )?;
    }

    // Migration for Immanentize Network
    let has_article_count: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('immanentize') WHERE name = 'article_count'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_article_count {
        // Add new columns to immanentize
        conn.execute_batch(
            r#"
            ALTER TABLE immanentize ADD COLUMN article_count INTEGER DEFAULT 0;
            ALTER TABLE immanentize ADD COLUMN embedding_at DATETIME;
            ALTER TABLE immanentize ADD COLUMN cluster_id INTEGER;
            ALTER TABLE immanentize ADD COLUMN is_canonical BOOLEAN DEFAULT TRUE;
            ALTER TABLE immanentize ADD COLUMN canonical_id INTEGER;
            ALTER TABLE immanentize ADD COLUMN first_seen DATETIME DEFAULT CURRENT_TIMESTAMP;
            "#,
        )?;

        // Update article_count from existing fnord_immanentize data
        conn.execute(
            r#"
            UPDATE immanentize SET article_count = (
                SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize
                WHERE immanentize_id = immanentize.id
            )
            "#,
            [],
        )?;
    }

    // Create immanentize_clusters if not exists (for existing DBs)
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS immanentize_clusters (
            id INTEGER PRIMARY KEY,
            name TEXT,
            description TEXT,
            auto_generated BOOLEAN DEFAULT TRUE,
            keyword_count INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS immanentize_sephiroth (
            immanentize_id INTEGER NOT NULL,
            sephiroth_id INTEGER NOT NULL,
            weight REAL DEFAULT 1.0,
            article_count INTEGER DEFAULT 1,
            first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (immanentize_id, sephiroth_id),
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE,
            FOREIGN KEY (sephiroth_id) REFERENCES sephiroth(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS immanentize_neighbors (
            immanentize_id_a INTEGER NOT NULL,
            immanentize_id_b INTEGER NOT NULL,
            cooccurrence INTEGER DEFAULT 1,
            embedding_similarity REAL,
            combined_weight REAL DEFAULT 0.0,
            first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (immanentize_id_a, immanentize_id_b),
            FOREIGN KEY (immanentize_id_a) REFERENCES immanentize(id) ON DELETE CASCADE,
            FOREIGN KEY (immanentize_id_b) REFERENCES immanentize(id) ON DELETE CASCADE,
            CHECK (immanentize_id_a < immanentize_id_b)
        );

        -- Create missing indexes
        CREATE INDEX IF NOT EXISTS idx_immanentize_cluster ON immanentize(cluster_id);
        CREATE INDEX IF NOT EXISTS idx_immanentize_canonical ON immanentize(canonical_id);
        CREATE INDEX IF NOT EXISTS idx_immanentize_article_count ON immanentize(article_count DESC);
        CREATE INDEX IF NOT EXISTS idx_immanentize_sephiroth_seph ON immanentize_sephiroth(sephiroth_id);
        CREATE INDEX IF NOT EXISTS idx_immanentize_neighbors_a ON immanentize_neighbors(immanentize_id_a);
        CREATE INDEX IF NOT EXISTS idx_immanentize_neighbors_b ON immanentize_neighbors(immanentize_id_b);
        CREATE INDEX IF NOT EXISTS idx_immanentize_neighbors_weight ON immanentize_neighbors(combined_weight DESC);
        CREATE INDEX IF NOT EXISTS idx_fnord_immanentize_imm ON fnord_immanentize(immanentize_id);
        "#,
    )?;

    // Migration: Add content_full to fnord_revisions for existing databases
    let has_revision_content_full: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('fnord_revisions') WHERE name = 'content_full'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_revision_content_full {
        conn.execute_batch("ALTER TABLE fnord_revisions ADD COLUMN content_full TEXT;")?;
    }

    // Migration for immanentize_daily table (trend data)
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS immanentize_daily (
            immanentize_id INTEGER NOT NULL,
            date TEXT NOT NULL,
            count INTEGER DEFAULT 0,
            PRIMARY KEY (immanentize_id, date),
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_immanentize_daily_date ON immanentize_daily(date DESC);
        CREATE INDEX IF NOT EXISTS idx_immanentize_daily_id ON immanentize_daily(immanentize_id);
        "#,
    )?;

    // Backfill immanentize_daily from existing data (only if table is empty)
    let daily_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM immanentize_daily", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    if daily_count == 0 {
        // Aggregate existing fnord_immanentize data by date
        conn.execute(
            r#"INSERT OR IGNORE INTO immanentize_daily (immanentize_id, date, count)
               SELECT fi.immanentize_id,
                      DATE(COALESCE(f.published_at, f.fetched_at)) as date,
                      COUNT(*) as count
               FROM fnord_immanentize fi
               JOIN fnords f ON f.id = fi.fnord_id
               WHERE DATE(COALESCE(f.published_at, f.fetched_at)) IS NOT NULL
               GROUP BY fi.immanentize_id, DATE(COALESCE(f.published_at, f.fetched_at))"#,
            [],
        )?;
    }

    // Migration: Add quality_score and embedding columns to immanentize
    let has_quality_score: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('immanentize') WHERE name = 'quality_score'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_quality_score {
        conn.execute_batch(
            r#"
            ALTER TABLE immanentize ADD COLUMN quality_score REAL DEFAULT NULL;
            ALTER TABLE immanentize ADD COLUMN embedding BLOB DEFAULT NULL;
            ALTER TABLE immanentize ADD COLUMN quality_calculated_at DATETIME DEFAULT NULL;
            "#,
        )?;
    }

    // Create index for quality_score (after migration ensures column exists)
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_immanentize_quality ON immanentize(quality_score DESC);
        "#,
    )?;

    // Table for dismissed synonym pairs
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS dismissed_synonyms (
            keyword_a_id INTEGER NOT NULL,
            keyword_b_id INTEGER NOT NULL,
            dismissed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (keyword_a_id, keyword_b_id),
            FOREIGN KEY (keyword_a_id) REFERENCES immanentize(id) ON DELETE CASCADE,
            FOREIGN KEY (keyword_b_id) REFERENCES immanentize(id) ON DELETE CASCADE
        );
        "#,
    )?;

    // Table for embedding queue (automatic embedding generation)
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS embedding_queue (
            id INTEGER PRIMARY KEY,
            immanentize_id INTEGER NOT NULL UNIQUE,
            priority INTEGER DEFAULT 0,
            queued_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            attempts INTEGER DEFAULT 0,
            last_error TEXT,
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_embedding_queue_priority ON embedding_queue(priority DESC, queued_at ASC);
        "#,
    )?;

    // Migration for analysis retry fields
    let has_analysis_attempts: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('fnords') WHERE name = 'analysis_attempts'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_analysis_attempts {
        conn.execute_batch(
            r#"
            ALTER TABLE fnords ADD COLUMN analysis_attempts INTEGER DEFAULT 0;
            ALTER TABLE fnords ADD COLUMN analysis_error TEXT;
            ALTER TABLE fnords ADD COLUMN analysis_hopeless BOOLEAN DEFAULT FALSE;
            "#,
        )?;
    }

    // Index for finding articles that need (re)analysis
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_fnords_analysis_status ON fnords(analysis_hopeless, analysis_attempts, processed_at);
        "#,
    )?;

    // Migration 8: Add embedding-related indexes for performance
    conn.execute_batch(
        r#"
        -- Index for finding keywords without embeddings (used by queue system)
        CREATE INDEX IF NOT EXISTS idx_immanentize_no_embedding
            ON immanentize(article_count DESC)
            WHERE embedding IS NULL;

        -- Index for finding keywords without quality scores
        CREATE INDEX IF NOT EXISTS idx_immanentize_no_quality
            ON immanentize(id)
            WHERE embedding IS NOT NULL AND quality_score IS NULL;

        -- Index for finding neighbor pairs without similarity calculated
        CREATE INDEX IF NOT EXISTS idx_neighbors_no_similarity
            ON immanentize_neighbors(immanentize_id_a, immanentize_id_b)
            WHERE embedding_similarity IS NULL;
        "#,
    )?;

    // Migration 9: Create vec0 virtual table for fast vector similarity search
    // This enables O(log n) approximate nearest neighbor search via sqlite-vec
    let has_vec_table: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_immanentize'",
        )?
        .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

    if !has_vec_table {
        // Create virtual table for 1024-dim embeddings (snowflake-arctic-embed2)
        // Using cosine distance metric for semantic similarity search
        conn.execute(
            "CREATE VIRTUAL TABLE vec_immanentize USING vec0(immanentize_id INTEGER PRIMARY KEY, embedding float[1024] distance_metric=cosine)",
            [],
        )?;

        // Populate from existing embeddings
        conn.execute(
            r#"INSERT INTO vec_immanentize (immanentize_id, embedding)
               SELECT id, embedding FROM immanentize WHERE embedding IS NOT NULL"#,
            [],
        )?;
    }

    // Migration 10: Add embedding column to fnords table for article embeddings
    let has_fnord_embedding: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('fnords') WHERE name = 'embedding'")?
        .query_row([], |row| row.get(0))?;

    if !has_fnord_embedding {
        conn.execute_batch(
            r#"
            ALTER TABLE fnords ADD COLUMN embedding BLOB DEFAULT NULL;
            ALTER TABLE fnords ADD COLUMN embedding_at DATETIME DEFAULT NULL;
            "#,
        )?;
    }

    // Create vec_fnords virtual table for fast article similarity search
    let has_vec_fnords: bool = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_fnords'")?
        .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

    if !has_vec_fnords {
        // Create virtual table for 1024-dim embeddings (snowflake-arctic-embed2)
        conn.execute(
            "CREATE VIRTUAL TABLE vec_fnords USING vec0(fnord_id INTEGER PRIMARY KEY, embedding float[1024] distance_metric=cosine)",
            [],
        )?;

        // Populate from existing embeddings (if any)
        conn.execute(
            r#"INSERT INTO vec_fnords (fnord_id, embedding)
               SELECT id, embedding FROM fnords WHERE embedding IS NOT NULL"#,
            [],
        )?;
    }

    // Index for finding articles without embeddings
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_fnords_no_embedding
            ON fnords(processed_at DESC)
            WHERE embedding IS NULL AND processed_at IS NOT NULL;
        "#,
    )?;

    // Migration 11: Add parent_id and level columns to sephiroth for hierarchical categories
    let has_parent_id: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('sephiroth') WHERE name = 'parent_id'")?
        .query_row([], |row| row.get(0))?;

    // Check if migration data restructuring is complete (main category "Wissen & Technologie" exists)
    let migration_complete: bool = conn
        .prepare("SELECT COUNT(*) FROM sephiroth WHERE id = 1 AND name LIKE '%&%'")?
        .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

    // Check if sephiroth.name has UNIQUE constraint (needs to be removed for hierarchical categories)
    let has_unique_name: bool = {
        let schema: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='sephiroth'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();
        schema.contains("name TEXT NOT NULL UNIQUE")
    };

    // Migration 11a: Remove UNIQUE constraint from sephiroth.name
    // This is needed because subcategories have the same names as some main categories
    if has_unique_name && has_parent_id {
        conn.execute("PRAGMA foreign_keys = OFF", [])?;

        conn.execute_batch(
            r#"
            -- Create new table without UNIQUE constraint on name
            CREATE TABLE sephiroth_new (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                color TEXT,
                icon TEXT,
                article_count INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                parent_id INTEGER REFERENCES sephiroth(id) ON DELETE CASCADE,
                level INTEGER DEFAULT 0
            );

            -- Copy data
            INSERT INTO sephiroth_new SELECT * FROM sephiroth;

            -- Drop old table
            DROP TABLE sephiroth;

            -- Rename new table
            ALTER TABLE sephiroth_new RENAME TO sephiroth;

            -- Recreate indexes
            CREATE INDEX idx_sephiroth_parent ON sephiroth(parent_id);
            CREATE INDEX idx_sephiroth_level ON sephiroth(level);
            "#,
        )?;

        conn.execute("PRAGMA foreign_keys = ON", [])?;
    }

    if !has_parent_id {
        // Temporarily disable foreign key constraints for this migration
        conn.execute("PRAGMA foreign_keys = OFF", [])?;

        conn.execute_batch(
            r#"
            ALTER TABLE sephiroth ADD COLUMN parent_id INTEGER REFERENCES sephiroth(id) ON DELETE CASCADE;
            ALTER TABLE sephiroth ADD COLUMN level INTEGER DEFAULT 0;
            "#,
        )?;

        // Migration: Convert old flat categories (IDs 1-13) to new hierarchical structure
        // Old structure: Technik(1), Politik(2), Wirtschaft(3), Wissenschaft(4), Kultur(5),
        //                Sport(6), Gesellschaft(7), Umwelt(8), Sicherheit(9), Gesundheit(10),
        //                Verteidigung(11), Energie(12), Recht(13)
        // New structure: 6 main categories (IDs 1-6) + 13 subcategories (IDs 101-602)

        // Step 1: Map old category IDs to new subcategory IDs in fnord_sephiroth
        // This preserves all article-category associations
        conn.execute_batch(
            r#"
            UPDATE fnord_sephiroth SET sephiroth_id = 101 WHERE sephiroth_id = 1;  -- Technik -> 101
            UPDATE fnord_sephiroth SET sephiroth_id = 201 WHERE sephiroth_id = 2;  -- Politik -> 201
            UPDATE fnord_sephiroth SET sephiroth_id = 301 WHERE sephiroth_id = 3;  -- Wirtschaft -> 301
            UPDATE fnord_sephiroth SET sephiroth_id = 102 WHERE sephiroth_id = 4;  -- Wissenschaft -> 102
            UPDATE fnord_sephiroth SET sephiroth_id = 601 WHERE sephiroth_id = 5;  -- Kultur -> 601
            UPDATE fnord_sephiroth SET sephiroth_id = 602 WHERE sephiroth_id = 6;  -- Sport -> 602
            UPDATE fnord_sephiroth SET sephiroth_id = 202 WHERE sephiroth_id = 7;  -- Gesellschaft -> 202
            UPDATE fnord_sephiroth SET sephiroth_id = 401 WHERE sephiroth_id = 8;  -- Umwelt -> 401
            UPDATE fnord_sephiroth SET sephiroth_id = 501 WHERE sephiroth_id = 9;  -- Sicherheit -> 501
            UPDATE fnord_sephiroth SET sephiroth_id = 402 WHERE sephiroth_id = 10; -- Gesundheit -> 402
            UPDATE fnord_sephiroth SET sephiroth_id = 502 WHERE sephiroth_id = 11; -- Verteidigung -> 502
            UPDATE fnord_sephiroth SET sephiroth_id = 302 WHERE sephiroth_id = 12; -- Energie -> 302
            UPDATE fnord_sephiroth SET sephiroth_id = 203 WHERE sephiroth_id = 13; -- Recht -> 203
            "#,
        )?;

        // Step 2: Also update immanentize_sephiroth associations
        conn.execute_batch(
            r#"
            UPDATE immanentize_sephiroth SET sephiroth_id = 101 WHERE sephiroth_id = 1;
            UPDATE immanentize_sephiroth SET sephiroth_id = 201 WHERE sephiroth_id = 2;
            UPDATE immanentize_sephiroth SET sephiroth_id = 301 WHERE sephiroth_id = 3;
            UPDATE immanentize_sephiroth SET sephiroth_id = 102 WHERE sephiroth_id = 4;
            UPDATE immanentize_sephiroth SET sephiroth_id = 601 WHERE sephiroth_id = 5;
            UPDATE immanentize_sephiroth SET sephiroth_id = 602 WHERE sephiroth_id = 6;
            UPDATE immanentize_sephiroth SET sephiroth_id = 202 WHERE sephiroth_id = 7;
            UPDATE immanentize_sephiroth SET sephiroth_id = 401 WHERE sephiroth_id = 8;
            UPDATE immanentize_sephiroth SET sephiroth_id = 501 WHERE sephiroth_id = 9;
            UPDATE immanentize_sephiroth SET sephiroth_id = 402 WHERE sephiroth_id = 10;
            UPDATE immanentize_sephiroth SET sephiroth_id = 502 WHERE sephiroth_id = 11;
            UPDATE immanentize_sephiroth SET sephiroth_id = 302 WHERE sephiroth_id = 12;
            UPDATE immanentize_sephiroth SET sephiroth_id = 203 WHERE sephiroth_id = 13;
            "#,
        )?;

        // Step 3: Delete old categories
        conn.execute("DELETE FROM sephiroth WHERE id BETWEEN 1 AND 13", [])?;

        // Step 4: Insert new main categories (level 0)
        conn.execute_batch(
            r#"
            INSERT INTO sephiroth (id, name, parent_id, level, icon) VALUES
                (1, 'Wissen & Technologie', NULL, 0, 'fa-solid fa-microchip'),
                (2, 'Politik & Gesellschaft', NULL, 0, 'fa-solid fa-landmark'),
                (3, 'Wirtschaft', NULL, 0, 'fa-solid fa-chart-line'),
                (4, 'Umwelt & Gesundheit', NULL, 0, 'fa-solid fa-leaf'),
                (5, 'Sicherheit', NULL, 0, 'fa-solid fa-shield-halved'),
                (6, 'Kultur & Leben', NULL, 0, 'fa-solid fa-masks-theater');
            "#,
        )?;

        // Step 5: Insert new subcategories (level 1)
        conn.execute_batch(
            r#"
            INSERT INTO sephiroth (id, name, parent_id, level, icon) VALUES
                (101, 'Technik', 1, 1, 'fa-solid fa-microchip'),
                (102, 'Wissenschaft', 1, 1, 'fa-solid fa-flask'),
                (201, 'Politik', 2, 1, 'fa-solid fa-landmark'),
                (202, 'Gesellschaft', 2, 1, 'fa-solid fa-users'),
                (203, 'Recht', 2, 1, 'fa-solid fa-scale-balanced'),
                (301, 'Wirtschaft', 3, 1, 'fa-solid fa-chart-line'),
                (302, 'Energie', 3, 1, 'fa-solid fa-bolt'),
                (401, 'Umwelt', 4, 1, 'fa-solid fa-leaf'),
                (402, 'Gesundheit', 4, 1, 'fa-solid fa-heart-pulse'),
                (501, 'Sicherheit', 5, 1, 'fa-solid fa-shield-halved'),
                (502, 'Verteidigung', 5, 1, 'fa-solid fa-medal'),
                (601, 'Kultur', 6, 1, 'fa-solid fa-masks-theater'),
                (602, 'Sport', 6, 1, 'fa-solid fa-futbol');
            "#,
        )?;

        // Re-enable foreign key constraints
        conn.execute("PRAGMA foreign_keys = ON", [])?;
    } else if !migration_complete {
        // Columns exist but data restructuring not complete (e.g., previous migration attempt failed)
        // Temporarily disable foreign key constraints for this migration
        conn.execute("PRAGMA foreign_keys = OFF", [])?;

        // Step 1: Map old category IDs to new subcategory IDs in fnord_sephiroth
        conn.execute_batch(
            r#"
            UPDATE fnord_sephiroth SET sephiroth_id = 101 WHERE sephiroth_id = 1;  -- Technik -> 101
            UPDATE fnord_sephiroth SET sephiroth_id = 201 WHERE sephiroth_id = 2;  -- Politik -> 201
            UPDATE fnord_sephiroth SET sephiroth_id = 301 WHERE sephiroth_id = 3;  -- Wirtschaft -> 301
            UPDATE fnord_sephiroth SET sephiroth_id = 102 WHERE sephiroth_id = 4;  -- Wissenschaft -> 102
            UPDATE fnord_sephiroth SET sephiroth_id = 601 WHERE sephiroth_id = 5;  -- Kultur -> 601
            UPDATE fnord_sephiroth SET sephiroth_id = 602 WHERE sephiroth_id = 6;  -- Sport -> 602
            UPDATE fnord_sephiroth SET sephiroth_id = 202 WHERE sephiroth_id = 7;  -- Gesellschaft -> 202
            UPDATE fnord_sephiroth SET sephiroth_id = 401 WHERE sephiroth_id = 8;  -- Umwelt -> 401
            UPDATE fnord_sephiroth SET sephiroth_id = 501 WHERE sephiroth_id = 9;  -- Sicherheit -> 501
            UPDATE fnord_sephiroth SET sephiroth_id = 402 WHERE sephiroth_id = 10; -- Gesundheit -> 402
            UPDATE fnord_sephiroth SET sephiroth_id = 502 WHERE sephiroth_id = 11; -- Verteidigung -> 502
            UPDATE fnord_sephiroth SET sephiroth_id = 302 WHERE sephiroth_id = 12; -- Energie -> 302
            UPDATE fnord_sephiroth SET sephiroth_id = 203 WHERE sephiroth_id = 13; -- Recht -> 203
            "#,
        )?;

        // Step 2: Also update immanentize_sephiroth associations
        conn.execute_batch(
            r#"
            UPDATE immanentize_sephiroth SET sephiroth_id = 101 WHERE sephiroth_id = 1;
            UPDATE immanentize_sephiroth SET sephiroth_id = 201 WHERE sephiroth_id = 2;
            UPDATE immanentize_sephiroth SET sephiroth_id = 301 WHERE sephiroth_id = 3;
            UPDATE immanentize_sephiroth SET sephiroth_id = 102 WHERE sephiroth_id = 4;
            UPDATE immanentize_sephiroth SET sephiroth_id = 601 WHERE sephiroth_id = 5;
            UPDATE immanentize_sephiroth SET sephiroth_id = 602 WHERE sephiroth_id = 6;
            UPDATE immanentize_sephiroth SET sephiroth_id = 202 WHERE sephiroth_id = 7;
            UPDATE immanentize_sephiroth SET sephiroth_id = 401 WHERE sephiroth_id = 8;
            UPDATE immanentize_sephiroth SET sephiroth_id = 501 WHERE sephiroth_id = 9;
            UPDATE immanentize_sephiroth SET sephiroth_id = 402 WHERE sephiroth_id = 10;
            UPDATE immanentize_sephiroth SET sephiroth_id = 502 WHERE sephiroth_id = 11;
            UPDATE immanentize_sephiroth SET sephiroth_id = 302 WHERE sephiroth_id = 12;
            UPDATE immanentize_sephiroth SET sephiroth_id = 203 WHERE sephiroth_id = 13;
            "#,
        )?;

        // Step 3: Delete old categories
        conn.execute("DELETE FROM sephiroth WHERE id BETWEEN 1 AND 13", [])?;

        // Step 4: Insert new main categories (level 0)
        conn.execute_batch(
            r#"
            INSERT INTO sephiroth (id, name, parent_id, level, icon) VALUES
                (1, 'Wissen & Technologie', NULL, 0, 'fa-solid fa-microchip'),
                (2, 'Politik & Gesellschaft', NULL, 0, 'fa-solid fa-landmark'),
                (3, 'Wirtschaft', NULL, 0, 'fa-solid fa-chart-line'),
                (4, 'Umwelt & Gesundheit', NULL, 0, 'fa-solid fa-leaf'),
                (5, 'Sicherheit', NULL, 0, 'fa-solid fa-shield-halved'),
                (6, 'Kultur & Leben', NULL, 0, 'fa-solid fa-masks-theater');
            "#,
        )?;

        // Step 5: Insert new subcategories (level 1)
        conn.execute_batch(
            r#"
            INSERT INTO sephiroth (id, name, parent_id, level, icon) VALUES
                (101, 'Technik', 1, 1, 'fa-solid fa-microchip'),
                (102, 'Wissenschaft', 1, 1, 'fa-solid fa-flask'),
                (201, 'Politik', 2, 1, 'fa-solid fa-landmark'),
                (202, 'Gesellschaft', 2, 1, 'fa-solid fa-users'),
                (203, 'Recht', 2, 1, 'fa-solid fa-scale-balanced'),
                (301, 'Wirtschaft', 3, 1, 'fa-solid fa-chart-line'),
                (302, 'Energie', 3, 1, 'fa-solid fa-bolt'),
                (401, 'Umwelt', 4, 1, 'fa-solid fa-leaf'),
                (402, 'Gesundheit', 4, 1, 'fa-solid fa-heart-pulse'),
                (501, 'Sicherheit', 5, 1, 'fa-solid fa-shield-halved'),
                (502, 'Verteidigung', 5, 1, 'fa-solid fa-medal'),
                (601, 'Kultur', 6, 1, 'fa-solid fa-masks-theater'),
                (602, 'Sport', 6, 1, 'fa-solid fa-futbol');
            "#,
        )?;

        // Re-enable foreign key constraints
        conn.execute("PRAGMA foreign_keys = ON", [])?;
    }

    // Category colors are now managed via CSS variables (--category-1 through --category-6)
    // No hardcoded colors in the database - the frontend uses getCategoryColorVar() helper

    // Migration 12: Add source and confidence to fnord_immanentize for statistical analysis
    let has_fi_source: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('fnord_immanentize') WHERE name = 'source'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_fi_source {
        conn.execute_batch(
            r#"
            ALTER TABLE fnord_immanentize ADD COLUMN source TEXT DEFAULT 'ai' CHECK(source IN ('ai', 'statistical', 'manual'));
            ALTER TABLE fnord_immanentize ADD COLUMN confidence REAL DEFAULT 1.0;
            "#,
        )?;
    }

    // Create indexes for fnord_immanentize source and confidence
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_fnord_immanentize_source ON fnord_immanentize(source);
        "#,
    )?;

    // Migration 13: Create bias_weights table for learning system
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS bias_weights (
            id INTEGER PRIMARY KEY,
            weight_type TEXT NOT NULL,  -- 'keyword_boost', 'category_term', 'source_weight'
            context_key TEXT NOT NULL,  -- Keyword-Name, Kategorie-ID, oder Source-Typ
            term TEXT,                  -- Bei category_term: das gewichtete Wort
            weight REAL DEFAULT 1.0,
            correction_count INTEGER DEFAULT 0,
            last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_bias_weights_type_context ON bias_weights(weight_type, context_key);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_bias_weights_unique ON bias_weights(weight_type, context_key, COALESCE(term, ''));
        "#,
    )?;

    // Migration 14: Create corpus_stats table for corpus-wide TF-IDF
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS corpus_stats (
            term TEXT PRIMARY KEY,
            document_count INTEGER DEFAULT 1,
            last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_corpus_stats_count ON corpus_stats(document_count DESC);
        "#,
    )?;

    // Migration 15: Create stopwords table (system + user stopwords)
    // Check if we need to migrate from old user_stopwords table
    let has_user_stopwords_table: bool = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='user_stopwords'")?
        .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

    let has_stopwords_table: bool = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='stopwords'")?
        .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

    if !has_stopwords_table {
        // Create new stopwords table with source and language columns
        conn.execute_batch(
            r#"
            CREATE TABLE stopwords (
                id INTEGER PRIMARY KEY,
                word TEXT NOT NULL UNIQUE COLLATE NOCASE,
                source TEXT NOT NULL DEFAULT 'system' CHECK(source IN ('system', 'user')),
                language TEXT,  -- 'de', 'en', 'technical', 'news', or NULL for user
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX idx_stopwords_word ON stopwords(word);
            CREATE INDEX idx_stopwords_source ON stopwords(source);
            CREATE INDEX idx_stopwords_language ON stopwords(language);
            "#,
        )?;

        // Migrate data from old user_stopwords table if it exists
        if has_user_stopwords_table {
            conn.execute(
                r#"INSERT OR IGNORE INTO stopwords (word, source, language, created_at)
                   SELECT word, 'user', NULL, added_at FROM user_stopwords"#,
                [],
            )?;
            // Drop old table
            conn.execute("DROP TABLE user_stopwords", [])?;
        }
    } else if has_user_stopwords_table {
        // Both tables exist - migrate remaining data and drop old table
        conn.execute(
            r#"INSERT OR IGNORE INTO stopwords (word, source, language, created_at)
               SELECT word, 'user', NULL, added_at FROM user_stopwords"#,
            [],
        )?;
        conn.execute("DROP TABLE user_stopwords", [])?;
    }

    // Migration 16: Add keyword_type to immanentize table
    let has_keyword_type: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('immanentize') WHERE name = 'keyword_type'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_keyword_type {
        conn.execute_batch(
            r#"
            ALTER TABLE immanentize ADD COLUMN keyword_type TEXT DEFAULT 'concept' CHECK(keyword_type IN ('concept', 'person', 'organization', 'location', 'acronym'));
            "#,
        )?;
    }

    // Create index for keyword_type
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_immanentize_keyword_type ON immanentize(keyword_type);
        "#,
    )?;

    // Migration 17: Create keyword_type_prototypes table
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS keyword_type_prototypes (
            type_name TEXT PRIMARY KEY,
            embedding BLOB,
            example_keywords TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )?;

    // Migration 18: Create recommendation_feedback table for user feedback on recommendations
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS recommendation_feedback (
            id INTEGER PRIMARY KEY,
            fnord_id INTEGER NOT NULL,
            action TEXT NOT NULL CHECK(action IN ('save', 'hide', 'click')),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(fnord_id, action),
            FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_recommendation_feedback_action ON recommendation_feedback(action);
        CREATE INDEX IF NOT EXISTS idx_recommendation_feedback_fnord ON recommendation_feedback(fnord_id);
        CREATE INDEX IF NOT EXISTS idx_recommendation_feedback_created ON recommendation_feedback(created_at DESC);
        "#,
    )?;

    // Migration 19: Add full_text_fetch_error column to fnords table
    // Tracks errors during full-text retrieval: NULL (no error), "404", "timeout", "parse_error", "blocked", etc.
    let has_fetch_error: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('fnords') WHERE name = 'full_text_fetch_error'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_fetch_error {
        conn.execute_batch(
            r#"
            ALTER TABLE fnords ADD COLUMN full_text_fetch_error TEXT DEFAULT NULL;
            "#,
        )?;
    }

    // Create index for finding articles with fetch errors
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_fnords_fetch_error ON fnords(full_text_fetch_error) WHERE full_text_fetch_error IS NOT NULL;
        "#,
    )?;

    // Migration 20: Create preserved_compounds table for compound keyword protection
    // Keywords in this table will not be split by split_compound_keywords
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS preserved_compounds (
            immanentize_id INTEGER PRIMARY KEY,
            preserved_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
        );
        "#,
    )?;

    // Migration 21: Create compound_decisions table for compound keyword splitting decisions
    // Replaces preserved_compounds with more flexible preserve/split decisions
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compound_decisions (
            immanentize_id INTEGER PRIMARY KEY,
            decision TEXT NOT NULL CHECK(decision IN ('preserve', 'split')),
            decided_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_compound_decisions_decision ON compound_decisions(decision);
        "#,
    )?;

    // Migrate data from preserved_compounds to compound_decisions (one-time migration)
    let has_preserved_data: bool = conn
        .prepare("SELECT COUNT(*) FROM preserved_compounds")?
        .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

    if has_preserved_data {
        // Check if we already migrated (to avoid duplicate runs)
        let already_migrated: bool = conn
            .prepare("SELECT COUNT(*) FROM compound_decisions WHERE decision = 'preserve'")?
            .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

        if !already_migrated {
            conn.execute(
                r#"INSERT OR IGNORE INTO compound_decisions (immanentize_id, decision, decided_at)
                   SELECT immanentize_id, 'preserve', preserved_at FROM preserved_compounds"#,
                [],
            )?;
            info!("Migrated preserved_compounds to compound_decisions");
        }
    }

    // Migration 22: Create analysis_cache table for content-hash based caching
    // This avoids re-analyzing identical article content
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS analysis_cache (
            content_hash TEXT PRIMARY KEY,
            summary TEXT,
            categories TEXT,          -- JSON array of category names
            keywords TEXT,            -- JSON array of keywords
            political_bias INTEGER,
            sachlichkeit INTEGER,
            article_type TEXT DEFAULT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            hit_count INTEGER DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_analysis_cache_created ON analysis_cache(created_at DESC);
        "#,
    )?;

    // Migration 23: Create ai_cost_log table for API cost tracking
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS ai_cost_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            estimated_cost_usd REAL NOT NULL DEFAULT 0.0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_ai_cost_log_created ON ai_cost_log(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_ai_cost_log_provider ON ai_cost_log(provider);
        "#,
    )?;

    // Composite index for immanentize_daily trending queries (date range + aggregation)
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_immanentize_daily_date_id_count
            ON immanentize_daily(date, immanentize_id, count);
        "#,
    )?;

    // Migration 25: DELETE trigger for vec_immanentize cleanup
    // Ensures orphaned vec_immanentize entries are removed when keywords are deleted
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS immanentize_delete_vec
            AFTER DELETE ON immanentize
        BEGIN
            DELETE FROM vec_immanentize WHERE immanentize_id = OLD.id;
        END;
        "#,
    )?;

    // Migration 26: Add article_type column to fnords and analysis_cache tables
    // Stores the article type classification from LLM analysis
    // (news/analysis/opinion/satire/ad/unknown)
    let has_article_type: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('fnords') \
             WHERE name = 'article_type'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_article_type {
        conn.execute_batch(
            r#"
            ALTER TABLE fnords ADD COLUMN article_type TEXT DEFAULT 'unknown';
            CREATE INDEX idx_fnords_article_type ON fnords(article_type);
            "#,
        )?;
        info!("Migration 26: Added article_type column to fnords");
    }

    // Also add article_type to analysis_cache if missing
    let has_cache_article_type: bool = conn
        .prepare(
            "SELECT COUNT(*) FROM pragma_table_info('analysis_cache') \
             WHERE name = 'article_type'",
        )?
        .query_row([], |row| row.get(0))?;

    if !has_cache_article_type {
        conn.execute_batch(
            "ALTER TABLE analysis_cache \
             ADD COLUMN article_type TEXT DEFAULT NULL;",
        )?;
        info!("Migration 26: Added article_type column to analysis_cache");
    }

    // One-time cleanup: remove orphaned vec_immanentize entries
    // (entries where the parent keyword no longer exists)
    let orphaned_count: i64 = conn
        .prepare(
            "SELECT COUNT(*) FROM vec_immanentize WHERE immanentize_id NOT IN (SELECT id FROM immanentize)",
        )?
        .query_row([], |row| row.get(0))?;

    if orphaned_count > 0 {
        conn.execute(
            "DELETE FROM vec_immanentize WHERE immanentize_id NOT IN (SELECT id FROM immanentize)",
            [],
        )?;
        info!(
            "Cleaned up {} orphaned vec_immanentize entries",
            orphaned_count
        );
    }

    // Migration 27: Create briefings table for AI-generated news briefings
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS briefings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            period_type TEXT NOT NULL CHECK(period_type IN ('daily', 'weekly')),
            period_start DATETIME NOT NULL,
            period_end DATETIME NOT NULL,
            content TEXT NOT NULL,
            top_keywords TEXT,
            article_count INTEGER NOT NULL DEFAULT 0,
            model_used TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(period_type, period_start)
        );

        CREATE INDEX IF NOT EXISTS idx_briefings_created
            ON briefings(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_briefings_period
            ON briefings(period_type, period_start DESC);
        "#,
    )?;

    // Migration 28: Create story_clusters tables for article clustering
    // Groups related articles by topic for perspective comparison
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS story_clusters (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            summary TEXT,
            perspective_comparison TEXT,
            article_count INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS story_cluster_articles (
            cluster_id INTEGER NOT NULL REFERENCES story_clusters(id) ON DELETE CASCADE,
            fnord_id INTEGER NOT NULL REFERENCES fnords(id) ON DELETE CASCADE,
            similarity_score REAL NOT NULL DEFAULT 0.0,
            PRIMARY KEY (cluster_id, fnord_id)
        );

        CREATE INDEX IF NOT EXISTS idx_sca_fnord ON story_cluster_articles(fnord_id);
        "#,
    )?;

    // Migration 29: Create entities and fnord_entities tables for Named Entity Recognition
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            entity_type TEXT NOT NULL CHECK(entity_type IN ('person', 'organization', 'location', 'event')),
            normalized_name TEXT NOT NULL,
            article_count INTEGER NOT NULL DEFAULT 0,
            first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(normalized_name, entity_type)
        );

        CREATE TABLE IF NOT EXISTS fnord_entities (
            fnord_id INTEGER NOT NULL REFERENCES fnords(id) ON DELETE CASCADE,
            entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
            mention_count INTEGER NOT NULL DEFAULT 1,
            confidence REAL NOT NULL DEFAULT 0.8,
            PRIMARY KEY (fnord_id, entity_id)
        );

        CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
        CREATE INDEX IF NOT EXISTS idx_entities_normalized ON entities(normalized_name);
        CREATE INDEX IF NOT EXISTS idx_fnord_entities_entity ON fnord_entities(entity_id);
        "#,
    )?;

    Ok(())
}

pub fn init(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        r#"
        -- ============================================================
        -- PENTACLES (Feed-Quellen)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS pentacles (
            id INTEGER PRIMARY KEY,
            url TEXT NOT NULL UNIQUE,
            title TEXT,
            description TEXT,
            site_url TEXT,
            icon_url TEXT,

            -- Sync-Einstellungen
            last_sync DATETIME,
            sync_interval INTEGER DEFAULT 1800,
            is_truncated BOOLEAN DEFAULT FALSE,

            -- Qualitätsbewertung
            default_quality INTEGER DEFAULT 3 CHECK(default_quality BETWEEN 1 AND 5),

            -- Metadaten
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

            -- Statistik
            article_count INTEGER DEFAULT 0,
            error_count INTEGER DEFAULT 0,
            last_error TEXT
        );

        -- ============================================================
        -- FNORDS (Artikel)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS fnords (
            id INTEGER PRIMARY KEY,
            pentacle_id INTEGER NOT NULL,

            -- Identifikation
            guid TEXT NOT NULL,
            url TEXT NOT NULL,

            -- Inhalt
            title TEXT NOT NULL,
            author TEXT,
            content_raw TEXT,
            content_full TEXT,
            summary TEXT,
            image_url TEXT,

            -- Zeitstempel
            published_at DATETIME,
            fetched_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            processed_at DATETIME,
            read_at DATETIME,

            -- Status
            status TEXT DEFAULT 'concealed' CHECK(status IN ('concealed', 'illuminated', 'golden_apple')),
            full_text_fetched BOOLEAN DEFAULT FALSE,

            -- Greyface Alert
            political_bias INTEGER CHECK(political_bias BETWEEN -2 AND 2),
            sachlichkeit INTEGER CHECK(sachlichkeit BETWEEN 0 AND 4),
            quality_score INTEGER CHECK(quality_score BETWEEN 1 AND 5),

            -- Relevanz
            relevance_score REAL DEFAULT 0.0,

            -- Änderungserkennung
            content_hash TEXT,
            has_changes BOOLEAN DEFAULT FALSE,
            changed_at DATETIME,
            revision_count INTEGER DEFAULT 0,

            -- Analyse-Retry
            analysis_attempts INTEGER DEFAULT 0,
            analysis_error TEXT,
            analysis_hopeless BOOLEAN DEFAULT FALSE,

            -- Embeddings (Phase 3)
            embedding BLOB DEFAULT NULL,
            embedding_at DATETIME DEFAULT NULL,

            -- Full-text fetch error tracking
            full_text_fetch_error TEXT DEFAULT NULL,

            -- Article type classification (from LLM analysis)
            article_type TEXT DEFAULT 'unknown',

            -- Constraints
            FOREIGN KEY (pentacle_id) REFERENCES pentacles(id) ON DELETE CASCADE,
            UNIQUE(pentacle_id, guid)
        );

        -- ============================================================
        -- FNORD_REVISIONS (Artikel-Versionshistorie)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS fnord_revisions (
            id INTEGER PRIMARY KEY,
            fnord_id INTEGER NOT NULL,

            -- Snapshot der Felder zum Zeitpunkt der Revision
            title TEXT NOT NULL,
            author TEXT,
            content_raw TEXT,
            content_full TEXT,
            summary TEXT,

            -- Hash zur schnellen Vergleichsprüfung
            content_hash TEXT NOT NULL,

            -- Zeitstempel
            revision_at DATETIME DEFAULT CURRENT_TIMESTAMP,

            -- Constraint
            FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE
        );

        -- ============================================================
        -- SEPHIROTH (Kategorien - Hierarchisch)
        -- level 0 = Hauptkategorie (6), level 1 = Unterkategorie (13)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS sephiroth (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            parent_id INTEGER REFERENCES sephiroth(id) ON DELETE CASCADE,
            level INTEGER DEFAULT 0,
            description TEXT,
            color TEXT,
            icon TEXT,
            article_count INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        -- Note: idx_sephiroth_parent and idx_sephiroth_level are created in migrations
        -- because existing databases may not have these columns yet

        -- ============================================================
        -- IMMANENTIZE_CLUSTERS (Themen-Cluster)
        -- Muss vor immanentize erstellt werden wegen FK
        -- ============================================================
        CREATE TABLE IF NOT EXISTS immanentize_clusters (
            id INTEGER PRIMARY KEY,
            name TEXT,
            description TEXT,
            auto_generated BOOLEAN DEFAULT TRUE,
            keyword_count INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        -- ============================================================
        -- IMMANENTIZE (Schlagworte/Tags)
        -- Erweitert für Immanentize Network
        -- ============================================================
        CREATE TABLE IF NOT EXISTS immanentize (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,

            -- Statistik
            count INTEGER DEFAULT 1,
            article_count INTEGER DEFAULT 0,

            -- Quality & Embeddings
            quality_score REAL DEFAULT NULL,
            quality_calculated_at DATETIME DEFAULT NULL,
            embedding BLOB DEFAULT NULL,
            embedding_at DATETIME,

            -- Clustering
            cluster_id INTEGER,

            -- Synonym-Handling
            is_canonical BOOLEAN DEFAULT TRUE,
            canonical_id INTEGER,

            -- Keyword Type (concept, person, organization, location, acronym)
            keyword_type TEXT DEFAULT 'concept' CHECK(keyword_type IN ('concept', 'person', 'organization', 'location', 'acronym')),

            -- Zeitstempel
            first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            last_used DATETIME DEFAULT CURRENT_TIMESTAMP,

            FOREIGN KEY (canonical_id) REFERENCES immanentize(id) ON DELETE SET NULL,
            FOREIGN KEY (cluster_id) REFERENCES immanentize_clusters(id) ON DELETE SET NULL
        );

        -- ============================================================
        -- IMMANENTIZE_SEPHIROTH (Schlagwort ↔ Kategorie)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS immanentize_sephiroth (
            immanentize_id INTEGER NOT NULL,
            sephiroth_id INTEGER NOT NULL,
            weight REAL DEFAULT 1.0,
            article_count INTEGER DEFAULT 1,
            first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (immanentize_id, sephiroth_id),
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE,
            FOREIGN KEY (sephiroth_id) REFERENCES sephiroth(id) ON DELETE CASCADE
        );

        -- ============================================================
        -- IMMANENTIZE_NEIGHBORS (Kookkurrenz-Netzwerk)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS immanentize_neighbors (
            immanentize_id_a INTEGER NOT NULL,
            immanentize_id_b INTEGER NOT NULL,
            cooccurrence INTEGER DEFAULT 1,
            embedding_similarity REAL,
            combined_weight REAL DEFAULT 0.0,
            first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (immanentize_id_a, immanentize_id_b),
            FOREIGN KEY (immanentize_id_a) REFERENCES immanentize(id) ON DELETE CASCADE,
            FOREIGN KEY (immanentize_id_b) REFERENCES immanentize(id) ON DELETE CASCADE,
            CHECK (immanentize_id_a < immanentize_id_b)
        );

        -- ============================================================
        -- FNORD ↔ SEPHIROTH (Artikel-Kategorien-Zuordnung)
        -- 1:n Beziehung - Ein Artikel hat mindestens eine Kategorie
        -- ============================================================
        CREATE TABLE IF NOT EXISTS fnord_sephiroth (
            fnord_id INTEGER NOT NULL,
            sephiroth_id INTEGER NOT NULL,
            confidence REAL DEFAULT 1.0,
            source TEXT DEFAULT 'ai' CHECK(source IN ('ai', 'statistical', 'manual')),
            assigned_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (fnord_id, sephiroth_id),
            FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE,
            FOREIGN KEY (sephiroth_id) REFERENCES sephiroth(id) ON DELETE CASCADE
        );

        -- ============================================================
        -- FNORD ↔ IMMANENTIZE (Artikel-Stichworte-Zuordnung)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS fnord_immanentize (
            fnord_id INTEGER NOT NULL,
            immanentize_id INTEGER NOT NULL,
            source TEXT DEFAULT 'ai' CHECK(source IN ('ai', 'statistical', 'manual')),
            confidence REAL DEFAULT 1.0,
            PRIMARY KEY (fnord_id, immanentize_id),
            FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE,
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
        );

        -- ============================================================
        -- IMMANENTIZE_DAILY (Zeitreihen für Trend-Analyse)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS immanentize_daily (
            immanentize_id INTEGER NOT NULL,
            date TEXT NOT NULL,  -- 'YYYY-MM-DD'
            count INTEGER DEFAULT 0,
            PRIMARY KEY (immanentize_id, date),
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_immanentize_daily_date ON immanentize_daily(date DESC);
        CREATE INDEX IF NOT EXISTS idx_immanentize_daily_id ON immanentize_daily(immanentize_id);

        -- ============================================================
        -- EMBEDDING_QUEUE (Warteschlange für Embedding-Generierung)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS embedding_queue (
            id INTEGER PRIMARY KEY,
            immanentize_id INTEGER NOT NULL UNIQUE,
            priority INTEGER DEFAULT 0,  -- Höher = wichtiger
            queued_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            attempts INTEGER DEFAULT 0,
            last_error TEXT,
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_embedding_queue_priority ON embedding_queue(priority DESC, queued_at ASC);

        -- ============================================================
        -- BIAS_WEIGHTS (Lerngewichtungen für statistische Analyse)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS bias_weights (
            id INTEGER PRIMARY KEY,
            weight_type TEXT NOT NULL,  -- 'keyword_boost', 'category_term', 'source_weight'
            context_key TEXT NOT NULL,  -- Keyword-Name, Kategorie-ID, oder Source-Typ
            term TEXT,                  -- Bei category_term: das gewichtete Wort
            weight REAL DEFAULT 1.0,
            correction_count INTEGER DEFAULT 0,
            last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_bias_weights_type_context ON bias_weights(weight_type, context_key);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_bias_weights_unique ON bias_weights(weight_type, context_key, COALESCE(term, ''));

        -- ============================================================
        -- KEYWORD_TYPE_PROTOTYPES (Embedding-Prototypen für Typ-Erkennung)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS keyword_type_prototypes (
            type_name TEXT PRIMARY KEY,  -- 'person', 'organization', 'location', 'acronym', 'concept'
            embedding BLOB,              -- 1024-dim snowflake-arctic-embed2
            example_keywords TEXT,       -- Komma-separierte Beispiel-Keywords
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        -- ============================================================
        -- SETTINGS (Benutzereinstellungen)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Default settings
        INSERT OR IGNORE INTO settings (key, value) VALUES
            ('locale', 'de'),
            ('theme', 'mocha'),
            ('showTerminologyTooltips', 'true'),
            ('ollama_num_ctx', '4096'),
            ('ollama_concurrency', '1');

        -- ============================================================
        -- INDIZES
        -- ============================================================
        CREATE INDEX IF NOT EXISTS idx_fnords_status ON fnords(status);
        CREATE INDEX IF NOT EXISTS idx_fnords_published ON fnords(published_at DESC);
        CREATE INDEX IF NOT EXISTS idx_fnords_pentacle ON fnords(pentacle_id);
        CREATE INDEX IF NOT EXISTS idx_fnords_processed ON fnords(processed_at);
        CREATE INDEX IF NOT EXISTS idx_fnords_relevance ON fnords(relevance_score DESC);
        CREATE INDEX IF NOT EXISTS idx_pentacles_last_sync ON pentacles(last_sync);

        -- Indizes für Kategorie-Statistiken
        CREATE INDEX IF NOT EXISTS idx_fnord_sephiroth_sephiroth ON fnord_sephiroth(sephiroth_id);
        -- Note: idx_fnord_sephiroth_assigned and idx_fnord_sephiroth_source are created in migrations
        -- because existing databases may not have these columns yet
        CREATE INDEX IF NOT EXISTS idx_immanentize_count ON immanentize(count DESC);
        CREATE INDEX IF NOT EXISTS idx_revisions_fnord ON fnord_revisions(fnord_id, revision_at DESC);

        -- Note: Immanentize Network indexes are created in migrations
        -- because existing databases may not have these columns yet

        -- Note: Default sephiroth categories are inserted after migrations
        -- to ensure parent_id and level columns exist
        "#,
    )?;

    // Run migrations for existing databases
    run_migrations(conn)?;

    // Create sephiroth indexes (after migrations ensure columns exist)
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sephiroth_parent ON sephiroth(parent_id);
        CREATE INDEX IF NOT EXISTS idx_sephiroth_level ON sephiroth(level);
        "#,
    )?;

    // Insert default sephiroth categories (only if main category 1 doesn't exist)
    // This runs AFTER migrations, so parent_id and level columns exist
    let has_main_categories: bool = conn
        .prepare("SELECT COUNT(*) FROM sephiroth WHERE id = 1 AND level = 0")?
        .query_row([], |row| row.get::<_, i64>(0).map(|c| c > 0))?;

    if !has_main_categories {
        conn.execute_batch(
            r#"
            -- Hauptkategorien (level 0) - Farben werden über CSS-Variablen gesteuert
            INSERT OR IGNORE INTO sephiroth (id, name, parent_id, level, icon) VALUES
                (1, 'Wissen & Technologie', NULL, 0, 'fa-solid fa-microchip'),
                (2, 'Politik & Gesellschaft', NULL, 0, 'fa-solid fa-landmark'),
                (3, 'Wirtschaft', NULL, 0, 'fa-solid fa-chart-line'),
                (4, 'Umwelt & Gesundheit', NULL, 0, 'fa-solid fa-leaf'),
                (5, 'Sicherheit', NULL, 0, 'fa-solid fa-shield-halved'),
                (6, 'Kultur & Leben', NULL, 0, 'fa-solid fa-masks-theater');

            -- Unterkategorien (level 1) - für KI-Klassifizierung & Blind Spots
            INSERT OR IGNORE INTO sephiroth (id, name, parent_id, level, icon) VALUES
                (101, 'Technik', 1, 1, 'fa-solid fa-microchip'),
                (102, 'Wissenschaft', 1, 1, 'fa-solid fa-flask'),
                (201, 'Politik', 2, 1, 'fa-solid fa-landmark'),
                (202, 'Gesellschaft', 2, 1, 'fa-solid fa-users'),
                (203, 'Recht', 2, 1, 'fa-solid fa-scale-balanced'),
                (301, 'Wirtschaft', 3, 1, 'fa-solid fa-chart-line'),
                (302, 'Energie', 3, 1, 'fa-solid fa-bolt'),
                (401, 'Umwelt', 4, 1, 'fa-solid fa-leaf'),
                (402, 'Gesundheit', 4, 1, 'fa-solid fa-heart-pulse'),
                (501, 'Sicherheit', 5, 1, 'fa-solid fa-shield-halved'),
                (502, 'Verteidigung', 5, 1, 'fa-solid fa-medal'),
                (601, 'Kultur', 6, 1, 'fa-solid fa-masks-theater'),
                (602, 'Sport', 6, 1, 'fa-solid fa-futbol');
            "#,
        )?;
    }

    // Restore default stopwords from embedded txt files (only if no system stopwords exist)
    let system_stopword_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stopwords WHERE source = 'system'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if system_stopword_count == 0 {
        restore_default_stopwords(conn)?;
    }

    // Seed known keywords with their types (persons, organizations, locations, etc.)
    // This uses INSERT OR IGNORE, so existing keywords are not overwritten
    let _ = seed_known_keywords(conn);

    // Update keyword types for any existing keywords that match known entities
    let _ = update_types_from_seeds(conn);

    Ok(())
}
