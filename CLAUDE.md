# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Wichtige Projektdokumentation

**WICHTIG:** Bei jedem groesseren Arbeitsschritt muessen folgende Dokumente geprueft, hinterfragt und ggf. aktualisiert werden:

| Dokument | Zweck | Pruefen bei |
|----------|-------|------------|
| `README.md` | Oeffentliche Projektbeschreibung, Features, Installation | Neue Features, API-Aenderungen, Installationsaenderungen |
| `docs/ANFORDERUNGEN.md` | Roadmap, Governance, Entscheidungen | Phase-Updates, Architekturaenderungen |
| `CLAUDE.md` | Entwickler-Kontext fuer Claude Code | Build-Aenderungen, neue Patterns, Strukturaenderungen |
| `docs/guides/QUALITY_CHECKLIST.md` | Frontend-Backend-Kommunikation Checkliste | Neue invoke-Calls, Event-Listener, State-Updates |

### Dokumentations-Workflow

1. **Vor Implementierung:** Anforderungsdokument lesen und verstehen
2. **Waehrend Implementierung:** Bei Abweichungen vom Plan dokumentieren warum
3. **Nach Implementierung:** README.md und CLAUDE.md aktualisieren
4. **Bei Commits:** Pruefen ob Dokumentation angepasst werden musst

## Quick Links - Referenzdokumentation

| Dokument | Inhalt |
|----------|--------|
| [docs/api/TAURI_COMMANDS_REFERENCE.md](docs/api/TAURI_COMMANDS_REFERENCE.md) | Alle Tauri Commands (Frontend -> Backend) |
| [docs/architecture/AI_PROCESSING_PIPELINE.md](docs/architecture/AI_PROCESSING_PIPELINE.md) | KI-Pipeline, Greyface Alert, Prompt-Design, Keyword-Extraktion |
| [docs/architecture/DATABASE_SCHEMA.md](docs/architecture/DATABASE_SCHEMA.md) | Datenbank-Tabellen, Revisionsverwaltung, Settings |
| [docs/guides/TESTING.md](docs/guides/TESTING.md) | Test-Befehle, Patterns, Anforderungen |
| [docs/guides/HARDWARE_OPTIMIZATION.md](docs/guides/HARDWARE_OPTIMIZATION.md) | VRAM-Optimierung, Ollama-Konfiguration |
| [docs/guides/CI_CD_SETUP.md](docs/guides/CI_CD_SETUP.md) | CI/CD Pipeline, Gitea Actions Runner Setup |
| [README.md](README.md) | Technology Stack, Illuminatus! Terminologie, Ollama Setup |

## Project Overview

fuckupRSS is an RSS aggregator/reader with local AI integration, named after F.U.C.K.U.P. from the Illuminatus! trilogy. It supports both Ollama (local) and OpenAI-compatible APIs for text generation. Ollama remains required for embeddings. The AI provider is configurable via Settings.

**Status:** Phase 3 abgeschlossen, Phase 4 (Polish) in Entwicklung

