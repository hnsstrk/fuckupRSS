# Graph Library Tech Evaluation

**Status:** PHASE 3
**Datum:** 2026-01-17

## 1. Anforderungsprofil

Basierend auf der Discovery-Phase (PHASE 1):

| Anforderung | Gewicht | Begründung |
|-------------|---------|------------|
| Performance (100-500 Nodes) | Hoch | Gefilterte Darstellung, nicht volle 12k Keywords |
| Theming/CSS-Variablen | Hoch | Bestehendes Design-System |
| TypeScript-Support | Mittel | Entwicklungseffizienz |
| Svelte-Integration | Hoch | Framework-Kompatibilität |
| Layout-Algorithmen | Mittel | Force-directed reicht aktuell |
| Lernkurve/Migration | Hoch | Bestehender Code vorhanden |
| Bundle-Size | Niedrig | Desktop-App, kein Web-Deployment |

## 2. Kandidaten

### 2.1 Cytoscape.js (AKTUELL)

**Version:** ~3.x
**Rendering:** Canvas
**Lizenz:** MIT

| Kriterium | Bewertung | Details |
|-----------|-----------|---------|
| Performance | ⭐⭐⭐ | Gut bis 10k Nodes, danach langsam |
| Theming | ⭐⭐ | Keine native CSS-Var-Unterstützung |
| TypeScript | ⭐⭐⭐ | @types/cytoscape verfügbar |
| Svelte-Integration | ⭐⭐⭐⭐ | Funktioniert, bereits implementiert |
| Layout-Algorithmen | ⭐⭐⭐⭐⭐ | COSE, dagre, cola, etc. eingebaut |
| Lernkurve | ⭐⭐⭐⭐ | Gut dokumentiert |
| Graph-Analyse | ⭐⭐⭐⭐⭐ | PageRank, Betweenness, etc. eingebaut |

**Vorteile:**
- Bereits implementiert und funktionsfähig
- Reichhaltige Analyse-Algorithmen
- Große Community (10k+ GitHub Stars)
- Extensions verfügbar (cytoscape-dagre, cytoscape-cola)

**Nachteile:**
- Kein WebGL (Canvas-limitiert)
- CSS-Variablen müssen manuell gelesen werden
- Kein natives Dark/Light-Mode-Switching

**Bundle-Size:** ~400KB (minified)

---

### 2.2 Sigma.js

**Version:** v3.x
**Rendering:** WebGL
**Lizenz:** MIT

| Kriterium | Bewertung | Details |
|-----------|-----------|---------|
| Performance | ⭐⭐⭐⭐⭐ | Hervorragend, WebGL-optimiert |
| Theming | ⭐⭐⭐ | Programmatisch, keine CSS-Vars |
| TypeScript | ⭐⭐⭐⭐ | Native TS-Unterstützung in v3 |
| Svelte-Integration | ⭐⭐ | Keine offizielle Unterstützung |
| Layout-Algorithmen | ⭐⭐ | Extern via graphology-layout |
| Lernkurve | ⭐⭐ | Schwache Dokumentation |
| Graph-Analyse | ⭐⭐⭐ | Via graphology-Ökosystem |

**Vorteile:**
- Beste Performance für große Graphen
- WebGL-Rendering
- Modernes TypeScript-Design
- Leichtgewichtig

**Nachteile:**
- Keine offizielle Svelte-Integration
- Dokumentation mangelhaft
- Layout-Algorithmen extern
- Komplett neue Implementierung nötig

**Bundle-Size:** ~150KB (sigma + graphology)

---

### 2.3 D3.js (d3-force)

**Version:** v7.x
**Rendering:** SVG/Canvas
**Lizenz:** BSD-3-Clause

| Kriterium | Bewertung | Details |
|-----------|-----------|---------|
| Performance | ⭐⭐⭐ | Moderat, SVG-limitiert bei vielen Nodes |
| Theming | ⭐⭐⭐⭐⭐ | Volle CSS-Integration möglich |
| TypeScript | ⭐⭐⭐⭐ | @types/d3 verfügbar |
| Svelte-Integration | ⭐⭐⭐⭐ | Gut, aber manuell |
| Layout-Algorithmen | ⭐⭐⭐ | d3-force, manuell zu konfigurieren |
| Lernkurve | ⭐ | Sehr steil, viel Boilerplate |
| Graph-Analyse | ⭐⭐ | Nicht eingebaut, manuell |

