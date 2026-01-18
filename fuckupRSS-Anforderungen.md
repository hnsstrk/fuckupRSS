# fuckupRSS – Anforderungsdokument

**Version:** 0.6
**Datum:** 2026-01-18
**Status:** Phase 3 abgeschlossen (Semantic Search, Recommendations), Phase 4 (Polish) in Entwicklung

---

## Inhaltsverzeichnis

1. [Projektübersicht](#1-projektübersicht)
2. [Illuminatus!-Terminologie](#2-illuminatus-terminologie)
3. [Technologie-Stack](#3-technologie-stack)
4. [KI-Integration](#4-ki-integration)
5. [Bias-Erkennung (Greyface Alert)](#5-bias-erkennung-greyface-alert)
6. [Embeddings und Vektorsuche](#6-embeddings-und-vektorsuche)
6b. [Schlagwort-Netzwerk (Immanentize Network)](#6b-schlagwort-netzwerk-immanentize-network)
6c. [Operation Mindfuck (Bias-Spiegel)](#6c-operation-mindfuck-bias-spiegel)
7. [Batch-Verarbeitung (Fnord Processing)](#7-batch-verarbeitung-fnord-processing)
8. [Volltext-Abruf (Hagbard's Retrieval)](#8-volltext-abruf-hagbards-retrieval)
8b. [Revisionsverwaltung (Fnord History)](#8b-revisionsverwaltung-fnord-history)
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
| Geänderte Artikel | **Fnords** | Artikel mit Revisionshistorie |
| Ungelesene Artikel | **Concealed** | Verborgen bis zur Erleuchtung |
| Gelesene Artikel | **Illuminated** | Erleuchtete/verarbeitete Artikel |
| Favoriten/Wichtig | **Golden Apple** 🍎 | Markierte Artikel |
| KI-Zusammenfassung | **Discordian Analysis** | Automatische Analyse |
| Bias-Warnung | **Greyface Alert** | Hinweis auf einseitige Berichterstattung |
| Bias-Spiegel | **Operation Mindfuck (OM)** | Filterblase aufzeigen, nicht verstärken |
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

| Modell | Größe | Zweck | Hinweis |
|--------|-------|-------|---------|
| `ministral-3:latest` | 6.0 GB | **Hauptmodell** für Textanalyse | Empfohlen (schnell) |
| `qwen3-vl:8b` | 6.1 GB | Alternative mit Vision-Support | Langsamer, für Bildanalyse |
| `nomic-embed-text` | 274 MB | Embeddings für Vektorsuche | |

**Wichtig:** `qwen3-vl` ist ein Vision-Language-Modell und hat deutlich mehr Overhead bei reinen Textaufgaben. Für die Greyface-Analyse ist `ministral-3` ~4x schneller.

**VRAM-Bedarf:** ~6-7 GB + Overhead = ~8-9 GB

### 4.1b Performance-Optimierungen

| Optimierung | Wert | Effekt |
|-------------|------|--------|
| `num_ctx` | 8192 | Reduziert RAM von ~27 GB auf ~8 GB |
| Parallele Calls | Ja | Summary + Analyse gleichzeitig |
| Modell-Wechsel | Vermeiden | Entladen/Laden kostet Zeit |

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

## 6b. Schlagwort-Netzwerk (Immanentize Network)

### 6b.1 Konzept: Semantisches Wissensnetz

Das Immanentize Network ist ein hybrides System, das **statistische Kookkurrenz** mit **semantischen Embeddings** kombiniert, um ein intelligentes Schlagwort-Netzwerk aufzubauen.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SEMANTISCHES SCHLAGWORT-NETZWERK                         │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                     EMBEDDING-SPACE (768 dim)                       │  │
│   │                                                                     │  │
│   │         "KI" ●────── 0.95 ──────● "AI"                             │  │
│   │              \                  /                                   │  │
│   │          0.82 \              / 0.78                                 │  │
│   │                \            /                                       │  │
│   │                 ● "Machine Learning"                                │  │
│   │                        |                                            │  │
│   │                   0.71 |                                            │  │
│   │                        ▼                                            │  │
│   │              ● "Deep Learning"                                      │  │
│   │                                                                     │  │
│   │   ● "EU" ←──── 0.45 ────→ ● "USA"  (thematisch verwandt)          │  │
│   │       ↑                                                             │  │
│   │  0.88 │  (Kookkurrenz hoch)                                        │  │
│   │       ↓                                                             │  │
│   │   ● "Brüssel"                                                       │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│   Gewicht = α × Kookkurrenz + β × Embedding-Ähnlichkeit                    │
│             (statistisch)       (semantisch)                                │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6b.2 Zwei-Säulen-Ansatz

| Säule | Methode | Stärke | Beispiel |
|-------|---------|--------|----------|
| **Kookkurrenz** | Statistisch | Kontextuelle Beziehung | "EU" + "Brüssel" (oft im selben Artikel) |
| **Embeddings** | Semantisch | Bedeutungsgleichheit | "KI" ≈ "AI" (auch ohne gemeinsame Artikel) |

**Kombination:** Ein Schlagwort-Paar kann hohe Kookkurrenz haben (oft zusammen genannt) ODER hohe Embedding-Ähnlichkeit (ähnliche Bedeutung) ODER beides.

### 6b.3 Datenmodell

#### Erweiterte Schlagwort-Tabelle (immanentize)

```sql
CREATE TABLE immanentize (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,

    -- Statistik
    count INTEGER DEFAULT 1,              -- Anzahl Verwendungen
    article_count INTEGER DEFAULT 0,      -- Anzahl Artikel mit diesem Schlagwort

    -- Embedding-Status
    embedding_at DATETIME,                -- Wann Embedding erstellt wurde

    -- Clustering
    cluster_id INTEGER,                   -- Zugehöriger Themen-Cluster

    -- Synonym-Handling
    is_canonical BOOLEAN DEFAULT TRUE,    -- Ist dies das Haupt-Schlagwort?
    canonical_id INTEGER,                 -- Verweis auf Haupt-Synonym (falls Duplikat)

    -- Zeitstempel
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_used DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (canonical_id) REFERENCES immanentize(id),
    FOREIGN KEY (cluster_id) REFERENCES immanentize_clusters(id)
);
```

#### Schlagwort-Embeddings (immanentize_vss)

```sql
-- Virtuelle Tabelle für Vektor-Suche (sqlite-vec)
CREATE VIRTUAL TABLE immanentize_vss USING vss0(
    embedding(768)  -- nomic-embed-text Dimension
);

-- rowid entspricht immanentize.id
```

#### Schlagwort-Kategorien (immanentize_sephiroth)

```sql
CREATE TABLE immanentize_sephiroth (
    immanentize_id INTEGER NOT NULL,
    sephiroth_id INTEGER NOT NULL,

    -- Stärke der Assoziation
    weight REAL DEFAULT 1.0,              -- Normalisierte Gewichtung (0.0-1.0)
    article_count INTEGER DEFAULT 1,      -- Wie oft zusammen vorgekommen

    -- Zeitstempel
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (immanentize_id, sephiroth_id),
    FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE,
    FOREIGN KEY (sephiroth_id) REFERENCES sephiroth(id) ON DELETE CASCADE
);
```

**Beispiel:**

| Schlagwort | Kategorie | weight | article_count |
|------------|-----------|--------|---------------|
| EU | Politik | 0.85 | 127 |
| EU | Wirtschaft | 0.45 | 68 |
| EU | Recht | 0.30 | 42 |
| KI | Technik | 0.92 | 234 |
| KI | Wirtschaft | 0.38 | 89 |

#### Nachbar-Netzwerk (immanentize_neighbors)

```sql
CREATE TABLE immanentize_neighbors (
    immanentize_id_a INTEGER NOT NULL,    -- Schlagwort A (kleinere ID)
    immanentize_id_b INTEGER NOT NULL,    -- Schlagwort B (größere ID)

    -- Statistische Beziehung
    cooccurrence INTEGER DEFAULT 0,       -- Wie oft zusammen in Artikeln

    -- Semantische Beziehung
    embedding_similarity REAL,            -- Cosine-Ähnlichkeit (0.0-1.0)

    -- Kombiniertes Gewicht
    combined_weight REAL DEFAULT 0.0,     -- α×cooc_norm + β×embed_sim

    -- Zeitstempel
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (immanentize_id_a, immanentize_id_b),
    FOREIGN KEY (immanentize_id_a) REFERENCES immanentize(id) ON DELETE CASCADE,
    FOREIGN KEY (immanentize_id_b) REFERENCES immanentize(id) ON DELETE CASCADE,
    CHECK (immanentize_id_a < immanentize_id_b)  -- Symmetrie vermeiden
);
```

#### Themen-Cluster (immanentize_clusters)

```sql
CREATE TABLE immanentize_clusters (
    id INTEGER PRIMARY KEY,
    name TEXT,                            -- "KI & Machine Learning", "EU-Politik"
    description TEXT,

    -- Cluster-Metadaten
    auto_generated BOOLEAN DEFAULT TRUE,  -- Automatisch oder manuell erstellt
    keyword_count INTEGER DEFAULT 0,      -- Anzahl Schlagworte im Cluster

    -- Zeitstempel
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Cluster-Zentroid für schnelle Zuordnung neuer Schlagworte
CREATE VIRTUAL TABLE immanentize_clusters_vss USING vss0(
    centroid(768)
);
```

### 6b.4 Gewichtungs-Algorithmus

#### Kookkurrenz-Normalisierung

```
cooc_normalized = cooccurrence(A,B) / min(total_articles(A), total_articles(B))
```

**Beispiel:** "EU" erscheint in 100 Artikeln, "Brüssel" in 50 Artikeln, beide zusammen in 45:
- `cooc_normalized = 45 / min(100, 50) = 45 / 50 = 0.90`

#### Embedding-Ähnlichkeit

```
embedding_similarity = cosine_similarity(embedding(A), embedding(B))
```

Berechnet über sqlite-vec bei Embedding-Erstellung.

#### Kombiniertes Gewicht

```
combined_weight = α × cooc_normalized + β × embedding_similarity

Standardwerte:
- α = 0.4 (Kookkurrenz-Gewicht)
- β = 0.6 (Embedding-Gewicht)
```

### 6b.5 Verarbeitungs-Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    IMMANENTIZING PIPELINE                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ARTIKEL WIRD ANALYSIERT (Discordian Analysis)                             │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 1. KEYWORD EXTRACTION (ministral/qwen)                              │   │
│  │    Keywords: ["Künstliche Intelligenz", "EU", "Regulierung"]        │   │
│  │    Kategorien: [Politik, Technik]                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 2. SCHLAGWORT-VERARBEITUNG                                          │   │
│  │    Für jedes Keyword:                                               │   │
│  │    a) INSERT/UPDATE immanentize (count++, article_count++)          │   │
│  │    b) INSERT fnord_immanentize (Artikel ↔ Schlagwort)               │   │
│  │    c) Falls NEU: Embedding via nomic-embed-text generieren          │   │
│  │       → INSERT immanentize_vss                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 3. KATEGORIE-ASSOZIATION                                            │   │
│  │    Für jedes Keyword × Kategorie des Artikels:                      │   │
│  │    UPDATE immanentize_sephiroth (weight neu berechnen)              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 4. NACHBAR-AKTUALISIERUNG                                           │   │
│  │    Für alle Keyword-Paare im Artikel (n*(n-1)/2):                   │   │
│  │    a) UPDATE cooccurrence++                                         │   │
│  │    b) embedding_similarity aus VSS (falls noch nicht berechnet)     │   │
│  │    c) combined_weight neu berechnen                                 │   │
│  │                                                                     │   │
│  │    Beispiel: 4 Keywords = 6 Paare                                   │   │
│  │    KI↔EU, KI↔Regulierung, KI↔Brüssel,                              │   │
│  │    EU↔Regulierung, EU↔Brüssel, Regulierung↔Brüssel                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 5. SYNONYM-ERKENNUNG (bei neuen Schlagworten)                       │   │
│  │    VSS-Suche: Gibt es Schlagwort mit embedding_similarity > 0.92?   │   │
│  │    → Falls ja: canonical_id setzen, is_canonical = FALSE            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                    CLUSTERING JOB (periodisch, z.B. täglich)               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. Alle Schlagwort-Embeddings aus immanentize_vss laden                   │
│  2. K-Means oder DBSCAN Clustering durchführen                             │
│  3. Cluster-Zentroide berechnen und in immanentize_clusters_vss speichern  │
│  4. immanentize.cluster_id für jedes Schlagwort aktualisieren              │
│  5. Cluster-Namen automatisch generieren (häufigstes Schlagwort)           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6b.6 API-Endpoints

| Endpoint | Parameter | Rückgabe | Beschreibung |
|----------|-----------|----------|--------------|
| `get_keyword_neighbors` | id, limit | `Vec<Neighbor>` | Top-N Nachbar-Schlagworte |
| `get_keyword_categories` | id | `Vec<CategoryWeight>` | Kategorien eines Schlagworts |
| `get_category_keywords` | sephiroth_id, limit | `Vec<Keyword>` | Top-Schlagworte einer Kategorie |
| `get_keyword_cluster` | id | `Cluster` | Cluster-Info eines Schlagworts |
| `get_cluster_keywords` | cluster_id | `Vec<Keyword>` | Alle Schlagworte im Cluster |
| `get_similar_keywords` | id, limit | `Vec<Keyword>` | Semantisch ähnliche (via Embedding) |
| `get_trending_keywords` | days, limit | `Vec<TrendingKeyword>` | Trending Schlagworte |
| `find_synonyms` | id | `Vec<Keyword>` | Erkannte Synonyme |
| `merge_keywords` | source_id, target_id | - | Manuelles Zusammenführen |

### 6b.7 Anwendungsfälle

| Feature | Methode | Beispiel |
|---------|---------|----------|
| **Synonym-Erkennung** | Embedding-Distanz < 0.08 | "KI" ≈ "AI" ≈ "Künstliche Intelligenz" |
| **Themen-Cluster** | K-Means auf Embeddings | Cluster "Technologie": KI, Software, Digital |
| **Semantische Suche** | VSS-Query | "Klimawandel" findet auch "Erderwärmung" |
| **Trend-Erkennung** | Zeitreihe + Wachstumsrate | Neue Begriffe mit hoher Frequenz |
| **Ähnliche Artikel** | Schlagwort-Überlappung + Embedding | "Zeige mir Artikel wie diesen" |
| **Kategorie-Analyse** | immanentize_sephiroth | "Welche Schlagworte dominieren Politik?" |
| **Netzwerk-Visualisierung** | Graph aus neighbors | Interaktive Themen-Karte |

### 6b.8 Beispiel-Abfragen

```sql
-- Top-10 Nachbarn eines Schlagworts
SELECT
    i.name,
    n.cooccurrence,
    n.embedding_similarity,
    n.combined_weight
FROM immanentize_neighbors n
JOIN immanentize i ON i.id = CASE
    WHEN n.immanentize_id_a = ?1 THEN n.immanentize_id_b
    ELSE n.immanentize_id_a
END
WHERE n.immanentize_id_a = ?1 OR n.immanentize_id_b = ?1
ORDER BY n.combined_weight DESC
LIMIT 10;

-- Semantisch ähnliche Schlagworte (via Embedding)
SELECT i.name, distance
FROM immanentize_vss v
JOIN immanentize i ON i.id = v.rowid
WHERE vss_search(v.embedding, (
    SELECT embedding FROM immanentize_vss WHERE rowid = ?1
))
AND v.rowid != ?1
ORDER BY distance ASC
LIMIT 10;

-- Top-Schlagworte einer Kategorie
SELECT i.name, ims.weight, ims.article_count
FROM immanentize_sephiroth ims
JOIN immanentize i ON i.id = ims.immanentize_id
WHERE ims.sephiroth_id = ?1
ORDER BY ims.weight DESC
LIMIT 20;

-- Trending: Schlagworte mit starkem Wachstum (letzte 7 Tage)
SELECT
    i.name,
    i.article_count as total,
    (SELECT COUNT(DISTINCT fi.fnord_id)
     FROM fnord_immanentize fi
     JOIN fnords f ON f.id = fi.fnord_id
     WHERE fi.immanentize_id = i.id
     AND f.published_at > datetime('now', '-7 days')) as last_week
FROM immanentize i
WHERE i.article_count > 5
ORDER BY last_week DESC
LIMIT 20;

-- Cluster-Übersicht
SELECT
    c.id,
    c.name,
    c.keyword_count,
    GROUP_CONCAT(i.name, ', ') as top_keywords
FROM immanentize_clusters c
JOIN immanentize i ON i.cluster_id = c.id
GROUP BY c.id
ORDER BY c.keyword_count DESC;
```

### 6b.9 Konfiguration

| Parameter | Default | Beschreibung |
|-----------|---------|--------------|
| `COOC_WEIGHT` (α) | 0.4 | Gewicht der Kookkurrenz |
| `EMBED_WEIGHT` (β) | 0.6 | Gewicht der Embedding-Ähnlichkeit |
| `SYNONYM_THRESHOLD` | 0.92 | Embedding-Ähnlichkeit für Synonym-Erkennung |
| `MIN_COOCCURRENCE` | 2 | Mindest-Kookkurrenz für Nachbar-Speicherung |
| `CLUSTER_MIN_SIZE` | 5 | Mindestgröße für Cluster |
| `TRENDING_DAYS` | 7 | Zeitraum für Trend-Berechnung |

---

## 6c. Operation Mindfuck (Bias-Spiegel)

### 6c.1 Konzept

**Operation Mindfuck** ist im Illuminatus!-Kontext die discordianische Praxis, Menschen aus ihren gewohnten Denkmustern herauszureißen. In fuckupRSS wird dieses Konzept umgekehrt: Das System zeigt dem User seinen eigenen Bias und seine Filterblase – um ihn darauf aufmerksam zu machen.

**Kernidee**: Nicht personalisierte Empfehlungen, die den Bias verstärken, sondern ein **Bias-Spiegel**, der dem User seine eigene Filterblase vorführt.

```
┌─────────────────────────────────────────────────────────────────┐
│  ⚠️ OPERATION MINDFUCK – Dein Bias-Spiegel                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  📊 Dein Leseprofil der letzten 30 Tage:                        │
│                                                                 │
│  Politische Tendenz deiner Quellen:                             │
│  Links ████████████░░░░░░░░ Rechts     (68% links)              │
│                                                                 │
│  Themen-Verteilung:                                             │
│  ▓▓▓▓▓▓▓▓▓▓ Politik (45%)                                       │
│  ▓▓▓▓▓▓░░░░ Technik (28%)                                       │
│  ▓▓▓░░░░░░░ Wirtschaft (12%)                                    │
│  ▓▓░░░░░░░░ Sonstiges (15%)                                     │
│                                                                 │
│  ⚠️ Blinde Flecken: Sport, Kultur, Gesundheit                   │
│                                                                 │
│  🎯 5 Artikel die dein Weltbild herausfordern könnten:          │
│  → [Artikel aus konservativer Quelle zu deinem Thema]           │
│  → [Artikel aus Bereich den du ignorierst]                      │
│  → ...                                                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 6c.2 Anti-Bubble-Philosophie

**Warum keine klassische Personalisierung?**

| Klassische Personalisierung | Operation Mindfuck |
|-----------------------------|-------------------|
| "Du magst X, hier ist mehr X" | "Du liest nur X, hier ist Y" |
| Verstärkt Filterblase | Zeigt Filterblase auf |
| User fühlt sich bestätigt | User wird herausgefordert |
| Algorithmus versteckt sich | Algorithmus ist transparent |

**Ziel**: Der User soll **bewusst entscheiden** können, ob er in seiner Blase bleiben will – nicht unbewusst hineinrutschen.

### 6c.3 Datenerfassung

#### Lesehistorie (reading_history)

```sql
CREATE TABLE reading_history (
    id INTEGER PRIMARY KEY,
    fnord_id INTEGER NOT NULL,
    read_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    reading_time INTEGER,       -- Sekunden (wie lange gelesen)
    scroll_depth REAL,          -- 0.0-1.0 (wie weit gescrollt)
    source TEXT DEFAULT 'view', -- 'view', 'click', 'favorite'

    FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE
);
```

#### Erfasste Signale

| Signal | Gewicht | Bedeutung |
|--------|---------|-----------|
| Artikel geöffnet | 1.0 | Grundinteresse |
| > 30 Sekunden gelesen | 1.5 | Echtes Interesse |
| > 2 Minuten gelesen | 2.0 | Tiefes Interesse |
| Favorisiert (Golden Apple) | 3.0 | Starkes Interesse |
| Ignoriert (in Liste übersprungen) | -0.5 | Desinteresse |

### 6c.4 Bias-Analyse

#### Aggregierte Metriken

```sql
-- Politische Tendenz der gelesenen Artikel
SELECT
    AVG(f.political_bias) as avg_bias,
    COUNT(*) as article_count,
    SUM(CASE WHEN f.political_bias < 0 THEN 1 ELSE 0 END) as left_count,
    SUM(CASE WHEN f.political_bias > 0 THEN 1 ELSE 0 END) as right_count
FROM reading_history rh
JOIN fnords f ON f.id = rh.fnord_id
WHERE rh.read_at > datetime('now', '-30 days');

-- Themen-Verteilung
SELECT
    s.name as category,
    COUNT(*) as read_count,
    ROUND(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER(), 1) as percentage
FROM reading_history rh
JOIN fnord_sephiroth fs ON fs.fnord_id = rh.fnord_id
JOIN sephiroth s ON s.id = fs.sephiroth_id
WHERE rh.read_at > datetime('now', '-30 days')
GROUP BY s.id
ORDER BY read_count DESC;

-- Quellen-Verteilung
SELECT
    p.title as source,
    COUNT(*) as read_count,
    AVG(f.political_bias) as source_bias
FROM reading_history rh
JOIN fnords f ON f.id = rh.fnord_id
JOIN pentacles p ON p.id = f.pentacle_id
WHERE rh.read_at > datetime('now', '-30 days')
GROUP BY p.id
ORDER BY read_count DESC;
```

#### Blinde Flecken erkennen

```sql
-- Kategorien die der User ignoriert (< 5% der Lesezeit)
SELECT s.name as blind_spot
FROM sephiroth s
LEFT JOIN (
    SELECT fs.sephiroth_id, COUNT(*) as read_count
    FROM reading_history rh
    JOIN fnord_sephiroth fs ON fs.fnord_id = rh.fnord_id
    WHERE rh.read_at > datetime('now', '-30 days')
    GROUP BY fs.sephiroth_id
) rc ON rc.sephiroth_id = s.id
WHERE COALESCE(rc.read_count, 0) < (
    SELECT COUNT(*) * 0.05 FROM reading_history
    WHERE read_at > datetime('now', '-30 days')
);
```

### 6c.5 Gegenpol-Empfehlungen

**Nicht "mehr vom Gleichen", sondern "das Gegenteil":**

```sql
-- Artikel die den User herausfordern könnten
SELECT f.id, f.title, f.political_bias, p.title as source
FROM fnords f
JOIN pentacles p ON p.id = f.pentacle_id
WHERE
    -- Artikel mit gegensätzlichem politischen Bias
    (f.political_bias * (SELECT AVG(political_bias) FROM reading_history rh
                         JOIN fnords f2 ON f2.id = rh.fnord_id
                         WHERE rh.read_at > datetime('now', '-30 days'))) < 0
    -- Oder aus Kategorien die der User ignoriert
    OR EXISTS (
        SELECT 1 FROM fnord_sephiroth fs
        JOIN sephiroth s ON s.id = fs.sephiroth_id
        WHERE fs.fnord_id = f.id
        AND s.name IN (/* blind_spots von oben */)
    )
    -- Aber trotzdem qualitativ gut
    AND f.sachlichkeit >= 3
    -- Und nicht schon gelesen
    AND f.id NOT IN (SELECT fnord_id FROM reading_history)
ORDER BY f.published_at DESC
LIMIT 5;
```

### 6c.6 User-Interessen (operation_mindfuck Tabelle)

Zusätzlich zur automatischen Analyse kann der User explizite Interessen angeben:

```sql
CREATE TABLE operation_mindfuck (
    id INTEGER PRIMARY KEY,

    -- Typ des Interesses
    type TEXT NOT NULL CHECK(type IN ('sephiroth', 'immanentize', 'pentacle', 'custom')),

    -- Referenz (je nach Typ)
    reference_id INTEGER,       -- ID der Kategorie/Stichwort/Feed
    custom_term TEXT,           -- Für freie Begriffe

    -- Gewichtung
    weight REAL DEFAULT 1.0 CHECK(weight BETWEEN -1.0 AND 2.0),
    -- -1.0 = aktiv NICHT interessiert (ausblenden)
    --  0.0 = neutral
    --  1.0 = interessiert (Standard)
    --  2.0 = sehr interessiert

    -- Quelle der Einstellung
    source TEXT DEFAULT 'manual' CHECK(source IN ('manual', 'learned')),

    -- Metadaten
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**Wichtig**: `weight = -1.0` bedeutet **nicht** "verstecken", sondern "mir ist bewusst, dass ich das meide" – die Artikel werden trotzdem angezeigt, aber der Bias-Spiegel berücksichtigt diese Selbsteinschätzung.

### 6c.7 UI-Komponenten

#### Bias-Dashboard (Hauptansicht)

| Element | Beschreibung |
|---------|--------------|
| **Bias-Meter** | Horizontaler Balken: Links ↔ Rechts |
| **Themen-Radar** | Kreisdiagramm der Kategorie-Verteilung |
| **Blinde-Flecken-Liste** | Kategorien die der User < 5% liest |
| **Quellen-Diversität** | Score 0-100% (wie viele verschiedene Quellen) |
| **Zeitlicher Verlauf** | Wie hat sich der Bias über Zeit entwickelt? |

#### Challenge-Vorschläge

```
┌─────────────────────────────────────────────┐
│ 🎯 Dein Weltbild herausfordern?             │
├─────────────────────────────────────────────┤
│ Du liest 73% linke Quellen.                 │
│                                             │
│ Hier sind 3 sachliche Artikel von der       │
│ anderen Seite zu deinen Lieblingsthemen:    │
│                                             │
│ • [FAZ: Wirtschaftspolitik-Analyse]         │
│ • [NZZ: Tech-Regulierung]                   │
│ • [Welt: Klimapolitik-Kommentar]            │
│                                             │
│ [Später] [Zeig mir mehr]                    │
└─────────────────────────────────────────────┘
```

### 6c.8 Datenschutz-Hinweise

- **Alle Daten bleiben lokal** – keine Cloud-Synchronisation der Lesehistorie
- **Opt-in**: Tracking muss in Settings aktiviert werden
- **Löschbar**: User kann Lesehistorie jederzeit löschen
- **Transparent**: User sieht genau welche Daten erfasst werden

### 6c.9 Abgrenzung zu klassischen Empfehlungssystemen

| Feature | Netflix/YouTube/etc. | fuckupRSS |
|---------|---------------------|-----------|
| **Ziel** | Engagement maximieren | Bewusstsein schaffen |
| **Methode** | Mehr vom Gleichen | Das Gegenteil zeigen |
| **Transparenz** | Black Box | Vollständig offen |
| **Datenspeicherung** | Cloud | Nur lokal |
| **Manipulation** | Ja (Dopamin-Loops) | Nein (Anti-Bubble) |

### 6c.10 Implementierungs-Phasen

| Phase | Feature | Priorität |
|-------|---------|-----------|
| **1** | Lesehistorie erfassen | Hoch |
| **2** | Bias-Berechnung (politisch + thematisch) | Hoch |
| **3** | Blinde-Flecken-Erkennung | Mittel |
| **4** | Bias-Dashboard UI | Mittel |
| **5** | Gegenpol-Empfehlungen | Niedrig |
| **6** | Zeitlicher Verlauf | Niedrig |

---

## 6d. Statistische Textanalyse & Bias-Lernen

### 6d.1 Architektur

Parallel zur LLM-basierten Discordian Analysis erfolgt eine statistische Textanalyse:

```
┌─────────────────────────────────────────────────────────────────┐
│                     Artikel-Verarbeitung                        │
├─────────────────────────────────────────────────────────────────┤
│  1. Sync → content_raw                                          │
│  2. Retrieval → content_full                                    │
│  3. ┌──────────────────────────────────────────────────────┐    │
│     │ PARALLEL ANALYSE                                      │    │
│     │ ├─ Statistische Analyse (TF-IDF, Wortfrequenz)       │    │
│     │ │   → keyword_candidates, category_scores            │    │
│     │ └─ LLM Discordian Analysis (bestehend)               │    │
│     │     → keywords, categories, summary                   │    │
│     └──────────────────────────────────────────────────────┘    │
│  4. Merge & Scoring                                             │
│  5. Speicherung mit source='statistical'|'ai'|'manual'          │
└─────────────────────────────────────────────────────────────────┘
```

### 6d.2 TF-IDF Keyword-Extraktion

| Komponente | Beschreibung |
|------------|--------------|
| Tokenisierung | Unicode-aware Word-Splitting |
| Stopwords | Deutsch + Englisch (~600 Wörter) |
| Min-Länge | 2 Zeichen |
| Max-Keywords | 15 pro Artikel |
| IDF-Korpus | Aus bestehenden Keywords oder Default 1.0 |

**Modul:** `src-tauri/src/text_analysis/tfidf.rs`

### 6d.3 Kategorie-Matching

13 Unterkategorien mit gewichteten Wortlisten:

| ID | Kategorie | Beispielterme |
|----|-----------|---------------|
| 101 | Technik | software, hardware, internet, ki, algorithmus |
| 102 | Wissenschaft | studie, forschung, experimente, theorie |
| 201 | Politik | regierung, minister, gesetz, partei, wahl |
| 202 | Gesellschaft | migration, gesellschaft, sozial, familie |
| 301 | Wirtschaft | unternehmen, aktie, börse, handel, inflation |
| 401 | Umwelt | klima, umwelt, emission, nachhaltig |
| 501 | Sicherheit | cyber, hacker, angriff, sicherheit, terror |
| 601 | Kultur | museum, künstler, theater, musik, film |

**Modul:** `src-tauri/src/text_analysis/category_matcher.rs`

### 6d.4 Bias-Lernsystem

```
┌─────────────────────────────────────────────────────────────────┐
│                     Bias-Lernsystem                             │
├─────────────────────────────────────────────────────────────────┤
│  bias_weights Tabelle:                                          │
│  ├─ keyword_boost: {"politik": 1.2, "wirtschaft": 0.8}         │
│  └─ category_term: {"sicherheit": {"cyber": 2.0}}              │
│                                                                 │
│  Lernen aus Korrekturen:                                        │
│  ├─ User entfernt Keyword → boost -= 0.1                       │
│  ├─ User fügt Keyword hinzu → boost += 0.1                      │
│  └─ Gewichtung begrenzt auf 0.1 - 3.0                          │
└─────────────────────────────────────────────────────────────────┘
```

**Modul:** `src-tauri/src/text_analysis/bias.rs`

### 6d.5 Datenmodell

| Tabelle/Feld | Änderung |
|--------------|----------|
| `fnord_immanentize.source` | TEXT ('ai', 'statistical', 'manual') |
| `fnord_immanentize.confidence` | REAL (0.0-1.0) |
| `fnord_sephiroth.source` | TEXT ('ai', 'manual') |
| `fnord_sephiroth.confidence` | REAL (0.0-1.0) |
| `bias_weights` | Lern-Gewichtungen (keyword_boost, category_term, source_weight) |
| `corpus_stats` | Document Frequencies für corpus-weite TF-IDF |

**Source-Gewichtungen (Default-Werte):**
| Source | Gewicht | Beschreibung |
|--------|---------|--------------|
| manual | 1.2 | Manuelle Einträge werden höher gewichtet |
| ai | 1.0 | LLM-generiert (Baseline) |
| statistical | 0.9 | Braucht LLM-Validierung |

**Corpus-Stats:**
- Bei >= 10 Artikeln: Echte IDF aus corpus_stats
- Davor: Fallback auf einfache TF-Analyse
- Automatische Aktualisierung nach jeder Analyse

### 6d.6 UI-Komponenten

| Komponente | Funktion |
|------------|----------|
| `ArticleKeywords.svelte` | Editierbare Keyword-Chips |
| `ArticleCategories.svelte` | Editierbare Kategorie-Chips |
| Source-Badges | Robot/Chart/User Icons |
| Autocomplete | Existierende Keywords vorschlagen |

### 6d.7 Tauri Commands

| Command | Beschreibung |
|---------|--------------|
| `get_article_keywords` | Keywords mit source/confidence |
| `add_article_keyword` | Manuell hinzufügen |
| `remove_article_keyword` | Keyword entfernen |
| `get_article_categories_detailed` | Kategorien mit source |
| `update_article_categories` | Kategorien setzen |
| `analyze_article_statistical` | Statistische Analyse |
| `record_correction` | Korrektur für Bias-Lernen |
| `get_bias_stats` | Bias-Statistiken |

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
│ • Raw-Artikel in DB speichern (Status: "concealed")             │
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
│ • Status: "concealed" bleibt (= ungelesen)                      │
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

### 8.1 Grundprinzip

**fuckupRSS versucht IMMER den Volltext zu laden** – unabhängig davon, ob der Feed gekürzt ist oder nicht. Der Volltext ist die primäre Quelle für:
- Die Anzeige im Artikel-View
- Die KI-Analyse (Greyface Alert, Discordian Analysis)
- Die Änderungserkennung (Revisionen)

### 8.2 Anzeige im UI

```
┌─────────────────────────────────────────────────────────────────┐
│ EU verabschiedet AI Act – Strengere Regeln                      │
│ heise.de • 05.01.2026 • 📰 Nachricht                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│ [Feed-Content / Teaser]                                         │
│ Die EU hat heute den AI Act verabschiedet...                    │
│                                                                 │
│ ─────────────────────────────────────────────────────────────── │
│                                                                 │
│ [Volltext – Hagbard's Retrieval]                                │
│ BRÜSSEL (dpa) – Nach jahrelangen Verhandlungen hat das          │
│ Europäische Parlament heute den AI Act final verabschiedet...   │
│ [vollständiger Artikel]                                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Bei Fehler:**
```
┌─────────────────────────────────────────────────────────────────┐
│ ─────────────────────────────────────────────────────────────── │
│                                                                 │
│ ⚠️ Volltext konnte nicht geladen werden                         │
│ Grund: HTTP 403 – Paywall erkannt                               │
│ [🔄 Erneut versuchen]                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 8.3 Entscheidungslogik (Legacy – jetzt immer Volltext)

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

### 8.6 Volltext für KI-Analyse

Die Greyface-Analyse (Bias-Erkennung) verwendet **immer den Volltext** (`content_full`), sofern verfügbar:

```rust
let content_for_analysis = fnord.content_full
    .as_ref()
    .unwrap_or(&fnord.content_raw);
```

**Reihenfolge der Präferenz:**
1. `content_full` – Volltext via Hagbard's Retrieval
2. `content_raw` – Original Feed-Content (Fallback)

---

## 8b. Revisionsverwaltung (Fnord History)

### 8b.1 Grundprinzip

Artikel können sich ändern – sei es durch Korrekturen, Updates oder "stille" Änderungen. fuckupRSS speichert **alle Versionen** eines Artikels und macht Änderungen sichtbar.

### 8b.2 Was wird auf Änderungen geprüft?

| Feld | Prüfung | Speicherung |
|------|---------|-------------|
| `title` | Ja | In Revision |
| `content_raw` | Ja (Hash) | In Revision |
| `content_full` | Ja (Hash) | In Revision |
| `author` | Ja | In Revision |
| `published_at` | Ja | In Revision |
| `summary` (KI) | Nein | Nur aktuell |

### 8b.3 Datenbank-Schema für Revisionen

```sql
CREATE TABLE fnord_revisions (
    id INTEGER PRIMARY KEY,
    fnord_id INTEGER NOT NULL,

    -- Snapshot des Artikels zu diesem Zeitpunkt
    title TEXT NOT NULL,
    author TEXT,
    content_raw TEXT,
    content_full TEXT,
    published_at DATETIME,

    -- Metadaten der Revision
    content_hash TEXT NOT NULL,        -- SHA256 von content_full oder content_raw
    revision_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    revision_number INTEGER NOT NULL,  -- 1 = Original, 2 = erste Änderung, etc.

    -- Was hat sich geändert?
    changes_summary TEXT,              -- JSON: {"title": true, "content": true, ...}

    FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE
);

CREATE INDEX idx_revisions_fnord ON fnord_revisions(fnord_id);
CREATE INDEX idx_revisions_date ON fnord_revisions(revision_at DESC);
```

### 8b.4 UI: Revisionen durchblättern

```
┌─────────────────────────────────────────────────────────────────┐
│ EU verabschiedet AI Act – Strengere Regeln                      │
│ ─────────────────────────────────────────────────────────────── │
│                                                                 │
│ 📜 Revisionen: 3 Versionen                                      │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ [◀ Vorherige]  Version 2 von 3  [Nächste ▶]                │ │
│ │                                                             │ │
│ │ 📅 05.01.2026 14:23 (Original: 05.01.2026 09:15)           │ │
│ │                                                             │ │
│ │ Änderungen:                                                 │ │
│ │ • Titel: ✓ geändert                                         │ │
│ │ • Inhalt: ✓ geändert (423 Zeichen Differenz)               │ │
│ │ • Autor: — unverändert                                      │ │
│ │                                                             │ │
│ │ [Diff anzeigen] [Zur aktuellen Version]                    │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│ [Alter Inhalt dieser Revision...]                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 8b.5 Metadaten pro Revision

Jede Revision enthält **vollständige Metadaten**:

| Feld | Beschreibung |
|------|--------------|
| `revision_number` | Fortlaufende Nummer (1 = Original) |
| `revision_at` | Wann diese Version erfasst wurde |
| `published_at` | Veröffentlichungsdatum laut Feed (kann sich ändern!) |
| `title` | Titel zu diesem Zeitpunkt |
| `author` | Autor zu diesem Zeitpunkt |
| `content_raw` | Feed-Content zu diesem Zeitpunkt |
| `content_full` | Volltext zu diesem Zeitpunkt |
| `content_hash` | Hash zur Änderungserkennung |
| `changes_summary` | JSON mit geänderten Feldern |

### 8b.6 Änderungserkennung beim Sync

```
Feed-Sync
    │
    ├─► Artikel existiert bereits? (via GUID)
    │       │
    │       ├─► Nein ──► Neuer Artikel, Revision 1 anlegen
    │       │
    │       └─► Ja ──► Content-Hash vergleichen
    │                   │
    │                   ├─► Hash identisch ──► Keine Aktion
    │                   │
    │                   └─► Hash unterschiedlich ──► Neue Revision anlegen
    │                                               ──► has_changes = TRUE
    │                                               ──► Volltext neu abrufen
```

### 8b.7 Kennzeichnung geänderter Artikel

In der Artikel-Liste:

```
┌──────────────────────────────────────────────────────────┐
│ 📰 heise.de                                              │
├──────────────────────────────────────────────────────────┤
│ ● EU verabschiedet AI Act                    vor 2 Std  │
│ ⚡ Microsoft kündigt Entlassungen an [🔄 3]   vor 4 Std  │  ← Geändert, 3 Revisionen
│ ● Neue GPT-5 Gerüchte                        vor 5 Std  │
└──────────────────────────────────────────────────────────┘
```

**Legende:**
- `●` = Normale Artikel
- `⚡` = Artikel mit Änderungen (Fnord!)
- `[🔄 3]` = 3 Revisionen vorhanden

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
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│    pentacles    │────<│     fnords      │────>│   fnords_vss    │
│  (Feed-Quellen) │     │    (Artikel)    │     │   (Embeddings)  │
└─────────────────┘     └────────┬────────┘     └─────────────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              ▼                  ▼                  ▼
      ┌─────────────┐    ┌─────────────────┐    ┌─────────────────┐
      │  sephiroth  │    │   immanentize   │    │ fnord_revisions │
      │ (Kategorien)│    │  (Schlagworte)  │    │   (Versionen)   │
      └──────┬──────┘    └────────┬────────┘    └─────────────────┘
             │                    │
             │    ┌───────────────┼───────────────┐
             │    ▼               ▼               ▼
             │  ┌───────────┐ ┌─────────────┐ ┌─────────────────┐
             │  │immanentize│ │ immanentize │ │   immanentize   │
             └─>│ _sephiroth│ │ _neighbors  │ │     _vss        │
                │(Kat↔Schlag)│ │(Kookkurrenz)│ │  (Embeddings)   │
                └───────────┘ └─────────────┘ └─────────────────┘
                                    │
                              ┌─────┴─────┐
                              ▼           ▼
                      ┌───────────┐ ┌───────────────┐
                      │immanentize│ │  immanentize  │
                      │ _clusters │ │ _clusters_vss │
                      │ (Themen)  │ │  (Zentroide)  │
                      └───────────┘ └───────────────┘

      ┌────────────────────────────────────────────┐
      │           operation_mindfuck               │
      │           (User-Interessen)                │
      └────────────────────────────────────────────┘
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
    status TEXT DEFAULT 'concealed' CHECK(status IN ('concealed', 'illuminated', 'golden_apple')),
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
-- IMMANENTIZE (Schlagworte/Tags)
-- Erweitert für Immanentize Network (siehe Kapitel 6b)
-- ============================================================
CREATE TABLE immanentize (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,

    -- Statistik
    count INTEGER DEFAULT 1,              -- Wie oft verwendet
    article_count INTEGER DEFAULT 0,      -- Anzahl Artikel mit diesem Schlagwort

    -- Embedding-Status
    embedding_at DATETIME,                -- Wann Embedding erstellt wurde

    -- Clustering
    cluster_id INTEGER,                   -- Zugehöriger Themen-Cluster

    -- Synonym-Handling
    is_canonical BOOLEAN DEFAULT TRUE,    -- Ist dies das Haupt-Schlagwort?
    canonical_id INTEGER,                 -- Verweis auf Haupt-Synonym

    -- Zeitstempel
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_used DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (canonical_id) REFERENCES immanentize(id),
    FOREIGN KEY (cluster_id) REFERENCES immanentize_clusters(id)
);

-- ============================================================
-- IMMANENTIZE_VSS (Schlagwort-Embeddings)
-- ============================================================
CREATE VIRTUAL TABLE immanentize_vss USING vss0(
    embedding(768)  -- nomic-embed-text Dimension, rowid = immanentize.id
);

-- ============================================================
-- IMMANENTIZE_SEPHIROTH (Schlagwort ↔ Kategorie)
-- ============================================================
CREATE TABLE immanentize_sephiroth (
    immanentize_id INTEGER NOT NULL,
    sephiroth_id INTEGER NOT NULL,
    weight REAL DEFAULT 1.0,              -- Normalisierte Stärke (0.0-1.0)
    article_count INTEGER DEFAULT 1,      -- Wie oft zusammen vorgekommen
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (immanentize_id, sephiroth_id),
    FOREIGN KEY (immanentize_id) REFERENCES immanentize(id) ON DELETE CASCADE,
    FOREIGN KEY (sephiroth_id) REFERENCES sephiroth(id) ON DELETE CASCADE
);

-- ============================================================
-- IMMANENTIZE_NEIGHBORS (Kookkurrenz-Netzwerk)
-- ============================================================
CREATE TABLE immanentize_neighbors (
    immanentize_id_a INTEGER NOT NULL,    -- Kleinere ID
    immanentize_id_b INTEGER NOT NULL,    -- Größere ID
    cooccurrence INTEGER DEFAULT 0,       -- Wie oft zusammen in Artikeln
    embedding_similarity REAL,            -- Cosine-Ähnlichkeit (0.0-1.0)
    combined_weight REAL DEFAULT 0.0,     -- α×cooc_norm + β×embed_sim
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (immanentize_id_a, immanentize_id_b),
    FOREIGN KEY (immanentize_id_a) REFERENCES immanentize(id) ON DELETE CASCADE,
    FOREIGN KEY (immanentize_id_b) REFERENCES immanentize(id) ON DELETE CASCADE,
    CHECK (immanentize_id_a < immanentize_id_b)
);

-- ============================================================
-- IMMANENTIZE_CLUSTERS (Themen-Cluster)
-- ============================================================
CREATE TABLE immanentize_clusters (
    id INTEGER PRIMARY KEY,
    name TEXT,                            -- "KI & ML", "EU-Politik"
    description TEXT,
    auto_generated BOOLEAN DEFAULT TRUE,
    keyword_count INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Cluster-Zentroide für schnelle Zuordnung
CREATE VIRTUAL TABLE immanentize_clusters_vss USING vss0(
    centroid(768)
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

-- Immanentize Network Indizes
CREATE INDEX idx_immanentize_cluster ON immanentize(cluster_id);
CREATE INDEX idx_immanentize_canonical ON immanentize(canonical_id);
CREATE INDEX idx_immanentize_sephiroth_seph ON immanentize_sephiroth(sephiroth_id);
CREATE INDEX idx_immanentize_neighbors_a ON immanentize_neighbors(immanentize_id_a);
CREATE INDEX idx_immanentize_neighbors_b ON immanentize_neighbors(immanentize_id_b);
CREATE INDEX idx_immanentize_neighbors_weight ON immanentize_neighbors(combined_weight DESC);
```

### 10.3 Standard-Kategorien (Sephiroth)

13 fest definierte Kategorien (nicht vom Benutzer änderbar):

```sql
INSERT INTO sephiroth (name, icon, color) VALUES
    ('Technik', '💻', '#3B82F6'),       -- IT, Software, Digital
    ('Politik', '🏛️', '#EF4444'),       -- Parteien, Wahlen, Regierung
    ('Wirtschaft', '📈', '#10B981'),    -- Finanzen, Unternehmen, Märkte
    ('Wissenschaft', '🔬', '#8B5CF6'),  -- Forschung, Studien
    ('Kultur', '🎭', '#F59E0B'),        -- Kunst, Musik, Film
    ('Sport', '⚽', '#06B6D4'),         -- Fußball, etc.
    ('Gesellschaft', '👥', '#EC4899'),  -- Soziales, Demografie
    ('Umwelt', '🌍', '#22C55E'),        -- Klima, Natur
    ('Sicherheit', '🔒', '#6366F1'),    -- Cybersecurity, Datenschutz
    ('Gesundheit', '🏥', '#F43F5E'),    -- Medizin, Krankheiten
    ('Verteidigung', '🎖️', '#78716C'), -- Militär, NATO, Bundeswehr
    ('Energie', '⚡', '#FBBF24'),       -- Strom, Öl, Erneuerbare
    ('Recht', '⚖️', '#7C3AED');        -- Justiz, Gesetze, Urteile
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
| Status | Concealed, Illuminated, Golden Apple | Ja |
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
- In SQLite-Datenbank persistiert

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

Einstellungen werden in der SQLite-Datenbank (`settings`-Tabelle) gespeichert:

```sql
-- Schema
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Beispiel-Daten
INSERT INTO settings (key, value) VALUES
    ('locale', 'de'),
    ('theme', 'mocha'),
    ('showTerminologyTooltips', 'true');
```

---

## 18. Plattform-spezifische Details

### 18.1 Datenpfade

Alle Daten werden im Projektordner gespeichert:

```
projektordner/
├── data/
│   ├── fuckup.db       # SQLite Datenbank (inkl. Settings)
│   └── fuckup.db-wal   # Write-Ahead Log
└── ...
```

**Hinweis:** Einstellungen (Sprache, Theme, Tooltips) werden in der `settings`-Tabelle der SQLite-Datenbank gespeichert, nicht in einer externen config.toml.

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

### Phase 3: KI-Features (Aktuell)
- [x] Keyword-Embeddings via nomic-embed-text
- [ ] Artikel-Embeddings + sqlite-vec VSS
- [ ] Ähnliche Artikel (Vektor-Ähnlichkeit)
- [ ] Semantische Suche

### Phase 4: Polish
- [ ] Operation Mindfuck (Bias-Spiegel) – siehe [Abschnitt 6c](#6c-operation-mindfuck-bias-spiegel)
  - [ ] Lesehistorie erfassen
  - [ ] Bias-Berechnung (politisch + thematisch)
  - [ ] Blinde-Flecken-Erkennung
  - [ ] Bias-Dashboard UI
  - [ ] Gegenpol-Empfehlungen
- [ ] OPML Import/Export
- [ ] Erweiterte Keyboard-Shortcuts (Vim-Style)
- [ ] Desktop-Notifications

### Phase 5: Release
- [ ] Linux-Paketierung (.deb, .rpm, AppImage)
- [ ] macOS-Build + Signierung
- [ ] Dokumentation finalisieren
- [ ] Release v1.0

---

## 21. Testing

### Grundsatz

**Alle neuen Features und Bugfixes müssen mit Tests abgedeckt werden.** Code ohne Tests wird nicht akzeptiert.

### Test-Übersicht

| Bereich | Anzahl Tests | Tool | Befehl |
|---------|-------------|------|--------|
| Rust Backend | 160 | `cargo test` | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Frontend Unit | 89 | Vitest | `npm run test` |
| E2E | 11 | Playwright | `npm run test:e2e` |
| **Gesamt** | **260** | | |

### Rust Tests

```
src-tauri/src/
├── db/tests.rs         # Datenbank-Tests (14 Tests)
│                       # Schema, CRUD, Cascade Delete, Settings
├── sync/tests.rs       # Sync-Tests (14 Tests)
│                       # Hash-Berechnung, Change Detection
├── retrieval/tests.rs  # Retrieval-Tests (22 Tests)
│                       # Truncation-Erkennung, Patterns
├── ollama/tests.rs     # Ollama-Tests (33 Tests)
│                       # JSON-Extraktion, Bias-Analyse, Locale
└── commands/
    ├── tests.rs        # Batch-Analyse Unit Tests (31 Tests)
    │                   # BatchProgress, BatchResult, Cancellation, Structs
    └── batch_integration_tests.rs  # DB-Integration (9 Tests)
                        # Unprocessed Query, Category/Keyword Assignment
```

**Muster:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let db = Database::new_in_memory().unwrap();

        // Act
        let result = function(&db);

        // Assert
        assert!(result.is_ok());
    }
}
```

### Frontend Tests (Vitest)

```
src/lib/__tests__/
├── setup.ts            # Test-Setup mit Tauri/i18n Mocks
├── stores/
│   └── state.test.ts   # Store-Tests (18 Tests)
└── components/
    └── Toast.test.ts   # Component-Tests (19 Tests)
```

**Muster:**
```typescript
import { describe, it, expect, vi } from 'vitest';

describe('Component/Store', () => {
  it('should do something', () => {
    // Test mit gemockten Tauri-Aufrufen
  });
});
```

### E2E Tests (Playwright)

```
e2e/
├── fixtures.ts         # Tauri API Mocks
└── app.spec.ts         # App-Tests (11 Tests)
                        # Layout, Sidebar, Settings, Accessibility
```

**Muster:**
```typescript
import { test, expect } from './fixtures';

test('user flow', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.element')).toBeVisible();
});
```

### Wann Tests schreiben?

| Änderung | Tests erforderlich |
|----------|-------------------|
| Neues Feature | Ja - Unit + ggf. E2E |
| Bugfix | Ja - Test der den Bug reproduziert |
| Refactoring | Bestehende Tests müssen grün bleiben |
| UI-Änderung | E2E Tests aktualisieren |
| API-Änderung | Command Tests aktualisieren |

---

## Anhang

### A. Verfügbare Ollama-Modelle (auf dem Entwicklungssystem)

```
NAME                       SIZE
ministral-3:latest         6.0 GB    ← Hauptmodell
snowflake-arctic-embed2    1.2 GB    ← Embeddings (DE/EN, 1024-dim)
qwen3-vl:8b                6.1 GB    ← Alternative mit Vision
phi4:latest                9.1 GB
qwen3:8b                   5.2 GB
```

### B. Datenbankschema für Settings

Einstellungen werden in der SQLite-Datenbank gespeichert (Key-Value-Store):

```sql
-- Tabelle
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Implementierte Settings
INSERT INTO settings (key, value) VALUES
    ('locale', 'de'),              -- Sprache: de, en
    ('theme', 'mocha'),            -- Theme: mocha, macchiato, frappe, latte
    ('showTerminologyTooltips', 'true');  -- Tooltips anzeigen

-- Geplante Settings (für spätere Phasen)
-- ('syncInterval', '30')          -- Sync-Intervall in Minuten
-- ('syncOnStart', 'true')         -- Sync bei App-Start
-- ('ollamaHost', 'http://localhost:11434')
-- ('mainModel', 'qwen3-vl:8b')
-- ('embeddingModel', 'nomic-embed-text')
```

### C. Embedding-Modell

#### C.1 Aktuelles Modell

**Standard:** `snowflake-arctic-embed2` (1024-dim, multilingual)

| Eigenschaft | Wert |
|-------------|------|
| Modell | snowflake-arctic-embed2 |
| Dimensionen | 1024 |
| Sprachen | 74 (inkl. Deutsch, Englisch) |
| Größe | 1.2 GB |
| VRAM | ~2-3 GB |
| Kontext | 8192 Tokens |

**Vorteile:**
- Explizite deutsche Sprachunterstützung (CLEF-Benchmarks)
- Bessere Performance bei kurzen Texten/Keywords
- Matryoshka Representation Learning (MRL) für Kompression
- Apache 2.0 Lizenz

**Bei Modellwechsel:** Alle Keywords müssen neu eingebettet werden (Settings → Wartung → Embeddings generieren), da unterschiedliche Dimensionen nicht kompatibel sind.

**Alternative:** `bge-m3` (100+ Sprachen, ebenfalls 1024-dim)

#### C.2 Offene Verbesserungen für Phase 3

| Priorität | Issue | Beschreibung |
|-----------|-------|--------------|
| Hoch | Artikel-Embeddings | `fnords.embedding` Spalte implementieren |
| Hoch | Ähnliche Artikel | `find_similar_articles` Command |
| Mittel | Semantische Suche | Volltext-Suche via Embeddings |
| Niedrig | VSS-Integration | sqlite-vec für performante Suche |

---

### D. Glossar

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
| Operation Mindfuck | Bias-Spiegel (Filterblase aufzeigen) |
| Hagbard's Retrieval | Volltext-Abruf |
| Discordian Analysis | KI-Zusammenfassung |
| Fnord Processing | Batch-Verarbeitung |

---

*Dokument erstellt: 2025-01-04*  
*fuckupRSS – "Immanentize the Eschaton, one feed at a time."*
