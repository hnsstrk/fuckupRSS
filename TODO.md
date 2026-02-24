# TODO

> Letzte umfassende Code-Review: 2026-02-24 (Claude Code Agent Team)
> Gesamt-Qualitaet: **8/10** — Production-ready, gezielte Verbesserungen moeglich

---

## Hoch-Prioritaet (vor v1.0 Release)

### Security

- [ ] **HTML Sanitization fuer Feed-Inhalte** — Raw HTML aus RSS-Feeds wird ohne Sanitization akzeptiert (`sync/mod.rs:107-111`). XSS-Risiko bei kompromittierten Feeds. Fix: `ammonia` Crate einbinden und Feed-Content sanitizen.
- [ ] **ESLint-Regel `svelte/no-at-html-tags` von `warn` auf `error` setzen** — XSS-Risiko, DOMPurify ist vorhanden aber die Regel ist zu locker konfiguriert.

### Performance

- [ ] **N+1 Query Pattern im Batch-Processing** — Keyword-Transfer in `batch_processor.rs:265-310` fuehrt einzelne INSERTs pro Keyword/Artikel aus. Bei 1.000 Artikeln x 10 Keywords = 10.000 INSERT Statements. Fix: Multi-Row INSERT oder Batch-Collect mit Chunks.

### Testing

- [ ] **Component-Tests fuer Hauptkomponenten fehlen** — ArticleView, KeywordNetwork, Sidebar sind untested. Frontend-Test-Coverage bei 6/10.
- [ ] **E2E-Tests limitiert (9 skipped)** — Svelte-State-Updates funktionieren nicht mit gemockten APIs. Entscheidung treffen: Mock-System verbessern ODER als bekannte Limitation dokumentieren.
- [ ] **Integration Tests fuer Batch-Processing fehlen** — Unit Tests vorhanden, aber keine E2E-Tests fuer `batch_processor.rs`, `article_analysis.rs` und Clustering-Features.

---

## Mittel-Prioritaet (Phase 4 / Polish)

### Rust-Backend

- [ ] **SQL Dynamic LIMIT mit `format!()` statt Prepared Statement** — `batch_processor.rs:162-188` nutzt String-Formatting fuer LIMIT. Fix: `LIMIT ?` mit `-1` als "no limit" Fallback.
- [ ] **Repetitive Error String Conversions** — 248x `.map_err(|e| e.to_string())?` im Code. Fix: `to_cmd_err!()` Macro erstellen.
- [ ] **Dead Code mit `#[allow(dead_code)]`** — 22 Annotationen, hauptsaechlich Clustering-Features. Fix: `#[cfg(feature = "clustering")]` Feature Flags in Cargo.toml nutzen.
- [ ] **`.expect()` in Production-Code** — 415 Vorkommen, die meisten akzeptabel, aber kritische Pfade wie `lib.rs:352` (Tauri-Start), `retrieval/mod.rs:48` (HTTP-Client) und `openai_provider.rs:156` sollten proper Error Handling bekommen.

### Svelte-Frontend — Grosse Komponenten aufteilen

- [ ] **MindfuckView.svelte (2356 Zeilen)** — Aufteilen in: OverviewTab, BlindSpotsTab, CounterPerspectivesTab, TrendsTab
- [ ] **SettingsMaintenance.svelte (1936 Zeilen)** — Aufteilen in separate MaintenanceSection-Komponenten
- [ ] **FnordView.svelte (1889 Zeilen)** — Aufteilen in Sub-Views
- [ ] **ArticleView.svelte (1884 Zeilen)** — Aufteilen in ArticleHeader, ArticleContent, AISidebar
- [ ] **SettingsOllama.svelte (1864 Zeilen)** — Aufteilen in OllamaConfig, OllamaStatus, ModelManager
- [ ] **CompoundKeywordManager.svelte (1559 Zeilen)** — Aufteilen in CrudForms + Tabelle
- [ ] **KeywordNetworkSynonyms.svelte (1549 Zeilen)** — Aufteilen in SynonymTabs
- [ ] **KeywordNetworkDetail.svelte (1505 Zeilen)** — Aufteilen in DetailSections
- [ ] **ArticleKeywords.svelte (1398 Zeilen)** — Aufteilen in KeywordLists + Manager

### Svelte-Frontend — Code Quality

- [ ] **Redundante Button-Styling-Patterns** — 20+ verschiedene Button-CSS-Klassen (`action-btn danger`, `btn btn-default`, `btn-action btn-danger`). Fix: `ActionButton.svelte` Komponente mit `variant` Props erstellen.
- [ ] **Store Event-Listener Redundanz** — 7 Komponenten lauschen auf `batch-complete`, 6 auf `keywords-changed`. Fix: `useNetworkEvents()` Composable oder zentrales Event-Dispatching.
- [ ] **Fehlerbehandlung inkonsistent** — Gemischte Patterns (`String(e)` vs. `console.error`). Fix: `formatError()` Helper-Funktion.
- [ ] **Inline Styles (25 Vorkommen)** — Sollten durch CSS-Variablen oder Utility-Klassen ersetzt werden.

### Accessibility (a11y)

- [ ] **Keyboard-Navigation nicht ueberall konsistent** — MindfuckView und ErisianArchives haben keine Tab-Navigation. Fix: Standardisierte `Tabs.svelte` Komponente.
- [ ] **ARIA Labels auf Icon-Only Buttons** — Einige Icon-Buttons ohne `aria-label`. Alle Icon-Only Buttons brauchen `aria-label` + FontAwesome Icons brauchen `aria-hidden="true"`.

