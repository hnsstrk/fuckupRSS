use log::info;
use rusqlite::Connection;
use serde::Deserialize;

/// Default stopwords embedded at compile time
const DEFAULT_STOPWORDS_JSON: &str = include_str!("../../data/default_stopwords.json");

/// Structure for parsing the default stopwords JSON file
#[derive(Debug, Deserialize)]
struct DefaultStopwordsFile {
    stopwords: Vec<String>,
}

/// Restore default stopwords from the embedded JSON file
///
/// This function is called during database initialization when the user_stopwords table is empty.
/// It ensures that a curated list of stopwords is always available, even when the database is
/// newly created or re-initialized.
///
/// Returns the number of stopwords that were restored.
pub fn restore_default_stopwords(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let file: DefaultStopwordsFile = serde_json::from_str(DEFAULT_STOPWORDS_JSON)
        .expect("Failed to parse embedded default_stopwords.json");

    let mut stmt = conn.prepare("INSERT OR IGNORE INTO user_stopwords (word) VALUES (?1)")?;
    let mut count = 0;

    for word in &file.stopwords {
        if stmt.execute([word])? > 0 {
            count += 1;
        }
    }

    if count > 0 {
        info!(
            "Restored {} default stopwords from embedded JSON file",
            count
        );
    }

    Ok(count)
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
            ALTER TABLE fnord_sephiroth ADD COLUMN source TEXT DEFAULT 'ai' CHECK(source IN ('ai', 'manual'));
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

    // Add confidence column to fnord_sephiroth if missing (for consistency with fnord_immanentize)
    let has_fs_confidence: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('fnord_sephiroth') WHERE name = 'confidence'")?
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
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_immanentize'")?
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
            INSERT INTO sephiroth (id, name, parent_id, level, icon, color) VALUES
                (1, 'Wissen & Technologie', NULL, 0, 'fa-solid fa-microchip', '#22d3ee'),
                (2, 'Politik & Gesellschaft', NULL, 0, 'fa-solid fa-landmark', '#ef4444'),
                (3, 'Wirtschaft', NULL, 0, 'fa-solid fa-chart-line', '#f97316'),
                (4, 'Umwelt & Gesundheit', NULL, 0, 'fa-solid fa-leaf', '#22c55e'),
                (5, 'Sicherheit', NULL, 0, 'fa-solid fa-shield-halved', '#a855f7'),
                (6, 'Kultur & Leben', NULL, 0, 'fa-solid fa-masks-theater', '#3b82f6');
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
            INSERT INTO sephiroth (id, name, parent_id, level, icon, color) VALUES
                (1, 'Wissen & Technologie', NULL, 0, 'fa-solid fa-microchip', '#22d3ee'),
                (2, 'Politik & Gesellschaft', NULL, 0, 'fa-solid fa-landmark', '#ef4444'),
                (3, 'Wirtschaft', NULL, 0, 'fa-solid fa-chart-line', '#f97316'),
                (4, 'Umwelt & Gesundheit', NULL, 0, 'fa-solid fa-leaf', '#22c55e'),
                (5, 'Sicherheit', NULL, 0, 'fa-solid fa-shield-halved', '#a855f7'),
                (6, 'Kultur & Leben', NULL, 0, 'fa-solid fa-masks-theater', '#3b82f6');
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

    // Fix main category colors (replace CSS variables with distinct hex colors)
    // This runs on every startup to ensure correct colors
    conn.execute_batch(
        r#"
        UPDATE sephiroth SET color = '#22d3ee' WHERE id = 1 AND level = 0;
        UPDATE sephiroth SET color = '#ef4444' WHERE id = 2 AND level = 0;
        UPDATE sephiroth SET color = '#f97316' WHERE id = 3 AND level = 0;
        UPDATE sephiroth SET color = '#22c55e' WHERE id = 4 AND level = 0;
        UPDATE sephiroth SET color = '#a855f7' WHERE id = 5 AND level = 0;
        UPDATE sephiroth SET color = '#3b82f6' WHERE id = 6 AND level = 0;
        "#,
    )?;

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

    // Migration 15: Create user_stopwords table for user-defined stopwords
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS user_stopwords (
            id INTEGER PRIMARY KEY,
            word TEXT NOT NULL UNIQUE COLLATE NOCASE,
            added_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_user_stopwords_word ON user_stopwords(word);
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
            source TEXT DEFAULT 'ai' CHECK(source IN ('ai', 'manual')),
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
            ('showTerminologyTooltips', 'true');

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
            -- Hauptkategorien (level 0) - für Visualisierung & Farben
            INSERT OR IGNORE INTO sephiroth (id, name, parent_id, level, icon, color) VALUES
                (1, 'Wissen & Technologie', NULL, 0, 'fa-solid fa-microchip', '#22d3ee'),
                (2, 'Politik & Gesellschaft', NULL, 0, 'fa-solid fa-landmark', '#ef4444'),
                (3, 'Wirtschaft', NULL, 0, 'fa-solid fa-chart-line', '#f97316'),
                (4, 'Umwelt & Gesundheit', NULL, 0, 'fa-solid fa-leaf', '#22c55e'),
                (5, 'Sicherheit', NULL, 0, 'fa-solid fa-shield-halved', '#a855f7'),
                (6, 'Kultur & Leben', NULL, 0, 'fa-solid fa-masks-theater', '#3b82f6');

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

    // Restore default stopwords from embedded JSON file (only if table is empty)
    let stopword_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM user_stopwords", [], |row| row.get(0))
        .unwrap_or(0);

    if stopword_count == 0 {
        restore_default_stopwords(conn)?;
    }

    Ok(())
}
