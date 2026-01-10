# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Wichtige Projektdokumentation

**WICHTIG:** Bei jedem größeren Arbeitsschritt müssen folgende Dokumente geprüft, hinterfragt und ggf. aktualisiert werden:

| Dokument | Zweck | Prüfen bei |
|----------|-------|------------|
| `README.md` | Öffentliche Projektbeschreibung, Features, Installation | Neue Features, API-Änderungen, Installationsänderungen |
| `fuckupRSS-Anforderungen.md` | Technische Spezifikation, Architektur, Entscheidungen | Architekturänderungen, neue Komponenten, Abweichungen vom Plan |
| `CLAUDE.md` | Entwickler-Kontext für Claude Code | Build-Änderungen, neue Patterns, Strukturänderungen |
| `TODO.md` | Zentrale Aufgabenliste, Entscheidungen | Neue Tasks, abgeschlossene Tasks, technische Entscheidungen |

### Dokumentations-Workflow

1. **Vor Implementierung:** Anforderungsdokument lesen und verstehen
2. **Während Implementierung:** Bei Abweichungen vom Plan dokumentieren warum
3. **Nach Implementierung:** README.md und CLAUDE.md aktualisieren
4. **Bei Commits:** Prüfen ob Dokumentation angepasst werden muss

## Project Overview

fuckupRSS is an RSS aggregator/reader with local AI integration, named after F.U.C.K.U.P. from the Illuminatus! trilogy. It uses Ollama for local AI processing with no cloud dependencies.

**Status:** Phase 2 abgeschlossen, Phase 3 in Entwicklung

