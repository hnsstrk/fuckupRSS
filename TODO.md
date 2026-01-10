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

1. **NUM_PARALLEL=2 statt 4** → ministral-3:latest (~7-8 GB) + Embedding (~0.7 GB) = ~8-9 GB
2. **ministral-3:3b verwenden** → 3b (~4-5 GB) + Embedding (~0.7 GB) = ~5-6 GB, erlaubt NUM_PARALLEL=4

**Trade-offs:**

| Aspekt | 8.9B + NUM_PARALLEL=2 | 3B + NUM_PARALLEL=4 |
|--------|----------------------|---------------------|
| Qualität | Bessere Zusammenfassungen | Ausreichend für RSS |
| Geschwindigkeit | ~2x langsamer | ~2-3x schneller |
| VRAM | ~8-9 GB | ~5-6 GB |
| Parallelisierung | 2 gleichzeitige Analysen | 4 gleichzeitige Analysen |

**Empfehlung:** Für 12 GB GPU (RTX 3080 Ti) mit Fokus auf Durchsatz:
- `ministral-3:3b` + `OLLAMA_NUM_PARALLEL=4`
- Alternativ: `ministral-3:latest` + `OLLAMA_NUM_PARALLEL=2`

**Entscheidung:** Offen - Benutzer kann in Settings wählen.

**TODO:** Hardware-Profile-Settings anpassen, um diese Empfehlungen zu reflektieren.

---

## Phase 3: KI-Features (Aktuell)

Status: In Entwicklung

### Hohe Priorität

- [ ] **Artikel-Embeddings implementieren**
  - `fnords.embedding` Spalte hinzufügen (1024-dim BLOB)
  - Embedding-Generierung bei Artikel-Analyse
  - Batch-Regenerierung für bestehende Artikel
  - Quelle: `fuckupRSS-Anforderungen.md` C.2

- [ ] **Ähnliche Artikel finden**
  - `find_similar_articles` Command implementieren
  - UI-Integration in ArticleView
  - Vektor-Ähnlichkeit via Cosine-Distance
  - Quelle: `fuckupRSS-Anforderungen.md` C.2

### Mittlere Priorität

- [ ] **Semantische Suche**
  - Volltext-Suche via Embeddings
  - Query-Embedding generieren
  - Top-K ähnlichste Artikel zurückgeben
  - Quelle: `fuckupRSS-Anforderungen.md` C.2, `README.md`

### Niedrige Priorität

- [ ] **VSS-Integration optimieren**
  - sqlite-vec für performante Nearest-Neighbor-Suche
  - Index-Optimierung für große Datenmengen
  - Quelle: `fuckupRSS-Anforderungen.md` C.2

---

## Phase 4: Polish

Status: Geplant

### Operation Mindfuck (Bias-Spiegel)

- [ ] **Lesehistorie erfassen**
  - Tracking welche Artikel gelesen werden
  - Zeitstempel und Verweildauer

- [ ] **Bias-Berechnung**
  - Politische Tendenz der gelesenen Artikel aggregieren
  - Thematische Verteilung analysieren

- [ ] **Blinde-Flecken-Erkennung**
  - Unterrepräsentierte Kategorien identifizieren
  - Fehlende Perspektiven aufzeigen

- [ ] **Bias-Dashboard UI**
  - Visualisierung der eigenen Filterblase
  - Trends über Zeit

- [ ] **Gegenpol-Empfehlungen**
  - Artikel mit alternativen Perspektiven vorschlagen
  - Basierend auf Bias-Analyse

Quelle: `fuckupRSS-Anforderungen.md` Phase 4, `README.md`

### Import/Export

- [ ] **OPML Import**
  - Feed-Listen aus anderen Readern importieren
  - Kategorien übernehmen

- [ ] **OPML Export**
  - Feed-Liste exportieren
  - Kompatibilität mit anderen Readern

Quelle: `fuckupRSS-Anforderungen.md` Phase 4, `CLAUDE.md`

### UX-Verbesserungen

- [ ] **Erweiterte Keyboard-Shortcuts (Vim-Style)**
  - Navigation: j/k, gg/G
  - Aktionen: m (mark), s (star)
  - Quelle: `fuckupRSS-Anforderungen.md` Phase 4

- [ ] **Desktop-Notifications**
  - Benachrichtigung bei neuen Artikeln
  - Konfigurierbar pro Feed
  - Quelle: `fuckupRSS-Anforderungen.md` Phase 4

---

## Phase 5: Release

Status: Geplant

- [ ] **Linux-Paketierung**
  - .deb für Debian/Ubuntu
  - .rpm für Fedora/RHEL
  - AppImage für universelle Distribution
  - Quelle: `fuckupRSS-Anforderungen.md` Phase 5

- [ ] **macOS-Build**
  - Apple Silicon optimiert
  - Code-Signierung
  - DMG-Paketierung
  - Quelle: `fuckupRSS-Anforderungen.md` Phase 5

- [ ] **Dokumentation finalisieren**
  - README.md aktualisieren
  - Screenshots hinzufügen
  - Installation guides vervollständigen
  - Quelle: `fuckupRSS-Anforderungen.md` Phase 5

- [ ] **Release v1.0**
  - GitHub Release erstellen
  - Changelog schreiben
  - Quelle: `fuckupRSS-Anforderungen.md` Phase 5

---

## Technische Schulden

- [ ] **Hardware-Profile-Dokumentation erweitern**
  - Empfehlungen für verschiedene GPU-Größen (8GB, 12GB, 16GB+)
  - macOS Apple Silicon Konfiguration

- [ ] **Test-Coverage erhöhen**
  - Aktuell: 260 Tests
  - Ziel: Alle neuen Features testen

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
- [x] sqlite-vec Extension Loading implementiert

---

*Dokumentation basiert auf: `fuckupRSS-Anforderungen.md`, `README.md`, `CLAUDE.md`, `AGENTS.md`*