**Vorteile:**
- Maximale Flexibilität
- Native CSS/SVG-Theming
- Riesige Community
- Keine Vendor Lock-in

**Nachteile:**
- Hoher Implementierungsaufwand
- Kein Graph-spezifisches API
- Performance bei vielen Nodes problematisch
- Komplette Neuimplementierung nötig

**Bundle-Size:** ~250KB (d3 komplett) / ~50KB (nur d3-force)

---

## 3. Vergleichsmatrix

| Kriterium | Cytoscape.js | Sigma.js | D3.js |
|-----------|--------------|----------|-------|
| Performance (100-500 Nodes) | ✅ Ausreichend | ✅✅ Überdimensioniert | ✅ Ausreichend |
| Theming | ⚠️ Workaround | ⚠️ Programmatisch | ✅ Native |
| TypeScript | ✅ | ✅✅ | ✅ |
| Svelte-Integration | ✅✅ Vorhanden | ⚠️ Manuell | ⚠️ Manuell |
| Layout-Algorithmen | ✅✅ Eingebaut | ⚠️ Extern | ⚠️ Manuell |
| Migrations-Aufwand | ✅✅ Keiner | ❌ Hoch | ❌ Sehr hoch |
| Graph-Analyse | ✅✅ Eingebaut | ⚠️ Extern | ❌ Manuell |

**Legende:** ✅✅ Sehr gut | ✅ Gut | ⚠️ Eingeschränkt | ❌ Problematisch

---

## 4. Empfehlung

### Primäre Empfehlung: **Cytoscape.js beibehalten + optimieren**

**Begründung:**

1. **Bereits funktionsfähig:** Die Implementierung existiert und ist stabil
2. **Ausreichende Performance:** Mit Filterung (100 Nodes, 93 Edges) keine Performance-Probleme
3. **Kein Migrations-Risiko:** Wechsel würde ~2-3 Tage kosten für ungewissen Mehrwert
4. **Graph-Analyse:** Für zukünftige Features (Clustering, Zentralität) bereits eingebaut

### Optimierungen für Cytoscape.js

```typescript
// 1. Theme-Reaktivität verbessern
// Bei Theme-Wechsel Graph neu initialisieren
$effect(() => {
  if (currentTheme && cy) {
    cy.style().fromJson(getThemedStyles()).update();
  }
});

// 2. WebGL-Extension für bessere Performance
// npm install cytoscape-webgl
import cytoscapeWebgl from 'cytoscape-webgl';
cytoscape.use(cytoscapeWebgl);

// 3. Lazy Layout für große Graphen
layout: {
  name: 'cose',
  animate: 'end',  // Nur Endposition animieren
  fit: true,
  randomize: false,  // Deterministische Startpositionen
}
```

### Alternative: Sigma.js (nur bei Performance-Problemen)

Falls später Performance-Probleme auftreten (>1000 Nodes gleichzeitig), wäre Sigma.js der logische nächste Schritt:

- WebGL-Rendering skaliert besser
- graphology-Ökosystem für Layouts
- Aber: Höherer Implementierungsaufwand

---

## 5. Nicht empfohlen

### D3.js

- Zu hoher Implementierungsaufwand
- Keine Graph-spezifischen Features
- Kein Vorteil gegenüber Cytoscape für unseren Use-Case

---

## 6. Nächste Schritte

Basierend auf dieser Evaluation:

1. **PHASE 4:** Spike für Cytoscape-Optimierungen
   - Theme-Reaktivität testen
   - WebGL-Extension evaluieren
   - Tooltip-Implementation

2. **Fallback-Plan:** Sigma.js nur evaluieren, wenn:
   - User >500 Nodes gleichzeitig sehen will
   - Performance-Beschwerden auftreten

---

## Quellen

- [Cylynx: Comparison of JavaScript Graph Libraries](https://www.cylynx.io/blog/a-comparison-of-javascript-graph-network-visualisation-libraries/)
- [Memgraph: Graph Visualization Tool Comparison](https://memgraph.com/blog/you-want-a-fast-easy-to-use-and-popular-graph-visualization-tool)
- [Sigma.js Documentation](https://www.sigmajs.org/docs/)
- [Cytoscape.js Official Site](https://js.cytoscape.org/)
- [npm-compare: cytoscape vs vis-network](https://npm-compare.com/cytoscape,vis-network)
