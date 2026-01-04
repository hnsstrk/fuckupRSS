use rusqlite::Connection;

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
            status TEXT DEFAULT 'fnord' CHECK(status IN ('fnord', 'illuminated', 'golden_apple')),
            full_text_fetched BOOLEAN DEFAULT FALSE,

            -- Greyface Alert
            political_bias INTEGER CHECK(political_bias BETWEEN -2 AND 2),
            sachlichkeit INTEGER CHECK(sachlichkeit BETWEEN 0 AND 4),
            quality_score INTEGER CHECK(quality_score BETWEEN 1 AND 5),
            article_type TEXT CHECK(article_type IN ('news', 'analysis', 'opinion', 'satire', 'ad', 'unknown')),

            -- Relevanz
            relevance_score REAL DEFAULT 0.0,

            -- Constraints
            FOREIGN KEY (pentacle_id) REFERENCES pentacles(id) ON DELETE CASCADE,
            UNIQUE(pentacle_id, guid)
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
        -- ============================================================
        CREATE TABLE IF NOT EXISTS fnord_sephiroth (
            fnord_id INTEGER NOT NULL,
            sephiroth_id INTEGER NOT NULL,
            confidence REAL DEFAULT 1.0,
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
        -- INDIZES
        -- ============================================================
        CREATE INDEX IF NOT EXISTS idx_fnords_status ON fnords(status);
        CREATE INDEX IF NOT EXISTS idx_fnords_published ON fnords(published_at DESC);
        CREATE INDEX IF NOT EXISTS idx_fnords_pentacle ON fnords(pentacle_id);
        CREATE INDEX IF NOT EXISTS idx_fnords_processed ON fnords(processed_at);
        CREATE INDEX IF NOT EXISTS idx_fnords_relevance ON fnords(relevance_score DESC);
        CREATE INDEX IF NOT EXISTS idx_pentacles_last_sync ON pentacles(last_sync);
        CREATE INDEX IF NOT EXISTS idx_immanentize_count ON immanentize(count DESC);

        -- ============================================================
        -- DEFAULT SEPHIROTH (Kategorien)
        -- ============================================================
        INSERT OR IGNORE INTO sephiroth (name, icon, color) VALUES
            ('Tech', '💻', '#3B82F6'),
            ('Politik', '🏛️', '#EF4444'),
            ('Wirtschaft', '📈', '#10B981'),
            ('Wissenschaft', '🔬', '#8B5CF6'),
            ('Kultur', '🎭', '#F59E0B'),
            ('Sport', '⚽', '#06B6D4'),
            ('Gesellschaft', '👥', '#EC4899'),
            ('Umwelt', '🌍', '#22C55E'),
            ('Sicherheit', '🔒', '#6366F1'),
            ('Gesundheit', '🏥', '#F43F5E');
        "#,
    )?;

    Ok(())
}
