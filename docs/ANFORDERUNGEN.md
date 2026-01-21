# fuckupRSS – Anforderungsdokument

**Version:** 0.7
**Datum:** 2026-01-18
**Status:** Phase 4 (Polish) in Entwicklung

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
| **KI-Backend** | Ollama (lokal) |
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

### Phase 4: Polish (Aktuell)
- [x] Operation Mindfuck (Personalisierte Empfehlungen)
- [x] OPML Import/Export
- [ ] Performance-Optimierung für >10K Artikel
- ~~Erweiterte Keyboard-Shortcuts~~ (gestrichen)
- ~~Desktop-Notifications~~ (gestrichen)

### Phase 5: Release
- [ ] Linux-Paketierung (.deb, .rpm, AppImage)
- [ ] macOS-Build + Signierung
- [ ] Dokumentation finalisieren
- [ ] Release v1.0

### Technische Schulden

| Priorität | Thema | Beschreibung |
|-----------|-------|--------------|
| Mittel | Hardware-Profile | Vordefinierte Presets für 8GB/16GB+ |
| Niedrig | Test-Coverage | E2E-Tests für KI-Features erweitern |
| Niedrig | Clustering | Batch-Optimierung evaluieren |

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

---

*Dokument erstellt: 2025-01-04*
*Letzte Konsolidierung: 2026-01-18*
*fuckupRSS – "Immanentize the Eschaton, one feed at a time."*
