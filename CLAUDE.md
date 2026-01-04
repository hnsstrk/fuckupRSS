# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Wichtige Projektdokumentation

**WICHTIG:** Bei jedem größeren Arbeitsschritt müssen folgende Dokumente geprüft, hinterfragt und ggf. aktualisiert werden:

| Dokument | Zweck | Prüfen bei |
|----------|-------|------------|
| `README.md` | Öffentliche Projektbeschreibung, Features, Installation | Neue Features, API-Änderungen, Installationsänderungen |
| `fuckupRSS-Anforderungen.md` | Technische Spezifikation, Architektur, Entscheidungen | Architekturänderungen, neue Komponenten, Abweichungen vom Plan |
| `CLAUDE.md` | Entwickler-Kontext für Claude Code | Build-Änderungen, neue Patterns, Strukturänderungen |

### Dokumentations-Workflow

1. **Vor Implementierung:** Anforderungsdokument lesen und verstehen
2. **Während Implementierung:** Bei Abweichungen vom Plan dokumentieren warum
3. **Nach Implementierung:** README.md und CLAUDE.md aktualisieren
4. **Bei Commits:** Prüfen ob Dokumentation angepasst werden muss

## Project Overview

fuckupRSS is an RSS aggregator/reader with local AI integration, named after F.U.C.K.U.P. from the Illuminatus! trilogy. It uses Ollama for local AI processing with no cloud dependencies.

**Status:** Phase 1 abgeschlossen (Grundgerüst)

### Implementierte Phasen

- [x] **Phase 1:** Grundgerüst (Tauri + Svelte, SQLite, Basis-UI)
- [ ] **Phase 1.5:** i18n & UX (Mehrsprachigkeit, Tooltips, Einstellungen)
- [ ] **Phase 2:** Core Features (Feed-Parsing, Volltext, Ollama-Integration)
- [ ] **Phase 3:** KI-Features (Discordian Analysis, Greyface Alert, Embeddings)
- [ ] **Phase 4:** Polish (Operation Mindfuck, OPML, Shortcuts)
- [ ] **Phase 5:** Release

## Technology Stack

- **Framework:** Tauri 2.x (Rust backend + Svelte 5 frontend)
- **Database:** SQLite + sqlite-vec (vector search, pure Rust)
- **AI Backend:** Ollama (local) with qwen3-vl:8b and nomic-embed-text models
- **Styling:** TailwindCSS
- **i18n:** svelte-i18n (DE/EN)
- **Target Platforms:** Linux (primary), macOS (secondary)

## Build Commands

```bash
# Install dependencies
npm install

# Development mode (startet Vite + Tauri)
npm run tauri dev

# Production build
npm run tauri build

# Nur Frontend entwickeln (ohne Tauri)
npm run dev
```

## Git Workflow & Commit-Strategie

### Branch-Strategie

| Branch | Zweck |
|--------|-------|
| `main` | Stabiler, lauffähiger Code |
| `feature/*` | Neue Features (z.B. `feature/feed-sync`) |
| `fix/*` | Bugfixes (z.B. `fix/article-status`) |
| `refactor/*` | Code-Verbesserungen ohne Feature-Änderung |

### Commit-Konventionen