**Planung:** Alle Phasen und Tasks sind in [`docs/ANFORDERUNGEN.md`](docs/ANFORDERUNGEN.md#5-roadmap-nächste-schritte) dokumentiert.

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

**Alle npm Scripts:**
```bash
# Code-Qualitaet
npm run lint              # ESLint
npm run lint:fix          # ESLint auto-fix
npm run format            # Prettier write
npm run format:check      # Prettier check
npm run rust:fmt          # Rust formatieren
npm run rust:fmt:check    # Rust Format pruefen
npm run rust:clippy       # Clippy

# Security
npm run security:scan     # Semgrep (auto rules)
npm run security:owasp    # Semgrep (OWASP Top 10)
npm run security:audit    # npm audit + cargo audit

# SBOM
npm run sbom:generate     # Frontend + Backend SBOMs
npm run sbom:validate     # SBOMs validieren

# Release (Tag-basiert, loest Release-Workflow aus)
git tag v1.x.x && git push --tags

# macOS Build (lokal, kein CI-Runner verfuegbar)
./scripts/build-macos.sh             # Production Build
./scripts/build-macos.sh --debug     # Debug Build
./scripts/build-macos.sh --clean     # Clean Build
```

## Testing

Siehe [docs/guides/TESTING.md](docs/guides/TESTING.md) fuer die vollstaendige Test-Dokumentation.

**Quick Commands:**
```bash
npm run test                                      # Frontend (Vitest)
npm run test:e2e                                  # E2E Tests (Playwright)
npm run test:watch                                # Frontend Watch-Modus
npm run test:coverage                             # Frontend mit Coverage
cargo test --manifest-path src-tauri/Cargo.toml  # Backend (Rust)
```

**Playwright CLI (interaktives Browser-Testing):**
```bash
npm run pw:open                                   # Browser oeffnen (Chrome, localhost:1420)
npm run pw:snapshot                               # Accessibility Snapshot
npm run pw:screenshot                             # Screenshot
npm run pw:close                                  # Browser schliessen
```

Konfiguration: `playwright-cli.json` (Browser, baseURL, Timeout). Skills: `.claude/skills/playwright-cli/`.

**WICHTIG:** Alle neuen Features und Bugfixes MUESSEN mit Tests abgedeckt werden.

## Security & Quality Tools

Folgende Tools sind lokal installiert und muessen bei Code-Reviews und vor Releases verwendet werden:

### Semgrep (Static Analysis)

| Eigenschaft | Wert |
|-------------|------|
| Command | `semgrep` |
| Pfad | `/opt/homebrew/bin/semgrep` |
| Version | 1.151.0 |
| Installation | `brew install semgrep` |

**Verwendung:** Statische Code-Analyse fuer Security-Schwachstellen (OWASP Top 10, XSS, SQL-Injection, Command-Injection, Secrets).

**WICHTIG:** Semgrep-Scans sind verpflichtend bei Code-Reviews.

**Quick Commands:**
```bash
# Rust Backend scannen
semgrep scan --config auto src-tauri/src/

# Frontend scannen
semgrep scan --config auto src/

# OWASP Top 10 Pruefung
semgrep scan --config p/owasp-top-ten src-tauri/src/ src/

# Nur geaenderte Dateien scannen
semgrep scan --config auto <datei1> <datei2> ...
```

### CycloneDX (SBOM)

| Eigenschaft | Wert |
|-------------|------|
| Command | `cyclonedx` |
| Pfad | `/opt/homebrew/bin/cyclonedx` |
| Version | 0.30.0 |
| Installation | `brew install cyclonedx-cli` |

**Verwendung:** Software Bill of Materials (SBOM) - Erzeugen, Validieren und Analysieren von Abhaengigkeitslisten im CycloneDX-Format fuer Supply-Chain-Security.

**Quick Commands:**
```bash
# SBOMs generieren (Frontend + Backend)
npm run sbom:generate

# SBOMs validieren
npm run sbom:validate
```

## Linting & Formatting

### Frontend (ESLint + Prettier)

```bash
npm run lint              # ESLint pruefen
npm run lint:fix          # ESLint auto-fix
npm run format            # Prettier formatieren
npm run format:check      # Prettier pruefen (CI)
```

**Config-Dateien:**
- `eslint.config.js` - ESLint 9.x Flat Config (TypeScript + Svelte)
- `.prettierrc` - Prettier Config (printWidth=100, singleQuote=false)
- `.prettierignore` - Prettier Ausnahmen
- `.editorconfig` - Editor-Einstellungen (LF, indent)

### Rust (rustfmt + Clippy)

```bash
npm run rust:fmt          # Rust formatieren
npm run rust:fmt:check    # Rust Format pruefen (CI)
npm run rust:clippy       # Clippy Lint-Check
```

**Config-Dateien:**
- `src-tauri/rustfmt.toml` - max_width=100, edition=2021
- `src-tauri/clippy.toml` - too-many-arguments-threshold=8

## Git Hooks (Husky)

Automatische Qualitaetssicherung via Git Hooks (Husky 9 + lint-staged).

### Pre-commit (bei jedem Commit)
- **Frontend:** ESLint --fix + Prettier auf staged Dateien (via lint-staged)
- **Rust:** cargo fmt --check + cargo clippy (nur bei .rs-Aenderungen)

### Pre-push (vor jedem Push)
- Frontend-Tests (vitest)
- Rust-Tests (cargo test --lib --bins)
- svelte-check Typ-Pruefung

**Hooks umgehen (nur in Ausnahmen!):** `git commit --no-verify` / `git push --no-verify`

## CI/CD (Gitea Actions)

Pipeline in `.gitea/workflows/ci.yaml`. Release-Workflow in `.gitea/workflows/release.yaml`. Ausfuehrliches Setup-Guide: [docs/guides/CI_CD_SETUP.md](docs/guides/CI_CD_SETUP.md)

### Build-Strategie

| Was | Wo | Wie |
|-----|----|-----|
| Lint, Tests, Security, SBOM | **Callisto** (Linux-Runner) | Automatisch bei Push/PR |
| Linux-Build (.deb, .AppImage) | **Callisto** (Linux-Runner) | Automatisch in CI |
| macOS-Build (.dmg, .app) | **Lokal** (MacBook) | Manuell via `./scripts/build-macos.sh` |

**Entwickler-Workflow:** Push → Callisto prueft (Lint, Tests, Security, Linux-Build) → bei Bedarf `./scripts/build-macos.sh` lokal ausfuehren.

### CI-Pipeline (Callisto, Linux-only)

**Pipeline-Stages (parallelisiert):**
1. **Lint** (parallel) - `lint`: ESLint, Prettier, svelte-check, tsc --noEmit | `rust-lint`: cargo fmt, Clippy
2. **Tests** (parallel) - Vitest mit Coverage, cargo test, E2E (Playwright gegen Vite-Dev-Server)
3. **Security** - Semgrep (auto + OWASP Top 10), npm audit, cargo audit --deny warnings
4. **Build** - Linux (.deb, .AppImage)
5. **SBOM** - CycloneDX Frontend + Backend

**Coverage:** Frontend-Tests erzeugen Coverage-Artefakte (30 Tage Aufbewahrung).

### Release-Workflow (Tag-basiert)

- Ausgeloest durch `v*`-Tags (`git tag v1.x.x && git push --tags`)
- Baut Linux + macOS parallel, erstellt Gitea Release mit Changelog + Artefakten
- Benoetigt `GITEATOKEN` Secret in Gitea Repository-Settings

### Runner und Einschraenkungen

**Runner:** act_runner im Host-Modus, nur `linux-x64`. Docker-basiert (`docker.gitea.com/runner-images:ubuntu-latest`), Container laeuft als root.

**Bekannte Einschraenkungen:**
- **Kein macOS-Runner** — macOS-Builds nur lokal via `scripts/build-macos.sh`
- **Gitea Act Runner:** nur `upload-artifact@v3` (nicht v4), kein `sudo` im Container noetig (root)
- **E2E-Tests in CI:** laufen nur gegen Vite-Dev-Server (kein Tauri-Backend verfuegbar)
- **Node-Version:** gepinnt auf 22 (via CI, `.nvmrc`, `package.json engines`)

## Icons

Font Awesome 6.4 Pro liegt lokal unter `static/fontawesome/`. Eingebunden via `app.html`:

```html
<link rel="stylesheet" href="/fontawesome/css/all.min.css">
```

Nutzung:
```svelte
<i class="fa-solid fa-rss"></i>
<i class="fa-light fa-newspaper"></i>
```

Verfuegbare Styles: `fa-solid`, `fa-regular`, `fa-light`, `fa-thin`, `fa-brands`, `fa-duotone`

## Git Workflow & Commit-Strategie

### Branch-Strategie

| Branch | Zweck |
|--------|-------|
| `main` | Stabiler, lauffaehiger Code |
| `feature/*` | Neue Features (z.B. `feature/feed-sync`) |
| `fix/*` | Bugfixes (z.B. `fix/article-status`) |
| `refactor/*` | Code-Verbesserungen ohne Feature-Aenderung |

### Commit-Konventionen

Commits folgen dem [Conventional Commits](https://www.conventionalcommits.org/) Format:

```
<type>: <kurze Beschreibung>

<optionaler Body mit Details>

Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

**Types:**
- `feat:` Neues Feature
- `fix:` Bugfix
- `refactor:` Code-Umstrukturierung
- `docs:` Nur Dokumentation
- `style:` Formatierung (kein Code-Change)
- `test:` Tests hinzufuegen/aendern
- `chore:` Build-Prozess, Dependencies

### Wann committen?

- Nach Abschluss eines logischen Arbeitsschritts
- Vor groesseren Refactorings (Sicherungspunkt)
- Bei funktionierendem Zwischenstand
- **Nicht:** Bei kaputtem Code auf `main`

### Commit-Frequenz (Claude Code Richtlinie)

**WICHTIG:** Claude Code muss regelmaessig committen, um Arbeit nicht zu verlieren und den Fortschritt nachvollziehbar zu machen.

| Situation | Commit-Regel |
|-----------|--------------|
| Feature abgeschlossen | **Sofort committen** |
| 2-3 zusammenhaengende Aenderungen | **Committen** (nicht sammeln) |
| Bugfix erledigt | **Sofort committen** |
| Refactoring-Schritt fertig | **Committen** |
| Vor Themenwechsel | **Committen** (aktuelles Thema abschliessen) |
| Nach 15-20 Minuten Arbeit | **Pruefen** ob Commit sinnvoll |
| Benutzer fragt nach anderem Thema | **Erst committen**, dann Thema wechseln |

**Faustregel:** Lieber zu viele kleine Commits als zu wenige grosse.

**Anti-Pattern vermeiden:**
- Mehrere unabhaengige Features in einem Commit
- Stundenlang arbeiten ohne Commit
- "Ich committe spaeter" - NEIN, jetzt committen!
- Auf Benutzer-Erinnerung warten

**Selbst-Check nach jeder Aufgabe:**
```
Kompiliert der Code? -> git add && git commit
Feature/Fix fertig? -> git add && git commit
Wechsle ich das Thema? -> git add && git commit
```

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

### Pull Request Workflow (fuer groessere Features)

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
│   │       ├── state.svelte.ts   # Runes-basiertes State Management
│   │       └── network.svelte.ts # Immanentize Network Store (Keywords, Trending, Graph)
│   ├── App.svelte                # Haupt-Layout
│   └── app.css                   # TailwindCSS + Custom Styles
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── main.rs               # Entry Point
│   │   ├── lib.rs                # Tauri Setup + State
│   │   ├── ai_provider/          # AI Provider Abstraction
│   │   │   ├── mod.rs            # AiTextProvider Trait + Factory
│   │   │   ├── ollama_provider.rs # Ollama-Implementierung
│   │   │   └── openai_provider.rs # OpenAI-kompatible API-Implementierung
│   │   ├── db/                   # Datenbank-Layer
│   │   │   ├── mod.rs            # Database Struct
│   │   │   └── schema.rs         # SQLite Schema
│   │   └── commands/             # Tauri Commands (IPC)
│   │       ├── ai/               # KI-Provider Commands (Test, Kosten)
│   │       ├── pentacles.rs      # Feed-Operationen
│   │       └── fnords.rs         # Artikel-Operationen
│   ├── Cargo.toml
│   ├── rustfmt.toml              # Rust Formatierung (max_width=100)
│   └── clippy.toml               # Clippy Config (threshold=8)
├── docs/                         # Referenzdokumentation
│   ├── api/
│   │   └── TAURI_COMMANDS_REFERENCE.md
│   ├── architecture/
│   │   ├── AI_PROCESSING_PIPELINE.md
│   │   └── DATABASE_SCHEMA.md
│   └── guides/
│       ├── TESTING.md
│       ├── QUALITY_CHECKLIST.md
│       └── CI_CD_SETUP.md
├── .claude/                      # Claude Code Konfiguration
│   └── skills/
│       └── playwright-cli/       # Playwright CLI Skills
│           ├── SKILL.md          # Skill-Definition und Commands
│           └── references/       # Detaillierte Anleitungen
├── .gitea/                       # CI/CD
│   └── workflows/
│       └── ci.yaml               # Gitea Actions Pipeline
├── .husky/                       # Git Hooks
│   ├── pre-commit                # lint-staged + Rust checks
│   └── pre-push                  # Tests + svelte-check
├── eslint.config.js              # ESLint 9.x Flat Config
├── .prettierrc                   # Prettier Config
├── .prettierignore               # Prettier Ausnahmen
├── .editorconfig                 # Editor-Einstellungen
├── .semgrepignore                # Semgrep Ausnahmen
├── fuckupRSS-Anforderungen.md    # Technische Spezifikation
├── playwright-cli.json           # Playwright CLI Konfiguration
├── README.md                     # Projekt-Dokumentation
├── CLAUDE.md                     # Diese Datei
└── TODO.md                       # Zentrale Aufgabenliste
```

## i18n (Internationalisierung)

**Unterstuetzte Sprachen:** Deutsch (de), English (en)

**Struktur:**
```
src/lib/i18n/
├── index.ts          # i18n Setup
├── de.json           # Deutsche Uebersetzungen
└── en.json           # English translations
```

**Verwendung in Svelte:**
```svelte
<script>
  import { _ } from 'svelte-i18n';
</script>
<h1>{$_('sidebar.title')}</h1>
```

## Tooltips fuer Illuminatus!-Begriffe

Alle Illuminatus!-Begriffe (Fnord, Pentacle, etc.) haben erklaerende Tooltips:
- In Settings deaktivierbar (`showTerminologyTooltips`)
- Einheitliche `<Tooltip>` Komponente
- Uebersetzungen in i18n-Dateien

Siehe [README.md](README.md#illuminatus-terminology) fuer die vollstaendige Terminologie-Tabelle.

## Key Rust Crates

| Purpose | Crate | Status |
|---------|-------|--------|
| Tauri Framework | `tauri` | aktiv |
| SQLite | `rusqlite` | aktiv |
| Serialization | `serde`, `serde_json` | aktiv |
| DateTime | `chrono` | aktiv |
| Error Handling | `thiserror` | aktiv |
| RSS/Atom Parsing | `feed-rs` | aktiv |
| HTTP Client | `reqwest` | aktiv |
| Readability | `readability` | aktiv |
| Ollama API | `ollama-rs` | aktiv |
| Vector Search | `sqlite-vec` | aktiv (bundled, O(log n) KNN) |
| OPML Parsing | `opml` | aktiv |
| Async Traits | `async-trait` | aktiv (AiTextProvider Trait) |

## Tauri Commands

Siehe [docs/api/TAURI_COMMANDS_REFERENCE.md](docs/api/TAURI_COMMANDS_REFERENCE.md) fuer die vollstaendige Command-Referenz.

**Haeufig verwendete Commands:**
```typescript
// Feeds
await invoke('get_pentacles');
await invoke('add_pentacle', { url, title });
await invoke('sync_all_feeds');

// Artikel
await invoke('get_fnords', { filter });
await invoke('update_fnord_status', { id, status });

// KI-Verarbeitung
await invoke('process_batch', { limit });
await invoke('check_ollama');

// KI-Provider
await invoke('test_ai_provider');
await invoke('get_monthly_cost', { year, month });
await invoke('get_cost_history', { months });
```

## Database Schema

Siehe [docs/architecture/DATABASE_SCHEMA.md](docs/architecture/DATABASE_SCHEMA.md) fuer die vollstaendige Schema-Dokumentation.

**Kern-Tabellen:**
- `pentacles` - Feed-Quellen
- `fnords` - Artikel
- `sephiroth` - Kategorien (13 fest definiert)
- `immanentize` - Keywords mit Embeddings
- `ai_cost_log` - Kostenprotokoll fuer OpenAI-kompatible API-Aufrufe

Schema-Definition: `src-tauri/src/db/schema.rs`

## Database Patterns & Best Practices

Dieses Projekt verwendet SQLite mit einem `Arc<Mutex<Connection>>` Pattern. Bei unsachgemaesser Verwendung koennen Database-Locks, Race Conditions und inkonsistente Daten entstehen.

### Lock-Halte-Regeln

**WICHTIG:** Database Locks muessen so kurz wie moeglich gehalten werden.

| Regel | Beschreibung |
|-------|--------------|
| **Kurze Locks** | Lock nur fuer die tatsaechliche DB-Operation halten |
| **Keine I/O waehrend Lock** | Keine Netzwerk-Requests, File-I/O oder LLM-Calls waehrend der Lock gehalten wird |
| **Pro-Item Locks** | In Loops: Lock pro Item, nicht fuer gesamte Schleife |
| **Yield nach Release** | Nach Lock-Release `tokio::task::yield_now().await` fuer bessere Concurrency |

**Korrektes Pattern (pro Item):**
```rust
for item in items {
    // Lock nur fuer DB-Operation
    {
        let conn = db.lock().unwrap();
        conn.execute("UPDATE ...", params![item.id])?;
    } // Lock wird hier released

    // Yield fuer andere Tasks
    tokio::task::yield_now().await;

    // Externe Operationen OHNE Lock
    let result = external_api_call().await?;
}
```

**Anti-Pattern (gesamte Schleife):**
```rust
// FALSCH: Lock fuer gesamte Operation
let conn = db.lock().unwrap();
for item in items {
    conn.execute("UPDATE ...", params![item.id])?;
    external_api_call().await?; // Lock blockiert andere!
}
```

### Transaction-Regeln

**WICHTIG:** Zusammengehoerige DB-Operationen MUESSEN in einer Transaction erfolgen.

| Regel | Beschreibung |
|-------|--------------|
| **Atomare Operationen** | INSERT/UPDATE/DELETE Gruppen in Transaction wrappen |
| **ROLLBACK bei Fehler** | Bei Fehlern explizit ROLLBACK ausfuehren |
| **Pro-Item Transactions** | In Batch-Operationen: Transaction pro Item, nicht global |

**Korrektes Transaction-Pattern:**
```rust
{
    let conn = db.lock().unwrap();
    conn.execute("BEGIN", [])?;

    match do_operations(&conn) {
        Ok(_) => conn.execute("COMMIT", [])?,
        Err(e) => {
            conn.execute("ROLLBACK", [])?;
            return Err(e);
        }
    };
}
```

**Pattern fuer Batch-Operationen:**
```rust
for item in items {
    {
        let conn = db.lock().unwrap();
        conn.execute("BEGIN", [])?;

        // Alle zusammengehoerigen Operationen fuer dieses Item
        conn.execute("UPDATE fnords SET ...", params![item.id])?;
        conn.execute("INSERT INTO fnord_immanentize ...", params![...])?;
        conn.execute("UPDATE processed_at ...", params![item.id])?;

        conn.execute("COMMIT", [])?;
    } // Lock released

    tokio::task::yield_now().await;
}
```

### Kritische Bug-Fixes (Januar 2025)

Folgende kritische Bugs wurden in den Datenbank-Operationen behoben:

#### 1. immanentize.rs - perform_merge()
**Problem:** Falsche Spaltennamen `keyword_id`/`neighbor_id` fuer `immanentize_neighbors` Tabelle.
**Fix:** Korrekte Spaltennamen `immanentize_id_a`/`immanentize_id_b` verwendet.

#### 2. immanentize.rs - merge_synonym_keywords()
**Problem:** Lock wurde waehrend gesamter Merge-Operation gehalten.
**Fix:** Refactored zu:
- Keywords mit kurzem Lock laden, dann Lock releasen
- Merge-Kandidaten ausserhalb des Locks identifizieren
- Lock erneut akquirieren, alle Merges in einer Transaction

#### 3. immanentize.rs - cleanup_garbage_keywords()
**Problem:** Mehrere DELETE-Operationen ohne Transaction.
**Fix:** Transaction-Wrapper (BEGIN/COMMIT) um alle Delete-Operationen.

#### 4. immanentize.rs - auto_prune_low_quality()
**Problem:** Delete-Operationen ohne Transaction-Safety.
**Fix:** Transaction-Wrapper um Delete-Operationen hinzugefuegt.

#### 5. batch_processor.rs - transfer_keywords_to_cluster_members()
**Problem:** Lock fuer gesamte Batch-Operation gehalten.
**Fix:**
- Lock-Akquirierung in die Loop verschoben (pro Artikel)
- Transaction-Wrapper pro Artikel
- Lock wird nach jedem Artikel released

#### 6. sync/mod.rs - store_feed()
**Problem:** Mehrere INSERT/UPDATE ohne Transaction, kein ROLLBACK bei Fehlern.
**Fix:**
- Gesamte Funktion in Transaction gewrappt (BEGIN/COMMIT)
- ROLLBACK bei Fehler-Pfaden
- Aufgeteilt in `store_feed()` (Transaction-Wrapper) und `store_feed_inner()` (Implementierung)

#### 7. article_analysis.rs - process_statistical_batch()
**Problem:** Lock waehrend gesamter Batch-Verarbeitung gehalten.
**Fix:**
- Lock zwischen Artikeln released
- `processed_at` Update nach jedem Artikel
- Transaction pro Artikel
- `tokio::task::yield_now().await` fuer Concurrency

### Relevante Module fuer DB-Operationen

| Modul | Funktion | Kritische Patterns |
|-------|----------|-------------------|
| `src-tauri/src/db/mod.rs` | Database Struct | Lock-Verwaltung |
| `src-tauri/src/immanentize.rs` | Keyword-Operationen | Merge, Cleanup, Prune |
| `src-tauri/src/sync/mod.rs` | Feed-Sync | store_feed Transaction |
| `src-tauri/src/commands/batch_processor.rs` | Batch-Verarbeitung | Per-Item Locks |
| `src-tauri/src/commands/article_analysis.rs` | Artikel-Analyse | Statistical Batch |

## Frontend Event-System (Daten-Refresh)

Komponenten die Backend-Daten anzeigen muessen auf CustomEvents lauschen, um nach Aenderungen aktualisiert zu werden.

### CustomEvents

| Event | Quelle | Wann | Listener |
|-------|--------|------|----------|
| `batch-complete` | `state.svelte.ts` | Nach Batch-Processing Abschluss | KeywordNetwork (via networkStore), FnordView, KeywordTable, CompoundKeywordManager, ArticleView |
| `keywords-changed` | `state.svelte.ts`, `networkStore` | Nach Keyword-Mutationen (create, merge, rename, delete, batch) | KeywordNetwork (via networkStore), FnordView, KeywordTable, CompoundKeywordManager |

### Pattern fuer neue Komponenten

Jede Komponente die Keyword-/Artikel-Daten anzeigt MUSS:
1. In `onMount`: `window.addEventListener('batch-complete', refreshHandler)` registrieren
2. In `onDestroy`: `window.removeEventListener('batch-complete', refreshHandler)` aufrufen
3. Bei Keyword-Daten zusaetzlich auf `keywords-changed` lauschen

Siehe [docs/guides/QUALITY_CHECKLIST.md](docs/guides/QUALITY_CHECKLIST.md) fuer die vollstaendige Checkliste.

### networkStore (Immanentize Network)

Der `networkStore` (`src/lib/stores/network.svelte.ts`) verwaltet den gesamten State des Immanentize Networks:
- Keywords, Trending, Stats, Graph-Daten
- Event-Listener Management via `setupEventListeners()` / `teardownEventListeners()`
- `refreshAll()` fuer vollstaendigen Daten-Refresh
- Navigation-Support (selectKeyword bei navigate-to-network)

**WICHTIG:** KeywordNetwork.svelte nutzt den networkStore - KEIN lokaler State fuer Keyword-Daten!

## AI Processing Pipeline

Siehe [docs/architecture/AI_PROCESSING_PIPELINE.md](docs/architecture/AI_PROCESSING_PIPELINE.md) fuer die vollstaendige Pipeline-Dokumentation.

**Kurzuebersicht:**
1. **Hagbard's Retrieval** - Volltext abrufen (automatisch nach Sync)
2. **Discordian Analysis** - Zusammenfassung, Kategorien, Keywords via ministral
3. **Article Embedding** - 1024-dim Embedding fuer Aehnlichkeitssuche
4. **Greyface Alert** - Bias-Erkennung (political_bias: -2 bis +2)
5. **Immanentize Network** - Schlagwort-Verarbeitung und Synonym-Erkennung

**Content-Felder:**
| Feld | Zweck | Quelle |
|------|-------|--------|
| `content_raw` | RSS-Feed Inhalt (Auszug) | Sync |
| `content_full` | Volltext der Webseite | Hagbard's Retrieval |

**Wichtig:** Alle KI-Analysen verwenden ausschliesslich `content_full`. Artikel ohne Volltext werden nicht analysiert.

## Ollama Setup

Siehe [README.md](README.md#ollama-setup) fuer die vollstaendige Ollama-Dokumentation.

**Quick Setup:**
```bash
ollama pull ministral-3:latest
ollama pull snowflake-arctic-embed2:latest
```

**Hinweis:** Bei Modellwechsel muessen alle Keywords neu eingebettet werden (Settings -> Wartung -> Embeddings generieren).

**Alternative Text-Generation:** Anstelle von Ollama kann fuer Textgenerierung (Zusammenfassungen, Kategorisierung, Bias-Erkennung) auch eine OpenAI-kompatible API verwendet werden. Die Konfiguration erfolgt in Settings -> KI-Provider (API-URL, API-Key, Modellname). Ollama bleibt weiterhin erforderlich fuer Embeddings (snowflake-arctic-embed2).

**OpenAI Default-Modell:** `gpt-5-nano` ($0.05/$0.40 pro 1M Tokens). Sehr schnell, kosteneffizient, "great for summarization and classification tasks". Konstante: `ai_provider::DEFAULT_OPENAI_MODEL`. Alternative: `gpt-5-mini` ($0.25/$2.00) fuer hoehere Qualitaet bei Bias-Erkennung.

**Cost-Tracking:** Nach jedem OpenAI-API-Call werden Token-Counts und Kosten in `ai_cost_log` geloggt. Pricing wird anhand des Modellnamens bestimmt (`helpers::get_model_pricing()`). Ollama-Calls werden nicht geloggt (keine Token-Counts verfuegbar).

## Data Paths

Datenbank wird im src-tauri Ordner gespeichert:
- **Relativer Pfad:** `src-tauri/data/fuckup.db`
- **Absoluter Pfad:** `/Users/hnsstrk/Repositories/fuckupRSS/src-tauri/data/fuckup.db`
- **Format:** SQLite mit WAL-Modus
- **Hinweis:** `data/` ist in `.gitignore` eingetragen

**Technischer Hintergrund:** Die Datenbank wird relativ zum Arbeitsverzeichnis erstellt (`./data/fuckup.db`). Da Tauri den Rust-Binary aus dem `src-tauri/` Verzeichnis startet, ist der effektive Pfad `src-tauri/data/fuckup.db`.

**Schneller DB-Zugriff (Claude Code):**
```bash
sqlite3 /Users/hnsstrk/Repositories/fuckupRSS/src-tauri/data/fuckup.db
```

## MCP-Server (Claude Code Integration)

Fuer die Entwicklung mit Claude Code sind folgende MCP-Server konfiguriert:

### Konfigurierte Server

| Server | Zweck | Tools |
|--------|-------|-------|
| **ollama** | Lokale KI-Interaktion | `ollama_chat`, `ollama_generate`, `ollama_embed`, `ollama_list`, `ollama_pull` |
| **fetch** | Web-Requests | `fetch` (ohne Einschraenkungen) |
| **memory** | Persistenter Kontext | `create_entities`, `search_nodes`, `read_graph` |

### Konfiguration

Die MCP-Server sind in `.mcp.json` im Projektverzeichnis konfiguriert:

```json
{
  "ollama": {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "ollama-mcp"]
  },
  "fetch": {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "mcp-fetch-server"]
  },
  "memory": {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-memory"]
  }
}
```

**Hinweis:** Die offiziellen MCP-Server von Anthropic sind Python-basiert (via `uvx`). Die npm-Pakete (`mcp-fetch-server`, `ollama-mcp`) sind Community-Implementierungen.

### Anwendungsfaelle

**ollama:**
- Direkt mit ministral-3:latest oder anderen Modellen chatten
- Embeddings generieren ohne Rust-Code
- Modelle herunterladen und verwalten

**fetch:**
- RSS-Feeds testen ohne App zu starten
- Webseiten-Struktur fuer Readability analysieren
- API-Endpoints testen

**memory:**
- Wichtige Projektinfos zwischen Sessions speichern
- Kontext ueber laengere Entwicklungszyklen behalten
