# fuckupRSS – Anforderungsdokument

**Version:** 0.8
**Datum:** 2026-03-12
**Status:** Phase 4 (Polish & Advanced Features) weitgehend abgeschlossen, Phase 5 (Release) in Planung

> Dieses Dokument ist der zentrale Roadmap- und Governance-Index für fuckupRSS.
> Technische Details wurden in modulare Dokumentation ausgelagert.

---

## Inhaltsverzeichnis

1. [Projektübersicht](#1-projektübersicht)
2. [Illuminatus!-Terminologie](#2-illuminatus-terminologie)
3. [Dokumentationsstruktur](#3-dokumentationsstruktur)
4. [Zusammenfassung der Entscheidungen](#4-zusammenfassung-der-entscheidungen)
5. [Roadmap (Nächste Schritte)](#5-roadmap-nächste-schritte)
6. [Glossar](#6-glossar)

---

## 1. Projektübersicht

### 1.1 Name und Konzept

**fuckupRSS** – Ein RSS-Aggregator und Reader mit lokaler KI-Integration.

Der Name ist eine Hommage an **F.U.C.K.U.P.** (First Universal Cybernetic-Kinetic Ultra-micro Programmer) aus der Illuminatus!-Trilogie von Robert Shea und Robert Anton Wilson.

### 1.2 Zielplattformen

| Plattform | Priorität | Hardware |
|-----------|-----------|----------|
| Linux | Primary | NVIDIA GPU (12 GB VRAM) |
| macOS | Secondary | Apple Silicon (M4 Pro, 48 GB) |

### 1.3 Kernanforderungen

- Native GUI-Anwendung (Tauri + Svelte)
- Lokale KI-Verarbeitung via Ollama
- Keine Cloud-Abhängigkeit für KI-Features
- Volltext-Abruf bei gekürzten Feeds
- Flexible Sortierung und Filterung
- Bias-Erkennung für Artikel
- Semantische Suche via Embeddings

---

## 2. Illuminatus!-Terminologie

Die Software verwendet durchgängig Begriffe aus der Illuminatus!-Trilogie:

| Konzept | Fnord-Bezeichnung | Beschreibung |
|---------|-------------------|--------------|
| Geänderte Artikel | **Fnords** | Artikel mit Revisionshistorie |
| Ungelesene Artikel | **Concealed** | Verborgen bis zur Erleuchtung |
| Gelesene Artikel | **Illuminated** | Erleuchtete/verarbeitete Artikel |
| Favoriten/Wichtig | **Golden Apple** | Markierte Artikel |
| KI-Zusammenfassung | **Discordian Analysis** | Automatische Analyse |
| Bias-Warnung | **Greyface Alert** | Hinweis auf einseitige Berichterstattung |
| Bias-Spiegel | **Operation Mindfuck** | Filterblase aufzeigen, nicht verstärken |
| Kategorien | **Sephiroth** | Thematische Einordnung |
| Stichworte/Tags | **Immanentize** | Extrahierte Keywords |
| Feed-Quellen | **Pentacles** | Abonnierte RSS/Atom-Feeds |
| Volltextabruf | **Hagbard's Retrieval** | Nachladen gekürzter Artikel |
| Batch-Verarbeitung | **Fnord Processing** | Hintergrund-KI-Pipeline |

---

## 3. Dokumentationsstruktur

Die technische Dokumentation wurde modularisiert:

### Architektur & Technologie

| Thema | Dokument |
|-------|----------|
| Technologie-Stack | [README.md](../README.md) + [CLAUDE.md](../CLAUDE.md) |
| KI-Pipeline & Prompts | [architecture/AI_PROCESSING_PIPELINE.md](architecture/AI_PROCESSING_PIPELINE.md) |
| Datenbank-Schema | [architecture/DATABASE_SCHEMA.md](architecture/DATABASE_SCHEMA.md) |

### API & Referenz

| Thema | Dokument |
|-------|----------|
| Tauri Commands (IPC) | [api/TAURI_COMMANDS_REFERENCE.md](api/TAURI_COMMANDS_REFERENCE.md) |

### Features

| Feature | Dokument |
|---------|----------|
| Immanentize Network | [features/immanentize/](features/immanentize/) |
| Operation Mindfuck | [recommendations/](recommendations/) |
| Sortierung & Filter | [features/ui/SORTING_FILTERING.md](features/ui/SORTING_FILTERING.md) |
| Import/Export | [features/ui/IMPORT_EXPORT.md](features/ui/IMPORT_EXPORT.md) |
| Keyboard-Shortcuts | [features/ui/KEYBOARD_SHORTCUTS.md](features/ui/KEYBOARD_SHORTCUTS.md) |

### Guides

| Thema | Dokument |
|-------|----------|
| Testing | [guides/TESTING.md](guides/TESTING.md) |
| Hardware-Optimierung | [guides/HARDWARE_OPTIMIZATION.md](guides/HARDWARE_OPTIMIZATION.md) |
| Qualitäts-Checkliste | [guides/QUALITY_CHECKLIST.md](guides/QUALITY_CHECKLIST.md) |

### Navigation

Für eine vollständige Übersicht aller Dokumente: **[README.md](README.md)**

---

## 4. Zusammenfassung der Entscheidungen

| Aspekt | Entscheidung |
|--------|--------------|
| **Plattformen** | Linux (primary), macOS (secondary) |
| **GUI-Framework** | Tauri 2.x |
| **Frontend** | Svelte 5 |
| **Backend** | Rust |
| **Datenbank** | SQLite + sqlite-vec |
| **KI-Backend** | Ollama (lokal) oder OpenAI-kompatible API |
| **Hauptmodell** | ministral-3:latest (6 GB) |
| **Embedding-Modell** | snowflake-arctic-embed2 (1.2 GB, 1024-dim) |
| **Bias: Politisch** | Skala -2 bis +2 |
| **Bias: Sachlichkeit** | Skala 0-4 |
| **Bias: Quellenqualität** | 1-5 Sterne (berechnet) |
| **Bias: Artikeltyp** | 6 Kategorien (KI-ermittelt) |
| **Sync-Intervall** | 30 min (Standard, konfigurierbar) |
| **Volltext-Abruf** | Automatisch bei gekürzten Feeds |
| **Keyboard-Shortcuts** | Vim-Style |
| **i18n** | Deutsch (Standard), Englisch |

---

## 5. Roadmap (Nächste Schritte)

### Phase 1: Grundgerüst ✅
- [x] Tauri + Svelte Projekt aufsetzen
- [x] SQLite-Schema implementieren
- [x] Basis-UI (Feed-Liste, Artikel-Ansicht)

### Phase 1.5: Internationalisierung & UX ✅
- [x] i18n-System (svelte-i18n) mit Deutsch und Englisch
- [x] Tooltips für Illuminatus!-Terminologie
- [x] Einstellungen-Dialog
- [x] Persistente Benutzereinstellungen

### Phase 2: Core-Features ✅
- [x] Feed-Parsing (feed-rs)
- [x] Automatische Feed-Synchronisation
- [x] Hagbard's Retrieval (Volltext)
- [x] Ollama-Integration (ollama-rs)
- [x] Discordian Analysis (Zusammenfassung, Kategorien, Stichworte)
- [x] Greyface Alert (Bias-Erkennung)
- [x] Immanentize Network (Keyword-Qualität, Synonyme, Embeddings)

### Phase 3: KI-Features ✅
- [x] Keyword-Embeddings via snowflake-arctic-embed2
- [x] Artikel-Embeddings + sqlite-vec
- [x] Ähnliche Artikel (Vektor-Ähnlichkeit)
- [x] Semantische Suche
- [x] Artikeltyp-Klassifikation (news, analysis, opinion, satire, ad, unknown)

### Phase 4: Polish & Advanced Features ✅
- [x] Operation Mindfuck (Personalisierte Empfehlungen)
- [x] OPML Import/Export
- [x] Erweiterte Keyboard-Shortcuts (Vim-Style: j/k, o/Enter, v, r/u, s, a, /, f)
- [x] Tägliche Briefings (KI-generierte Zusammenfassungen der Top-Artikel)
- [x] ~~Story Clustering (Union-Find Algorithmus, Embedding-Ähnlichkeit > 0.78, Perspektiven-Vergleich)~~ — Ersetzt durch Theme Reports (April 2026)
- [x] Theme Reports (Multi-Signal Topic Detection + LLM-Tiefenanalyse, ersetzt Story Clustering)
- [x] Named Entity Recognition (NER) – Personen, Organisationen, Orte, Events
- [x] OpenAI-kompatible API als Alternative zu Ollama für Textgenerierung
- [x] Ollama API Modernisierung (Structured Outputs, /api/chat, /api/embed Batch)
- [ ] Performance-Optimierung für >10K Artikel (PRAGMA-Settings, Indexierung)
- ~~Desktop-Notifications~~ (gestrichen)

### Phase 4.5: Refactoring & Tech Debt (Laufend)
- [ ] Große Frontend-Komponenten aufteilen (CompoundKeywordManager, KeywordNetworkSynonyms, KeywordNetworkDetail, ArticleKeywords, SettingsOllama – je >1400 Zeilen)
- [ ] Deprecated KeywordWithSource durch KeywordWithMetadata ersetzen
- [ ] Orphaned immanentize_clusters nach Keyword-Deletion bereinigen
- [ ] E2E-Tests: Mock-System verbessern oder Limitationen dokumentieren (9 skipped)
- [ ] ollama-rs Crate evaluieren (aktuell manuelle reqwest-Aufrufe)

### Phase 5: Release
- [ ] Linux-Paketierung (.deb, AppImage – vorhanden; .rpm – ausstehend)
- [ ] macOS-Build (vorhanden) + Code-Signing + Notarization
- [ ] CI/CD-Optimierung (Caching: Rust-Toolchain, Semgrep, Cargo-Tools; Runner-Image pinnen; Dependency-Automation)
- [ ] API-Versionierung und CHANGELOG
- [ ] Performance Benchmarks (criterion.rs)
- [ ] Dokumentation finalisieren
- [ ] Release v1.0

### Phase 6: KI-Erweiterungen (Tier 3+4)

> Basierend auf der [Ollama-Potenzialanalyse](../reports/OLLAMA_KI_RECHERCHE_2026.md). Tier 1+2 wurden in Phase 4 implementiert (März 2026). Design-Dokument: [plans/2026-03-10-ollama-potenzial-tier1-tier2-design.md](plans/2026-03-10-ollama-potenzial-tier1-tier2-design.md)

#### Tier 3: Fortgeschrittene Features (höherer Aufwand)

- [ ] **Argumentationsanalyse** — Pro/Contra-Argumente aus Meinungsartikeln extrahieren; kombiniert mit Theme-Report-Perspektiven ("Welche Argumente bringt die FAZ, welche die taz?")
- [ ] **Claim Detection / Fact-Check-Hints** — Zentrale Behauptungen extrahieren, prüfen ob andere Artikel stützen oder widersprechen ("Diese Behauptung wird in 3 anderen Quellen anders dargestellt")
- [ ] **Bias-Drift-Erkennung über Zeit** — `political_bias` pro Feed über Monate tracken und visualisieren ("Feed X ist 0.5 Punkte nach rechts gedriftet"); Daten vorhanden, Auswertung/Visualisierung fehlt
- [ ] **Bild-Analyse mit Vision-Modellen** — Artikelbilder analysieren (Symbolbild vs. echtes Foto, Manipulation); `image_url` bei vielen Artikeln vorhanden; Gemma 3 / Llama 3.2 Vision
- [ ] **RAG: "Frag deine Artikel"** — Nutzer stellt Frage → semantische Suche via sqlite-vec → LLM antwortet mit Quellenverweisen; Embedding-Infrastruktur bereits vorhanden, Chat-Schnittstelle fehlt

#### Tier 4: Langfrist-Vision (experimentell)

- [ ] **Agentic Workflows** — Multi-Step-Analyse über Ollama Tool Calling; LLM entscheidet selbst ob tiefere Analyse, Ähnlichkeitssuche oder Fact-Check nötig ist
- [ ] **Vorhersage-Modelle auf Trending-Daten** — Frühwarnung für aufkommende Themen aus `immanentize_daily` Zeitreihen ("Keyword X zeigt exponentielles Wachstum")
- [ ] **Feed-übergreifende Quellen-Analyse** — Welche Quellen berichten über dieselben Keyword-Cluster? Welche haben exklusive Themen? Nützlich für bewusste Feed-Kuratierung

### Technische Schulden

Details in [DB_INFRASTRUCTURE_DEBT.md](DB_INFRASTRUCTURE_DEBT.md) und [TECH_DEBT_REPORT.md](TECH_DEBT_REPORT.md).

| Priorität | Thema | Beschreibung |
|-----------|-------|--------------|
| Hoch | Race Condition | `selectView("changed")` kopiert Daten vor async Load |
| Mittel | Hardware-Dokumentation | Konfigurationsrichtlinien aktualisieren |
| Mittel | GTK3-Bindings | cargo-audit Warnings durch transitive Tauri-Dependencies |
| Niedrig | Test-Coverage | E2E-Tests für KI-Features erweitern |
| Niedrig | SQLite-Tuning | PRAGMA-Settings, Indexierung für >10K Artikel |
| Niedrig | Theme Reports Feature Flag | `#[cfg(feature = "clustering")]` Aktivierung prüfen (Theme Reports) |

---

## 6. Glossar

| Begriff | Bedeutung |
|---------|-----------|
| Fnord | Geänderter Artikel (mit Revisionen) |
| Concealed | Ungelesener Artikel |
| Illuminated | Gelesener Artikel |
| Golden Apple | Favorisierter Artikel |
| Pentacle | Feed-Quelle |
| Sephiroth | Kategorie |
| Immanentize | Stichwort/Tag |
| Greyface Alert | Bias-Warnung |
| Operation Mindfuck | Personalisierte Empfehlungen |
| Hagbard's Retrieval | Volltext-Abruf |
| Discordian Analysis | KI-Zusammenfassung |
| Fnord Processing | Batch-Verarbeitung |
| Briefing | Tägliche/wöchentliche KI-Zusammenfassung der Top-Artikel |
| Theme Report | Thematisch gruppierte Artikel mit LLM-Tiefenanalyse (ersetzt Story Cluster) |
| Entity / NER | Erkannte Entitäten (Person, Organization, Location, Event) |

---

*Dokument erstellt: 2025-01-04*
*Letzte Aktualisierung: 2026-03-12*
*fuckupRSS – "Immanentize the Eschaton, one feed at a time."*
