# AGENTS.md

AI Agent Guidelines for fuckupRSS Development

---

## Project Overview

**fuckupRSS** is a local-first RSS aggregator with AI-powered analysis, named after F.U.C.K.U.P. from the Illuminatus! trilogy. It uses Ollama for local AI processing with zero cloud dependencies.

### Tech Stack

| Layer | Technology | Notes |
|-------|------------|-------|
| Framework | Tauri 2.x | Rust backend + WebView frontend |
| Backend | Rust | SQLite, async with tokio |
| Frontend | Svelte 5 | Runes-based reactivity |
| Database | SQLite + sqlite-vec | WAL mode, vector search |
| AI | Ollama | Local models: ministral-3:latest, snowflake-arctic-embed2 |
| Styling | TailwindCSS | Catppuccin color scheme |
| i18n | svelte-i18n | DE/EN support |

---

## Code Style Guidelines

### Rust Backend

```rust
// Use thiserror for error types
#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

// Tauri commands use Result<T, String>
#[tauri::command]
pub fn get_fnords(state: State<AppState>) -> Result<Vec<Fnord>, String> {
    // Lock db mutex, map errors to String
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // ...
}

// Tests go in mod tests {} blocks or separate test files
#[cfg(test)]
mod tests {
    use super::*;
    // ...
}
```

### Svelte 5 Frontend

```typescript
// Use Svelte 5 runes for state ($state, $derived, $effect)
let count = $state(0);
let doubled = $derived(count * 2);

// Class-based stores with $state for complex state
class AppStore {
  items = $state<Item[]>([]);
  loading = $state(false);
  
  async load(): Promise<void> {
    this.loading = true;
    this.items = await invoke<Item[]>("get_items");
    this.loading = false;
  }
}

// Export singleton instances
export const appStore = new AppStore();
```

### File Organization

```
src/
├── lib/
│   ├── components/     # Svelte components (PascalCase.svelte)
│   ├── stores/         # State management (*.svelte.ts)
│   ├── types.ts        # TypeScript interfaces
│   ├── i18n/           # Translations
│   └── utils/          # Helper functions
├── App.svelte          # Main app component
└── main.ts             # Entry point

src-tauri/
├── src/
│   ├── commands/       # Tauri IPC commands (mod.rs + feature files)
│   ├── db/             # Database layer (mod.rs, schema.rs)
│   ├── ollama/         # AI integration
│   ├── sync/           # Feed synchronization
│   ├── retrieval/      # Full-text fetching
│   ├── lib.rs          # Tauri setup + AppState
│   └── main.rs         # Entry point
└── Cargo.toml
```

---

## Illuminatus! Terminology

The codebase uses terms from the Illuminatus! trilogy:

| Term | Meaning | Code Usage |
|------|---------|------------|
| Fnord | Article (especially changed ones) | `fnords` table, Fnord struct |
| Concealed | Unread article | `status = 'concealed'` |
| Illuminated | Read article | `status = 'illuminated'` |
| Golden Apple | Favorited article | `status = 'golden_apple'` |
| Pentacle | Feed source | `pentacles` table |
| Sephiroth | Category | `sephiroth` table (13 fixed) |
| Immanentize | Keyword/tag | `immanentize` table |
| Greyface Alert | Bias warning | `political_bias`, `sachlichkeit` |
| Discordian Analysis | AI summary | `summary` field |
| Hagbard's Retrieval | Full-text fetch | `content_full` field |

---

## Database Schema (Key Tables)

### Core Tables

- `pentacles` - Feed sources (URL, title, sync settings)
- `fnords` - Articles (content, status, bias scores)
- `fnord_revisions` - Article version history
- `sephiroth` - 13 fixed categories
- `immanentize` - Keywords/tags
- `settings` - Key-value store for app settings

### Immanentize Network

- `immanentize_neighbors` - Keyword cooccurrence network
- `immanentize_sephiroth` - Keyword ↔ Category associations
- `immanentize_daily` - Daily keyword counts for trends
- `fnord_immanentize` - Article ↔ Keyword mapping
- `fnord_sephiroth` - Article ↔ Category mapping
- `dismissed_synonyms` - Dismissed synonym pair suggestions

---

## Tauri Commands

Commands are organized by domain in `src-tauri/src/commands/`:

