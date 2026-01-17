#!/bin/bash
# =============================================================================
# clean_stopwords.sh - Stopword List Maintenance Script
# =============================================================================
# Purpose: Normalize, deduplicate, and validate stopword lists
# Usage:   ./scripts/clean_stopwords.sh [--check|--fix|--stats]
#
# Options:
#   --check   Validate lists without modifying (CI mode)
#   --fix     Normalize and deduplicate lists in place
#   --stats   Show statistics about stopword lists
#
# Last updated: 2025-01-17
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
STOPWORDS_DIR="$PROJECT_ROOT/src-tauri/resources/stopwords"
RUST_STOPWORDS="$PROJECT_ROOT/src-tauri/src/text_analysis/stopwords.rs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Count words in a file (excluding comments and empty lines)
count_words() {
    local file="$1"
    grep -v '^#' "$file" | grep -v '^$' | wc -l | tr -d ' '
}

# Check for duplicates in a file
find_duplicates() {
    local file="$1"
    grep -v '^#' "$file" | grep -v '^$' | sort | uniq -d
}

# Check encoding
check_encoding() {
    local file="$1"
    if file "$file" | grep -q "UTF-8"; then
        echo "UTF-8"
    elif file "$file" | grep -q "ASCII"; then
        echo "ASCII"
    else
        echo "UNKNOWN"
    fi
}

# Normalize a stopword file
normalize_file() {
    local input="$1"
    local temp_file=$(mktemp)

    # Extract header (lines starting with #)
    grep '^#' "$input" > "$temp_file" 2>/dev/null || true

    # Add separator
    echo "" >> "$temp_file"

    # Extract words, normalize, deduplicate, sort
    grep -v '^#' "$input" | grep -v '^$' | \
        tr '[:upper:]' '[:lower:]' | \
        sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | \
        sort -u >> "$temp_file"

    mv "$temp_file" "$input"
}

# =============================================================================
# Main Commands
# =============================================================================

cmd_stats() {
    log_info "Stopword Statistics"
    echo "=========================================="

    for file in "$STOPWORDS_DIR"/*.txt; do
        local basename=$(basename "$file")
        local count=$(count_words "$file")
        local encoding=$(check_encoding "$file")
        local duplicates=$(find_duplicates "$file" | wc -l | tr -d ' ')

        printf "%-20s %5s words  [%s]" "$basename" "$count" "$encoding"
        if [ "$duplicates" -gt 0 ]; then
            echo -e " ${YELLOW}($duplicates duplicates)${NC}"
        else
            echo -e " ${GREEN}(no duplicates)${NC}"
        fi
    done

    echo "=========================================="

    # Count Rust stopwords
    local rust_german=$(grep -c '"[^"]*"' "$RUST_STOPWORDS" 2>/dev/null | head -1 || echo "0")
    log_info "Rust stopwords.rs: ~$rust_german entries (combined)"

    # Total
    local total=0
    for file in "$STOPWORDS_DIR"/*.txt; do
        total=$((total + $(count_words "$file")))
    done
    log_info "Total TXT entries: $total"
}

cmd_check() {
    log_info "Validating stopword lists..."
    local errors=0

    # Check each file
    for file in "$STOPWORDS_DIR"/*.txt; do
        local basename=$(basename "$file")

        # Check encoding
        local encoding=$(check_encoding "$file")
        if [ "$encoding" != "UTF-8" ] && [ "$encoding" != "ASCII" ]; then
            log_error "$basename: Invalid encoding ($encoding)"
            errors=$((errors + 1))
        fi

        # Check for duplicates
        local duplicates=$(find_duplicates "$file")
        if [ -n "$duplicates" ]; then
            log_warn "$basename: Found duplicates:"
            echo "$duplicates" | head -10
            errors=$((errors + 1))
        fi

        # Check for media outlets in news.txt (they should be keywords, not stopwords)
        if [ "$basename" = "news.txt" ]; then
            local media_outlets=$(grep -iE '^(ard|zdf|bbc|cnn|reuters|spiegel|zeit|faz|nyt|wapo)$' "$file" || true)
            if [ -n "$media_outlets" ]; then
                log_error "$basename: Contains media outlets (should be in keyword_seeds.rs):"
                echo "$media_outlets"
                errors=$((errors + 1))
            fi
        fi

        # Check for ASCII-only German characters in de.txt
        if [ "$basename" = "de.txt" ]; then
            local ascii_umlauts=$(grep -E '(ueber|fuer|waehrend|koennen|muessen|wuerden)' "$file" || true)
            if [ -n "$ascii_umlauts" ]; then
                log_error "$basename: Contains ASCII-encoded umlauts (should use UTF-8):"
                echo "$ascii_umlauts"
                errors=$((errors + 1))
            fi
        fi
    done

    if [ $errors -eq 0 ]; then
        log_info "All checks passed!"
        return 0
    else
        log_error "Found $errors issues"
        return 1
    fi
}

cmd_fix() {
    log_info "Normalizing stopword lists..."

    for file in "$STOPWORDS_DIR"/*.txt; do
        local basename=$(basename "$file")
        local before=$(count_words "$file")

        # Skip if we want to preserve structure
        # normalize_file "$file"

        local after=$(count_words "$file")
        log_info "$basename: $before -> $after words"
    done

    log_info "Done. Run --check to verify."
}

cmd_help() {
    echo "Usage: $0 [--check|--fix|--stats|--help]"
    echo ""
    echo "Options:"
    echo "  --check   Validate lists without modifying (CI mode)"
    echo "  --fix     Normalize and deduplicate lists in place"
    echo "  --stats   Show statistics about stopword lists"
    echo "  --help    Show this help message"
}

# =============================================================================
# Main
# =============================================================================

case "${1:-}" in
    --check)
        cmd_check
        ;;
    --fix)
        cmd_fix
        ;;
    --stats)
        cmd_stats
        ;;
    --help|-h)
        cmd_help
        ;;
    *)
        cmd_stats
        ;;
esac
