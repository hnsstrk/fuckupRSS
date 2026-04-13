# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

fuckupRSS is an RSS aggregator/reader with local AI integration (Tauri v2 + Svelte 5 + Rust + SQLite), named after F.U.C.K.U.P. from the Illuminatus! trilogy. It supports Ollama (local) and OpenAI-compatible APIs for text generation. Ollama remains required for embeddings.

**Status:** Experimental hobby project — actively developed. Features include Ollama Modernization, Briefings, Theme Reports, NER, Article Type Classification.

## Reference Documentation (docs/)

| Document | Contents |
|----------|----------|
| [docs/README.md](docs/README.md) | Navigation hub for all docs/ |
| [docs/PROCESSING_PIPELINE.md](docs/PROCESSING_PIPELINE.md) | How articles are processed (AI pipeline, providers, batch flow) |
| [docs/guides/TESTING.md](docs/guides/TESTING.md) | Test commands, patterns, requirements |
| [docs/guides/QUALITY_CHECKLIST.md](docs/guides/QUALITY_CHECKLIST.md) | Frontend-backend communication checklist |
| [README.md](README.md) | Project overview, tech stack, setup |

**IMPORTANT:** For major changes, check and update README.md, CLAUDE.md, and docs/guides/QUALITY_CHECKLIST.md.

## Task Management

All open tasks tracked in **Taskwarrior** (project: `fuckupRSS`):

```bash
task project:fuckupRSS list          # All open tasks
task project:fuckupRSS +bug list     # Bugs only
task project:fuckupRSS +refactor list
```

## Build Commands

```bash
npm install                          # Dependencies
npm run tauri dev                    # Development (Vite + Tauri)
npm run tauri build                  # Production build
npm run dev                          # Frontend only (without Tauri)
```

## Testing

See [docs/guides/TESTING.md](docs/guides/TESTING.md) for full documentation.

```bash
npm run test                                     # Frontend (Vitest)
npm run test:e2e                                 # E2E (Playwright)
cargo test --manifest-path src-tauri/Cargo.toml  # Backend (Rust)
```

**IMPORTANT:** All new features and bugfixes MUST include tests.

## Linting & Formatting

```bash
npm run lint && npm run lint:fix     # ESLint
npm run format                       # Prettier
npm run rust:fmt                     # Rust formatting
npm run rust:clippy                  # Clippy
npm run security:scan                # Semgrep
```

## Project Structure

```
fuckupRSS/
├── src/                          # Svelte 5 Frontend
│   ├── lib/
│   │   ├── components/           # UI components
│   │   │   ├── article/          # Article detail (KeywordChip, Suggestions, Search)
│   │   │   ├── keywords/         # Compound keyword management (Toolbar, Table)
│   │   │   ├── network/          # Keyword network (Detail, Synonyms, sub-components)
│   │   │   ├── recommendation/   # Recommendation cards
│   │   │   ├── theme/            # Theme Reports sub-components
│   │   │   └── settings/         # Settings sub-components (AI, Ollama, Prompts, etc.)
│   │   ├── stores/               # Runes-based state management
│   │   ├── i18n/                 # Internationalization (de, en)
│   │   └── utils/                # Utility functions (sanitizer.ts)
│   ├── App.svelte
│   └── app.css                   # TailwindCSS + custom styles
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── lib.rs                # Tauri setup + state
│   │   ├── ai_provider/          # AI provider abstraction (Ollama + OpenAI)
│   │   ├── proxy.rs              # Ollama LAN proxy
│   │   ├── db/                   # Database (schema.rs)
│   │   ├── theme_clustering.rs   # Theme Reports (Multi-Signal Topic Detection + LLM)
│   │   ├── commands/             # Tauri commands (IPC) — incl. theme_report
│   │   └── keywords/             # Keyword extraction + deduplication
│   └── Cargo.toml
├── docs/                         # Reference documentation (see docs/README.md)
├── scripts/                      # Build scripts (build-macos.sh)
├── .github/workflows/            # CI/CD (ci.yml, release.yml)
└── .husky/                       # Git hooks (pre-commit, pre-push)
```

