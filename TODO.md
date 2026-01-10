# TODO.md

Zentrale Aufgabenliste für fuckupRSS. Diese Datei konsolidiert alle offenen Tasks aus dem Projekt.

**Letzte Aktualisierung:** 2026-01-10

---

## Entscheidungen & Analysen

### Hardware-Optimierung: Modellwahl für 12 GB VRAM (2026-01-10)

**Problem:** Mit `ministral-3:latest` (8.9B Parameter) und `OLLAMA_NUM_PARALLEL=4` werden ~9.3 GB VRAM belegt. Das lässt keinen Platz für das zweite Modell `snowflake-arctic-embed2`.

**Analyse:**

| Modell | Parameter | Disk | VRAM (NUM_PARALLEL=4) |
|--------|-----------|------|----------------------|
| ministral-3:latest | 8.9B | 6.0 GB | ~9-10 GB |
| ministral-3:3b | 3B | 3.0 GB | ~4-5 GB |
| snowflake-arctic-embed2 | - | 1.2 GB | ~0.7 GB |

**Beobachtung:** `ollama ps` zeigt:
```
ministral-3:latest  size_vram: 10017202816 (~9.3 GB)
context_length: 8192
```

Der hohe VRAM-Verbrauch kommt vom **KV-Cache** für parallele Anfragen.

**Optionen:**

| Konfiguration | VRAM | Parallelisierung | Qualität |
|---------------|------|------------------|----------|
| 8.9B + NUM_PARALLEL=2 | ~8-9 GB | 2 gleichzeitig | Besser |
| 3B + NUM_PARALLEL=4 | ~5-6 GB | 4 gleichzeitig | Ausreichend |

**Empfehlung:**
- **12 GB GPU (RTX 3080 Ti):** `OLLAMA_NUM_PARALLEL=2` mit 8.9B ODER `NUM_PARALLEL=4` mit 3B
- **16+ GB GPU:** `OLLAMA_NUM_PARALLEL=4` mit 8.9B problemlos möglich
- **8 GB GPU:** `ministral-3:3b` + `NUM_PARALLEL=2`

**Status:** Benutzer wählt Modell in Settings, Hardware-Profile dokumentiert.

---

## Phase 3: KI-Features (Aktuell)

Status: In Entwicklung

### 1. Artikel-Embeddings (HÖCHSTE PRIORITÄT)

**Warum zuerst?** Basis für alle folgenden Features (Ähnliche Artikel, Semantische Suche).

- [ ] **Schema-Migration**
  - `fnords.embedding` Spalte hinzufügen (1024-dim BLOB)
  - Index für Performance

- [ ] **Embedding-Generierung**
  - Bei Artikel-Analyse: Titel + Content embedden
  - In `process_article_discordian` integrieren

- [ ] **Batch-Regenerierung**
  - Command für bestehende Artikel ohne Embedding
  - Fortschrittsanzeige wie bei Batch-Analyse

Quelle: `fuckupRSS-Anforderungen.md` C.2

### 2. Ähnliche Artikel (HOHE PRIORITÄT)

**Abhängigkeit:** Artikel-Embeddings müssen implementiert sein.

- [ ] **Backend**
  - `find_similar_articles(fnord_id, limit)` Command
  - Cosine-Distance-Berechnung
  - Threshold für Mindest-Ähnlichkeit

- [ ] **Frontend**
  - Sektion "Ähnliche Artikel" in ArticleView
  - Klickbare Links zu verwandten Artikeln

Quelle: `fuckupRSS-Anforderungen.md` C.2

### 3. Semantische Suche (MITTLERE PRIORITÄT)

**Abhängigkeit:** Artikel-Embeddings müssen implementiert sein.

- [ ] **Backend**
  - `semantic_search(query, limit)` Command
  - Query-Text → Embedding → Nearest Neighbors

- [ ] **Frontend**
  - Such-UI erweitern
  - Umschalten zwischen Volltext/Semantisch

Quelle: `fuckupRSS-Anforderungen.md` C.2, `README.md`

### 4. VSS-Optimierung (NIEDRIGE PRIORITÄT)

**Abhängigkeit:** Artikel-Embeddings + signifikante Datenmenge.

- [ ] sqlite-vec Index für performante Nearest-Neighbor-Suche
- [ ] Benchmark bei >10.000 Artikeln

Quelle: `fuckupRSS-Anforderungen.md` C.2

---

## Phase 4: Polish

Status: Geplant (nach Phase 3)

### Operation Mindfuck (Bias-Spiegel)

**Abhängigkeitsreihenfolge:**

1. [ ] **Lesehistorie erfassen** (zuerst)
   - `read_at` Timestamp beim Öffnen eines Artikels
   - Optional: Verweildauer tracken

