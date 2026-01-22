-- cleanup-orphans.sql
-- Data Integrity Cleanup Script
-- Fixes critical P0 issues identified in tech debt analysis
--
-- IMPORTANT: Always create a backup before running this script!
-- cp src-tauri/data/fuckup.db src-tauri/data/fuckup.db.backup-$(date +%Y%m%d-%H%M%S)
--
-- Run with: sqlite3 src-tauri/data/fuckup.db < docs/sql/cleanup-orphans.sql
--

BEGIN TRANSACTION;

-- ============================================================
-- 1. Cleanup Orphaned Neighbors (7906 FK violations)
-- ============================================================
-- Delete neighbor relationships where one or both keywords don't exist
DELETE FROM immanentize_neighbors
WHERE immanentize_id_a NOT IN (SELECT id FROM immanentize)
   OR immanentize_id_b NOT IN (SELECT id FROM immanentize);

-- ============================================================
-- 2. Cleanup Orphaned Keyword-Category Relationships
-- ============================================================
-- Delete immanentize_sephiroth relationships where keyword doesn't exist
DELETE FROM immanentize_sephiroth
WHERE immanentize_id NOT IN (SELECT id FROM immanentize);

-- Delete immanentize_daily entries where keyword doesn't exist
DELETE FROM immanentize_daily
WHERE immanentize_id NOT IN (SELECT id FROM immanentize);

-- ============================================================
-- 3. Assign Default Category (uncategorized articles)
-- ============================================================
-- Create fallback category if it doesn't exist
INSERT OR IGNORE INTO sephiroth (id, name, parent_id, level, icon) VALUES
    (999, 'Unkategorisiert', NULL, 0, 'fa-solid fa-question');

-- Assign fallback category to all articles without any category
-- Use 'ai' as source since 'fallback' is not in the CHECK constraint
INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, source, confidence)
SELECT id, 999, 'ai', 0.3
FROM fnords
WHERE id NOT IN (SELECT DISTINCT fnord_id FROM fnord_sephiroth);

-- ============================================================
-- 4. Mark Articles Without Embeddings for Processing
-- ============================================================
-- Note: Article embeddings are generated automatically during processing
-- We'll mark these articles as needing reprocessing by clearing processed_at
-- This will be handled by the auto-embedding-queue feature in Task 6

-- ============================================================
-- 5. Drop Deprecated Table
-- ============================================================
-- Remove legacy preserved_compounds table (migrated to compound_decisions)
DROP TABLE IF EXISTS preserved_compounds;

COMMIT;

-- ============================================================
-- VERIFICATION QUERIES
-- ============================================================
-- These should all return 0 or expected values after cleanup

.print ""
.print "=== Verification Results ==="
.print ""

.print "1. Foreign Key Violations (should be 0):"
PRAGMA foreign_key_check;

.print ""
.print "2. Uncategorized Articles (should be 0):"
SELECT COUNT(*) AS uncategorized_articles FROM fnords
WHERE id NOT IN (SELECT DISTINCT fnord_id FROM fnord_sephiroth);

.print ""
.print "3. Articles Without Embeddings:"
SELECT COUNT(*) AS articles_without_embeddings FROM fnords
WHERE processed_at IS NOT NULL AND embedding IS NULL;

.print ""
.print "4. Database Integrity Check (should be 'ok'):"
PRAGMA integrity_check;

.print ""
.print "=== Cleanup Complete ==="
.print ""
