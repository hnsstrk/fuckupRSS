# Graph Architecture - Immanentize Network

**Status:** Discovery abgeschlossen
**Datum:** 2026-01-17

## 1. Übersicht

Das Immanentize Network ist ein Schlagwort-Wissensnetz, das Beziehungen zwischen extrahierten Keywords aus Artikeln visualisiert. Es gibt zwei Graph-Komponenten:

1. **NetworkGraph.svelte** - Vollständiger Netzwerk-Graph (Top N Keywords)
2. **NeighborGraph.svelte** - Ego-Graph eines einzelnen Keywords

## 2. Tech-Stack

### 2.1 Aktuelle Bibliothek

| Komponente | Technologie | Version |
|------------|-------------|---------|
| Graph-Library | Cytoscape.js | ^3.x |
| Rendering | Canvas (Cytoscape default) | - |
| Layout (NetworkGraph) | COSE (Compound Spring Embedder) | built-in |
| Layout (NeighborGraph) | Concentric | built-in |

### 2.2 Integration

- **Framework:** Svelte 5 mit Runes ($state, $props, $effect)
- **State Management:** `network.svelte.ts` Store (Class-based mit $state)
- **Backend API:** Tauri Commands (`get_network_graph`, `get_keyword_neighbors`)
- **Theming:** CSS Variables via `getCssVar()` Helper (Cytoscape unterstützt keine CSS Vars nativ)

## 3. Datenmodell

### 3.1 Backend-Strukturen (Rust)

```rust
// Knoten
pub struct GraphNode {
    pub id: i64,
    pub name: String,
    pub count: i64,           // Gesamt-Verwendungen
    pub article_count: i64,   // Anzahl Artikel
    pub cluster_id: Option<i64>,
}

// Kanten
pub struct GraphEdge {
    pub source: i64,
    pub target: i64,
    pub weight: f64,          // combined_weight aus DB
    pub cooccurrence: i64,    // Anzahl gemeinsamer Artikel
}

// Graph-Container
pub struct NetworkGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
```

### 3.2 Frontend-Transformation

NetworkGraph.svelte transformiert die Daten für Cytoscape:

```typescript
// Node -> Cytoscape Element
{
  group: 'nodes',
  data: {
    id: String(node.id),
    label: node.name,
    count: node.count,
    articleCount: node.article_count,
    clusterId: node.cluster_id,
    size: 20 + (node.article_count / maxArticleCount) * 40,  // 20-60px
  }
}

// Edge -> Cytoscape Element
{
  group: 'edges',
  data: {
    id: `${edge.source}-${edge.target}`,
    source: String(edge.source),
    target: String(edge.target),
    weight: edge.weight,
    cooccurrence: edge.cooccurrence,
    width: 1 + (edge.weight / maxWeight) * 4,  // 1-5px
  }
}
```

## 4. API-Aufrufe

### 4.1 get_network_graph

```typescript
// Aufruf aus KeywordNetwork.svelte
await invoke<NetworkGraph>("get_network_graph", {
  limit: 100,      // Max. Knoten
  minWeight: 0.01, // Min. Kantengewicht (derzeit ignoriert im Backend?)
});
```

**Backend-Logik:**
1. Top N Keywords nach `article_count DESC` laden
2. Edges filtern: nur zwischen geladenen Nodes
3. Edges mit `combined_weight >= min_weight` zurückgeben

### 4.2 get_keyword_neighbors

```typescript
// Aufruf für Ego-Graph
await invoke<KeywordNeighbor[]>("get_keyword_neighbors", {
  id: keywordId,
  limit: 20,
});
```

## 5. Theming-Mechanismus

### 5.1 CSS Variable Mapping

```typescript
function getThemeColors() {
  return {
    primary: getCssVar('--accent-primary'),
    primaryHover: getCssVar('--accent-secondary'),
    selected: getCssVar('--golden-apple-color'),
    text: getCssVar('--text-primary'),
    textOutline: getCssVar('--bg-base'),
    edge: getCssVar('--text-muted'),
    edgeSelected: getCssVar('--accent-primary'),
  };
}
```

### 5.2 Problem: Kein dynamisches Theme-Switching

Die Farben werden nur bei `initGraph()` gelesen. Theme-Wechsel zur Laufzeit erfordert Graph-Neuinitialisierung.

## 6. Layout-Konfiguration

### 6.1 NetworkGraph (COSE)

```javascript
layout: {
  name: 'cose',
  animate: true,
  animationDuration: 500,
  nodeRepulsion: () => 8000,
  idealEdgeLength: () => 100,
  edgeElasticity: () => 100,
  nestingFactor: 1.2,
  gravity: 0.25,
  numIter: 1000,
  coolingFactor: 0.95,
}
```

### 6.2 NeighborGraph (Concentric)

```javascript
layout: {
  name: 'concentric',
  concentric: (node) => node.data('isCenter') ? 2 : 1,
  levelWidth: () => 1,
  minNodeSpacing: 50,
  animate: true,
  animationDuration: 300,
}
```

## 7. Interaktionen

| Aktion | NetworkGraph | NeighborGraph |
|--------|--------------|---------------|
| Node Click | `onNodeClick(id)` | `onNodeClick(id)` (nur Nachbarn) |
| Hover | Farbwechsel zu `primaryHover` | Farbwechsel zu `neighborHover` |
| Zoom | +/- Buttons, Scroll (0.2-3x) | Scroll (0.5-2x) |
| Pan | Drag | Drag |
| Reset | Fit-Button | Auto-Fit nach Layout |

## 8. Bekannte Limitierungen

1. **Performance:** COSE-Layout kann bei >100 Nodes langsam werden
2. **Theme-Switching:** Erfordert komplette Neuinitialisierung
3. **Keine Cluster-Visualisierung:** `cluster_id` wird nicht genutzt
4. **Kein dynamisches Nachladen:** Alle Nodes werden initial geladen
5. **Keine Edge-Labels:** Gewichte nur über Liniendicke sichtbar
6. **Canvas-Only:** Kein WebGL für große Graphen

## 9. Dateien

| Datei | Zweck |
|-------|-------|
| `src/lib/components/NetworkGraph.svelte` | Haupt-Netzwerkgraph |
| `src/lib/components/NeighborGraph.svelte` | Ego-Graph für Keywords |
| `src/lib/components/KeywordNetwork.svelte` | Container mit Tabs |
| `src/lib/stores/network.svelte.ts` | State Management |
| `src-tauri/src/commands/immanentize.rs` | Backend-API |
