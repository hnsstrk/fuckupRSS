# fuckupRSS Dokumentation

Willkommen zur technischen Dokumentation von fuckupRSS - dem RSS-Aggregator mit lokaler KI-Integration, benannt nach F.U.C.K.U.P. aus der Illuminatus!-Trilogie.

Diese Dokumentation ist in thematische Bereiche gegliedert. Die Hauptdokumentation (README, CLAUDE.md) befindet sich im Projekt-Root, waehrend detaillierte technische Spezifikationen hier im `docs/`-Verzeichnis organisiert sind.

Architekturentscheidungen und Projektwissen: Siehe Obsidian Vault [[fuckupRSS Arc42 Architekturdokumentation]]

---

## Schnellzugriff

| Dokument | Beschreibung |
|----------|--------------|
| [README.md](../README.md) | Projektbeschreibung, Features, Installation |
| [CLAUDE.md](../CLAUDE.md) | Entwickler-Kontext fuer Claude Code |
| [ANFORDERUNGEN.md](ANFORDERUNGEN.md) | Roadmap, Governance, Entscheidungen |
| [FEEDS.md](FEEDS.md) | Standard-RSS-Feeds fuer Entwicklung und Tests |

---

## Architektur

Technische Grundlagen und Systemdesign.

| Dokument | Beschreibung |
|----------|--------------|
| [AI_PROCESSING_PIPELINE.md](architecture/AI_PROCESSING_PIPELINE.md) | KI-Verarbeitungspipeline: Retrieval, Analyse, Greyface Alert, Prompt-Design |
| [DATABASE_SCHEMA.md](architecture/DATABASE_SCHEMA.md) | SQLite-Schema: Tabellen, Revisionsverwaltung, Settings, Embeddings |

---

## API-Referenz

Schnittstellen zwischen Frontend und Backend.

| Dokument | Beschreibung |
|----------|--------------|
| [TAURI_COMMANDS_REFERENCE.md](api/TAURI_COMMANDS_REFERENCE.md) | Alle Tauri Commands mit Parametern und Rueckgabewerten |

---

## Guides

Anleitungen und Best Practices.

| Dokument | Beschreibung |
|----------|--------------|
| [TESTING.md](guides/TESTING.md) | Test-Strategie: Unit Tests, E2E Tests, Coverage |
| [QUALITY_CHECKLIST.md](guides/QUALITY_CHECKLIST.md) | Checkliste fuer Frontend-Backend-Kommunikation |
| [HARDWARE_OPTIMIZATION.md](guides/HARDWARE_OPTIMIZATION.md) | Hardware-Profile, VRAM-Optimierung, Ollama-Konfiguration |
| [CI_CD_SETUP.md](guides/CI_CD_SETUP.md) | CI/CD Pipeline, Gitea Actions Runner Setup |
| [RELEASE_CHECKLIST.md](guides/RELEASE_CHECKLIST.md) | Release-Checkliste und Versionierung |

---

## Features

### UI-Features

Benutzeroberflaechen-Funktionen.

| Dokument | Beschreibung |
|----------|--------------|
| [SORTING_FILTERING.md](features/ui/SORTING_FILTERING.md) | Sortier- und Filteroptionen |
| [IMPORT_EXPORT.md](features/ui/IMPORT_EXPORT.md) | OPML Import/Export, Artikel-Export |
| [KEYBOARD_SHORTCUTS.md](features/ui/KEYBOARD_SHORTCUTS.md) | Vim-Style Tastaturkuerzel |

### Immanentize Network (Schlagwort-Wissensnetz)

Das semantische Keyword-Netzwerk fuer intelligente Artikel-Verknuepfung.

| Dokument | Beschreibung |
|----------|--------------|
| [ARCHITECTURE_GRAPH.md](features/immanentize/ARCHITECTURE_GRAPH.md) | Graph-Architektur und Visualisierung |
| [GRAPH_REQUIREMENTS.md](features/immanentize/GRAPH_REQUIREMENTS.md) | Anforderungen an das Graph-System |
| [GRAPH_TECH_EVAL.md](features/immanentize/GRAPH_TECH_EVAL.md) | Technologie-Evaluation fuer Graph-Visualisierung |

### Recommendations (Operation Mindfuck)

Das personalisierte Empfehlungssystem basierend auf Nutzerinteressen.