## Critical Conventions

### Commit Conventions

[Conventional Commits](https://www.conventionalcommits.org/) format: `feat:`, `fix:`, `refactor:`, `docs:`, `style:`, `test:`, `chore:`

**Commit frequency:** Commit after each logical work step. Prefer many small commits over few large ones.

### Branch Strategy

`main` (stable), `feature/*`, `fix/*`, `refactor/*`

### Database Patterns

SQLite with `Arc<Mutex<Connection>>`. Critical rules:

- **Keep locks short** — only for DB operations, no I/O while holding lock
- **Per-item locks** — in loops, acquire lock per item, not for entire loop
- **Transactions** — related operations MUST use transactions
- **Yield after release** — `tokio::task::yield_now().await` for concurrency

Full patterns: [docs/architecture/DATABASE_SCHEMA.md](docs/architecture/DATABASE_SCHEMA.md)

### Frontend Event System

Components displaying backend data MUST listen to CustomEvents:

| Event | When |
|-------|------|
| `batch-complete` | After batch processing |
| `keywords-changed` | After keyword mutations |

Pattern: `onMount` → `addEventListener`, `onDestroy` → `removeEventListener`

### Markdown Rendering (LLM Text)

Central function: `src/lib/utils/sanitizer.ts` → `renderMarkdown(markdown)`

```svelte
<div class="markdown-content">{@html renderMarkdown(text)}</div>
```

**IMPORTANT:** For LLM text always use `renderMarkdown()`, never `sanitizeArticleContent()`.

### Icons

Font Awesome Free 6.7.2: `static/fontawesome/`. Styles: `fa-solid`, `fa-regular`, `fa-brands`

## Ollama Setup

```bash
ollama pull ministral-3:latest           # Text generation
ollama pull snowflake-arctic-embed2:latest # Embeddings
```

- **API:** `/api/chat` (Structured Outputs, JSON Schema) — requires Ollama 0.5.0+
- **Embeddings:** `/api/embed` batch endpoint, snowflake-arctic-embed2 (8,192 tokens)
- **Alternative:** OpenAI-compatible API configurable in Settings (Ollama remains required for embeddings)
- **Concurrency:** `ollama_concurrency` setting (default: 1)

## AI Pipeline (Overview)

1. **Hagbard's Retrieval** — Full-text fetching
2. **Discordian Analysis** — Summary, categories, keywords, article type, NER (Structured Outputs)
3. **Article Embedding** — title + summary + content_full (snowflake-arctic-embed2)
4. **Greyface Alert** — Bias detection (political_bias, sachlichkeit)
5. **Immanentize Network** — Keyword processing and synonyms
6. **Theme Reports** — Multi-Signal Topic Detection + LLM deep analysis
7. **Briefings** — Hybrid scoring + diversity post-processing (BRIEFING_NUM_CTX=16384)
8. **NER** — Entities (person, organization, location, event)

Details: [docs/PROCESSING_PIPELINE.md](docs/PROCESSING_PIPELINE.md)

## Data Paths

- **Database:** `src-tauri/data/fuckup.db` (SQLite WAL, in .gitignore)

## MCP Servers

Configuration in `.mcp.json`: `ollama` (AI interaction), `fetch` (web requests), `memory` (persistent context)

## Git Hooks (Husky)

- **Pre-commit:** ESLint+Prettier (staged), cargo fmt+clippy (.rs)
- **Pre-push:** Vitest, cargo test, svelte-check

## CI/CD

Pipeline: `.github/workflows/ci.yml` — Lint, tests, Rust clippy, Svelte type check, npm audit, cargo audit (on `ubuntu-latest`). Release: `.github/workflows/release.yml`, triggered by tag push (`git tag v1.x.x && git push --tags`), builds Linux artifacts (.deb, .AppImage) and creates a GitHub Release. macOS build: local via `scripts/build-macos.sh`.
