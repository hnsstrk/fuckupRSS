# fuckupRSS – Anforderungsdokument

**Version:** 0.4
**Datum:** 2026-01-04
**Status:** Phase 1.5 abgeschlossen (i18n & UX)

---

## Inhaltsverzeichnis

1. [Projektübersicht](#1-projektübersicht)
2. [Illuminatus!-Terminologie](#2-illuminatus-terminologie)
3. [Technologie-Stack](#3-technologie-stack)
4. [KI-Integration](#4-ki-integration)
5. [Bias-Erkennung (Greyface Alert)](#5-bias-erkennung-greyface-alert)
6. [Embeddings und Vektorsuche](#6-embeddings-und-vektorsuche)
7. [Batch-Verarbeitung (Fnord Processing)](#7-batch-verarbeitung-fnord-processing)
8. [Volltext-Abruf (Hagbard's Retrieval)](#8-volltext-abruf-hagbards-retrieval)
9. [Sync-Verhalten](#9-sync-verhalten)
10. [Datenbank-Schema](#10-datenbank-schema)
11. [Prompt-Design](#11-prompt-design)
12. [UI-Feedback](#12-ui-feedback)
13. [Sortierung und Filterung](#13-sortierung-und-filterung)
14. [Import/Export](#14-importexport)
15. [Keyboard-Shortcuts](#15-keyboard-shortcuts)
16. [Internationalisierung (i18n)](#16-internationalisierung-i18n)
17. [Benutzereinstellungen](#17-benutzereinstellungen)
18. [Plattform-spezifische Details](#18-plattform-spezifische-details)
19. [Zusammenfassung der Entscheidungen](#19-zusammenfassung-der-entscheidungen)
20. [Nächste Schritte](#20-nächste-schritte)
21. [Anhang](#anhang)

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
| Ungelesene Artikel | **Fnords** | Unsichtbar bis man sie sieht |
| Gelesene Artikel | **Illuminated** | Erleuchtete/verarbeitete Artikel |
| Favoriten/Wichtig | **Golden Apple** 🍎 | Markierte Artikel |
| KI-Zusammenfassung | **Discordian Analysis** | Automatische Analyse |
| Bias-Warnung | **Greyface Alert** | Hinweis auf einseitige Berichterstattung |
| Interessenprofil | **Operation Mindfuck (OM)** | User-Präferenzen |
| Kategorien | **Sephiroth** | Thematische Einordnung |
| Stichworte/Tags | **Immanentize** | Extrahierte Keywords |
| Feed-Quellen | **Pentacles** | Abonnierte RSS/Atom-Feeds |
| Volltextabruf | **Hagbard's Retrieval** | Nachladen gekürzter Artikel |
| Batch-Verarbeitung | **Fnord Processing** | Hintergrund-KI-Pipeline |
| Embedding-Erstellung | **Immanentizing** | Vektor-Generierung |

---

## 3. Technologie-Stack

### 3.1 Architektur

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri Frontend                       │
│                       (Svelte)                          │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │ Feed-Liste  │  │ Artikel-    │  │ Fnord-      │     │
│  │ (Pentacles) │  │ Ansicht     │  │ Processing  │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
│                                                         │
└────────────────────────┬────────────────────────────────┘
                         │ Tauri Commands (IPC)
                         ▼
┌─────────────────────────────────────────────────────────┐
│                    Rust Backend                         │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │ Feed-Parser │  │ Ollama-     │  │ Hagbard's   │     │
│  │ (feed-rs)   │  │ Client      │  │ Retrieval   │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │              SQLite + SQLite-VSS                │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Komponenten

| Komponente | Technologie | Begründung |
|------------|-------------|------------|
| GUI-Framework | Tauri 2.x | Ressourcenschonend, Cross-Platform |
| Backend | Rust | Performance, Typsicherheit |
| Frontend | Svelte | Leichtgewichtig, reaktiv, wenig Boilerplate |
| Datenbank | SQLite + SQLite-VSS | Alles in einer Datei, Vektorsuche |
| KI-Backend | Ollama (lokal) | Keine Cloud-Abhängigkeit |

### 3.3 Rust Crates

| Zweck | Crate |
|-------|-------|
| RSS/Atom Parsing | `feed-rs` |
| HTTP Client | `reqwest` |
| Async Runtime | `tokio` |
| SQLite | `rusqlite` |
| Vektor-Suche | `sqlite-vss` (C-Binding) |
| HTML Parsing | `scraper` |
| Readability | `readability` |
| Ollama API | `ollama-rs` |
| OPML Parsing | `opml` |
| Serialisierung | `serde`, `serde_json` |

### 3.4 Warum Tauri?

1. **Ressourcen-schonend**
   - Binary: ~5-10 MB (vs. Electron: 150-200 MB)
   - RAM: ~30-50 MB (vs. Electron: 200-500 MB)
   - Wichtig, weil KI-Modelle bereits viel Speicher brauchen

2. **Cross-Platform ohne Aufwand**
   - Linux: WebKitGTK
   - macOS: WKWebView (nativ)
   - Nutzt native WebView-Komponenten

3. **Rust-Backend**
   - Perfekt für SQLite + SQLite-VSS Integration
   - Async-Runtime (tokio) für Ollama-API-Calls
   - Typsicherheit für komplexe Datenstrukturen

### 3.5 Warum Svelte?

1. **Kompiliert zu Vanilla JS** – kein Runtime-Overhead
2. **Reaktives Modell** – einfaches State-Management
3. **Wenig Boilerplate** – schnelle Entwicklung
4. **Gute Tauri-Integration** – offizielles Template verfügbar

---

## 4. KI-Integration

### 4.1 Modell-Auswahl

| Modell | Größe | Zweck |
|--------|-------|-------|
| `qwen3-vl:8b` | 6.1 GB | Hauptmodell für alle Tasks |
| `nomic-embed-text` | 274 MB | Embeddings für Vektorsuche |

**Gesamter VRAM-Bedarf:** ~6.4 GB + Overhead = ~9-10 GB (passt in 12 GB)

Beide Modelle können **gleichzeitig** im VRAM geladen bleiben.

### 4.2 Warum qwen3-vl:8b?

- **Vision + Language:** Kann auch Bilder in Artikeln analysieren
- **Text-Performance:** Erreicht die Qualität des Flaggschiff-Modells (Qwen3-235B) bei reinen Text-Tasks
- **Universell einsetzbar:** Ein Modell für Zusammenfassung, Kategorisierung, Bias-Erkennung
- **256K Kontext:** Kann sehr lange Artikel verarbeiten

### 4.3 Ollama-Konfiguration

Beide Modelle gleichzeitig laden:

```bash
# /etc/systemd/system/ollama.service.d/override.conf
[Service]
Environment="OLLAMA_MAX_LOADED_MODELS=2"
Environment="OLLAMA_FLASH_ATTENTION=1"
Environment="OLLAMA_KV_CACHE_TYPE=q8_0"
```

Nach Änderung:

```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

### 4.4 Alternative Modelle

Falls qwen3-vl bei bestimmten Tasks schwächelt:

| Modell | Stärke | VRAM |
|--------|--------|------|
| `phi4` | Analytische Aufgaben | 9.1 GB |
| `deepseek-r1:14b` | Reasoning | 9.0 GB |
| `qwen3:8b` | Schneller (nur Text) | 5.2 GB |

### 4.5 KI-Features Übersicht

| Feature | Modell | Beschreibung |
|---------|--------|--------------|
| Zusammenfassung | qwen3-vl | 2-3 Sätze pro Artikel |
| Kategorisierung | qwen3-vl | Sephiroth zuweisen |
| Stichworte | qwen3-vl | Immanentize-Tags extrahieren |
| Bias-Erkennung | qwen3-vl | Greyface Alert |
| Bild-Analyse | qwen3-vl | Optional bei relevanten Bildern |
| Embeddings | nomic-embed-text | Vektoren für Ähnlichkeitssuche |
| Ähnliche Artikel | nomic-embed-text | Basierend auf Vektor-Distanz |
| Semantische Suche | nomic-embed-text | Bedeutungsbasierte Suche |

---

## 5. Bias-Erkennung (Greyface Alert)

### 5.1 Übersicht

```
┌─────────────────────────────────────────────────────────────┐
│                     GREYFACE ALERT                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Politische Tendenz        Sachlichkeit                    │
│  ◀━━━━━━━━●━━━━━━━━▶       ◀━━━━━━━━━━●━━▶                 │
│  Links    Mitte   Rechts   Emotional   Sachlich            │
│                                                             │
│  Quellenqualität           Kategorie                       │
│  ★★★★☆                     📰 Nachricht                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 Dimension 1: Politische Tendenz

| Wert | Bedeutung | Beispiel-Indikatoren |
|------|-----------|----------------------|
| -2 | Stark links | Kapitalismuskritik, Klassenkampf-Rhetorik |
| -1 | Leicht links | Soziale Gerechtigkeit, Umverteilung positiv |
| 0 | Neutral/Mitte | Ausgewogene Darstellung, multiple Perspektiven |
| +1 | Leicht rechts | Marktliberalismus, Traditionswerte positiv |
| +2 | Stark rechts | Nationalismus, Anti-Establishment |

**Datentyp:** INTEGER (-2 bis +2)  
**UI-Darstellung:** Slider oder farbige Skala

### 5.3 Dimension 2: Sachlichkeit

| Wert | Bedeutung | Indikatoren |
|------|-----------|-------------|
| 0 | Stark emotional | Superlative, Ausrufezeichen, Clickbait, Angstmache |
| 1 | Emotional | Wertende Adjektive, einseitige Wortwahl |
| 2 | Gemischt | Fakten mit Meinung vermischt |
| 3 | Überwiegend sachlich | Faktenbasiert mit leichter Färbung |
| 4 | Sachlich | Neutrale Sprache, Quellenangaben, Fakten |

**Datentyp:** INTEGER (0 bis 4)  
**UI-Darstellung:** 5-Stufen-Anzeige oder Prozent (0-100%)

### 5.4 Dimension 3: Quellenqualität

| Sterne | Bedeutung | Kriterien |
|--------|-----------|-----------|
| ★☆☆☆☆ | Fragwürdig | Keine Quellenangaben, bekannte Desinformation |
| ★★☆☆☆ | Schwach | Wenig Belege, stark meinungsgetrieben |
| ★★★☆☆ | Mittel | Einige Quellen, erkennbare Perspektive |
| ★★★★☆ | Gut | Solide Recherche, transparente Methodik |
| ★★★★★ | Exzellent | Primärquellen, Peer-Review, etablierte Redaktion |

**Datentyp:** INTEGER (1 bis 5)  
**Berechnung:** Kombination aus Feed-Basis-Wert + Artikel-Modifikatoren

**Berechnungslogik:**

```rust
fn calculate_quality(pentacle: &Pentacle, fnord: &Fnord) -> i32 {
    let mut score = pentacle.default_quality as f32;
    
    // Positive Modifikatoren
    if fnord.has_sources { score += 1.0; }
    if fnord.author.is_some() { score += 0.5; }
    if fnord.sachlichkeit >= 3 { score += 0.5; }
    
    // Negative Modifikatoren
    if fnord.is_clickbait { score -= 1.0; }
    if fnord.sachlichkeit <= 1 { score -= 0.5; }
    
    score.clamp(1.0, 5.0).round() as i32
}
```

### 5.5 Dimension 4: Artikel-Kategorie

| Kategorie | Icon | DB-Wert | Beschreibung |
|-----------|------|---------|--------------|
| Nachricht | 📰 | `news` | Faktenbericht, 5 W-Fragen |
| Analyse | 🔍 | `analysis` | Einordnung mit Hintergrund |
| Meinung | 💭 | `opinion` | Kommentar, Editorial, Kolumne |
| Satire | 🎭 | `satire` | Satirischer Inhalt |
| Werbung | 📢 | `ad` | Sponsored Content, PR |
| Unbekannt | ❓ | `unknown` | Nicht einordbar |

**Datentyp:** TEXT (enum)  
**Ermittlung:** Durch KI (qwen3-vl)

---

## 6. Embeddings und Vektorsuche

### 6.1 Was sind Embeddings?

Ein Embedding ist eine numerische Repräsentation von Text – ein Vektor aus 768 Zahlen. Ähnliche Texte haben ähnliche Vektoren.

```
"RSS-Reader für Linux"     → [0.23, -0.45, 0.87, ..., 0.12]
"Feed-Aggregator Ubuntu"   → [0.21, -0.43, 0.85, ..., 0.14]  ← sehr ähnlich!
"Kochrezept für Pasta"     → [-0.67, 0.12, -0.34, ..., 0.89] ← ganz anders
```

### 6.2 Anwendungsfälle in fuckupRSS

| Anwendung | Beschreibung |
|-----------|--------------|
| **Ähnliche Artikel** | Beim Lesen werden verwandte Artikel vorgeschlagen |
| **Semantische Suche** | "Datenschutz" findet auch "DSGVO", "Privacy", "GDPR" |
| **Interessen-Clustering** | Gelesene Artikel gruppieren sich automatisch |
| **Relevanz-Scoring** | Artikel nach Übereinstimmung mit Interessen bewerten |

### 6.3 nomic-embed-text

- **Größe:** 274 MB
- **Dimension:** 768
- **Stärke:** Schnell, hochwertige Vektoren
- **Einsatz:** Kann parallel zu qwen3-vl geladen bleiben

### 6.4 SQLite-VSS

SQLite-VSS ist eine Erweiterung für SQLite, die Vektorsuche ermöglicht:

```sql
-- Virtuelle Tabelle erstellen
CREATE VIRTUAL TABLE fnords_vss USING vss0(
    embedding(768)
);

-- Vektor speichern
INSERT INTO fnords_vss (rowid, embedding) 
VALUES (123, ?);  -- ? = 768-dim Float-Array

-- Ähnliche Artikel finden
SELECT rowid, distance 
FROM fnords_vss 
WHERE vss_search(embedding, ?)  -- ? = Query-Vektor
LIMIT 10;
```

### 6.5 Ablauf: Embedding-Erstellung

```
1. Artikel-Text (Titel + Content)
        │
        ▼
2. An nomic-embed-text senden
        │
        ▼
3. 768-dimensionaler Vektor zurück
   [0.23, -0.45, 0.87, ..., 0.12]
        │
        ▼
4. In fnords_vss speichern
   INSERT INTO fnords_vss (rowid, embedding) VALUES (artikel_id, vektor)
        │
        ▼
5. Später: Ähnlichkeitssuche
   SELECT * FROM fnords_vss WHERE vss_search(embedding, query_vektor) LIMIT 10
```

---

## 7. Batch-Verarbeitung (Fnord Processing)

### 7.1 Pipeline-Ablauf

```
┌─────────────────────────────────────────────────────────────────┐
│                    FEED SYNC (alle X Minuten)                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 1: Hagbard's Retrieval                                    │
│ ─────────────────────────────                                   │
│ • Neue Artikel aus Feeds holen                                  │
│ • Gekürzte Feeds: Volltext nachladen (Readability)              │
│ • Raw-Artikel in DB speichern (Status: "fnord")                 │
│                                                                 │
│ [████████████████░░░░] 15 von 23 Artikel geladen                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 2: Immanentizing (nomic-embed-text)                       │
│ ─────────────────────────────────────────                       │
│ • Embeddings für alle neuen Artikel erzeugen                    │
│ • Läuft parallel (eigenes Modell im VRAM)                       │
│ • Vektoren in SQLite-VSS speichern                              │
│                                                                 │
│ [██████████░░░░░░░░░░] 10 von 23 Embeddings                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 3: Discordian Analysis (qwen3-vl:8b)                      │
│ ─────────────────────────────────────────                       │
│ Für jeden Artikel:                                              │
│ • Zusammenfassung (2-3 Sätze)                                   │
│ • Kategorien (Sephiroth) extrahieren                            │
│ • Stichworte (Immanentize-Tags) extrahieren                     │
│ • Bias-Einschätzung (political_bias, sachlichkeit, article_type)│
│ • Optional: Bild-Analyse wenn relevant                          │
│                                                                 │
│ [████░░░░░░░░░░░░░░░░] 4 von 23 analysiert                      │
│ ⏱ ~8 Sek/Artikel │ ETA: 2:32 min                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 4: Finalisierung                                          │
│ ────────────────────                                            │
│ • Quellenqualität berechnen                                     │
│ • Relevanz-Score basierend auf User-Interessen                  │
│ • Status: "fnord" bleibt (= ungelesen)                          │
│ • processed_at Timestamp setzen                                 │
│ • UI benachrichtigen: "23 neue Artikel bereit"                  │
└─────────────────────────────────────────────────────────────────┘
```

### 7.2 Trigger-Strategie

| Trigger | Verhalten |
|---------|-----------|
| **App-Start** | Sync starten wenn letzter Sync > 5 min |
| **Timer** | Alle 30 min (konfigurierbar) |
| **Manuell** | Button in der UI |
| **Abbruch** | Jederzeit möglich, Fortschritt bleibt erhalten |

### 7.3 Parallelisierung

```
┌─────────────────┐     ┌─────────────────┐
│ nomic-embed-text│     │   qwen3-vl:8b   │
│   (274 MB)      │     │    (6.1 GB)     │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
    Embeddings              Analyse
    (schnell)              (langsamer)
         │                       │
         └───────────┬───────────┘
                     ▼
               SQLite speichern
```

Phase 2 (Embeddings) und Phase 3 (Analyse) können **parallel** laufen, da beide Modelle im VRAM bleiben.

---

## 8. Volltext-Abruf (Hagbard's Retrieval)

### 8.1 Problem

Viele RSS-Feeds liefern nur gekürzte Artikel (Teaser). Das ist unerwünscht.

### 8.2 Entscheidungslogik

```
Neuer Artikel
    │
    ├─► Feed als "is_truncated" markiert?
    │       │
    │       ├─► Ja ──► Volltext abrufen
    │       │
    │       └─► Nein ──► Content-Länge prüfen
    │                       │
    │                       ├─► < 500 Zeichen ──► Volltext abrufen
    │                       │
    │                       └─► ≥ 500 Zeichen ──► Original behalten
    │
    └─► Volltext-Abruf
            │
            ├─► Readability erfolgreich? ──► content_full speichern
            │
            └─► Fehlgeschlagen ──► content_raw behalten, Flag setzen
```

### 8.3 Technische Umsetzung

```rust
async fn hagbards_retrieval(fnord: &mut Fnord) -> Result<()> {
    // 1. HTTP Request mit Timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let html = client.get(&fnord.url)
        .header("User-Agent", "fuckupRSS/0.1")
        .send()
        .await?
        .text()
        .await?;
    
    // 2. Readability extrahieren
    match readability::extract(&html, &fnord.url) {
        Ok(readable) => {
            fnord.content_full = Some(readable.content);
            fnord.full_text_fetched = true;
            
            // Hauptbild extrahieren falls nicht vorhanden
            if fnord.image_url.is_none() {
                fnord.image_url = readable.lead_image_url;
            }
        }
        Err(_) => {
            // Fallback: HTML als content_full speichern
            fnord.content_full = Some(html);
            fnord.full_text_fetched = false;
        }
    }
    
    Ok(())
}
```

### 8.4 Fehlerbehandlung

| Fehler | Aktion |
|--------|--------|
| Timeout (30s) | Original behalten, retry_count erhöhen |
| HTTP 404 | Original behalten, nicht erneut versuchen |
| HTTP 403/401 | Paywall markieren, Original behalten |
| Readability fehlgeschlagen | Raw HTML speichern |
| Netzwerkfehler | Retry bei nächstem Sync |

### 8.5 Bilder

| Strategie | Beschreibung |
|-----------|--------------|
| **Nur URLs** (Standard) | Bilder werden bei Bedarf geladen |
| **Thumbnail-Cache** (Optional) | Komprimierte Vorschau lokal speichern |
| **Größenlimit** | Cache max. 500 MB |

---

## 9. Sync-Verhalten

### 9.1 Standard-Einstellungen

| Einstellung | Default | Konfigurierbar |
|-------------|---------|----------------|
| Globales Intervall | 30 Minuten | Ja |
| Pro-Feed-Intervall | Überschreibt global | Ja |
| Sync bei App-Start | Ja (wenn > 5 min seit letztem Sync) | Ja |
| Hintergrund-Sync | Nein (nur bei offener App) | Ja |
| Parallele Feeds | 5 | Ja |
| Timeout pro Feed | 30 Sekunden | Ja |
| Max. Artikel pro Feed | 100 | Ja |

### 9.2 Sync-Ablauf

```
App-Start
    │
    ├─► Letzter Sync > 5 min?
    │       │
    │       ├─► Ja ──► Fnord Processing im Hintergrund starten
    │       │
    │       └─► Nein ──► Nur UI laden
    │
    └─► UI sofort nutzbar mit gecachten Daten


Timer (alle 30 min bei offener App)
    │
    └─► Feeds prüfen ──► Neue Artikel? ──► Fnord Processing
```

### 9.3 Feed-spezifische Intervalle

Einige Feeds brauchen häufigere Updates als andere:

```sql
-- News-Seiten: häufiger
UPDATE pentacles SET sync_interval = 900 
WHERE url LIKE '%heise.de%' OR url LIKE '%spiegel.de%';  -- 15 min

-- Blogs: seltener
UPDATE pentacles SET sync_interval = 7200 
WHERE url LIKE '%blog%';  -- 2 Stunden

-- Podcasts: sehr selten
UPDATE pentacles SET sync_interval = 86400 
WHERE title LIKE '%Podcast%';  -- 1 Tag
```

---

## 10. Datenbank-Schema

### 10.1 Übersicht

```
┌─────────────────┐     ┌─────────────────┐
│    pentacles    │────<│     fnords      │
│  (Feed-Quellen) │     │    (Artikel)    │
└─────────────────┘     └─────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
      ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
      │  sephiroth  │  │ immanentize │  │ fnords_vss  │
      │ (Kategorien)│  │ (Stichworte)│  │  (Vektoren) │
      └─────────────┘  └─────────────┘  └─────────────┘
              ▲                ▲
              │                │
      ┌───────┴────────────────┴───────┐
      │       operation_mindfuck       │
      │       (User-Interessen)        │
      └────────────────────────────────┘
```

### 10.2 Tabellen-Definitionen

```sql
-- ============================================================
-- PENTACLES (Feed-Quellen)
-- ============================================================
CREATE TABLE pentacles (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL UNIQUE,
    title TEXT,
    description TEXT,
    site_url TEXT,
    icon_url TEXT,
    
    -- Sync-Einstellungen
    last_sync DATETIME,
    sync_interval INTEGER DEFAULT 1800,  -- Sekunden (30 min)
    is_truncated BOOLEAN DEFAULT FALSE,  -- Liefert gekürzte Artikel
    
    -- Qualitätsbewertung
    default_quality INTEGER DEFAULT 3 CHECK(default_quality BETWEEN 1 AND 5),
    
    -- Metadaten
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- Statistik
    article_count INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    last_error TEXT
);

-- ============================================================
-- FNORDS (Artikel)
-- ============================================================
CREATE TABLE fnords (
    id INTEGER PRIMARY KEY,
    pentacle_id INTEGER NOT NULL,
    
    -- Identifikation
    guid TEXT NOT NULL,
    url TEXT NOT NULL,
    
    -- Inhalt
    title TEXT NOT NULL,
    author TEXT,
    content_raw TEXT,           -- Original-Feed-Inhalt
    content_full TEXT,          -- Volltext (Hagbard's Retrieval)
    summary TEXT,               -- KI-Zusammenfassung
    image_url TEXT,
    
    -- Zeitstempel
    published_at DATETIME,
    fetched_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    processed_at DATETIME,      -- Wann KI-Analyse abgeschlossen
    read_at DATETIME,           -- Wann gelesen
    
    -- Status
    status TEXT DEFAULT 'fnord' CHECK(status IN ('fnord', 'illuminated', 'golden_apple')),
    full_text_fetched BOOLEAN DEFAULT FALSE,
    
    -- Greyface Alert
    political_bias INTEGER CHECK(political_bias BETWEEN -2 AND 2),
    sachlichkeit INTEGER CHECK(sachlichkeit BETWEEN 0 AND 4),
    quality_score INTEGER CHECK(quality_score BETWEEN 1 AND 5),
    article_type TEXT CHECK(article_type IN ('news', 'analysis', 'opinion', 'satire', 'ad', 'unknown')),
    
    -- Relevanz
    relevance_score REAL DEFAULT 0.0,  -- Basierend auf User-Interessen (0.0 - 1.0)
    
    -- Constraints
    FOREIGN KEY (pentacle_id) REFERENCES pentacles(id) ON DELETE CASCADE,
    UNIQUE(pentacle_id, guid)
);

-- ============================================================
-- SEPHIROTH (Kategorien)
-- ============================================================
CREATE TABLE sephiroth (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    color TEXT,                 -- Hex-Farbe für UI (#3B82F6)
    icon TEXT,                  -- Emoji oder Icon-Name
    article_count INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- IMMANENTIZE (Stichworte/Tags)
-- ============================================================
CREATE TABLE immanentize (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    count INTEGER DEFAULT 1,    -- Wie oft verwendet
    last_used DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- FNORD ↔ SEPHIROTH (Artikel-Kategorien-Zuordnung)
-- ============================================================
CREATE TABLE fnord_sephiroth (
    fnord_id INTEGER NOT NULL,
    sephiroth_id INTEGER NOT NULL,
    confidence REAL DEFAULT 1.0,  -- KI-Konfidenz 0.0 - 1.0
    PRIMARY KEY (fnord_id, sephiroth_id),
    FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE,
    FOREIGN KEY (sephiroth_id) REFERENCES sephiroth(id) ON DELETE CASCADE
);

-- ============================================================
-- FNORD ↔ IMMANENTIZE (Artikel-Stichworte-Zuordnung)
-- ============================================================
CREATE TABLE fnord_immanentize (
    fnord_id INTEGER NOT NULL,
    immanentize_id INTEGER NOT NULL,
    PRIMARY KEY (fnord_id, immanentize_id),
    FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE,
    FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE
);

-- ============================================================
-- OPERATION MINDFUCK (User-Interessen)
-- ============================================================
CREATE TABLE operation_mindfuck (
    id INTEGER PRIMARY KEY,
    
    -- Typ des Interesses
    type TEXT NOT NULL CHECK(type IN ('sephiroth', 'immanentize', 'pentacle', 'custom')),
    
    -- Referenz (je nach Typ)
    reference_id INTEGER,       -- ID der Kategorie/Stichwort/Feed
    custom_term TEXT,           -- Für freie Begriffe
    
    -- Gewichtung
    weight REAL DEFAULT 1.0 CHECK(weight BETWEEN -1.0 AND 2.0),
    -- -1.0 = aktiv nicht interessiert
    --  0.0 = neutral
    --  1.0 = interessiert (Standard)
    --  2.0 = sehr interessiert
    
    -- Quelle
    source TEXT DEFAULT 'manual' CHECK(source IN ('manual', 'learned')),
    
    -- Metadaten
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- READING HISTORY (Lesehistorie für Lernfunktion)
-- ============================================================
CREATE TABLE reading_history (
    id INTEGER PRIMARY KEY,
    fnord_id INTEGER NOT NULL,
    read_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    reading_time INTEGER,       -- Sekunden (wie lange gelesen)
    scrolled_percent REAL,      -- Wie weit gescrollt 0-100
    FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE
);

-- ============================================================
-- FNORDS_VSS (Vektor-Tabelle für Ähnlichkeitssuche)
-- ============================================================
CREATE VIRTUAL TABLE fnords_vss USING vss0(
    embedding(768)  -- nomic-embed-text Dimension
);

-- ============================================================
-- INDIZES
-- ============================================================
CREATE INDEX idx_fnords_status ON fnords(status);
CREATE INDEX idx_fnords_published ON fnords(published_at DESC);
CREATE INDEX idx_fnords_pentacle ON fnords(pentacle_id);
CREATE INDEX idx_fnords_processed ON fnords(processed_at);
CREATE INDEX idx_fnords_relevance ON fnords(relevance_score DESC);
CREATE INDEX idx_pentacles_last_sync ON pentacles(last_sync);
CREATE INDEX idx_immanentize_count ON immanentize(count DESC);
CREATE INDEX idx_reading_history_fnord ON reading_history(fnord_id);
```

### 10.3 Standard-Kategorien (Sephiroth)

```sql
INSERT INTO sephiroth (name, icon, color) VALUES
    ('Tech', '💻', '#3B82F6'),
    ('Politik', '🏛️', '#EF4444'),
    ('Wirtschaft', '📈', '#10B981'),
    ('Wissenschaft', '🔬', '#8B5CF6'),
    ('Kultur', '🎭', '#F59E0B'),
    ('Sport', '⚽', '#06B6D4'),
    ('Gesellschaft', '👥', '#EC4899'),
    ('Umwelt', '🌍', '#22C55E'),
    ('Sicherheit', '🔒', '#6366F1'),
    ('Gesundheit', '🏥', '#F43F5E');
```

---

## 11. Prompt-Design

### 11.1 Haupt-Analyse-Prompt

Ein einzelner Prompt für alle Text-Tasks:

```
Du bist ein Nachrichtenanalyst. Analysiere den folgenden Artikel und antworte NUR mit validem JSON.

ARTIKEL:
Titel: {title}
Quelle: {source}
Inhalt: {content}

Antworte mit diesem JSON-Format:
{
  "summary": "2-3 Sätze Zusammenfassung auf Deutsch",
  "categories": ["Kategorie1", "Kategorie2"],
  "keywords": ["Stichwort1", "Stichwort2", "Stichwort3"],
  "greyface": {
    "political_bias": 0,
    "sachlichkeit": 3,
    "article_type": "news"
  }
}

REGELN:
- summary: Deutsch, neutral, nur Kernaussagen, 2-3 Sätze
- categories: Maximal 3, NUR aus dieser Liste wählen:
  Tech, Politik, Wirtschaft, Wissenschaft, Kultur, Sport, Gesellschaft, Umwelt, Sicherheit, Gesundheit
- keywords: 3-7 spezifische Begriffe (Personen, Orte, Konzepte, Organisationen)
- political_bias: -2 (stark links) bis +2 (stark rechts), 0 = neutral/ausgewogen
- sachlichkeit: 0 (stark emotional) bis 4 (sachlich/faktenbasiert)
- article_type: news | analysis | opinion | satire | ad | unknown

Antworte NUR mit dem JSON, keine Erklärungen, kein Markdown.
```

### 11.2 Beispiel-Input

```
ARTIKEL:
Titel: EU verabschiedet AI Act – Strengere Regeln für KI-Systeme
Quelle: heise.de
Inhalt: Die Europäische Union hat heute den AI Act final verabschiedet. 
Das Gesetz sieht vor, dass Hochrisiko-KI-Anwendungen künftig strenge 
Auflagen erfüllen müssen. Verboten werden unter anderem Social-Scoring-Systeme 
und biometrische Echtzeit-Überwachung. Die Verordnung tritt 2025 in Kraft.
Kritiker bemängeln, dass die Regeln Innovation bremsen könnten.
```

### 11.3 Beispiel-Output

```json
{
  "summary": "Die EU hat den AI Act verabschiedet, der strenge Auflagen für Hochrisiko-KI vorsieht. Social Scoring und biometrische Echtzeit-Überwachung werden verboten. Das Gesetz tritt 2025 in Kraft.",
  "categories": ["Tech", "Politik"],
  "keywords": ["EU", "AI Act", "KI-Regulierung", "Hochrisiko-KI", "Social Scoring", "Biometrie"],
  "greyface": {
    "political_bias": 0,
    "sachlichkeit": 4,
    "article_type": "news"
  }
}
```

### 11.4 Parsing im Rust-Backend

```rust
#[derive(Deserialize)]
struct DiscordianAnalysis {
    summary: String,
    categories: Vec<String>,
    keywords: Vec<String>,
    greyface: GreyfaceAlert,
}

#[derive(Deserialize)]
struct GreyfaceAlert {
    political_bias: i8,
    sachlichkeit: u8,
    article_type: String,
}

async fn analyze_article(fnord: &Fnord) -> Result<DiscordianAnalysis> {
    let prompt = format!(
        r#"Du bist ein Nachrichtenanalyst...
        
        ARTIKEL:
        Titel: {}
        Quelle: {}
        Inhalt: {}
        "#,
        fnord.title,
        fnord.source_name,
        fnord.content_full.as_ref().unwrap_or(&fnord.content_raw)
    );
    
    let response = ollama.generate("qwen3-vl:8b", &prompt).await?;
    let analysis: DiscordianAnalysis = serde_json::from_str(&response)?;
    
    Ok(analysis)
}
```

---

## 12. UI-Feedback

### 12.1 Kombinierter Ansatz

Kompakte Statusleiste + expandierbares Detail-Panel:

```
┌────────────────────────────────────────────────────────────────┐
│ FEED-LISTE        │ ARTIKEL-ANSICHT                           │
│                   │                                            │
│ ▼ Tech            │  ┌──────────────────────────────────────┐ │
│   • Heise (12)    │  │ EU verabschiedet AI Act              │ │
│   • Golem (5)     │  │                                      │ │
│                   │  │ Die Europäische Union hat heute...   │ │
│ ▼ Politik         │  │                                      │ │
│   • Tagesschau    │  │ ┌────────────────────────────────┐  │ │
│                   │  │ │ GREYFACE ALERT                 │  │ │
│ ─────────────────│  │ │ Tendenz: ━━━●━━ Neutral        │  │ │
│ 📊 Statistik      │  │ │ Sachlich: ★★★★☆               │  │ │
│   47 Fnords       │  │ │ Typ: 📰 Nachricht              │  │ │
│   23 Illuminated  │  │ └────────────────────────────────┘  │ │
│   3 Golden Apple  │  │                                      │ │
├───────────────────┴──┴──────────────────────────────────────┴─┤
│ 🔄 Discordian Analysis: 12/47 │ ████████░░░░░░ │ ~3:24 │ [▼]  │
└───────────────────────────────────────────────────────────────┘
```

### 12.2 Expandiertes Detail-Panel

Klick auf [▼] öffnet:

```
┌─────────────────────────────────────────────────────────────┐
│ ⚙️ Fnord Processing                                      ✕ │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ ✓ Hagbard's Retrieval        47/47 Artikel                 │
│ ✓ Immanentizing              47/47 Vektoren                │
│ ◐ Discordian Analysis        12/47 analysiert              │
│   └─ heise.de: "EU verabschiedet AI Act..."                │
│                                                             │
│ ┌─────────────────────────────────────────────────────┐    │
│ │████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░│ 25%         │    │
│ └─────────────────────────────────────────────────────┘    │
│                                                             │
│ Geschwindigkeit: ~8 Sek/Artikel                            │
│ Verbleibend: ~3:24                                          │
│                                                             │
│ [⏸ Pausieren]                              [✕ Abbrechen]   │
└─────────────────────────────────────────────────────────────┘
```

### 12.3 Desktop-Notifications

| Ereignis | Notification |
|----------|--------------|
| Processing gestartet | Keine (nicht nerven) |
| Processing fertig | "✨ 47 Fnords wurden illuminiert" |
| Fehler aufgetreten | "⚠️ 3 Artikel konnten nicht verarbeitet werden" |
| Neue Artikel (Hintergrund) | "📰 12 neue Artikel verfügbar" |

### 12.4 Status-Icons

| Icon | Bedeutung |
|------|-----------|
| 🔄 | Verarbeitung läuft |
| ⏸ | Pausiert |
| ✓ | Phase abgeschlossen |
| ◐ | Phase läuft |
| ⚠️ | Fehler aufgetreten |
| ✨ | Alles fertig |

---

## 13. Sortierung und Filterung

### 13.1 Sortieroptionen

| Sortierung | Beschreibung | SQL |
|------------|--------------|-----|
| Neueste zuerst | Nach Veröffentlichungsdatum | `ORDER BY published_at DESC` |
| Älteste zuerst | Umgekehrt chronologisch | `ORDER BY published_at ASC` |
| Relevanz | Nach User-Interessen | `ORDER BY relevance_score DESC` |
| Quelle A-Z | Alphabetisch nach Feed | `ORDER BY pentacle_title ASC` |
| Sachlichkeit | Sachlichste zuerst | `ORDER BY sachlichkeit DESC` |

### 13.2 Filteroptionen

| Filter | Werte | Mehrfachauswahl |
|--------|-------|-----------------|
| Status | Fnord, Illuminated, Golden Apple | Ja |
| Zeitraum | Heute, Diese Woche, Dieser Monat, Benutzerdefiniert | Nein |
| Quelle (Pentacle) | Alle abonnierten Feeds | Ja |
| Kategorie (Sephiroth) | Alle verfügbaren Kategorien | Ja |
| Stichworte (Immanentize) | Alle extrahierten Tags | Ja |
| Artikeltyp | news, analysis, opinion, etc. | Ja |
| Politische Tendenz | Links, Mitte, Rechts (Slider) | Nein |
| Sachlichkeit | Min-Wert (0-4) | Nein |
| Quellenqualität | Min-Sterne (1-5) | Nein |

### 13.3 Suche

| Suchmodus | Beschreibung |
|-----------|--------------|
| **Volltext** | Klassische Keyword-Suche in Titel + Content |
| **Semantisch** | Via Embeddings, findet auch verwandte Begriffe |
| **Kombiniert** | Volltext + Semantisch, beste Ergebnisse |

---

## 14. Import/Export

### 14.1 OPML-Import

Import bestehender Feed-Abonnements aus anderen Readern:

```rust
async fn import_opml(path: &Path) -> Result<Vec<Pentacle>> {
    let content = fs::read_to_string(path)?;
    let document = opml::parse(&content)?;
    
    let mut pentacles = Vec::new();
    
    for outline in document.body.outlines {
        if let Some(xml_url) = outline.xml_url {
            pentacles.push(Pentacle {
                url: xml_url,
                title: outline.title.or(outline.text),
                ..Default::default()
            });
        }
    }
    
    Ok(pentacles)
}
```

### 14.2 OPML-Export

```rust
fn export_opml(pentacles: &[Pentacle]) -> String {
    // OPML 2.0 Format generieren
}
```

### 14.3 Artikel-Export

| Format | Zweck | Inhalt |
|--------|-------|--------|
| **Markdown** | Einzelner Artikel | Titel, Content, Metadaten |
| **HTML** | Mit Formatierung | Vollständiger Artikel |
| **JSON** | Mit allen Metadaten | Artikel + KI-Analyse |
| **PDF** | Zum Archivieren | Formatierter Artikel |

---

## 15. Keyboard-Shortcuts

### 15.1 Navigation

| Taste | Aktion |
|-------|--------|
| `j` | Nächster Artikel |
| `k` | Vorheriger Artikel |
| `g g` | Zum Anfang der Liste |
| `G` | Zum Ende der Liste |
| `h` | Vorheriger Feed/Ordner |
| `l` | Nächster Feed/Ordner |

### 15.2 Aktionen

| Taste | Aktion |
|-------|--------|
| `o` / `Enter` | Artikel öffnen/schließen |
| `v` | Im Browser öffnen |
| `r` | Als gelesen markieren |
| `u` | Als ungelesen markieren |
| `s` | Golden Apple (Favorit) toggle |
| `a` | Alle als gelesen markieren |

### 15.3 Suche & Filter

| Taste | Aktion |
|-------|--------|
| `/` | Suche öffnen |
| `Esc` | Suche/Dialog schließen |
| `f` | Filter-Panel toggle |

### 15.4 System

| Taste | Aktion |
|-------|--------|
| `Ctrl+R` | Feeds synchronisieren |
| `Ctrl+,` | Einstellungen öffnen |
| `?` | Shortcut-Hilfe anzeigen |

---

## 16. Internationalisierung (i18n)

### 16.1 Unterstützte Sprachen

| Sprache | Code | Status |
|---------|------|--------|
| Deutsch | `de` | Primär |
| Englisch | `en` | Sekundär |

### 16.2 Technische Umsetzung

**Library:** `svelte-i18n`

```typescript
// src/lib/i18n/index.ts
import { init, register, locale } from 'svelte-i18n';

register('de', () => import('./locales/de.json'));
register('en', () => import('./locales/en.json'));

init({
  fallbackLocale: 'en',
  initialLocale: getLocaleFromNavigator(),
});
```

### 16.3 Übersetzungsstruktur

```json
// src/lib/i18n/locales/de.json
{
  "app": {
    "title": "fuckupRSS",
    "tagline": "Immanentize the Eschaton"
  },
  "terminology": {
    "fnord": {
      "label": "Fnord",
      "tooltip": "Ungelesener Artikel – aus der Illuminatus!-Trilogie"
    },
    "illuminated": {
      "label": "Illuminated",
      "tooltip": "Gelesener Artikel – du hast die Wahrheit gesehen"
    },
    "golden_apple": {
      "label": "Golden Apple",
      "tooltip": "Favorit – markiert mit dem Symbol der Eris"
    },
    "pentacle": {
      "label": "Pentacle",
      "tooltip": "Feed-Quelle – dein Portal zur Information"
    },
    "sephiroth": {
      "label": "Sephiroth",
      "tooltip": "Kategorie – aus der kabbalistischen Tradition"
    },
    "greyface": {
      "label": "Greyface Alert",
      "tooltip": "Bias-Warnung – Hinweis auf einseitige Berichterstattung"
    }
  },
  "settings": {
    "title": "Einstellungen",
    "language": "Sprache",
    "tooltips": {
      "label": "Terminologie-Tooltips",
      "description": "Erklärungen für Illuminatus!-Begriffe anzeigen"
    }
  }
}
```

### 16.4 Tooltips für Terminologie

**Implementierung:**
- Wiederverwendbare `<Tooltip>` Komponente
- Automatische Erkennung von Terminologie-Begriffen
- Hover-Delay: 500ms
- Position: automatisch (oben/unten je nach Platz)

**Einstellbar:**
- Tooltips aktivieren/deaktivieren
- In `config.toml` persistiert

```svelte
<!-- Beispiel Verwendung -->
<Tooltip term="fnord">
  <span class="fnord-indicator">●</span>
</Tooltip>
```

---

## 17. Benutzereinstellungen

### 17.1 Einstellungen-Dialog

Erreichbar über:
- Menü: Einstellungen
- Shortcut: `Ctrl+,`

### 17.2 Verfügbare Einstellungen

| Kategorie | Einstellung | Typ | Default |
|-----------|-------------|-----|---------|
| **Allgemein** | Sprache | Select | System-Sprache |
| **Allgemein** | Theme | Select | dark |
| **Anzeige** | Terminologie-Tooltips | Toggle | true |
| **Anzeige** | Greyface Alert anzeigen | Toggle | true |
| **Anzeige** | Thumbnails anzeigen | Toggle | true |
| **Sync** | Sync-Intervall (Minuten) | Number | 30 |
| **Sync** | Sync bei App-Start | Toggle | true |

### 17.3 Persistenz

Einstellungen werden in `config.toml` gespeichert:

```toml
[general]
language = "de"
theme = "dark"

[display]
show_tooltips = true
show_greyface = true
show_thumbnails = true

[sync]
interval_minutes = 30
sync_on_start = true
```

---

## 18. Plattform-spezifische Details

### 18.1 Datenpfade

| Plattform | Datenverzeichnis |
|-----------|------------------|
| Linux | `~/.local/share/fuckupRSS/` |
| macOS | `~/Library/Application Support/fuckupRSS/` |

Struktur:

```
fuckupRSS/
├── fuckup.db           # SQLite Datenbank
├── fuckup.db-wal       # Write-Ahead Log
├── cache/
│   └── thumbnails/     # Bild-Cache (optional)
├── logs/
│   └── fuckup.log      # Anwendungs-Log
└── config.toml         # Benutzer-Einstellungen
```

### 18.2 Linux-spezifisch

**Abhängigkeiten:**
- WebKitGTK (für Tauri)
- Ollama Service

**Installation Ollama:**

```bash
curl -fsSL https://ollama.com/install.sh | sh
```

**Modelle herunterladen:**

```bash
ollama pull qwen3-vl:8b
ollama pull nomic-embed-text
```

**Ollama-Konfiguration:**

```bash
sudo systemctl edit ollama.service
```

```ini
[Service]
Environment="OLLAMA_MAX_LOADED_MODELS=2"
Environment="OLLAMA_FLASH_ATTENTION=1"
Environment="OLLAMA_KV_CACHE_TYPE=q8_0"
```

```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

### 18.3 macOS-spezifisch

**Installation Ollama:**

```bash
brew install ollama
```

**Ollama-Konfiguration (launchctl):**

```bash
launchctl setenv OLLAMA_MAX_LOADED_MODELS 2
launchctl setenv OLLAMA_FLASH_ATTENTION 1
```

**Besonderheiten:**
- 48 GB Unified Memory = größere Modelle möglich
- Notarization für Distribution außerhalb App Store nötig
- Für eigene Nutzung: Ad-hoc Signierung reicht

---

## 19. Zusammenfassung der Entscheidungen

| Aspekt | Entscheidung |
|--------|--------------|
| **Plattformen** | Linux (primary), macOS (secondary) |
| **GUI-Framework** | Tauri 2.x |
| **Frontend** | Svelte |
| **Backend** | Rust |
| **Datenbank** | SQLite + SQLite-VSS |
| **KI-Backend** | Ollama (lokal) |
| **Hauptmodell** | qwen3-vl:8b (6.1 GB) |
| **Embedding-Modell** | nomic-embed-text (274 MB) |
| **Bias: Politisch** | Skala -2 bis +2 |
| **Bias: Sachlichkeit** | Skala 0-4 |
| **Bias: Quellenqualität** | 1-5 Sterne (berechnet) |
| **Bias: Artikeltyp** | 6 Kategorien (KI-ermittelt) |
| **Sync-Intervall** | 30 min (Standard, konfigurierbar) |
| **Sync bei Start** | Ja (wenn > 5 min seit letztem Sync) |
| **Volltext-Abruf** | Automatisch bei gekürzten Feeds |
| **Bilder** | URLs speichern, optional Thumbnail-Cache |
| **UI-Feedback** | Statusleiste + expandierbares Panel |
| **Notifications** | Bei Abschluss und Fehlern |
| **OPML-Import** | Ja |
| **Keyboard-Shortcuts** | Vim-Style |

---

## 20. Nächste Schritte

### Phase 1: Grundgerüst ✅
- [x] Tauri + Svelte Projekt aufsetzen
- [x] SQLite-Schema implementieren
- [x] Basis-UI (Feed-Liste, Artikel-Ansicht)

### Phase 1.5: Internationalisierung & UX-Grundlagen
- [ ] i18n-System (svelte-i18n) mit Deutsch und Englisch
- [ ] Tooltips für Illuminatus!-Terminologie
- [ ] Einstellungen-Dialog (Sprache, Tooltips ein/aus)
- [ ] Persistente Benutzereinstellungen

### Phase 2: Core-Features
- [ ] Feed-Parsing (feed-rs)
- [ ] Automatische Feed-Synchronisation
- [ ] Hagbard's Retrieval (Volltext)
- [ ] Ollama-Integration (ollama-rs)
- [ ] Basis-KI-Pipeline

### Phase 3: KI-Features
- [ ] Discordian Analysis (Zusammenfassung, Kategorien, Stichworte)
- [ ] Greyface Alert (Bias-Erkennung)
- [ ] Embeddings + sqlite-vec
- [ ] Ähnliche Artikel

### Phase 4: Polish
- [ ] Operation Mindfuck (Interessen-Profil)
- [ ] Relevanz-Scoring
- [ ] OPML Import/Export
- [ ] Erweiterte Keyboard-Shortcuts
- [ ] Desktop-Notifications

### Phase 5: Release
- [ ] Linux-Paketierung (.deb, .rpm, AppImage)
- [ ] macOS-Build + Signierung
- [ ] Dokumentation
- [ ] Release v0.1

---

## Anhang

### A. Verfügbare Ollama-Modelle (auf dem Entwicklungssystem)

```
NAME                       SIZE
qwen3-vl:8b                6.1 GB    ← Hauptmodell
nomic-embed-text:latest    274 MB    ← Embeddings
minicpm-v:8b               5.5 GB
phi4:latest                9.1 GB
qwen3:8b                   5.2 GB
ministral-3:latest         6.0 GB
deepseek-r1:14b            9.0 GB
deepseek-r1:latest         5.2 GB
gemma3:4b                  3.3 GB
ministral-3:14b            9.1 GB
qwen2.5-coder:1.5b         986 MB
qwen2.5-coder:14b          9.0 GB
gemma3:12b                 8.1 GB
qwen3:latest               5.2 GB
```

### B. Beispiel-Konfigurationsdatei

```toml
# ~/.local/share/fuckupRSS/config.toml

[general]
language = "de"
theme = "dark"

[sync]
interval_minutes = 30
sync_on_start = true
parallel_feeds = 5
timeout_seconds = 30

[ollama]
host = "http://localhost:11434"
main_model = "qwen3-vl:8b"
embedding_model = "nomic-embed-text"

[ui]
show_greyface = true
show_thumbnails = true
articles_per_page = 50

[shortcuts]
vim_mode = true

[cache]
thumbnail_enabled = true
thumbnail_max_mb = 500
```

### C. Glossar

| Begriff | Bedeutung |
|---------|-----------|
| Fnord | Ungelesener Artikel |
| Illuminated | Gelesener Artikel |
| Golden Apple | Favorisierter Artikel |
| Pentacle | Feed-Quelle |
| Sephiroth | Kategorie |
| Immanentize | Stichwort/Tag |
| Greyface Alert | Bias-Warnung |
| Operation Mindfuck | User-Interessen |
| Hagbard's Retrieval | Volltext-Abruf |
| Discordian Analysis | KI-Zusammenfassung |
| Fnord Processing | Batch-Verarbeitung |

---

*Dokument erstellt: 2025-01-04*  
*fuckupRSS – "Immanentize the Eschaton, one feed at a time."*
