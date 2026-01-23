#!/bin/bash
# cleanup-db-infrastructure.sh
#
# Bereinigt Legacy-Datenbanken, führt VACUUM aus und verifiziert Integrität
# Basiert auf: docs/DB_INFRASTRUCTURE_DEBT.md (Phase 0: Cleanup)

set -e

PROJECT_ROOT="/Users/hnsstrk/Repositories/fuckupRSS"
DB_PATH="$PROJECT_ROOT/src-tauri/data/fuckup.db"
BACKUP_PATH="$DB_PATH.backup-$(date +%Y%m%d-%H%M%S)"

echo "🧹 fuckupRSS DB Infrastructure Cleanup"
echo "======================================"

# Check if DB exists
if [ ! -f "$DB_PATH" ]; then
    echo "❌ Error: Database not found at $DB_PATH"
    exit 1
fi

# 1. Safety: Backup
echo ""
echo "📦 Step 1: Creating backup..."
cp "$DB_PATH" "$BACKUP_PATH"
echo "✅ Backup: $BACKUP_PATH"

# 2. Delete Legacy DBs
echo ""
echo "🗑️  Step 2: Removing legacy databases..."

if [ -f "$PROJECT_ROOT/src-tauri/data.db" ]; then
    rm -f "$PROJECT_ROOT/src-tauri/data.db"
    echo "✅ Deleted: src-tauri/data.db"
else
    echo "ℹ️  src-tauri/data.db not found (already deleted)"
fi

if [ -f "$PROJECT_ROOT/database.db" ]; then
    rm -f "$PROJECT_ROOT/database.db"
    echo "✅ Deleted: database.db"
else
    echo "ℹ️  database.db not found (already deleted)"
fi

# 3. Update .gitignore
echo ""
echo "📝 Step 3: Updating .gitignore..."
if ! grep -q "^database.db$" "$PROJECT_ROOT/.gitignore"; then
    echo "database.db" >> "$PROJECT_ROOT/.gitignore"
    echo "✅ Added 'database.db' to .gitignore"
else
    echo "ℹ️  'database.db' already in .gitignore"
fi

# 4. VACUUM (App muss gestoppt sein!)
echo ""
echo "🔧 Step 4: Running VACUUM (may take 1-2 minutes)..."
echo "⚠️  Make sure the Tauri app is NOT running!"
read -p "Press Enter to continue or Ctrl+C to abort..."

sqlite3 "$DB_PATH" "PRAGMA wal_checkpoint(TRUNCATE);"
sqlite3 "$DB_PATH" "VACUUM;"
sqlite3 "$DB_PATH" "ANALYZE;"
echo "✅ VACUUM complete"

# 5. Verify Integrity
echo ""
echo "🔍 Step 5: Verifying database integrity..."
INTEGRITY=$(sqlite3 "$DB_PATH" "PRAGMA integrity_check;")
if [ "$INTEGRITY" = "ok" ]; then
    echo "✅ Integrity check: OK"
else
    echo "❌ Integrity check FAILED: $INTEGRITY"
    exit 1
fi

# 6. Foreign Key Check
echo ""
echo "🔍 Step 6: Checking foreign key constraints..."
FK_VIOLATIONS=$(sqlite3 "$DB_PATH" "PRAGMA foreign_key_check;" | wc -l | xargs)
if [ "$FK_VIOLATIONS" -eq 0 ]; then
    echo "✅ Foreign key check: OK (0 violations)"
else
    echo "⚠️  Foreign key check: $FK_VIOLATIONS violations found"
    echo "    Run: sqlite3 $DB_PATH 'PRAGMA foreign_key_check;' for details"
fi

# 7. Stats
echo ""
echo "📊 Database Stats:"
SIZE_MB=$(du -h "$DB_PATH" | awk '{print $1}')
TABLES=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table';")
FREELIST=$(sqlite3 "$DB_PATH" "PRAGMA freelist_count;")
echo "  Size:       $SIZE_MB"
echo "  Tables:     $TABLES"
echo "  Freelist:   $FREELIST pages"

echo ""
echo "✨ Cleanup complete!"
echo ""
echo "📋 Next steps:"
echo "  1. Start the Tauri app and verify everything works"
echo "  2. If OK, delete backup: rm $BACKUP_PATH"
echo "  3. If issues, restore backup: cp $BACKUP_PATH $DB_PATH"
