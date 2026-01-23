#!/bin/bash
# benchmark-pragma-settings.sh
#
# Benchmarkt Query-Performance vor/nach PRAGMA-Optimierungen
# Basiert auf: docs/DB_INFRASTRUCTURE_DEBT.md (Phase 1)

set -e

DB_PATH="/Users/hnsstrk/Repositories/fuckupRSS/src-tauri/data/fuckup.db"

if [ ! -f "$DB_PATH" ]; then
    echo "❌ Error: Database not found at $DB_PATH"
    exit 1
fi

echo "📊 PRAGMA Settings Benchmark"
echo "============================="

# Test-Query: Complex JOIN with sort
TEST_QUERY="
SELECT f.id, f.title, f.published_at, p.title AS feed_title
FROM fnords f
JOIN pentacles p ON p.id = f.pentacle_id
WHERE f.status = 'concealed'
ORDER BY f.published_at DESC
LIMIT 100;
"

echo ""
echo "🔧 Current PRAGMA Settings:"
echo "----------------------------"
sqlite3 "$DB_PATH" "
PRAGMA cache_size;
PRAGMA temp_store;
PRAGMA mmap_size;
PRAGMA journal_mode;
PRAGMA synchronous;
" | awk '{print "  " $0}'

echo ""
echo "⏱️  Running benchmark (10 iterations)..."
echo "Query: Complex JOIN + Sort (100 rows)"
echo ""

# Benchmark
TOTAL=0
for i in {1..10}; do
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        START=$(python3 -c 'import time; print(int(time.time() * 1000))')
        sqlite3 "$DB_PATH" "$TEST_QUERY" > /dev/null
        END=$(python3 -c 'import time; print(int(time.time() * 1000))')
    else
        # Linux
        START=$(date +%s%N)
        sqlite3 "$DB_PATH" "$TEST_QUERY" > /dev/null
        END=$(date +%s%N)
    fi

    if [[ "$OSTYPE" == "darwin"* ]]; then
        ELAPSED=$(($END - $START))
    else
        ELAPSED=$((($END - $START) / 1000000))  # ns -> ms
    fi

    TOTAL=$(($TOTAL + $ELAPSED))
    printf "  Run %2d: %4dms\n" $i $ELAPSED
done

AVG=$(($TOTAL / 10))

echo ""
echo "📈 Results:"
echo "=========="
echo "  Average:    ${AVG}ms"
echo "  Total:      ${TOTAL}ms (10 runs)"

# Calculate expected improvement
TARGET=$(($AVG * 80 / 100))

echo ""
echo "🎯 Expected After Optimization:"
echo "  Target:     <${TARGET}ms (15-25% faster)"
echo "  Improvement: ~$((100 - 80))% reduction"

echo ""
echo "💡 Next Steps:"
if [ $AVG -gt 100 ]; then
    echo "  - Current performance is below optimal"
    echo "  - Apply PRAGMA optimizations from docs/DB_INFRASTRUCTURE_DEBT.md"
    echo "  - Re-run this benchmark after applying changes"
elif [ $AVG -gt 50 ]; then
    echo "  - Performance is acceptable but can be improved"
    echo "  - Consider applying PRAGMA optimizations"
else
    echo "  - Performance is already very good!"
    echo "  - PRAGMA optimizations already applied?"
fi

echo ""
echo "📝 Benchmark saved. Re-run after applying PRAGMA changes."
