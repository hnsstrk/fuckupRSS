-- migration-22-performance.sql
-- Performance Optimization Migration
-- Adds composite indexes and auto-sync triggers for embeddings
--
-- Expected Performance Gains:
-- - get_fnords() 2-3x faster
-- - Similarity searches 10-50x faster (when using sqlite-vec KNN)
-- - Category lookups 1.5x faster
--
-- IMPORTANT: Always create a backup before running this script!
-- cp src-tauri/data/fuckup.db src-tauri/data/fuckup.db.backup-$(date +%Y%m%d-%H%M%S)
--
-- Run with: sqlite3 src-tauri/data/fuckup.db < docs/sql/migration-22-performance.sql
--

BEGIN TRANSACTION;

-- ============================================================
-- 1. Composite Indexes for Frequent Query Patterns
-- ============================================================

-- Article lists (most frequent query: get_fnords with status + sort by published)
CREATE INDEX IF NOT EXISTS idx_fnords_status_published
  ON fnords(status, published_at DESC);

-- Covering index for list views (avoids table lookups)
CREATE INDEX IF NOT EXISTS idx_fnords_list_covering
  ON fnords(status, published_at DESC, pentacle_id, guid, url, title);

-- Pentacle + Status filter (feed-specific views)
CREATE INDEX IF NOT EXISTS idx_fnords_pentacle_status_published
  ON fnords(pentacle_id, status, published_at DESC);

-- Category lookups (JOIN optimization)
CREATE INDEX IF NOT EXISTS idx_fnord_sephiroth_covering
  ON fnord_sephiroth(fnord_id, sephiroth_id, confidence);

-- Neighbor queries (sorted by weight)
CREATE INDEX IF NOT EXISTS idx_neighbors_weight_cooccurrence
  ON immanentize_neighbors(combined_weight DESC, cooccurrence DESC);

-- ============================================================
-- 2. Partial Indexes (Smaller, Faster)
-- ============================================================

-- Recommendation pool (only articles with embeddings)
CREATE INDEX IF NOT EXISTS idx_fnords_recommendation_pool
  ON fnords(published_at DESC, id)
  WHERE status = 'concealed' AND embedding IS NOT NULL;

-- Unprocessed articles queue
CREATE INDEX IF NOT EXISTS idx_fnords_unprocessed
  ON fnords(published_at DESC, id)
  WHERE processed_at IS NULL AND content_full IS NOT NULL;

-- ============================================================
-- 3. Drop Redundant Indexes
-- ============================================================

-- This index was never used according to tech debt analysis
DROP INDEX IF EXISTS idx_fnords_relevance;

-- ============================================================
-- 4. Auto-Sync Triggers for Embedding Tables
-- ============================================================
-- These triggers ensure vec_immanentize and vec_fnords are always
-- in sync with the main tables, eliminating manual sync operations

-- Keyword embeddings: Insert
CREATE TRIGGER IF NOT EXISTS sync_immanentize_embedding_insert
AFTER UPDATE OF embedding ON immanentize
WHEN NEW.embedding IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO vec_immanentize (immanentize_id, embedding)
    VALUES (NEW.id, NEW.embedding);
END;

-- Keyword embeddings: Delete
CREATE TRIGGER IF NOT EXISTS sync_immanentize_embedding_delete
AFTER UPDATE OF embedding ON immanentize
WHEN NEW.embedding IS NULL
BEGIN
    DELETE FROM vec_immanentize WHERE immanentize_id = NEW.id;
END;

-- Article embeddings: Insert
CREATE TRIGGER IF NOT EXISTS sync_fnords_embedding_insert
AFTER UPDATE OF embedding ON fnords
WHEN NEW.embedding IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO vec_fnords (fnord_id, embedding)
    VALUES (NEW.id, NEW.embedding);
END;

-- Article embeddings: Delete
CREATE TRIGGER IF NOT EXISTS sync_fnords_embedding_delete
AFTER UPDATE OF embedding ON fnords
WHEN NEW.embedding IS NULL
BEGIN
    DELETE FROM vec_fnords WHERE fnord_id = NEW.id;
END;

-- ============================================================
-- 5. Update Query Planner Statistics
-- ============================================================
-- ANALYZE updates internal statistics used by the query planner
-- to choose optimal query execution plans
ANALYZE;

COMMIT;

-- ============================================================
-- VERIFICATION QUERIES
-- ============================================================

.print ""
.print "=== Performance Migration Complete ==="
.print ""

.print "Indexes created:"
SELECT name FROM sqlite_master
WHERE type = 'index'
AND name LIKE 'idx_fnords%' OR name LIKE 'idx_neighbors%' OR name LIKE 'idx_fnord_sephiroth%'
ORDER BY name;

.print ""
.print "Triggers created:"
SELECT name FROM sqlite_master
WHERE type = 'trigger'
AND name LIKE 'sync_%'
ORDER BY name;

.print ""
.print "Test query plan (should use idx_fnords_status_published):"
EXPLAIN QUERY PLAN
SELECT * FROM fnords
WHERE status = 'concealed'
ORDER BY published_at DESC
LIMIT 50;

.print ""
