# RECS_CURRENT_STATE.md — Phase 1 Discovery Report

**Erstellt:** 2026-01-18
**Status:** Abgeschlossen
**Autor:** System Archaeologist

---

## Executive Summary

Das "Empfehlungen"-Feature in Operation Mindfuck ist **KEIN reines Mockup**, sondern ein **funktionales aber primitives System**. Die Infrastruktur existiert und liefert echte Artikel, jedoch basiert die Logik ausschließlich auf Bias-Ausgleich ohne semantische Ähnlichkeit.

**Kernbefund:** Die Datenbank enthält reichhaltige Signale (Embeddings, Keywords, Graph), die aktuell **nicht für Empfehlungen genutzt werden**. Das Potential für einen hochwertigen Recommender ist vorhanden.

---

## 1. Code-Lokation

### Frontend

| Datei | Zeilen | Funktion |
|-------|--------|----------|
| `src/lib/components/MindfuckView.svelte` | 2473 | Haupt-UI für Operation Mindfuck |
| `src/lib/components/article/ArticleCard.svelte` | - | Wiederverwendbare Artikel-Karte |
| `src/lib/types.ts` | - | TypeScript-Typen für CounterPerspective |

**Tab-Struktur in MindfuckView:**
1. `overview` — Leseprofil, Kategorie-Verteilung, Bias-Spektrum
2. `blindSpots` — Unterrepräsentierte Kategorien
3. `recommendations` — **Counter-Perspectives (Empfehlungen)**
4. `trends` — Lese-Trends über Zeit

### Backend

| Datei | Zeilen | Funktion |
|-------|--------|----------|
| `src-tauri/src/commands/mindfuck.rs` | 797 | Tauri Commands für Operation Mindfuck |

**Tauri Commands:**
```rust
#[tauri::command] get_reading_profile()       // Leseprofil
#[tauri::command] get_blind_spots()           // Blinde Flecken
#[tauri::command] get_counter_perspectives()  // EMPFEHLUNGEN
#[tauri::command] get_reading_trends()        // Trends
```

---

## 2. Ist es ein Mockup?

### Befund: **NEIN** — aber sehr primitiv

| Aspekt | Status | Beleg |
|--------|--------|-------|
| Backend-API existiert | ✅ | `get_counter_perspectives()` in mindfuck.rs:567-638 |
| Liefert echte Artikel | ✅ | SQL-Query auf `fnords` Tabelle |
| Verwendet Mock-Daten | ❌ | Keine hardkodierten Arrays |
| Nutzt Embeddings | ❌ | Embeddings existieren, werden ignoriert |
| Nutzt Keywords | ❌ | Keywords existieren, werden ignoriert |
| Hat Feedback-Loop | ❌ | Keine save/hide/like Funktion |

### Aktuelle Empfehlungs-Logik (mindfuck.rs:567-638)

```rust
// Pseudocode der aktuellen Logik:
fn get_counter_perspectives() {
    let avg_bias = user_average_political_bias();

    if avg_bias < 0.0 {
        // Nutzer liest links → empfehle rechts
        return articles_where(political_bias > 0);
    } else if avg_bias > 0.0 {
        // Nutzer liest rechts → empfehle links
        return articles_where(political_bias < 0);
    } else {
        // Nutzer ist balanced → empfehle starke Meinungen
        return articles_where(abs(political_bias) >= 1);
    }
}
```

**Probleme:**
1. **Kein Content-Matching** — Artikel werden NUR nach Bias ausgewählt
2. **Keine Interessen-Berücksichtigung** — Technik-Leser bekommen Politik-Empfehlungen
3. **Keine Freshness** — Alte Artikel werden gleich behandelt
4. **Keine Diversität** — Alle Empfehlungen haben gleichen Bias
5. **Statische Erklärungen** — "Bietet eine konservativere Perspektive" für alle

---

## 3. Daten-Inventar

### 3.1 Artikel-Daten (fnords)

| Feld | Verfügbarkeit | Wert für Empfehlungen |
|------|---------------|------------------------|
| `id` | 770 (100%) | Identifikation |
| `title` | 770 (100%) | Matching, Display |
| `content_full` | 770 (100%) | Content-based Matching |
| `summary` | 770 (100%) | Display, Quick-Match |
| `embedding` | 655 (85%) | **Semantic Similarity** |
| `political_bias` | 703 (91%) | Bias-Balance |
| `sachlichkeit` | 703 (91%) | Qualitäts-Filter |
| `published_at` | 770 (100%) | Freshness |
| `read_at` | 43 (5.6%) | User-Signal |
| `status` | 770 (100%) | User-Signal |
| `pentacle_id` | 770 (100%) | Source-Diversity |

### 3.2 Keywords (immanentize)

