use rusqlite::Connection;

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
        "CREATE INDEX IF NOT EXISTS idx_fnords_has_changes ON fnords(has_changes);"
    )?;

    // Rename "Tech" to "Technik" if exists
    conn.execute(
        "UPDATE sephiroth SET name = 'Technik' WHERE name = 'Tech'",
        [],
    )?;

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
            article_type TEXT CHECK(article_type IN ('news', 'analysis', 'opinion', 'satire', 'ad', 'unknown')),

            -- Relevanz
            relevance_score REAL DEFAULT 0.0,

            -- Änderungserkennung
            content_hash TEXT,
            has_changes BOOLEAN DEFAULT FALSE,
            changed_at DATETIME,
            revision_count INTEGER DEFAULT 0,

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
            summary TEXT,

            -- Hash zur schnellen Vergleichsprüfung
            content_hash TEXT NOT NULL,

            -- Zeitstempel
            revision_at DATETIME DEFAULT CURRENT_TIMESTAMP,

            -- Constraint
            FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE
        );

        -- ============================================================
        -- SEPHIROTH (Kategorien)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS sephiroth (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            color TEXT,
            icon TEXT,
            article_count INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        -- ============================================================
        -- IMMANENTIZE (Stichworte/Tags)
        -- ============================================================
        CREATE TABLE IF NOT EXISTS immanentize (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            count INTEGER DEFAULT 1,
            last_used DATETIME DEFAULT CURRENT_TIMESTAMP,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
            PRIMARY KEY (fnord_id, immanentize_id),
            FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE,
            FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
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
        CREATE INDEX IF NOT EXISTS idx_fnord_sephiroth_assigned ON fnord_sephiroth(assigned_at DESC);
        CREATE INDEX IF NOT EXISTS idx_fnord_sephiroth_source ON fnord_sephiroth(source);
        CREATE INDEX IF NOT EXISTS idx_immanentize_count ON immanentize(count DESC);
        CREATE INDEX IF NOT EXISTS idx_revisions_fnord ON fnord_revisions(fnord_id, revision_at DESC);

        -- ============================================================
        -- DEFAULT SEPHIROTH (Kategorien)
        -- ============================================================
        INSERT OR IGNORE INTO sephiroth (name, icon, color) VALUES
            ('Technik', '💻', '#3B82F6'),
            ('Politik', '🏛️', '#EF4444'),
            ('Wirtschaft', '📈', '#10B981'),
            ('Wissenschaft', '🔬', '#8B5CF6'),
            ('Kultur', '🎭', '#F59E0B'),
            ('Sport', '⚽', '#06B6D4'),
            ('Gesellschaft', '👥', '#EC4899'),
            ('Umwelt', '🌍', '#22C55E'),
            ('Sicherheit', '🔒', '#6366F1'),
            ('Gesundheit', '🏥', '#F43F5E'),
            ('Verteidigung', '🎖️', '#78716C'),
            ('Energie', '⚡', '#FBBF24'),
            ('Recht', '⚖️', '#7C3AED');
        "#,
    )?;

    // Run migrations for existing databases
    run_migrations(conn)?;

    Ok(())
}
