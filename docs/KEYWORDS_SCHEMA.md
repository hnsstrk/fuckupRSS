# Keywords Schema - Immanentize Network

**Status:** Discovery abgeschlossen
**Datum:** 2026-01-17

## 1. Datenbankschema

### 1.1 immanentize (Keywords)

```sql
CREATE TABLE immanentize (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    count INTEGER DEFAULT 1,              -- Gesamt-Verwendungen
    article_count INTEGER DEFAULT 0,      -- Anzahl verknüpfter Artikel
    quality_score REAL DEFAULT NULL,      -- 0.0-1.0, berechnet
    quality_calculated_at DATETIME,
    embedding BLOB DEFAULT NULL,          -- 1024-dim Float32 (snowflake-arctic-embed2)
    embedding_at DATETIME,
    cluster_id INTEGER,                   -- FK zu immanentize_clusters
    is_canonical BOOLEAN DEFAULT TRUE,    -- Haupt-Keyword oder Synonym
    canonical_id INTEGER,                 -- FK zu Haupt-Keyword bei Synonymen
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_used DATETIME DEFAULT CURRENT_TIMESTAMP,
    keyword_type TEXT DEFAULT 'concept'   -- 'concept', 'entity', 'location', etc.
);
```

### 1.2 immanentize_neighbors (Kanten)

```sql
CREATE TABLE immanentize_neighbors (
    immanentize_id_a INTEGER NOT NULL,
    immanentize_id_b INTEGER NOT NULL,
    cooccurrence INTEGER DEFAULT 1,       -- Anzahl gemeinsamer Artikel
    embedding_similarity REAL,            -- Kosinus-Ähnlichkeit (0.0-1.0)
    combined_weight REAL DEFAULT 0.0,     -- Gewichtete Kombination
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (immanentize_id_a, immanentize_id_b)
);
```

### 1.3 Verwandte Tabellen

```sql
-- Keyword ↔ Kategorie Zuordnung
CREATE TABLE immanentize_sephiroth (
    immanentize_id INTEGER NOT NULL,
    sephiroth_id INTEGER NOT NULL,
    weight REAL DEFAULT 1.0,
    article_count INTEGER DEFAULT 1,
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (immanentize_id, sephiroth_id)
);

-- Artikel ↔ Keyword Zuordnung
CREATE TABLE fnord_immanentize (
    fnord_id INTEGER NOT NULL,
    immanentize_id INTEGER NOT NULL,
    source TEXT DEFAULT 'ai',              -- 'ai', 'statistical', 'manual'
    confidence REAL DEFAULT 1.0,           -- 0.0-1.0
    PRIMARY KEY (fnord_id, immanentize_id)
);

-- Tägliche Keyword-Statistiken (für Trends)
CREATE TABLE immanentize_daily (
    immanentize_id INTEGER NOT NULL,
    date TEXT NOT NULL,                    -- YYYY-MM-DD
    count INTEGER DEFAULT 0,
    PRIMARY KEY (immanentize_id, date)
);

-- Themen-Cluster
CREATE TABLE immanentize_clusters (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    auto_generated BOOLEAN DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Ignorierte Synonym-Vorschläge
CREATE TABLE dismissed_synonyms (
    keyword_a_id INTEGER NOT NULL,
    keyword_b_id INTEGER NOT NULL,
    dismissed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (keyword_a_id, keyword_b_id)
);
```

## 2. Größenprofil (Aktueller Stand)

| Metrik | Wert |
|--------|------|
| **Knoten (Keywords)** | 12.530 |
| Keywords mit Embeddings | 12.189 (97%) |
| **Kanten (Neighbors)** | 84.484 |
| Kanten mit combined_weight >= 0.1 | 1.928 (2.3%) |
| Kanten mit combined_weight >= 0.5 | 7 (0.01%) |
| Artikel (Fnords) | 850 |
| Artikel mit Keywords | 799 (94%) |
| Keyword-Artikel-Links | 9.548 |
| Cluster | 0 (Feature nicht aktiv) |
| Tägliche Stats-Einträge | 14.332 |

### 2.1 Verteilung

- **Durchschnitt Keywords/Artikel:** ~12 Keywords pro Artikel
- **Graph-Dichte:** Sehr gering bei minWeight=0.1 (nur 2.3% der Kanten)
- **Embedding-Abdeckung:** 97% aller Keywords haben Embeddings

## 3. API-Strukturen

### 3.1 Rust Backend

```rust
// Vollständiges Keyword-Objekt
pub struct Keyword {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub article_count: i64,
    pub cluster_id: Option<i64>,
    pub is_canonical: bool,
    pub canonical_id: Option<i64>,
    pub first_seen: Option<String>,
    pub last_used: Option<String>,
    pub quality_score: Option<f64>,
    pub keyword_type: String,
}

// Für Nachbar-Abfragen
pub struct KeywordNeighbor {
    pub id: i64,
    pub name: String,
    pub cooccurrence: i64,
    pub embedding_similarity: Option<f64>,
    pub combined_weight: f64,
}

// Für Graph-Visualisierung (reduziert)
pub struct GraphNode {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub article_count: i64,
    pub cluster_id: Option<i64>,
}

pub struct GraphEdge {
    pub source: i64,
    pub target: i64,
    pub weight: f64,        // = combined_weight
    pub cooccurrence: i64,
}
```

### 3.2 TypeScript Frontend

```typescript
interface NetworkGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

interface GraphNode {
  id: number;
  name: string;
  count: number;
  article_count: number;
  cluster_id: number | null;
}

interface GraphEdge {
  source: number;
  target: number;
  weight: number;
  cooccurrence: number;
}
```

## 4. Keyword-Typen

| Typ | Beschreibung | Beispiel |
|-----|--------------|----------|
| `concept` | Allgemeiner Begriff | "Digitalisierung", "Klimawandel" |
| `entity` | Benannte Entität | "Donald Trump", "Volkswagen" |
| `location` | Ortsangabe | "Berlin", "Kopenhagen" |
| `event` | Ereignis | "CES 2026", "Bundestagswahl" |

## 5. Qualitäts-Metriken

### 5.1 Quality Score

Berechnet aus:
- Embedding-Vorhandensein
- Artikel-Anzahl
- Kookkurrenz-Stärke
- Kategorie-Zuordnungen

### 5.2 Combined Weight (Kanten)

```
combined_weight = α × normalized_cooccurrence + β × embedding_similarity
```

Derzeit: `α = 0.5`, `β = 0.5`

## 6. Aktuelle Limitierungen

1. **Cluster nicht aktiv:** `immanentize_clusters` existiert, aber ist leer
2. **keyword_type:** Meistens 'concept', selten spezifischer
3. **Viele schwache Kanten:** 98% der Kanten haben weight < 0.1
4. **Keine hierarchische Struktur:** Flache Keyword-Liste