Commits folgen dem [Conventional Commits](https://www.conventionalcommits.org/) Format:

```
<type>: <kurze Beschreibung>

<optionaler Body mit Details>

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

**Types:**
- `feat:` Neues Feature
- `fix:` Bugfix
- `refactor:` Code-Umstrukturierung
- `docs:` Nur Dokumentation
- `style:` Formatierung (kein Code-Change)
- `test:` Tests hinzufügen/ändern
- `chore:` Build-Prozess, Dependencies

### Wann committen?

- Nach Abschluss eines logischen Arbeitsschritts
- Vor größeren Refactorings (Sicherungspunkt)
- Bei funktionierendem Zwischenstand
- **Nicht:** Bei kaputtem Code auf `main`

### Push-Strategie

```bash
# Vor dem Push: Sicherstellen dass alles baut
npm run build
cargo check --manifest-path src-tauri/Cargo.toml

# Push zum Remote
git push origin main

# Oder bei Feature-Branches
git push origin feature/my-feature
```

### Pull Request Workflow (für größere Features)

1. Feature-Branch erstellen: `git checkout -b feature/name`
2. Entwickeln und committen
3. PR erstellen mit Beschreibung
4. Review und Merge

## Projektstruktur

```
fuckupRSS/
├── src/                          # Svelte 5 Frontend
│   ├── lib/
│   │   ├── components/           # UI-Komponenten
│   │   │   ├── Sidebar.svelte    # Feed-Liste (Pentacles)
│   │   │   ├── ArticleList.svelte # Artikel-Liste (Fnords)
│   │   │   └── ArticleView.svelte # Artikel-Ansicht
│   │   └── stores/
│   │       └── state.svelte.ts   # Runes-basiertes State Management
│   ├── App.svelte                # Haupt-Layout
│   └── app.css                   # TailwindCSS + Custom Styles
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── main.rs               # Entry Point
│   │   ├── lib.rs                # Tauri Setup + State
│   │   ├── db/                   # Datenbank-Layer
│   │   │   ├── mod.rs            # Database Struct
│   │   │   └── schema.rs         # SQLite Schema
│   │   └── commands/             # Tauri Commands (IPC)
│   │       ├── pentacles.rs      # Feed-Operationen
│   │       └── fnords.rs         # Artikel-Operationen
│   └── Cargo.toml
├── fuckupRSS-Anforderungen.md    # Technische Spezifikation
├── README.md                     # Projekt-Dokumentation
└── CLAUDE.md                     # Diese Datei
```

## i18n (Internationalisierung)

Phase 1.5 führt Mehrsprachigkeit mit `svelte-i18n` ein:

**Unterstützte Sprachen:** Deutsch (de), English (en)

**Struktur:**
```
src/lib/i18n/
├── index.ts          # i18n Setup
├── de.json           # Deutsche Übersetzungen
└── en.json           # English translations
```

**Verwendung in Svelte:**
```svelte
<script>
  import { _ } from 'svelte-i18n';
</script>
<h1>{$_('sidebar.title')}</h1>
```

## Tooltips für Illuminatus!-Begriffe

Alle Illuminatus!-Begriffe (Fnord, Pentacle, etc.) haben erklärende Tooltips:
- In Settings deaktivierbar (`showTerminologyTooltips`)
- Einheitliche `<Tooltip>` Komponente
- Übersetzungen in i18n-Dateien

## Key Rust Crates

| Purpose | Crate | Status |
|---------|-------|--------|
| Tauri Framework | `tauri` | ✅ Implementiert |
| SQLite | `rusqlite` | ✅ Implementiert |
| Serialization | `serde`, `serde_json` | ✅ Implementiert |
| DateTime | `chrono` | ✅ Implementiert |
| Error Handling | `thiserror` | ✅ Implementiert |
| RSS/Atom Parsing | `feed-rs` | ⏳ Phase 2 |
| HTTP Client | `reqwest` | ⏳ Phase 2 |
| Readability | `readability` | ⏳ Phase 2 |
| Ollama API | `ollama-rs` | ⏳ Phase 2 |
| Vector Search | `sqlite-vec` | ⏳ Phase 3 |
| OPML Parsing | `opml` | ⏳ Phase 4 |

## Illuminatus! Terminology

The codebase uses terms from the Illuminatus! trilogy:

| Code Term | Meaning | DB Table/Field |
|-----------|---------|----------------|
| Fnord | Unread article | `fnords.status = 'fnord'` |
| Illuminated | Read article | `fnords.status = 'illuminated'` |
| Golden Apple | Favorited article | `fnords.status = 'golden_apple'` |
| Pentacle | Feed source | `pentacles` |
| Sephiroth | Category | `sephiroth` |
| Immanentize | Keyword/tag | `immanentize` |
| Greyface Alert | Bias warning | `fnords.political_bias`, `sachlichkeit` |
| Discordian Analysis | AI summary | `fnords.summary` |
| Operation Mindfuck | User interests | `operation_mindfuck` |
| Hagbard's Retrieval | Full-text fetching | `fnords.content_full` |

## Database Schema Key Tables

- `pentacles` - Feed sources (URL, title, sync settings)
- `fnords` - Articles (content, status, bias scores)
- `sephiroth` - Categories (with default set)
- `immanentize` - Keywords/tags
- `fnord_sephiroth` - Article ↔ Category mapping
- `fnord_immanentize` - Article ↔ Tag mapping

Schema-Definition: `src-tauri/src/db/schema.rs`

## Tauri Commands (Frontend → Backend)

| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `get_pentacles` | - | `Vec<Pentacle>` | Alle Feeds mit Counts |
| `add_pentacle` | `url`, `title?` | `Pentacle` | Feed hinzufügen |
| `delete_pentacle` | `id` | - | Feed löschen |
| `get_fnords` | `filter?` | `Vec<Fnord>` | Artikel mit Filter |
| `get_fnord` | `id` | `Fnord` | Einzelner Artikel |
| `update_fnord_status` | `id`, `status` | - | Status ändern |

## AI Processing Pipeline (Phase 2+)

1. **Hagbard's Retrieval** - Fetch full text for truncated feeds
2. **Immanentizing** - Generate embeddings via nomic-embed-text
3. **Discordian Analysis** - Summarize, categorize, extract keywords via qwen3-vl:8b
4. **Greyface Alert** - Bias detection (political_bias: -2 to +2, sachlichkeit: 0-4)

## Ollama Setup (für Phase 2+)

```bash
# Install models
ollama pull qwen3-vl:8b
ollama pull nomic-embed-text

# Configure for dual model loading (Linux)
sudo systemctl edit ollama.service
# Add:
# [Service]
# Environment="OLLAMA_MAX_LOADED_MODELS=2"
# Environment="OLLAMA_FLASH_ATTENTION=1"
```

## Data Paths

- **Linux:** `~/.local/share/fuckupRSS/`
- **macOS:** `~/Library/Application Support/fuckupRSS/`

Datenbank: `fuckup.db` (SQLite mit WAL-Modus)