**Planung:** Alle Phasen und Tasks sind in [`fuckupRSS-Anforderungen.md`](fuckupRSS-Anforderungen.md#20-nächste-schritte) dokumentiert.

## Technology Stack

- **Framework:** Tauri 2.x (Rust backend + Svelte 5 frontend)
- **Database:** SQLite + sqlite-vec (bundled vector search extension, O(log n) KNN)
- **AI Backend:** Ollama (local) with native JSON mode for stability
- **Models:** ministral-3:latest (text) and snowflake-arctic-embed2 (embeddings)
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

## Testing (PFLICHT)

**WICHTIG:** Alle neuen Features und Bugfixes MÜSSEN mit Tests abgedeckt werden. Code ohne Tests wird nicht akzeptiert.

### Test-Befehle

```bash
# Alle Tests ausführen
npm run test           # Frontend (Vitest)
npm run test:e2e       # E2E Tests (Playwright)
cargo test --manifest-path src-tauri/Cargo.toml  # Backend (Rust)

# Tests im Watch-Modus
npm run test:watch

# Test-Coverage
npm run test:coverage
cargo tarpaulin --manifest-path src-tauri/Cargo.toml
```

### Test-Übersicht

| Bereich | Anzahl Tests | Tool |
|---------|-------------|------|
| Rust Backend | 160 Tests | `cargo test` |
| Frontend (Vitest) | 89 Tests | `npm run test` |
| E2E (Playwright) | 11 Tests | `npm run test:e2e` |
| **Gesamt** | **260 Tests** | |

### Test-Struktur

```
fuckupRSS/
├── src/
│   └── lib/
│       └── __tests__/           # Frontend Unit Tests (Vitest)
│           ├── setup.ts         # Test-Setup mit Mocks
│           ├── stores/          # Store Tests
│           │   ├── state.test.ts      # State Management Tests (18 Tests)
│           │   ├── network.test.ts    # Immanentize Network Tests (31 Tests)
│           │   └── navigation.test.ts # Navigation Events Tests (21 Tests)
│           └── components/      # Component Tests
│               └── Toast.test.ts      # Toast Component Tests (19 Tests)
├── e2e/                         # E2E Tests (Playwright)
│   ├── fixtures.ts              # Tauri API Mocks
│   └── app.spec.ts              # App-Tests
├── src-tauri/
│   └── src/
│       ├── db/
│       │   └── tests.rs         # DB Unit Tests (14 Tests)
│       ├── sync/
│       │   └── tests.rs         # Sync Unit Tests (14 Tests)
│       ├── retrieval/
│       │   └── tests.rs         # Retrieval Unit Tests (22 Tests)
│       ├── ollama/
│       │   └── tests.rs         # Ollama Unit Tests (33 Tests)
│       └── commands/
│           ├── tests.rs         # Batch-Analyse Unit Tests (31 Tests)
│           └── batch_integration_tests.rs  # DB-Integration (9 Tests)
```

### Test-Anforderungen

| Bereich | Anforderung | Tool |
|---------|-------------|------|
| Rust Backend | Unit Tests für alle Module | `cargo test` |
| Tauri Commands | Integration Tests | `cargo test` |
| Svelte Stores | Unit Tests für State-Logik | Vitest |
| Svelte Components | Component Tests | Vitest + Testing Library |
| User Flows | E2E Tests | Playwright |

### Test-Patterns

**Rust Unit Test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = ...;

        // Act
        let result = function_name(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

**Frontend Component Test (Vitest):**
```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Component from './Component.svelte';

describe('Component', () => {
  it('renders correctly', () => {
    render(Component, { props: { ... } });
    expect(screen.getByText('...')).toBeInTheDocument();
  });
});
```

**E2E Test (Playwright):**
```typescript
import { test, expect } from '@playwright/test';

test('user can add a feed', async ({ page }) => {
  await page.goto('/');
  await page.fill('[data-testid="feed-url"]', 'https://example.com/feed.xml');
  await page.click('[data-testid="add-feed"]');
  await expect(page.locator('.feed-item')).toBeVisible();
});
```

### Wann Tests schreiben?

- **VOR dem Implementieren:** TDD bevorzugt
- **WÄHREND der Implementierung:** Bei komplexer Logik
- **NACH der Implementierung:** Mindestens für alle public APIs
- **Bei Bugfixes:** Erst Test schreiben der Bug reproduziert, dann fixen

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

### Commit-Frequenz (Claude Code Richtlinie)

**WICHTIG:** Claude Code muss regelmäßig committen, um Arbeit nicht zu verlieren und den Fortschritt nachvollziehbar zu machen.

| Situation | Commit-Regel |
|-----------|--------------|
| Feature abgeschlossen | **Sofort committen** |
| 2-3 zusammenhängende Änderungen | **Committen** (nicht sammeln) |
| Bugfix erledigt | **Sofort committen** |
| Refactoring-Schritt fertig | **Committen** |
| Vor Themenwechsel | **Committen** (aktuelles Thema abschließen) |
| Nach 15-20 Minuten Arbeit | **Prüfen** ob Commit sinnvoll |
| Benutzer fragt nach anderem Thema | **Erst committen**, dann Thema wechseln |

**Faustregel:** Lieber zu viele kleine Commits als zu wenige große.

**Anti-Pattern vermeiden:**
- ❌ Mehrere unabhängige Features in einem Commit
- ❌ Stundenlang arbeiten ohne Commit
- ❌ "Ich committe später" - NEIN, jetzt committen!
- ❌ Auf Benutzer-Erinnerung warten

**Selbst-Check nach jeder Aufgabe:**
```
✓ Kompiliert der Code? → git add && git commit
✓ Feature/Fix fertig? → git add && git commit
✓ Wechsle ich das Thema? → git add && git commit
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
├── CLAUDE.md                     # Diese Datei
└── TODO.md                       # Zentrale Aufgabenliste
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
| Tauri Framework | `tauri` | ✅ |
| SQLite | `rusqlite` | ✅ |
| Serialization | `serde`, `serde_json` | ✅ |
| DateTime | `chrono` | ✅ |
| Error Handling | `thiserror` | ✅ |
| RSS/Atom Parsing | `feed-rs` | ✅ |
| HTTP Client | `reqwest` | ✅ |
| Readability | `readability` | ✅ |
| Ollama API | `ollama-rs` | ✅ |
| Vector Search | `sqlite-vec` | ✅ (bundled, O(log n) KNN) |
| OPML Parsing | `opml` | ⏳ Phase 4 |

## Illuminatus! Terminology

The codebase uses terms from the Illuminatus! trilogy:

| Code Term | Meaning | DB Table/Field |
|-----------|---------|----------------|
| Fnord | Changed article (with revisions) | `fnords.has_changes = TRUE` |
| Concealed | Unread article | `fnords.status = 'concealed'` |
| Illuminated | Read article | `fnords.status = 'illuminated'` |
| Golden Apple | Favorited article | `fnords.status = 'golden_apple'` |
| Pentacle | Feed source | `pentacles` |
| Sephiroth | Category (13 fixed) | `sephiroth` |
| Immanentize | Keyword/tag | `immanentize` |
| Immanentize Network | Semantic keyword graph | `immanentize_*` tables |
| Greyface Alert | Bias warning | `fnords.political_bias`, `sachlichkeit` |
| Discordian Analysis | AI summary | `fnords.summary` |
| Operation Mindfuck | User interests | `operation_mindfuck` |
| Hagbard's Retrieval | Full-text fetching | `fnords.content_full` |

## Database Schema Key Tables

### Core Tables
- `pentacles` - Feed sources (URL, title, sync settings)
- `fnords` - Articles (content, status, bias scores)
- `fnord_revisions` - Article version history

### Kategorien (13 fest definiert)
- `sephiroth` - Categories: Technik, Politik, Wirtschaft, Wissenschaft, Kultur, Sport, Gesellschaft, Umwelt, Sicherheit, Gesundheit, Verteidigung, Energie, Recht
- `fnord_sephiroth` - Article ↔ Category mapping (source: 'ai'|'manual', assigned_at)

### Immanentize Network (Schlagwort-Wissensnetz)
- `immanentize` - Keywords mit embedding BLOB, quality_score, canonical_id
- `immanentize_sephiroth` - Schlagwort ↔ Kategorie Assoziation
- `immanentize_neighbors` - Kookkurrenz-Netzwerk (cooccurrence + embedding_similarity)
- `immanentize_clusters` - Themen-Cluster
- `immanentize_daily` - Tägliche Keyword-Zählungen für Trends
- `fnord_immanentize` - Article ↔ Tag mapping
- `dismissed_synonyms` - Ignorierte Synonym-Vorschläge

### Embeddings & Vector Search
- Keywords: Embeddings als BLOB in `immanentize.embedding` (1024-dim, snowflake-arctic-embed2)
- Vector Index: `vec_immanentize` (sqlite-vec virtual table, cosine distance, O(log n) KNN)
- Artikel: ⏳ Phase 3 - `fnords.embedding` geplant

Schema-Definition: `src-tauri/src/db/schema.rs`
Dokumentation: `fuckupRSS-Anforderungen.md` Kapitel 6b + 10

## Tauri Commands (Frontend → Backend)

### Pentacles (Feeds)
| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `get_pentacles` | - | `Vec<Pentacle>` | Alle Feeds mit Counts |
| `add_pentacle` | `url`, `title?` | `Pentacle` | Feed hinzufügen |
| `delete_pentacle` | `id` | - | Feed löschen |

### Fnords (Artikel)
| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `get_fnords` | `filter?` | `Vec<Fnord>` | Artikel mit Filter |
| `get_fnord` | `id` | `Fnord` | Einzelner Artikel |
| `update_fnord_status` | `id`, `status` | - | Status ändern |
| `get_changed_fnords` | - | `Vec<Fnord>` | Geänderte Artikel |
| `acknowledge_changes` | `id` | - | Änderung bestätigen |
| `get_fnord_revisions` | `fnord_id` | `Vec<FnordRevision>` | Revisionshistorie |

### Sync
| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `sync_all_feeds` | - | `SyncResponse` | Alle Feeds aktualisieren |
| `sync_feed` | `pentacle_id` | `SyncResultResponse` | Einzelnen Feed aktualisieren |

### Retrieval (Volltext)
| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `fetch_full_content` | `fnord_id` | `RetrievalResponse` | Volltext abrufen |
| `fetch_truncated_articles` | `pentacle_id?`, `limit?` | `Vec<RetrievalResponse>` | Gekürzte Artikel abrufen |

### Ollama (KI)
| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `check_ollama` | - | `OllamaStatus` | Ollama-Verfügbarkeit prüfen |
| `generate_summary` | `fnord_id`, `model` | `SummaryResponse` | Zusammenfassung generieren |
| `analyze_article` | `fnord_id`, `model` | `AnalysisResponse` | Bias-Analyse durchführen |
| `process_article` | `fnord_id`, `model` | `(Summary, Analysis)` | Beides kombiniert |
| `get_unprocessed_count` | - | `UnprocessedCount` | Unverarbeitete Artikel zählen |
| `process_batch` | `model`, `limit?` | `BatchResult` | Batch-Verarbeitung |
| `pull_model` | `model` | `ModelPullResult` | Modell herunterladen |
| `get_prompts` | - | `PromptTemplates` | Aktuelle Prompts laden |
| `set_prompts` | `summary_prompt`, `analysis_prompt` | - | Prompts speichern |
| `reset_prompts` | - | `PromptTemplates` | Prompts zurücksetzen |
| `get_default_prompts` | - | `DefaultPrompts` | Standard-Prompts |

### Settings
| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `get_settings` | - | `Settings` | Alle Einstellungen |
| `set_setting` | `key`, `value` | - | Einstellung speichern |
| `get_setting` | `key` | `Option<String>` | Einstellung laden |

### Immanentize (Keyword Quality & Synonyms)
| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `calculate_keyword_quality_scores` | `limit?` | `QualityScoreResult` | Quality-Scores berechnen |
| `get_low_quality_keywords` | `threshold`, `limit` | `Vec<LowQualityKeyword>` | Low-Quality Keywords |
| `auto_prune_low_quality` | `quality_threshold`, `min_age_days`, `dry_run` | `PruneResult` | Low-Quality bereinigen |
| `queue_missing_embeddings` | - | `i64` | Fehlende Embeddings in Queue einreihen |
| `find_synonym_candidates` | `threshold?`, `limit?` | `Vec<SynonymCandidate>` | Synonym-Kandidaten finden |
| `merge_keyword_pair` | `keep_id`, `remove_id` | `MergeSynonymsResult` | Keywords zusammenführen |
| `dismiss_synonym_pair` | `keyword_a_id`, `keyword_b_id` | - | Synonym-Vorschlag ignorieren |

### Hardware Profiles
| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `get_hardware_profiles` | - | `Vec<HardwareProfile>` | Verfügbare Profile laden |
| `save_hardware_profile` | `profile` | - | Profil erstellen/aktualisieren |
| `delete_hardware_profile` | `id` | - | Profil löschen |
| `apply_hardware_profile` | `profile_id` | - | Profil aktivieren |

**HardwareProfile Struktur:**
```rust
struct HardwareProfile {
    id: String,
    name: String,
    description: String,
    ai_parallelism: usize,  // Parallele Artikel bei Batch-Verarbeitung
}
```

**Vordefinierte Profile:**
| Profil | ai_parallelism | Beschreibung |
|--------|----------------|--------------|
| Standard | 1 | Sicher für alle Systeme |
| Moderat | 4 | Guter Kompromiss |
| Hohe Leistung | 8 | Für High-End Hardware |

## AI Processing Pipeline

1. **Hagbard's Retrieval** - Fetch full text for ALL new articles (automatic after sync)
   - Alle Artikel werden vollständig abgerufen, nicht nur truncated Feeds
   - Volltext wird in `content_full` gespeichert
   - `content_raw` bleibt für Änderungserkennung und Fallback-Anzeige
2. **Discordian Analysis** - Summarize, categorize, extract keywords via ministral
   - **Verwendet NUR `content_full`** - kein Fallback auf `content_raw`
   - Artikel ohne Volltext werden nicht zur Analyse vorgeschlagen
   - Einzelartikel können jederzeit neu analysiert werden (Button in ArticleView)
   - "Alle neu analysieren" (Settings → Wartung) mit Fortschrittsanzeige
3. **Greyface Alert** - Bias detection (political_bias: -2 to +2, sachlichkeit: 0-4)
4. **Immanentize Network** - Schlagwort-Verarbeitung:
   - Neue Schlagworte: Embedding via snowflake-arctic-embed2
   - Kategorie-Assoziation: immanentize_sephiroth aktualisieren
   - Nachbar-Update: Kookkurrenz + Embedding-Similarity
   - Synonym-Erkennung: Bei embedding_similarity > 0.92

### Content-Felder in fnords

| Feld | Zweck | Quelle |
|------|-------|--------|
| `content_raw` | RSS-Feed Inhalt (Auszug) | Sync |
| `content_full` | Volltext der Webseite | Hagbard's Retrieval |

**Wichtig:** Alle KI-Analysen verwenden ausschließlich `content_full`. Artikel ohne Volltext werden nicht analysiert.

## Ollama Setup

Ollama muss separat installiert und gestartet werden. fuckupRSS verbindet sich mit dem laufenden Ollama-Server.

```bash
# Modelle installieren (oder via Settings UI):
ollama pull ministral-3:latest
ollama pull snowflake-arctic-embed2
```

**Ollama-Konfiguration (optional für Performance):**

```bash
# Linux: systemd
sudo systemctl edit ollama.service
# [Service]
# Environment="OLLAMA_MAX_LOADED_MODELS=2"
# Environment="OLLAMA_FLASH_ATTENTION=1"
# Environment="OLLAMA_NUM_PARALLEL=4"

# macOS: Terminal (vor dem Start)
export OLLAMA_NUM_PARALLEL=4
export OLLAMA_FLASH_ATTENTION=1
ollama serve
```

**Hinweis:** Bei Modellwechsel müssen alle Keywords neu eingebettet werden (Settings → Wartung → Embeddings generieren).

## Data Paths

Datenbank wird im Projektordner gespeichert:
- **Pfad:** `./data/fuckup.db` (relativ zum Arbeitsverzeichnis)
- **Format:** SQLite mit WAL-Modus
- **Hinweis:** `data/` ist in `.gitignore` eingetragen

## MCP-Server (Claude Code Integration)

Für die Entwicklung mit Claude Code sind folgende MCP-Server konfiguriert:

### Konfigurierte Server

| Server | Zweck | Tools |
|--------|-------|-------|
| **ollama-mcp** | Lokale KI-Interaktion | `ollama_chat`, `ollama_generate`, `ollama_embed`, `ollama_list_models`, `ollama_pull` |
| **sqlite-mcp** | Direkte DB-Abfragen | `read_query`, `write_query`, `list_tables`, `describe_table` |
| **fetch-mcp** | Web-Requests | `fetch` (ohne Einschränkungen) |
| **memory-mcp** | Persistenter Kontext | `store`, `retrieve`, `search` |

### Konfiguration

Die MCP-Server sind in `~/.claude.json` unter `projects["/home/hnsstrk/Repositories/fuckupRSS"].mcpServers` konfiguriert:

```json
{
  "ollama-mcp": {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "ollama-mcp@latest"]
  },
  "sqlite-mcp": {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@anthropic/sqlite-mcp@latest", "/home/hnsstrk/Repositories/fuckupRSS/data/fuckup.db"]
  },
  "fetch-mcp": {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@anthropic/fetch-mcp@latest"]
  },
  "memory-mcp": {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@anthropic/memory-mcp@latest"]
  }
}
```

### Anwendungsfälle

**ollama-mcp:**
- Direkt mit ministral-3:latest oder anderen Modellen chatten
- Embeddings generieren ohne Rust-Code
- Modelle herunterladen und verwalten

**sqlite-mcp:**
- Datenbank-Debugging: `SELECT * FROM fnords WHERE summary IS NULL LIMIT 5`
- Schema-Analyse: `PRAGMA table_info(immanentize)`
- Datenintegrität prüfen: Orphaned Records, fehlende Embeddings

**fetch-mcp:**
- RSS-Feeds testen ohne App zu starten
- Webseiten-Struktur für Readability analysieren
- API-Endpoints testen

**memory-mcp:**
- Wichtige Projektinfos zwischen Sessions speichern
- Kontext über längere Entwicklungszyklen behalten