| Dokument | Beschreibung |
|----------|--------------|
| [RECS_PRODUCT_BRIEF.md](recommendations/RECS_PRODUCT_BRIEF.md) | Produktvision und Ziele |
| [RECS_ALGO_SPEC.md](recommendations/RECS_ALGO_SPEC.md) | Algorithmus-Spezifikation |
| [RECS_API_SPEC.md](recommendations/RECS_API_SPEC.md) | API-Endpunkte und Datenstrukturen |
| [RECS_UI_SPEC.md](recommendations/RECS_UI_SPEC.md) | UI/UX-Design und Komponenten |
| [RECS_SIGNAL_CATALOG.md](recommendations/RECS_SIGNAL_CATALOG.md) | Signale und Gewichtungen |
| [RECS_DATA_INVENTORY.md](recommendations/RECS_DATA_INVENTORY.md) | Datenquellen und -strukturen |
| [RECS_CURRENT_STATE.md](recommendations/RECS_CURRENT_STATE.md) | Aktueller Implementierungsstand |

---

## Plaene

Noch nicht implementierte Feature-Designs.

| Dokument | Beschreibung |
|----------|--------------|
| [2026-03-01-ollama-server-dropdown-design.md](plans/2026-03-01-ollama-server-dropdown-design.md) | Ollama-Server Dropdown mit History |

---

## Reports

Analysen und Recherchen.

| Dokument | Beschreibung |
|----------|--------------|
| [OLLAMA_KI_RECHERCHE_2026.md](reports/OLLAMA_KI_RECHERCHE_2026.md) | Ollama KI-Recherche und Modellvergleiche 2026 |

---

## SQL

Nuetzliche SQL-Skripte fuer Wartung und Migration.

| Dokument | Beschreibung |
|----------|--------------|
| [cleanup-orphans.sql](sql/cleanup-orphans.sql) | Verwaiste Datensaetze bereinigen |
| [migration-22-performance.sql](sql/migration-22-performance.sql) | Performance-Optimierung Migration 22 |

---

## Archiv

Aeltere Reports und archivierte Dokumente.

| Dokument | Beschreibung |
|----------|--------------|
| [STOPWORD_KEYWORD_REPORT.md](archive/STOPWORD_KEYWORD_REPORT.md) | Analyse: Stopwoerter und Keyword-Qualitaet |
| [TODO_LEGACY_2026-01.md](archive/TODO_LEGACY_2026-01.md) | Historische Aufgabenliste |

---

## Verzeichnisstruktur

```
docs/
├── README.md                    # Diese Datei (Navigation Hub)
├── ANFORDERUNGEN.md             # Roadmap, Governance, Entscheidungen
├── FEEDS.md                     # Standard-RSS-Feeds fuer Entwicklung
├── api/                         # API-Referenz
│   └── TAURI_COMMANDS_REFERENCE.md
├── architecture/                # Architektur-Dokumentation
│   ├── AI_PROCESSING_PIPELINE.md
│   └── DATABASE_SCHEMA.md
├── archive/                     # Archivierte Dokumente
│   ├── STOPWORD_KEYWORD_REPORT.md
│   └── TODO_LEGACY_2026-01.md
├── features/                    # Feature-Dokumentation
│   ├── immanentize/             # Immanentize Network
│   │   ├── ARCHITECTURE_GRAPH.md
│   │   ├── GRAPH_REQUIREMENTS.md
│   │   └── GRAPH_TECH_EVAL.md
│   └── ui/                      # UI-Features
│       ├── IMPORT_EXPORT.md
│       ├── KEYBOARD_SHORTCUTS.md
│       └── SORTING_FILTERING.md
├── guides/                      # Anleitungen
│   ├── CI_CD_SETUP.md
│   ├── HARDWARE_OPTIMIZATION.md
│   ├── QUALITY_CHECKLIST.md
│   ├── RELEASE_CHECKLIST.md
│   └── TESTING.md
├── plans/                       # Feature-Designs (noch nicht implementiert)
│   └── 2026-03-01-ollama-server-dropdown-design.md
├── recommendations/             # Empfehlungssystem (Operation Mindfuck)
│   ├── RECS_ALGO_SPEC.md
│   ├── RECS_API_SPEC.md
│   ├── RECS_CURRENT_STATE.md
│   ├── RECS_DATA_INVENTORY.md
│   ├── RECS_PRODUCT_BRIEF.md
│   ├── RECS_SIGNAL_CATALOG.md
│   └── RECS_UI_SPEC.md
├── reports/                     # Analysen und Recherchen
│   └── OLLAMA_KI_RECHERCHE_2026.md
└── sql/                         # SQL-Skripte
    ├── cleanup-orphans.sql
    └── migration-22-performance.sql
```
