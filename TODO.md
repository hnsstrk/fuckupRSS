# TODO.md

Zentrale Aufgabenliste für fuckupRSS. Diese Datei konsolidiert alle offenen Tasks aus dem Projekt.

**Letzte Aktualisierung:** 2026-01-14

---

## Entscheidungen & Analysen

### Hardware-Optimierung: Modellwahl für 12 GB VRAM (2026-01-10)

**Problem:** Mit `ministral-3:latest` (8.9B Parameter) und `OLLAMA_NUM_PARALLEL=4` werden ~9.3 GB VRAM belegt. Das lässt keinen Platz für das zweite Modell `snowflake-arctic-embed2`.

#### Benchmark-Ergebnisse (2026-01-10)

**Getestete Modelle:**

| Modell | Parameter | Disk | Quantization |
|--------|-----------|------|--------------|
| ministral-3:3b | 3.8B | 3.0 GB | Q4_K_M |
| ministral-3:latest | 8.9B | 6.0 GB | Q4_K_M |
| qwen3-vl:8b | 8.8B | 6.1 GB | Q4_K_M |

**Entscheidende Entdeckung: Context-Length-Optimierung**

Das ministral-3:latest Modell hat `num_ctx=32768` als Default. Das erklärt den hohen VRAM-Verbrauch.

| num_ctx | VRAM | GPU% | Zeit (warm) |
|---------|------|------|-------------|
| 32768 (Default) | 9.5 GB | 100% | ~22s |
| 8192 | 11 GB | 84% | ~6.5s |
| **4096** | **9.5 GB** | **100%** | **~1.5s** |

**fuckupRSS-Code aktualisiert:** `num_ctx: 4096` in `src-tauri/src/ollama/mod.rs`

→ 4K Context ist ausreichend (Content wird auf 6000 Zeichen gekürzt = ~1500 Tokens)

#### Qualitätsvergleich

| Modell | JSON-Zuverlässigkeit | Summary-Qualität | Gesamt |
|--------|---------------------|------------------|--------|
| ministral-3:3b | ⚠️ 2/3 | Gut | ⭐⭐⭐ |
| ministral-3:latest | ✅ 3/3 | Sehr gut | ⭐⭐⭐⭐⭐ |
| qwen3-vl:8b | ✅ 3/3 | Gut | ⭐⭐⭐⭐ |

#### Empfehlung nach Hardware

| GPU | Modell | num_ctx | NUM_PARALLEL | Erwartete Leistung |
|-----|--------|---------|--------------|-------------------|
| **12 GB** | ministral-3:latest | 4096 | 2-4 | ~1.5s/Artikel, Platz für Embedding-Modell |
| 16+ GB | ministral-3:latest | 4096 | 4-8 | ~1.5s/Artikel, sehr hoher Durchsatz |
| 8 GB | ministral-3:3b | 4096 | 2-4 | ~1s/Artikel, evtl. Qualitätseinbußen |

**Status:** ✅ Optimierung implementiert (`num_ctx: 4096`)

---

## Phase 3: KI-Features ✅

Status: Abgeschlossen

### 1. Artikel-Embeddings ✅

**Warum zuerst?** Basis für alle folgenden Features (Ähnliche Artikel, Semantische Suche).

- [x] **Schema-Migration**
  - `fnords.embedding` Spalte hinzufügen (1024-dim BLOB)
  - `vec_fnords` Virtual Table für schnelle Vektorsuche
  - Index `idx_fnords_no_embedding` für Performance

- [x] **Embedding-Generierung**
  - Bei Artikel-Analyse: Titel + Content embedden
  - In `process_article_discordian` integrieren
  - Automatisch bei Batch-Verarbeitung

- [x] **Batch-Regenerierung**
  - `generate_article_embeddings_batch` Command
  - `get_article_embedding_stats` Command
  - Fortschrittsanzeige mit Events

Quelle: `fuckupRSS-Anforderungen.md` C.2

### 2. Ähnliche Artikel ✅

- [x] **Backend**
  - `find_similar_articles(fnord_id, limit)` Command
  - sqlite-vec basierte Vektorsuche (O(log n))
  - Similarity-Threshold ≥ 0.5

- [x] **Frontend**
  - Sektion "Ähnliche Artikel" in ArticleView
  - Klickbare Links zu verwandten Artikeln
  - Similarity-Score-Anzeige

Quelle: `fuckupRSS-Anforderungen.md` C.2

### 3. Semantische Suche ✅

**Abhängigkeit:** Artikel-Embeddings müssen implementiert sein.

- [x] **Backend**
  - `semantic_search(query, limit)` Command
  - Query-Text → Embedding → Nearest Neighbors
  - Similarity-Threshold ≥ 0.3

