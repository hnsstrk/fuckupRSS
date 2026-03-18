# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

fuckupRSS is an RSS aggregator/reader with local AI integration (Tauri v2 + Svelte 5 + Rust + SQLite), named after F.U.C.K.U.P. from the Illuminatus! trilogy. It supports Ollama (local) and OpenAI-compatible APIs for text generation. Ollama remains required for embeddings.

**Status:** Phase 4 (Polish & Advanced Features) — Ollama Modernization, Briefings, Story Clustering, NER, Article Type Classification abgeschlossen.

**Architektur-Dokumentation:** Obsidian Vault [[fuckupRSS Arc42 Architekturdokumentation]]

## Referenzdokumentation (docs/)

| Dokument | Inhalt |
|----------|--------|
| [docs/README.md](docs/README.md) | Navigation Hub fuer alle docs/ |
| [docs/api/TAURI_COMMANDS_REFERENCE.md](docs/api/TAURI_COMMANDS_REFERENCE.md) | Alle Tauri Commands (Frontend -> Backend) |
| [docs/architecture/AI_PROCESSING_PIPELINE.md](docs/architecture/AI_PROCESSING_PIPELINE.md) | KI-Pipeline, Prompt-Design, Keyword-Extraktion |
| [docs/architecture/DATABASE_SCHEMA.md](docs/architecture/DATABASE_SCHEMA.md) | DB-Tabellen, Revisionsverwaltung, Settings |
| [docs/guides/TESTING.md](docs/guides/TESTING.md) | Test-Befehle, Patterns, Anforderungen |
| [docs/guides/CI_CD_SETUP.md](docs/guides/CI_CD_SETUP.md) | CI/CD Pipeline, Gitea Actions Runner |
| [docs/guides/QUALITY_CHECKLIST.md](docs/guides/QUALITY_CHECKLIST.md) | Frontend-Backend-Kommunikation Checkliste |
| [docs/guides/HARDWARE_OPTIMIZATION.md](docs/guides/HARDWARE_OPTIMIZATION.md) | VRAM-Optimierung, Ollama-Konfiguration |
| [docs/ANFORDERUNGEN.md](docs/ANFORDERUNGEN.md) | Roadmap, Governance, Entscheidungen |
| [README.md](README.md) | Technology Stack, Illuminatus! Terminologie, Ollama Setup |

**WICHTIG:** Bei groesseren Arbeitsschritten muessen README.md, CLAUDE.md, docs/ANFORDERUNGEN.md und docs/guides/QUALITY_CHECKLIST.md geprueft und ggf. aktualisiert werden.

## Task Management

Alle offenen Tasks in **Taskwarrior** (Projekt: `fuckupRSS`):

```bash
task project:fuckupRSS list          # Alle offenen Tasks
task project:fuckupRSS +bug list     # Nur Bugs
task project:fuckupRSS +refactor list
```

## Build Commands

```bash
npm install                          # Dependencies
npm run tauri dev                    # Development (Vite + Tauri)
npm run tauri build                  # Production Build
npm run dev                          # Nur Frontend (ohne Tauri)
```

## Testing

Siehe [docs/guides/TESTING.md](docs/guides/TESTING.md) fuer die vollstaendige Dokumentation.

```bash
npm run test                                     # Frontend (Vitest)
npm run test:e2e                                 # E2E (Playwright)
cargo test --manifest-path src-tauri/Cargo.toml  # Backend (Rust)
```

**WICHTIG:** Alle neuen Features und Bugfixes MUESSEN mit Tests abgedeckt werden.

## Linting & Formatting

```bash
npm run lint && npm run lint:fix     # ESLint
npm run format                       # Prettier
npm run rust:fmt                     # Rust formatieren
npm run rust:clippy                  # Clippy
npm run security:scan                # Semgrep
```

## Projektstruktur

```
fuckupRSS/
├── src/                          # Svelte 5 Frontend
│   ├── lib/
│   │   ├── components/           # UI-Komponenten
│   │   ├── stores/               # Runes-basiertes State Management
│   │   ├── i18n/                 # Internationalisierung (de, en)
│   │   └── utils/                # Hilfsfunktionen (sanitizer.ts)
│   ├── App.svelte
│   └── app.css                   # TailwindCSS + Custom Styles
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── lib.rs                # Tauri Setup + State
│   │   ├── ai_provider/          # AI Provider Abstraction (Ollama + OpenAI)
│   │   ├── proxy.rs              # Ollama LAN-Proxy
│   │   ├── db/                   # Database (schema.rs)
│   │   ├── commands/             # Tauri Commands (IPC)
│   │   └── keywords/             # Keyword-Extraktion + Deduplication
│   └── Cargo.toml
├── docs/                         # Referenzdokumentation (siehe docs/README.md)
├── scripts/                      # Build-Scripts (build-macos.sh)
├── .gitea/workflows/             # CI/CD (ci.yaml, release.yaml)
└── .husky/                       # Git Hooks (pre-commit, pre-push)
```