| Module | Commands |
|--------|----------|
| `pentacles.rs` | `get_pentacles`, `add_pentacle`, `delete_pentacle` |
| `fnords.rs` | `get_fnords`, `get_fnord`, `update_fnord_status`, `get_changed_fnords`, etc. |
| `sync.rs` | `sync_all_feeds`, `sync_feed` |
| `retrieval.rs` | `fetch_full_content`, `fetch_truncated_articles` |
| `ollama.rs` | `check_ollama`, `process_batch`, `process_article_discordian`, etc. |
| `categories.rs` | `get_all_categories`, `get_article_categories`, etc. |
| `tags.rs` | `get_all_tags`, `get_article_tags`, etc. |
| `immanentize.rs` | `get_keywords`, `get_keyword_neighbors`, `get_network_graph`, `calculate_keyword_quality_scores`, `generate_keyword_embeddings`, `find_synonym_candidates`, `merge_keyword_pair`, `dismiss_synonym_pair`, etc. |
| `settings.rs` | `get_settings`, `set_setting`, `get_setting` |

---

## Testing Requirements

**All code changes MUST include tests.**

```bash
# Run all tests
npm run test              # Frontend (Vitest)
cargo test --manifest-path src-tauri/Cargo.toml  # Backend

# E2E tests
npm run test:e2e          # Playwright
```

Test locations:
- Rust: `src-tauri/src/*/tests.rs` or inline `#[cfg(test)] mod tests`
- Frontend: `src/lib/__tests__/`
- E2E: `e2e/`

---

## Build Commands

```bash
npm install               # Install dependencies
npm run tauri dev         # Development mode
npm run tauri build       # Production build
npm run dev               # Frontend only (without Tauri)
```

---

## AI Processing Pipeline

1. **Feed Sync** - Fetch RSS/Atom feeds
2. **Hagbard's Retrieval** - Fetch full-text for truncated feeds
3. **Discordian Analysis** - AI analysis via Ollama:
   - Summary (2-3 sentences)
   - Categories (Sephiroth)
   - Keywords (Immanentize)
   - Bias scores (political_bias, sachlichkeit)
   - Article type (news, analysis, opinion, satire, ad)
4. **Network Update** - Update keyword cooccurrence network

---

## Important Patterns

### Database Access

```rust
// Always lock the mutex, map errors to String
let db = state.db.lock().map_err(|e| e.to_string())?;

// Use parameterized queries
db.conn().query_row(
    "SELECT * FROM fnords WHERE id = ?1",
    [id],
    |row| Ok(Fnord { ... })
)?;
```

### Event Emission (Rust → Frontend)

```rust
// Emit progress events for long-running operations
window.emit("batch-progress", BatchProgress { ... })?;
```

### Event Listening (Frontend)

```typescript
import { listen } from "@tauri-apps/api/event";

const unlisten = await listen<BatchProgress>("batch-progress", (event) => {
  appState.updateBatchProgress(event.payload);
});

onDestroy(() => unlisten());
```

---

## Common Pitfalls

1. **Don't suppress type errors** - No `as any`, `@ts-ignore`, `@ts-expect-error`
2. **Don't commit without tests** - All features need test coverage
3. **Lock ordering** - Be careful with mutex locks in Rust to avoid deadlocks
4. **Locale handling** - Use `$_('key')` for all user-visible strings
5. **Error handling** - Always show user-friendly errors via toasts

---

## Documentation Files

| File | Purpose | Update When |
|------|---------|-------------|
| `README.md` | Public project description | New features, installation changes |
| `CLAUDE.md` | Developer context for Claude Code | Build changes, patterns, structure |
| `AGENTS.md` | AI agent guidelines (this file) | Architecture changes, conventions |
| `fuckupRSS-Anforderungen.md` | **Master-Dokument:** Technische Spezifikation, Planung, Roadmap | Architecture decisions, schema changes, Phasen-Updates |

**Planung:** Alle Phasen und Tasks sind zentral in `fuckupRSS-Anforderungen.md` Kapitel 20 dokumentiert.

---

## Quick Reference

### Add a new Tauri command:

1. Create function in `src-tauri/src/commands/<module>.rs`
2. Add `#[tauri::command]` attribute
3. Register in `src-tauri/src/lib.rs` invoke_handler
4. Add TypeScript types in `src/lib/types.ts`
5. Call via `invoke<ReturnType>("command_name", { params })`

### Add a new component:

1. Create `src/lib/components/ComponentName.svelte`
2. Use Svelte 5 runes for state
3. Use `$_('key')` for i18n strings
4. Import in parent component

### Add a database migration:

1. Add `ALTER TABLE` in `src-tauri/src/db/schema.rs` → `run_migrations()`
2. Check if column exists before altering
3. Update any affected structs and queries