2. [ ] **Bias-Berechnung** (benötigt Lesehistorie)
   - Aggregierte politische Tendenz
   - Thematische Verteilung (Sephiroth)

3. [ ] **Blinde-Flecken-Erkennung** (benötigt Bias-Berechnung)
   - Unterrepräsentierte Kategorien
   - Fehlende politische Perspektiven

4. [ ] **Bias-Dashboard UI** (benötigt alle vorherigen)
   - Visualisierung der Filterblase
   - Trends über Zeit

5. [ ] **Gegenpol-Empfehlungen** (benötigt Dashboard)
   - Artikel mit alternativen Perspektiven
   - "Erweiter deinen Horizont"-Feature

Quelle: `fuckupRSS-Anforderungen.md` Phase 4, `README.md`

### Import/Export

- [ ] **OPML Import**
  - Feed-Listen aus anderen Readern
  - Kategorien-Mapping auf Sephiroth

- [ ] **OPML Export**
  - Kompatibilität mit Feedly, Inoreader, etc.

Quelle: `fuckupRSS-Anforderungen.md` Phase 4

### UX-Verbesserungen

- [ ] **Erweiterte Keyboard-Shortcuts (Vim-Style)**
  - Navigation: j/k, gg/G, Ctrl+d/u
  - Aktionen: m (mark read), s (star), o (open)

- [ ] **Desktop-Notifications**
  - Tauri Notification API
  - Konfigurierbar: Global, pro Feed, pro Kategorie

Quelle: `fuckupRSS-Anforderungen.md` Phase 4

---

## Phase 5: Release

Status: Geplant (nach Phase 4)

- [ ] **Linux-Paketierung**
  - .deb (Debian/Ubuntu)
  - .rpm (Fedora/RHEL)
  - AppImage (universal)
  - Flatpak (optional)

- [ ] **macOS-Build**
  - Universal Binary (x86_64 + arm64)
  - Code-Signierung + Notarisierung
  - DMG mit Installer

- [ ] **Dokumentation**
  - Screenshots für README
  - Video-Demo (optional)
  - Installation Guide pro Plattform

- [ ] **Release v1.0**
  - GitHub Release
  - Changelog
  - Ankündigung

Quelle: `fuckupRSS-Anforderungen.md` Phase 5

---

## Technische Schulden

- [ ] **Hardware-Profile erweitern**
  - Preset für 8GB GPU
  - Preset für 16GB+ GPU
  - macOS-spezifische Empfehlungen

- [ ] **Test-Coverage**
  - Aktuell: 260 Tests
  - Neue Features müssen getestet werden
  - E2E-Tests für KI-Features

---

## Abgeschlossene Meilensteine

### Phase 1: Grundgerüst ✅
- [x] Tauri + Svelte Projekt aufsetzen
- [x] SQLite-Schema implementieren
- [x] Basis-UI (Feed-Liste, Artikel-Ansicht)

### Phase 1.5: Internationalisierung & UX-Grundlagen ✅
- [x] i18n-System (svelte-i18n) mit Deutsch und Englisch
- [x] Tooltips für Illuminatus!-Terminologie
- [x] Einstellungen-Dialog (Sprache, Tooltips ein/aus)
- [x] Persistente Benutzereinstellungen

### Phase 2: Core-Features ✅
- [x] Feed-Parsing (feed-rs)
- [x] Automatische Feed-Synchronisation
- [x] Hagbard's Retrieval (Volltext)
- [x] Ollama-Integration (ollama-rs)
- [x] Basis-KI-Pipeline (Batch-Verarbeitung)
- [x] Discordian Analysis (Zusammenfassung, Kategorien, Stichworte)
- [x] Greyface Alert (Bias-Erkennung)
- [x] Immanentize Network (Keyword-Qualität, Synonyme, Embeddings)

### Phase 3: KI-Features (Teilweise) ✅
- [x] Keyword-Embeddings via snowflake-arctic-embed2
- [x] `find_similar_keywords` Command
- [x] sqlite-vec Extension Loading implementiert
- [x] Hardware-Profile-Settings UI

---

## Quick Reference: Nächste Schritte

**Empfohlene Reihenfolge für Phase 3:**

```
1. fnords.embedding Spalte     ─────┐
                                    ├──► 3. Ähnliche Artikel UI
2. Embedding bei Analyse       ─────┘
                                    ┌──► 4. Semantische Suche
3. find_similar_articles ──────────►┤
                                    └──► 5. VSS-Optimierung
```

---

*Dokumentation basiert auf: `fuckupRSS-Anforderungen.md`, `README.md`, `CLAUDE.md`, `AGENTS.md`*