| Metrik | Wert |
|--------|------|
| Gesamt Keywords | 13,627 |
| Mit Embedding | 13,299 (97.6%) |
| Mit Quality Score | 13,627 (100%) |
| Artikel-Keyword-Links | 8,439 |
| Avg Keywords/Artikel | 12.2 |

**Top 10 Keywords:**
1. Vereinigte Staaten (122 Artikel)
2. Donald Trump (80)
3. Proteste (55)
4. NATO (53)
5. CDU (38)
6. SPD (38)
7. Programm Deutschlandfunk (37)
8. Deutschland (36)
9. Grönland (33)
10. BBC (33)

### 3.3 Kategorien (sephiroth)

| Level | Anzahl | Beispiele |
|-------|--------|-----------|
| Haupt (0) | 6 | Wissen & Technologie, Politik & Gesellschaft |
| Unter (1) | 13 | Technik, Wissenschaft, Politik, Gesellschaft |

**Artikel-Kategorie-Verknüpfungen:** 1,141

### 3.4 Graph-Daten (immanentize_neighbors)

- **Kookkurrenz-Netzwerk** existiert
- **Embedding-Similarity** zwischen Keywords berechnet
- **Cluster** vorhanden (immanentize_clusters)

### 3.5 Feeds (pentacles)

| Feed | Artikel |
|------|---------|
| BBC News | 206 |
| Deutschlandfunk Nachrichten | 184 |
| Deutschlandfunk Politik | 143 |
| tagesschau.de | 132 |
| netzpolitik.org | 45 |
| Augen geradeaus! | 21 |
| LinuxNews.de | 20 |
| Bundeswehr | 19 |

---

## 4. Verfügbare Signale für Empfehlungen

### 4.1 Content-Signale (ungenutzt!)

| Signal | Tabelle/Feld | Nutzung heute | Potential |
|--------|--------------|---------------|-----------|
| Article Embeddings | `fnords.embedding` | ❌ | Semantic Similarity |
| Keyword Embeddings | `immanentize.embedding` | ❌ | Topic Matching |
| Keyword Co-occurrence | `immanentize_neighbors.cooccurrence` | ❌ | Graph Expansion |
| Embedding Similarity | `immanentize_neighbors.embedding_similarity` | ❌ | Related Topics |
| Content Full | `fnords.content_full` | ❌ | BM25/TF-IDF |
| Summary | `fnords.summary` | ❌ | Quick Matching |

### 4.2 User-Signale (teilweise genutzt)

| Signal | Verfügbar | Genutzt | Beschreibung |
|--------|-----------|---------|--------------|
| `read_at` | ✅ | ✅ | Artikel gelesen |
| `status` | ✅ | ❌ | concealed/illuminated/golden_apple |
| `political_bias` avg | ✅ | ✅ | Durchschnittlicher Bias |
| Kategorie-Verteilung | ✅ | ❌ | Lesepräferenzen |
| Keyword-Affinität | ✅ | ❌ | Interessenprofil |

### 4.3 Fehlende Signale

| Signal | Status | Benötigt für |
|--------|--------|--------------|
| Explicit Feedback (like/dislike) | ❌ fehlt | Präzisere Personalisierung |
| Hide/Dismiss | ❌ fehlt | Negative Signale |
| Save/Bookmark | ❌ fehlt | Starke positive Signale |
| Dwell Time | ❌ fehlt | Implizites Interesse |
| Click-Through | ❌ fehlt | Engagement-Messung |

---

## 5. Architektur-Analyse

### 5.1 Data Flow (aktuell)

```
┌─────────────┐
│ MindfuckView│
│   (Svelte)  │
└──────┬──────┘
       │ invoke("get_counter_perspectives")
       ▼
┌─────────────┐
│ mindfuck.rs │
│   Command   │
└──────┬──────┘
       │ SQL Query (nur Bias-Filter)
       ▼
┌─────────────┐
│   fnords    │
│   (SQLite)  │
└─────────────┘

NICHT genutzt:
┌─────────────┐   ┌───────────────┐   ┌─────────────┐
│ vec_fnords  │   │  immanentize  │   │imm_neighbors│
│ (Embeddings)│   │  (Keywords)   │   │   (Graph)   │
└─────────────┘   └───────────────┘   └─────────────┘
```

### 5.2 Existierende Infrastruktur (ungenutzt)

| Komponente | Tabelle | Beschreibung |
|------------|---------|--------------|
| Vector Index | `vec_fnords` | sqlite-vec für O(log n) KNN |
| Keyword Index | `vec_immanentize` | Keyword-Embeddings für Similarity |
| Similarity Search | `find_similar_articles` | Command existiert, nicht in MindfuckView |
| Semantic Search | `semantic_search` | Command existiert, nicht in MindfuckView |

---

## 6. UI-Analyse

### 6.1 Empty State (wenn keine Empfehlungen)

