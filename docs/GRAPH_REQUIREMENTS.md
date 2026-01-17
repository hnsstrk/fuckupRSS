# Graph Requirements & UX Fixes

**Status:** PHASE 2
**Datum:** 2026-01-17

## 1. Datenanalyse

### 1.1 Keyword-Verteilung

| article_count | Keywords | Kumulativ |
|---------------|----------|-----------|
| 10+ | 168 | 168 |
| 5-9 | 360 | 528 |
| 3-4 | 1.024 | 1.552 |
| 2 | 2.046 | 3.598 |
| 1 | 8.932 | 12.530 |

**Erkenntnis:** 71% aller Keywords erscheinen nur in 1 Artikel.

### 1.2 Edge-Gewichtung

| combined_weight | Edges | % |
|-----------------|-------|---|
| >= 0.5 | 7 | 0.01% |
| 0.3 - 0.5 | 21 | 0.02% |
| 0.2 - 0.3 | 57 | 0.07% |
| 0.1 - 0.2 | 1.843 | 2.2% |
| 0.05 - 0.1 | 42.606 | 50.4% |
| < 0.05 | 39.950 | 47.3% |

**Erkenntnis:** 97.7% der Kanten haben Gewicht < 0.1.

### 1.3 Top-100-Keywords Subgraph

| Threshold | Edges |
|-----------|-------|
| Alle | 835 |
| >= 0.1 | 93 |
| >= 0.2 | 40 |
| >= 0.3 | 23 |

---

## 2. Anforderungen

### 2.1 Filterung (KRITISCH)

| ID | Anforderung | Priorität |
|----|-------------|-----------|
| F1 | **Node-Filter:** Nur Keywords mit `article_count >= N` anzeigen | MUSS |
| F2 | **Edge-Filter:** Nur Kanten mit `combined_weight >= M` anzeigen | MUSS |
| F3 | **UI-Slider:** Benutzer kann Schwellenwerte anpassen | SOLL |
| F4 | **Presets:** Vordefinierte Filter (Minimal, Normal, Maximal) | KANN |

**Empfohlene Defaults:**
- `min_article_count = 3` (1.552 Keywords)
- `min_weight = 0.1` (93 Edges bei Top-100)

### 2.2 Performance

| ID | Anforderung | Priorität |
|----|-------------|-----------|
| P1 | Graph mit 100 Nodes in < 2s rendern | MUSS |
| P2 | Graph mit 500 Nodes ohne Freeze | SOLL |
| P3 | WebGL-Rendering für große Graphen | KANN |
| P4 | Progressive Loading bei > 200 Nodes | KANN |

### 2.3 Theming

| ID | Anforderung | Priorität |
|----|-------------|-----------|
| T1 | CSS-Variablen für alle Farben | MUSS |
| T2 | Dynamisches Theme-Switching ohne Reload | SOLL |
| T3 | Cluster-Farben basierend auf Kategorie | KANN |

### 2.4 Interaktion

| ID | Anforderung | Priorität |
|----|-------------|-----------|
| I1 | Node-Click zeigt Keyword-Details | MUSS (existiert) |
| I2 | Hover-Tooltip mit Metadaten | SOLL |
| I3 | Suche/Highlight einzelner Keywords | SOLL |
| I4 | Edge-Click zeigt gemeinsame Artikel | KANN |
| I5 | Drag-to-Pin für manuelle Anordnung | KANN |

### 2.5 Visualisierung

| ID | Anforderung | Priorität |
|----|-------------|-----------|
| V1 | Node-Größe proportional zu `article_count` | MUSS (existiert) |
| V2 | Edge-Dicke proportional zu `weight` | MUSS (existiert) |
| V3 | Cluster-Visualisierung (Farbcodierung) | SOLL |
| V4 | Kategorie-Overlay/Filter | KANN |
| V5 | Zeitbasierte Animation (Trend-Modus) | KANN |

### 2.6 Layout

| ID | Anforderung | Priorität |
|----|-------------|-----------|
| L1 | Force-directed für Übersicht | MUSS (existiert) |
| L2 | Hierarchisch für Cluster | KANN |
| L3 | Layout-Wechsel zur Laufzeit | KANN |

---

## 3. UX-Fixes (Sofort umsetzbar)

### 3.1 Backend-Änderungen

```rust
// immanentize.rs - get_network_graph anpassen

#[tauri::command]
pub fn get_network_graph(
    state: State<AppState>,
    limit: Option<i64>,
    min_weight: Option<f64>,
    min_article_count: Option<i64>,  // NEU
) -> Result<NetworkGraph, String> {
    let limit = limit.unwrap_or(100);
    let min_weight = min_weight.unwrap_or(0.1);  // Default erhöhen!
    let min_article_count = min_article_count.unwrap_or(3);  // NEU

    // Node-Query mit article_count Filter
    let sql = format!(
        "SELECT id, name, count, article_count, cluster_id
         FROM immanentize
         WHERE (is_canonical = TRUE OR is_canonical IS NULL)
         AND article_count >= ?
         ORDER BY article_count DESC
         LIMIT ?"
    );
    // ...
}
```

### 3.2 Frontend-Änderungen

```svelte
<!-- KeywordNetwork.svelte - Filter-UI hinzufügen -->
<div class="graph-filters">
  <label>
    Min. Artikel:
    <input type="range" min="1" max="10" bind:value={minArticleCount} />
    <span>{minArticleCount}</span>
  </label>
  <label>
    Min. Verbindungsstärke:
    <input type="range" min="0.05" max="0.5" step="0.05" bind:value={minWeight} />
    <span>{minWeight.toFixed(2)}</span>
  </label>
</div>
```

### 3.3 Sofort-Fixes

| Fix | Aufwand | Impact |
|-----|---------|--------|
| Default `min_weight` auf 0.1 erhöhen | 5 min | Hoch |
| `min_article_count` Parameter hinzufügen | 15 min | Hoch |
| Filter-Slider im UI | 30 min | Mittel |
| Tooltip mit Keyword-Metadaten | 45 min | Mittel |

---

## 4. Nicht-funktionale Anforderungen

### 4.1 Accessibility

- Keyboard-Navigation für Graph
- Screenreader-Support (ARIA-Labels)
- Kontrastreiche Farbschemata

### 4.2 Responsivität

- Touch-Gesten für Mobile
- Responsive Container
- Mindestgröße 300x300px

### 4.3 Fehlerbehandlung

- Leerer Graph: Hilfreiche Meldung
- Timeout bei großen Graphen
- Graceful Degradation bei WebGL-Fehler

---

## 5. Priorisierte Roadmap

### Phase 2a: Quick Wins (ERLEDIGT)

1. [x] Default `min_weight` auf 0.1 erhöhen
2. [x] `min_article_count` Parameter Backend (Default: 3)
3. [x] `min_article_count` Parameter Frontend
4. [x] Filter-Slider UI für beide Parameter
5. [x] Node/Edge Count Anzeige
6. [x] i18n Übersetzungen (DE/EN)

### Phase 2b: UX Polish (Optional)

1. [ ] Hover-Tooltips
2. [ ] Keyword-Suche/Highlight

### Phase 3: Tech Evaluation

Bereit für Bibliotheks-Evaluation.
