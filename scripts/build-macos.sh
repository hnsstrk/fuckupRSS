#!/usr/bin/env bash
# =============================================================================
# build-macos.sh — Lokaler macOS-Build für fuckupRSS
#
# Ausführung auf dem MacBook:
#   ./scripts/build-macos.sh          # Normaler Build
#   ./scripts/build-macos.sh --debug  # Debug-Build
#   ./scripts/build-macos.sh --clean  # Clean Build (löscht target/)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_MODE="release"
CLEAN=false

# --- Argument-Parsing ---
for arg in "$@"; do
	case $arg in
	--debug) BUILD_MODE="debug" ;;
	--clean) CLEAN=true ;;
	--help | -h)
		echo "Usage: $0 [--debug] [--clean]"
		echo "  --debug  Debug-Build (ohne Optimierung)"
		echo "  --clean  target/ vorher löschen"
		exit 0
		;;
	*)
		echo "Unbekanntes Argument: $arg"
		exit 1
		;;
	esac
done

cd "$PROJECT_DIR"

# --- Voraussetzungen prüfen ---
echo "🔍 Prüfe Voraussetzungen..."

missing=()
command -v node >/dev/null 2>&1 || missing+=("node")
command -v npm >/dev/null 2>&1 || missing+=("npm")
command -v cargo >/dev/null 2>&1 || missing+=("cargo (rustup)")
command -v rustc >/dev/null 2>&1 || missing+=("rustc")

if [[ ${#missing[@]} -gt 0 ]]; then
	echo "❌ Fehlende Tools: ${missing[*]}"
	exit 1
fi

echo "  Node:  $(node --version)"
echo "  npm:   $(npm --version)"
echo "  Rust:  $(rustc --version)"
echo "  Cargo: $(cargo --version)"

# --- Clean (optional) ---
if $CLEAN; then
	echo "🧹 Clean Build — lösche target/..."
	rm -rf src-tauri/target
fi

# --- Frontend Dependencies ---
echo "📦 Installiere Frontend-Dependencies..."
npm ci

# --- Lint-Checks (schneller Smoke-Test vor dem Build) ---
echo "🔎 Lint-Checks..."
npm run lint 2>/dev/null && echo "  ✅ ESLint" || echo "  ⚠️  ESLint-Warnings"
npm run format:check 2>/dev/null && echo "  ✅ Prettier" || echo "  ⚠️  Prettier-Warnings"

cd src-tauri
cargo fmt -- --check 2>/dev/null && echo "  ✅ rustfmt" || echo "  ⚠️  rustfmt-Warnings"
cd ..

# --- Build ---
echo ""
if [[ "$BUILD_MODE" == "debug" ]]; then
	echo "🔨 Starte Debug-Build..."
	npm run tauri build -- --debug
else
	echo "🔨 Starte Release-Build..."
	npm run tauri build
fi

# --- Ergebnis ---
echo ""
echo "✅ Build abgeschlossen!"
echo ""

DMG_DIR="src-tauri/target/${BUILD_MODE}/bundle/dmg"
APP_DIR="src-tauri/target/${BUILD_MODE}/bundle/macos"

if [[ -d "$DMG_DIR" ]]; then
	echo "📦 DMG-Dateien:"
	ls -lh "$DMG_DIR"/*.dmg 2>/dev/null || echo "  (keine .dmg gefunden)"
fi

if [[ -d "$APP_DIR" ]]; then
	echo ""
	echo "📦 App-Bundles:"
	ls -d "$APP_DIR"/*.app 2>/dev/null || echo "  (keine .app gefunden)"
fi