Das UI zeigt einen elaborierten Empty State mit:
- Progress-Tracking (Artikel gelesen, Ollama Status, Bias-Daten)
- "How it works" Erklärung
- Placeholder-Cards als Preview

**Beobachtung:** Der Empty State ist gut gestaltet, suggeriert aber ein Feature das nicht existiert.

### 6.2 Empfehlungs-Darstellung (wenn Daten vorhanden)

```svelte
<ArticleCard
  fnord_id={article.fnord_id}
  title={article.title}
  pentacle_title={article.pentacle_title}
  published_at={article.published_at}
  political_bias={article.political_bias}
  reason={article.reason}  <!-- Statischer Text -->
  showBias={true}
  showReason={true}
/>
```

**Fehlende Elemente:**
- Kein "Save" Button
- Kein "Hide" Button
- Kein "More like this"
- Keine Keyword-Tags
- Keine Similarity-Score Anzeige

---

## 7. Größenprofil

### 7.1 Datenmenge

| Metrik | Wert |
|--------|------|
| Artikel gesamt | 770 |
| Artikel letzte 7 Tage | 344 (45%) |
| Artikel letzte 30 Tage | 693 (90%) |
| Ältester Artikel | 2025-04-30 |
| Neuester Artikel | 2026-01-18 |

### 7.2 Datenqualität

| Feld | Vollständigkeit | Qualität |
|------|-----------------|----------|
| Volltext | 100% | ✅ Excellent |
| Summary | 100% | ✅ Excellent |
| Embeddings | 85% | ✅ Good |
| Bias | 91% | ✅ Good |
| Keywords | 100%* | ✅ Excellent |

*Artikel haben durchschnittlich 12.2 Keywords

### 7.3 Sprachen

| Sprache | Anteil (geschätzt) |
|---------|-------------------|
| Deutsch | ~75% |
| Englisch | ~25% |

**Hinweis:** BBC News ist englisch, Rest überwiegend deutsch.

---

## 8. Empfehlungs-Potential

### 8.1 Was sofort möglich wäre

| Feature | Aufwand | Impact |
|---------|---------|--------|
| Embedding-basierte Similarity | Niedrig | Hoch |
| Keyword-Overlap Matching | Niedrig | Mittel |
| Category-aware Recommendations | Niedrig | Mittel |
| Freshness-Scoring | Niedrig | Mittel |
| Source-Diversity | Niedrig | Mittel |

### 8.2 Was mehr Aufwand erfordert

| Feature | Aufwand | Impact |
|---------|---------|--------|
| User-Profil aus Lese-Historie | Mittel | Hoch |
| Feedback-Loop (save/hide) | Mittel | Hoch |
| Hybrid Ranking | Mittel | Hoch |
| Graph-based Expansion | Mittel | Mittel |
| A/B Testing Framework | Hoch | Mittel |

---

## 9. Zusammenfassung

### Was gut ist:
- ✅ Datengrundlage ist exzellent (770 Artikel, 100% Volltext)
- ✅ Embeddings existieren (85% Artikel, 97.6% Keywords)
- ✅ Keyword-Graph existiert
- ✅ UI-Grundstruktur vorhanden
- ✅ Theme-Integration korrekt

### Was fehlt:
- ❌ Semantische Ähnlichkeit wird ignoriert
- ❌ Keywords werden nicht für Matching genutzt
- ❌ Kein User-Interessenprofil
- ❌ Keine Feedback-Mechanismen
- ❌ Keine echten Erklärungen
- ❌ Keine Freshness/Diversity-Logik

### Empfehlung:
Das Feature ist **kein Mockup**, aber ein **MVP ohne das V** (Viable). Die Infrastruktur ist vorhanden, die Logik muss ersetzt werden.

---

## Anhang: Relevante Code-Stellen

### A.1 Empfehlungs-Query (mindfuck.rs:599-614)

```rust
let mut stmt = db.conn().prepare(r#"
    SELECT f.id, f.title, p.title, f.published_at, f.political_bias
    FROM fnords f
    LEFT JOIN pentacles p ON p.id = f.pentacle_id
    WHERE f.read_at IS NULL
      AND f.political_bias IS NOT NULL
      AND f.political_bias * ?1 > 0  -- Same sign as target_bias
      AND f.summary IS NOT NULL
    ORDER BY ABS(f.political_bias) DESC, f.published_at DESC
    LIMIT ?2
"#)?;
```

### A.2 Similarity Search (existiert, ungenutzt)

```rust
// In src-tauri/src/commands/similarity.rs
#[tauri::command]
pub async fn find_similar_articles(fnord_id: i64, limit: Option<i32>) -> Result<...>
```

### A.3 Semantic Search (existiert, ungenutzt)

```rust
// In src-tauri/src/commands/similarity.rs
#[tauri::command]
pub async fn semantic_search(query: String, limit: Option<i32>) -> Result<...>
```