- [x] **Frontend**
  - Suchfeld in Sidebar (mit Debounce 300ms)
  - Suchergebnisse mit Similarity-Score
  - ESC zum Löschen, Enter für sofortige Suche

Quelle: `fuckupRSS-Anforderungen.md` C.2, `README.md`

### 4. VSS-Optimierung (NIEDRIGE PRIORITÄT)

**Abhängigkeit:** Artikel-Embeddings + signifikante Datenmenge.

- [ ] sqlite-vec Index für performante Nearest-Neighbor-Suche
- [ ] Benchmark bei >10.000 Artikeln

Quelle: `fuckupRSS-Anforderungen.md` C.2

---

## Phase 4: Polish

Status: Geplant (nach Phase 3)

### Operation Mindfuck (Bias-Spiegel) ✅

**Abhängigkeitsreihenfolge:**

1. [x] **Lesehistorie erfassen** ✅
   - `read_at` Timestamp beim Öffnen eines Artikels
   - COALESCE-Logik bewahrt ersten Lesezeitpunkt

2. [x] **Bias-Berechnung** ✅
   - Aggregierte politische Tendenz
   - Thematische Verteilung (Sephiroth)

3. [x] **Blinde-Flecken-Erkennung** ✅
   - Unterrepräsentierte Kategorien
   - Fehlende politische Perspektiven

4. [x] **Bias-Dashboard UI** ✅
   - Visualisierung der Filterblase
   - Trends über Zeit
   - Tab-basierte Navigation (Übersicht, Blinde Flecken, Empfehlungen, Trends)

5. [x] **Gegenpol-Empfehlungen** ✅
   - Artikel mit alternativen Perspektiven
   - "Erweiter deinen Horizont"-Feature

Quelle: `fuckupRSS-Anforderungen.md` Phase 4, `README.md`

### Import/Export ✅

- [x] **OPML Import** ✅
  - Feed-Listen aus anderen Readern importieren
  - Preview der zu importierenden Feeds
  - Duplikaterkennung
  - Tauri Dialog für Dateiauswahl

- [x] **OPML Export** ✅
  - Export aller Feeds als OPML-Datei
  - Save-Dialog für Dateispeicherort
  - Kompatibilität mit Feedly, Inoreader, etc.

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
  - Aktuell: 269 Tests (160 Rust, 95 Vitest, 14 E2E)
  - Neue Features müssen getestet werden
  - E2E-Tests für KI-Features
  - [x] Tests für State-Konsistenz nach Backend-Operationen (2026-01-14):
    - Sync/Unprocessed-Count-Reaktivität
    - deletePentacle/fetchFullContent/processArticleDiscordian

- [x] **Frontend-Backend-Kommunikation Review (2026-01-14)**
  - 78 invoke-Calls analysiert
  - State-Konsistenz-Probleme behoben (5 Fixes)
  - Error-Handling in SettingsView verbessert
  - Checkliste erstellt: `QUALITY_CHECKLIST.md`

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

### Phase 3: KI-Features ✅
- [x] Keyword-Embeddings via snowflake-arctic-embed2
- [x] `find_similar_keywords` Command
- [x] sqlite-vec Extension Loading implementiert
- [x] Hardware-Profile-Settings UI
- [x] Artikel-Embeddings (fnords.embedding + vec_fnords)
- [x] `find_similar_articles` Command
- [x] "Ähnliche Artikel" UI in ArticleView
- [x] Batch-Regenerierung für Artikel-Embeddings
- [x] `semantic_search` Command
- [x] Semantische Suche UI in Sidebar

### Phase 4: Polish (In Arbeit)
- [x] OPML Import mit Preview und Duplikaterkennung
- [x] OPML Export mit Save-Dialog
- [x] Operation Mindfuck (Bias-Spiegel) komplett implementiert
  - Backend: get_reading_profile, get_blind_spots, get_counter_perspectives, get_reading_trends
  - UI: MindfuckView.svelte mit Tabs für Übersicht, Blinde Flecken, Empfehlungen, Trends

---

## Quick Reference: Nächste Schritte

**Phase 4 fast abgeschlossen!**

```
1. OPML Import                 ───── ✅

2. OPML Export                 ───── ✅

3. Operation Mindfuck          ───── ✅

4. VSS-Optimierung             <──── Bei Bedarf (>10.000 Artikel)

5. Phase 5: Release            <──── Nächster Schritt
```

---

*Dokumentation basiert auf: `fuckupRSS-Anforderungen.md`, `README.md`, `CLAUDE.md`, `AGENTS.md`, `QUALITY_CHECKLIST.md`*
