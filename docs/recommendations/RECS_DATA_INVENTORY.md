# RECS_DATA_INVENTORY.md — Dateninventar für Empfehlungssystem

**Erstellt:** 2026-01-18
**Status:** Phase 1 Complete

---

## 1. Übersicht

| Entität | Tabelle | Anzahl | Vollständigkeit |
|---------|---------|--------|-----------------|
| Artikel | `fnords` | 770 | 100% |
| Keywords | `immanentize` | 13,627 | 100% |
| Feeds | `pentacles` | 8 | 100% |
| Kategorien | `sephiroth` | 19 (6+13) | 100% |
| Keyword-Graph | `immanentize_neighbors` | ~10,000+ | Aktiv |

---

## 2. Artikel-Daten (fnords)

### 2.1 Schema

```sql
CREATE TABLE fnords (
    id INTEGER PRIMARY KEY,
    pentacle_id INTEGER NOT NULL,

    -- Identifikation
    guid TEXT NOT NULL,
    url TEXT NOT NULL,

    -- Inhalt
    title TEXT NOT NULL,
    author TEXT,
    content_raw TEXT,       -- Original RSS-Inhalt
    content_full TEXT,      -- Volltext (via Readability)
    summary TEXT,           -- KI-generierte Zusammenfassung
    image_url TEXT,

    -- Zeitstempel
    published_at DATETIME,
    fetched_at DATETIME,
    processed_at DATETIME,
    read_at DATETIME,

    -- Status
    status TEXT DEFAULT 'concealed',  -- concealed|illuminated|golden_apple
    full_text_fetched BOOLEAN,

    -- KI-Analyse
    political_bias INTEGER,     -- -2 bis +2
    sachlichkeit INTEGER,       -- 0 bis 4
    quality_score INTEGER,      -- 1 bis 5

    -- Embeddings
    embedding BLOB,
    embedding_at DATETIME
);
```

### 2.2 Feldverfügbarkeit

| Feld | Count | Prozent | Nutzung für Recs |
|------|-------|---------|------------------|
| `title` | 770 | 100% | Display, Matching |
| `content_full` | 770 | 100% | Semantic Match |
| `summary` | 770 | 100% | Quick Match |
| `embedding` | 655 | 85% | **Primary Signal** |
| `political_bias` | 703 | 91% | Diversity |
| `sachlichkeit` | 703 | 91% | Quality Filter |
| `published_at` | 770 | 100% | Freshness |
| `read_at` | 43 | 5.6% | User Signal |
| `pentacle_id` | 770 | 100% | Source Diversity |

### 2.3 Status-Verteilung

| Status | Bedeutung | Count | Prozent |
|--------|-----------|-------|---------|
| `concealed` | Ungelesen | 727 | 94.4% |
| `illuminated` | Gelesen | 43 | 5.6% |
| `golden_apple` | Favorit | 0 | 0% |

### 2.4 Bias-Verteilung

| Bias | Label | Count | Prozent |
|------|-------|-------|---------|
| -2 | Stark links | 0 | 0% |
| -1 | Leicht links | 0 | 0% |
| 0 | Neutral | 616 | 87.6% |
| +1 | Leicht rechts | 84 | 12.0% |
| +2 | Stark rechts | 3 | 0.4% |

**Beobachtung:** Die Bias-Verteilung ist stark neutral-lastig. Das Empfehlungssystem "Counter-Perspectives" wird wenig linke Artikel finden.

---

## 3. Keyword-Daten (immanentize)

### 3.1 Schema

```sql
CREATE TABLE immanentize (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,

    -- Statistik
    count INTEGER DEFAULT 1,
    article_count INTEGER DEFAULT 0,

    -- Quality & Embeddings
    quality_score REAL,
    embedding BLOB,
    embedding_at DATETIME,

    -- Clustering
    cluster_id INTEGER,

    -- Synonym-Handling
    is_canonical BOOLEAN DEFAULT TRUE,
    canonical_id INTEGER,

    -- Typ
    keyword_type TEXT DEFAULT 'concept'
        -- concept|person|organization|location|acronym
);
```

### 3.2 Statistiken

| Metrik | Wert |
|--------|------|
| Gesamt Keywords | 13,627 |
| Mit Embedding | 13,299 (97.6%) |
| Mit Quality Score | 13,627 (100%) |
| Canonical Keywords | ~13,000 |
| Synonyme (merged) | ~600 |

### 3.3 Artikel-Keyword Verknüpfung

```sql
CREATE TABLE fnord_immanentize (
    fnord_id INTEGER,
    immanentize_id INTEGER,
    source TEXT DEFAULT 'ai',  -- ai|statistical|manual
    confidence REAL DEFAULT 1.0,
    PRIMARY KEY (fnord_id, immanentize_id)
);
```

| Metrik | Wert |
|--------|------|
| Verknüpfungen gesamt | 8,439 |
| Avg Keywords/Artikel | 12.2 |
| Min Keywords/Artikel | 1 |
| Max Keywords/Artikel | 24 |

---

## 4. Kategorie-Daten (sephiroth)

### 4.1 Hierarchie

**Hauptkategorien (level=0):**
| ID | Name | Icon |
|----|------|------|
| 1 | Wissen & Technologie | fa-microchip |
| 2 | Politik & Gesellschaft | fa-landmark |
| 3 | Wirtschaft | fa-chart-line |
| 4 | Umwelt & Gesundheit | fa-leaf |
| 5 | Sicherheit | fa-shield-halved |
| 6 | Kultur & Leben | fa-masks-theater |