### CI/CD Verbesserungen

- [ ] **Rust-Toolchain cachen** — Aktuell wird bei jedem CI-Run `rustup.rs` frisch installiert. Tool-Cache konfigurieren oder Custom Runner-Image mit vorinstallierter Toolchain verwenden.
- [ ] **Semgrep und Cargo-Tools cachen** — `pip install semgrep` und `cargo install cargo-audit cargo-cyclonedx` laufen bei jedem Run. Entweder in Custom Runner-Image einbacken oder Cache erweitern.
- [ ] **Runner-Image pinnen** — Aktuell `:latest` Tag (`docker.gitea.com/runner-images:ubuntu-latest`). Fuer Reproduzierbarkeit auf spezifische Version pinnen.
- [ ] **SBOM-Validierung verbessern** — Aktuell nur `JSON.parse()`. Besser: CycloneDX Schema-Validierung.
- [ ] **macOS-Build Checkliste** — macOS-Build ist manuell (`scripts/build-macos.sh`). Release-Checkliste erstellen (`docs/guides/RELEASE_CHECKLIST.md`).
- [ ] **Dependency-Update-Automation** — Kein Renovate/Dependabot konfiguriert. Fuer automatische Dependency-Updates evaluieren (Gitea hat Renovate-Support).

### Dokumentation

- [ ] **Playwright CLI aktualisieren** — `@playwright/cli: ^0.1.0` ist sehr alt, sollte auf 1.x aktualisiert werden.

---

## Niedrig-Prioritaet (Nice-to-Have)

### Performance & Testing

- [ ] **Performance Benchmarks fehlen** — Kein `benches/` Verzeichnis. `criterion.rs` fuer Benchmarks von Batch-Processing, Lock-Contention und Embedding-Generierung einrichten.
- [ ] **Memoization-Kandidaten im Frontend** — Berechnungen in MindfuckView und anderen Komponenten sollten `$derived` nutzen statt bei jedem Render neu berechnet zu werden.
- [ ] **Lazy Loading fuer Sub-Komponenten** — ArticleView laedt StatisticalPreview, ArticleKeywords, ArticleCategories, SimilarArticles sofort. Opportunity: Lazy-load wenn nicht im Viewport.

### Konfiguration

- [ ] **`.DS_Store` in .gitignore** — macOS-spezifische Dateien koennten versehentlich committed werden.

### Architektur

- [ ] **`ollama-rs` Crate wird nicht genutzt** — APIs werden manuell via reqwest aufgerufen. Entweder Crate nutzen oder aus Cargo.toml entfernen.
- [ ] **API-Versionierung und CHANGELOG** — Fuer v1.0 Release sinnvoll.

---

## Bestehende Issues

### GTK3-Dependencies in cargo audit

- [ ] **GTK3-Bindings verursachen wiederkehrende cargo-audit Warnings** — alle sind transitive Dependencies von Tauri (wry, tao, gtk, glib, atk, etc.) und nicht direkt kontrollierbar
  - Betroffene Advisories: RUSTSEC-2024-0413 bis -0420 (unmaintained), RUSTSEC-2024-0429 (glib unsound), RUSTSEC-2024-0370
  - CI wurde angepasst: `--deny unsound` entfernt, da glib 0.18.5 nicht vermeidbar ist
- [ ] **Alternativen pruefen:**
  - GTK4-Migration in zukuenftigen Tauri-Versionen (Tauri v2+ / wry Updates)
  - webkit2gtk Updates verfolgen
  - Tauri-Roadmap bzgl. Linux-Backend-Alternativen beobachten
- [ ] **Langfristiges Ziel:** cargo audit ohne GTK3-Ausnahmen bestehen

### Linux-Rechner (RTX 3080Ti)

- [ ] **Ollama Config bereinigen** - `OLLAMA_NUM_PARALLEL` entfernen (nicht mehr verwendet)
  - Befehl: `sudo systemctl edit ollama.service`
  - Nur `OLLAMA_MAX_LOADED_MODELS=2` und `OLLAMA_FLASH_ATTENTION=1` behalten
  - Siehe: `docs/guides/HARDWARE_OPTIMIZATION.md`

---

## Positiv-Befunde (Staerken)

> Diese Bereiche sind vorbildlich und sollten beibehalten werden:

- **Dokumentation**: CLAUDE.md, docs/ und README.md exzellent konsistent mit Code (9/10)
- **Trait-basierte AI-Provider Abstraktion**: Sauberes Design fuer Ollama + OpenAI
- **Database-Patterns**: Lock-Halte-Regeln und Transaction-Safety gruendlich dokumentiert und umgesetzt
- **Type-Safety**: Frontend 95% Type-Coverage, 712 Zeilen Definitionen, 50+ Interfaces
- **Svelte 5 Runes**: Korrekt und modern eingesetzt ($state, $derived, $effect)
- **i18n**: 100% vollstaendig, keine hardcoded Strings
- **Event-Listener Lifecycle**: 53 korrekte onMount/onDestroy Paare, 0 Memory Leaks
- **Security-Scanning**: Semgrep, npm audit, cargo audit, SBOMs umfassend
- **Code-Konsistenz**: Erzwungen durch Husky Hooks (ESLint, Prettier, rustfmt, Clippy)
- **0 TODO/FIXME/HACK Kommentare** im bestehenden Code
