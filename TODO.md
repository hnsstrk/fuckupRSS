# TODO

> Letzte umfassende Code-Review: 2026-02-24 (Ralph Loop — 3 Iterationen mit je 2-3 Agenten)
> Gesamt-Qualitaet: **8.5/10** — Production-ready, wenige verbleibende Issues

---

## Erledigte Items (durch Agent-Teams am 2026-02-24 umgesetzt)

> Folgende Items aus der ersten Review wurden implementiert und gemerged:

- [x] HTML Sanitization fuer Feed-Inhalte (`ammonia` Crate in sync/mod.rs, retrieval/mod.rs)
- [x] ESLint `svelte/no-at-html-tags` auf `error` gesetzt
- [x] N+1 Query Fix: Batch-INSERTs in batch_processor.rs
- [x] SQL LIMIT als Prepared Statement statt `format!()`
- [x] `to_cmd_err!` Macro fuer Error-Conversions (~23 Stellen)
- [x] Feature-Flag `clustering` statt `#[allow(dead_code)]`
- [x] `.expect()` durch proper Error Handling ersetzt (kritische Pfade)
- [x] MindfuckView aufgeteilt (2356→300 Zeilen, 3 Sub-Komponenten)
- [x] SettingsMaintenance aufgeteilt (1936→800 Zeilen, 2 Sub-Komponenten)
- [x] FnordView aufgeteilt (1889→230 Zeilen, 4 Sub-Komponenten)
- [x] ArticleView aufgeteilt (1884→780 Zeilen, 3 Sub-Komponenten)
- [x] SettingsOllama aufgeteilt (1864→680 Zeilen, 2 Sub-Komponenten)
- [x] ActionButton.svelte Komponente (einheitliches Button-Styling)
- [x] formatError() Helper in 11 Komponenten
- [x] ARIA Labels auf Icon-Only Buttons (6 Komponenten)
- [x] Inline Styles durch CSS-Klassen ersetzt (7 Komponenten)
- [x] Component-Tests: Sidebar (55), ArticleView (61), ArticleList (29)
- [x] Release-Checkliste (docs/guides/RELEASE_CHECKLIST.md)

---

## Offene Tasks

> Alle offenen Tasks wurden am 2026-02-25 nach **Taskwarrior** migriert (Projekt: `fuckupRSS`).
> Abfrage: `task project:fuckupRSS list`

---

## Positiv-Befunde (Staerken)

> Diese Bereiche sind vorbildlich und sollten beibehalten werden:

- **Dokumentation**: CLAUDE.md, docs/ und README.md exzellent konsistent mit Code (9/10)
- **Trait-basierte AI-Provider Abstraktion**: Sauberes Design fuer Ollama + OpenAI
- **Database-Patterns**: Lock-Halte-Regeln und Transaction-Safety gruendlich dokumentiert und umgesetzt
- **Type-Safety**: Frontend 95% Type-Coverage, 712 Zeilen Definitionen, 50+ Interfaces
- **Svelte 5 Runes**: Korrekt und modern eingesetzt ($state, $derived, $effect)
- **i18n**: 100% vollstaendig, 0 fehlende Keys zwischen de.json und en.json
- **Event-Listener Lifecycle**: Alle Komponenten registrieren/deregistrieren Events korrekt via onMount/onDestroy
- **Security-Scanning**: Semgrep, npm audit, cargo audit, SBOMs umfassend
- **Code-Konsistenz**: Erzwungen durch Husky Hooks (ESLint, Prettier, rustfmt, Clippy)
- **Frontend/Backend Konsistenz**: Alle 98 invoke()-Calls haben korrespondierendes Backend-Command
- **Tauri 2 Parameter-Konvertierung**: camelCase↔snake_case funktioniert korrekt ueberall
- **0 TODO/FIXME/HACK Kommentare** im Quellcode