**Unterkategorien (level=1):**
| ID | Name | Parent |
|----|------|--------|
| 101 | Technik | 1 |
| 102 | Wissenschaft | 1 |
| 201 | Politik | 2 |
| 202 | Gesellschaft | 2 |
| 203 | Recht | 2 |
| 301 | Wirtschaft | 3 |
| 302 | Energie | 3 |
| 401 | Umwelt | 4 |
| 402 | Gesundheit | 4 |
| 501 | Sicherheit | 5 |
| 502 | Verteidigung | 5 |
| 601 | Kultur | 6 |
| 602 | Sport | 6 |

### 4.2 Artikel-Kategorie Verknüpfung

```sql
CREATE TABLE fnord_sephiroth (
    fnord_id INTEGER,
    sephiroth_id INTEGER,
    confidence REAL DEFAULT 1.0,
    source TEXT DEFAULT 'ai',
    PRIMARY KEY (fnord_id, sephiroth_id)
);
```

| Metrik | Wert |
|--------|------|
| Verknüpfungen | 1,141 |
| Avg Kategorien/Artikel | 1.5 |

---

## 5. Feed-Daten (pentacles)

### 5.1 Schema

```sql
CREATE TABLE pentacles (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL UNIQUE,
    title TEXT,
    description TEXT,
    site_url TEXT,
    icon_url TEXT,

    -- Sync
    last_sync DATETIME,
    sync_interval INTEGER DEFAULT 1800,
    is_truncated BOOLEAN DEFAULT FALSE,

    -- Quality
    default_quality INTEGER DEFAULT 3,

    -- Stats
    article_count INTEGER DEFAULT 0
);
```

### 5.2 Aktuelle Feeds

| Feed | Artikel | Sprache | Thema |
|------|---------|---------|-------|
| BBC News | 206 | EN | Allgemein |
| Deutschlandfunk Nachrichten | 184 | DE | Allgemein |
| Deutschlandfunk Politik | 143 | DE | Politik |
| tagesschau.de | 132 | DE | Allgemein |
| netzpolitik.org | 45 | DE | Netzpolitik |
| Augen geradeaus! | 21 | DE | Verteidigung |
| LinuxNews.de | 20 | DE | Technik |
| Bundeswehr | 19 | DE | Verteidigung |

---

## 6. Vector-Indizes

### 6.1 Artikel-Embeddings

```sql
CREATE VIRTUAL TABLE vec_fnords
USING vec0(
    fnord_id INTEGER PRIMARY KEY,
    embedding float[1024] distance_metric=cosine
);
```

| Metrik | Wert |
|--------|------|
| Einträge | 655 |
| Dimension | 1024 |
| Modell | snowflake-arctic-embed2 |
| Distanz | Cosine |

### 6.2 Keyword-Embeddings

```sql
CREATE VIRTUAL TABLE vec_immanentize
USING vec0(
    immanentize_id INTEGER PRIMARY KEY,
    embedding float[1024] distance_metric=cosine
);
```

| Metrik | Wert |
|--------|------|
| Einträge | 13,299 |
| Dimension | 1024 |
| Modell | snowflake-arctic-embed2 |

---

## 7. Graph-Daten

### 7.1 Keyword-Nachbarn

```sql
CREATE TABLE immanentize_neighbors (
    immanentize_id_a INTEGER,
    immanentize_id_b INTEGER,
    cooccurrence INTEGER DEFAULT 1,      -- Wie oft zusammen
    embedding_similarity REAL,           -- Semantische Ähnlichkeit
    combined_weight REAL DEFAULT 0.0,    -- Gewichteter Score
    PRIMARY KEY (immanentize_id_a, immanentize_id_b)
);
```

### 7.2 Keyword-Kategorie Assoziation

```sql
CREATE TABLE immanentize_sephiroth (
    immanentize_id INTEGER,
    sephiroth_id INTEGER,
    weight REAL DEFAULT 1.0,
    article_count INTEGER DEFAULT 1,
    PRIMARY KEY (immanentize_id, sephiroth_id)
);
```

---

## 8. Zeitreihen-Daten

### 8.1 Keyword-Trends

```sql
CREATE TABLE immanentize_daily (
    immanentize_id INTEGER,
    date TEXT,          -- 'YYYY-MM-DD'
    count INTEGER,
    PRIMARY KEY (immanentize_id, date)
);
```

**Nutzung:** Trending-Keywords, Themen-Radar

---

## 9. Datenqualität

### 9.1 Vollständigkeit

| Bereich | Score | Begründung |
|---------|-------|------------|
| Artikel-Inhalt | 10/10 | 100% Volltext + Summary |
| Artikel-Embeddings | 8.5/10 | 85% Coverage |
| Keyword-Embeddings | 9.5/10 | 97.6% Coverage |
| Kategorisierung | 7/10 | 1.5 Kategorien/Artikel (könnte besser) |
| User-Signale | 3/10 | Nur read_at, kein Feedback |

### 9.2 Bekannte Probleme

1. **Bias-Verteilung schief** — 88% neutral, kaum linke Artikel
2. **Wenig User-Interaktionen** — Nur 43 gelesene Artikel
3. **Kein Feedback-Mechanismus** — Keine likes/dislikes
4. **Sprach-Mix** — DE/EN ohne explizites Feld

---

## 10. Empfehlungen für MVP

### Primäre Datenquellen (ready to use):
1. `vec_fnords` — Artikel-Similarity via Embeddings
2. `fnord_immanentize` — Keyword-basiertes Matching
3. `fnords.published_at` — Freshness-Scoring
4. `fnords.pentacle_id` — Source-Diversity

### Sekundäre Datenquellen (nach Feedback-Implementierung):
1. `fnords.read_at` — Gelesen/Ungelesen
2. `fnords.status` — golden_apple als starkes Signal
3. User-Keywords — Aus gelesenen Artikeln aggregiert