## Kritische Konventionen

### Commit-Konventionen

[Conventional Commits](https://www.conventionalcommits.org/) Format: `feat:`, `fix:`, `refactor:`, `docs:`, `style:`, `test:`, `chore:`

**Commit-Frequenz:** Nach jedem logischen Arbeitsschritt sofort committen. Lieber zu viele kleine als zu wenige grosse Commits.

### Branch-Strategie

`main` (stabil), `feature/*`, `fix/*`, `refactor/*`

### Database Patterns

SQLite mit `Arc<Mutex<Connection>>`. Kritische Regeln:

- **Locks kurz halten** — nur fuer DB-Operation, keine I/O waehrend Lock
- **Pro-Item Locks** — in Loops Lock pro Item, nicht fuer gesamte Schleife
- **Transactions** — zusammengehoerige Operationen MUESSEN in Transaction
- **Yield nach Release** — `tokio::task::yield_now().await` fuer Concurrency

Vollstaendige Patterns: [docs/architecture/DATABASE_SCHEMA.md](docs/architecture/DATABASE_SCHEMA.md)

### Frontend Event-System

Komponenten die Backend-Daten anzeigen MUESSEN auf CustomEvents lauschen:

| Event | Wann |
|-------|------|
| `batch-complete` | Nach Batch-Processing |
| `keywords-changed` | Nach Keyword-Mutationen |

Pattern: `onMount` → `addEventListener`, `onDestroy` → `removeEventListener`

### Markdown-Rendering (LLM-Texte)

Zentrale Funktion: `src/lib/utils/sanitizer.ts` → `renderMarkdown(markdown)`

```svelte
<div class="markdown-content">{@html renderMarkdown(text)}</div>
```

**WICHTIG:** Fuer LLM-Texte immer `renderMarkdown()`, nie `sanitizeArticleContent()`.

### Icons

Font Awesome 6.4 Pro: `static/fontawesome/`. Styles: `fa-solid`, `fa-regular`, `fa-light`, `fa-thin`, `fa-brands`, `fa-duotone`

## Ollama Setup

```bash
ollama pull ministral-3:latest           # Text-Generierung
ollama pull snowflake-arctic-embed2:latest # Embeddings
```

- **API:** `/api/chat` (Structured Outputs, JSON Schema) — erfordert Ollama 0.5.0+
- **Embeddings:** `/api/embed` Batch-Endpunkt, snowflake-arctic-embed2 (8.192 Tokens)
- **Alternative:** OpenAI-kompatible API konfigurierbar in Settings (Ollama bleibt fuer Embeddings)
- **Concurrency:** `ollama_concurrency` Setting (Standard: 1)

## AI Pipeline (Kurzuebersicht)

1. **Hagbard's Retrieval** — Volltext abrufen
2. **Discordian Analysis** — Zusammenfassung, Kategorien, Keywords, Artikeltyp, NER (Structured Outputs)
3. **Article Embedding** — title + summary + content_full (snowflake-arctic-embed2)
4. **Greyface Alert** — Bias-Erkennung (political_bias, sachlichkeit)
5. **Immanentize Network** — Schlagwort-Verarbeitung und Synonyme
6. **Story Clustering** — Union-Find (Embedding-Aehnlichkeit > 0.78)
7. **Briefings** — Hybrid-Scoring + Diversitaets-Postprocessing (BRIEFING_NUM_CTX=16384)
8. **NER** — Entitaeten (person, organization, location, event)

Details: [docs/architecture/AI_PROCESSING_PIPELINE.md](docs/architecture/AI_PROCESSING_PIPELINE.md)

## Data Paths

- **Datenbank:** `src-tauri/data/fuckup.db` (SQLite WAL, in .gitignore)
- **Schnellzugriff:** `sqlite3 /Users/hnsstrk/Repositories/fuckupRSS/src-tauri/data/fuckup.db`

## MCP-Server

Konfiguration in `.mcp.json`: `ollama` (KI-Interaktion), `fetch` (Web-Requests), `memory` (persistenter Kontext)

## Git Hooks (Husky)

- **Pre-commit:** ESLint+Prettier (staged), cargo fmt+clippy (.rs)
- **Pre-push:** Vitest, cargo test, svelte-check

## CI/CD

Pipeline: `.gitea/workflows/ci.yaml` — Security Scan + SBOM auf Callisto (Linux). Release: Tag-basiert (`git tag v1.x.x && git push --tags`). macOS-Build: lokal via `scripts/build-macos.sh`.

Details: [docs/guides/CI_CD_SETUP.md](docs/guides/CI_CD_SETUP.md)
