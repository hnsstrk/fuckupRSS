<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import cytoscape from "cytoscape";

  interface GraphNode {
    id: number;
    name: string;
    count: number;
    article_count: number;
    cluster_id: number | null;
    primary_category_id: number | null;
    primary_category_name: string | null;
  }

  interface GraphEdge {
    source: number;
    target: number;
    weight: number;
    cooccurrence: number;
  }

  interface NetworkGraphData {
    nodes: GraphNode[];
    edges: GraphEdge[];
  }

  interface Props {
    graphData: NetworkGraphData;
    onNodeClick?: (id: number) => void;
    loading?: boolean;
  }

  let { graphData, onNodeClick, loading = false }: Props = $props();

  let container: HTMLDivElement;
  let cy: cytoscape.Core | null = null;

  // Track state
  let prevGraphDataKey = "";
  let mounted = false;
  let focusedNodeId: string | null = $state(null);
  let showTooltip = $state(false);
  let tooltipContent = $state({ name: "", category: "", articles: 0 });
  let tooltipPosition = $state({ x: 0, y: 0 });

  // Category color mapping (Sephiroth IDs)
  const CATEGORY_COLORS: Record<number, string> = {
    // Main categories (1-6)
    1: "#3B82F6", // Wissen & Technologie - Blue
    2: "#EF4444", // Politik & Gesellschaft - Red
    3: "#10B981", // Wirtschaft - Green
    4: "#22C55E", // Umwelt & Gesundheit - Emerald
    5: "#F59E0B", // Sicherheit - Amber
    6: "#8B5CF6", // Kultur & Leben - Purple
    // Subcategories (101-602)
    101: "#3B82F6", // Technik - Blue
    102: "#06B6D4", // Wissenschaft - Cyan
    201: "#EF4444", // Politik - Red
    202: "#F97316", // Gesellschaft - Orange
    203: "#6366F1", // Recht - Indigo
    301: "#10B981", // Wirtschaft - Green
    302: "#FBBF24", // Energie - Yellow
    401: "#22C55E", // Umwelt - Emerald
    402: "#EC4899", // Gesundheit - Pink
    501: "#F59E0B", // Sicherheit - Amber
    502: "#78716C", // Verteidigung - Stone
    601: "#8B5CF6", // Kultur - Purple
    602: "#14B8A6", // Sport - Teal
  };

  const DEFAULT_NODE_COLOR = "#6B7280"; // Gray for uncategorized

  // Get CSS variable values for Cytoscape
  function getCssVar(name: string): string {
    if (typeof document === "undefined") return "";
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  function getThemeColors() {
    return {
      text: getCssVar("--text-primary"),
      textOutline: getCssVar("--bg-base"),
      edge: getCssVar("--text-muted"),
      edgeHighlight: getCssVar("--accent-primary"),
      selectedBorder: getCssVar("--golden-apple-color"),
      dimmed: getCssVar("--text-faint"),
    };
  }

  function getCategoryColor(categoryId: number | null): string {
    if (categoryId === null) return DEFAULT_NODE_COLOR;
    return CATEGORY_COLORS[categoryId] || DEFAULT_NODE_COLOR;
  }

  function transformData(data: NetworkGraphData): cytoscape.ElementDefinition[] {
    const elements: cytoscape.ElementDefinition[] = [];

    // Find max values for scaling
    const maxArticleCount = Math.max(...data.nodes.map((n) => n.article_count), 1);
    const maxWeight = Math.max(...data.edges.map((e) => e.weight), 0.1);
    const minWeight = Math.min(...data.edges.map((e) => e.weight), 0.01);

    // Add nodes with category colors
    for (const node of data.nodes) {
      const size = 25 + (node.article_count / maxArticleCount) * 45;
      elements.push({
        group: "nodes",
        data: {
          id: String(node.id),
          label: node.name,
          count: node.count,
          articleCount: node.article_count,
          clusterId: node.cluster_id,
          categoryId: node.primary_category_id,
          categoryName: node.primary_category_name || "Unbekannt",
          size: size,
          color: getCategoryColor(node.primary_category_id),
        },
      });
    }

    // Add edges with opacity based on weight
    for (const edge of data.edges) {
      // Normalize weight to 0-1 range for opacity calculation
      const normalizedWeight = (edge.weight - minWeight) / (maxWeight - minWeight);
      const opacity = 0.15 + normalizedWeight * 0.7; // Range: 0.15 to 0.85
      const width = 1 + normalizedWeight * 4;

      elements.push({
        group: "edges",
        data: {
          id: `${edge.source}-${edge.target}`,
          source: String(edge.source),
          target: String(edge.target),
          weight: edge.weight,
          cooccurrence: edge.cooccurrence,
          width: width,
          opacity: opacity,
        },
      });
    }

    return elements;
  }

  function initGraph() {
    if (!container || !graphData || graphData.nodes.length === 0) return;

    // Destroy existing instance
    if (cy) {
      cy.destroy();
      cy = null;
    }

    focusedNodeId = null;
    const colors = getThemeColors();

    cy = cytoscape({
      container,
      elements: transformData(graphData),
      style: [
        // Normal nodes - colored by category
        {
          selector: "node",
          style: {
            "background-color": "data(color)",
            label: "data(label)",
            width: "data(size)",
            height: "data(size)",
            "font-size": "11px",
            "font-weight": 500,
            color: colors.text,
            "text-outline-color": colors.textOutline,
            "text-outline-width": 2,
            "text-valign": "bottom",
            "text-margin-y": 6,
            "border-width": 0,
            "transition-property": "opacity, border-width, border-color",
            "transition-duration": 200,
          },
        },
        // Selected node
        {
          selector: "node:selected",
          style: {
            "border-width": 4,
            "border-color": colors.selectedBorder,
          },
        },
        // Focused node (center of focus mode)
        {
          selector: "node.focused",
          style: {
            "border-width": 4,
            "border-color": colors.selectedBorder,
            "z-index": 999,
          },
        },
        // Neighbor of focused node
        {
          selector: "node.neighbor",
          style: {
            opacity: 1,
            "z-index": 100,
          },
        },
        // Dimmed nodes (not in focus)
        {
          selector: "node.dimmed",
          style: {
            opacity: 0.15,
            label: "",
          },
        },
        // Normal edges - opacity based on weight
        {
          selector: "edge",
          style: {
            width: "data(width)",
            "line-color": colors.edge,
            "curve-style": "bezier",
            opacity: "data(opacity)" as unknown as number,
            "transition-property": "opacity, line-color",
            "transition-duration": 200,
          },
        },
        // Highlighted edges (connected to focused node)
        {
          selector: "edge.highlighted",
          style: {
            "line-color": colors.edgeHighlight,
            opacity: 0.9,
            "z-index": 100,
          },
        },
        // Dimmed edges
        {
          selector: "edge.dimmed",
          style: {
            opacity: 0.05,
          },
        },
      ],
      layout: {
        name: "cose",
        animate: "end" as unknown as boolean,
        animationDuration: 400,
        animationEasing: "ease-out",
        nodeRepulsion: () => 10000,
        idealEdgeLength: () => 120,
        edgeElasticity: () => 100,
        nestingFactor: 1.2,
        gravity: 0.3,
        numIter: 1000,
        coolingFactor: 0.95,
        nodeDimensionsIncludeLabels: true,
      },
      minZoom: 0.2,
      maxZoom: 4,
      wheelSensitivity: 0.3,
    });

    // Node click handler - Focus Mode + Zoom
    cy.on("tap", "node", (evt: cytoscape.EventObject) => {
      const node = evt.target;
      const nodeId = node.id();

      // Toggle focus mode
      if (focusedNodeId === nodeId) {
        // Click same node again - exit focus mode
        exitFocusMode();
      } else {
        // Enter focus mode on this node
        enterFocusMode(nodeId);
      }

      // Notify parent
      onNodeClick?.(parseInt(nodeId, 10));
    });

    // Click on background - exit focus mode
    cy.on("tap", (evt: cytoscape.EventObject) => {
      if (evt.target === cy) {
        exitFocusMode();
      }
    });

    // Hover effects with tooltip
    cy.on("mouseover", "node", (evt: cytoscape.EventObject) => {
      const node = evt.target;

      // Show tooltip
      const data = node.data();
      tooltipContent = {
        name: data.label,
        category: data.categoryName,
        articles: data.articleCount,
      };

      const renderedPos = node.renderedPosition();
      tooltipPosition = {
        x: renderedPos.x,
        y: renderedPos.y - node.renderedWidth() / 2 - 10,
      };
      showTooltip = true;

      container.style.cursor = "pointer";

      // Slight highlight on hover (if not in focus mode or is relevant)
      if (!focusedNodeId || node.hasClass("focused") || node.hasClass("neighbor")) {
        node.style("border-width", 2);
        node.style("border-color", colors.edgeHighlight);
      }
    });

    cy.on("mouseout", "node", (evt: cytoscape.EventObject) => {
      showTooltip = false;
      container.style.cursor = "default";

      const node = evt.target;
      if (!node.hasClass("focused")) {
        node.style("border-width", 0);
      }
    });

    // Fit graph after layout
    cy.on("layoutstop", () => {
      cy?.fit(undefined, 50);
    });
  }

  function enterFocusMode(nodeId: string) {
    if (!cy) return;

    focusedNodeId = nodeId;
    const node = cy.getElementById(nodeId);

    // Get neighbors
    const neighbors = node.neighborhood().nodes();
    const connectedEdges = node.connectedEdges();

    // Reset all classes
    cy.elements().removeClass("focused neighbor highlighted dimmed");

    // Apply focus classes
    node.addClass("focused");
    neighbors.addClass("neighbor");
    connectedEdges.addClass("highlighted");

    // Dim everything else
    cy.nodes().not(node).not(neighbors).addClass("dimmed");
    cy.edges().not(connectedEdges).addClass("dimmed");

    // Zoom to focused node and neighbors
    const focusCollection = node.union(neighbors);
    cy.animate({
      fit: { eles: focusCollection, padding: 80 },
      duration: 300,
      easing: "ease-out",
    });
  }

  function exitFocusMode() {
    if (!cy) return;

    focusedNodeId = null;
    cy.elements().removeClass("focused neighbor highlighted dimmed");

    // Fit all
    cy.animate({
      fit: { eles: cy.elements(), padding: 50 },
      duration: 300,
      easing: "ease-out",
    });
  }

  function handleZoomIn() {
    if (cy) {
      const currentZoom = cy.zoom();
      cy.animate({
        zoom: currentZoom * 1.4,
        duration: 200,
      });
    }
  }

  function handleZoomOut() {
    if (cy) {
      const currentZoom = cy.zoom();
      cy.animate({
        zoom: currentZoom / 1.4,
        duration: 200,
      });
    }
  }

  function handleReset() {
    exitFocusMode();
    if (cy) {
      cy.animate({
        fit: { eles: cy.elements(), padding: 50 },
        duration: 300,
      });
    }
  }

  function generateGraphKey(data: NetworkGraphData | null): string {
    if (!data || data.nodes.length === 0) return "";
    const nodeIds = data.nodes
      .map((n) => n.id)
      .sort((a, b) => a - b)
      .join(",");
    return `${data.nodes.length}-${data.edges.length}-${nodeIds}`;
  }

  onMount(() => {
    mounted = true;
    if (graphData) {
      prevGraphDataKey = generateGraphKey(graphData);
    }
    initGraph();
  });

  onDestroy(() => {
    mounted = false;
    if (cy) {
      cy.destroy();
      cy = null;
    }
  });

  $effect(() => {
    if (!mounted) return;

    if (graphData && container) {
      const currentKey = generateGraphKey(graphData);
      if (currentKey !== prevGraphDataKey) {
        prevGraphDataKey = currentKey;
        initGraph();
      }
    }
  });
</script>

<div class="network-graph-container">
  <!-- Controls -->
  <div class="graph-controls">
    <button
      onclick={handleZoomIn}
      title={$_("network.zoomIn") || "Zoom In"}
      aria-label={$_("network.zoomIn") || "Zoom In"}
    >
      <i class="fa-solid fa-plus" aria-hidden="true"></i>
    </button>
    <button
      onclick={handleZoomOut}
      title={$_("network.zoomOut") || "Zoom Out"}
      aria-label={$_("network.zoomOut") || "Zoom Out"}
    >
      <i class="fa-solid fa-minus" aria-hidden="true"></i>
    </button>
    <button
      onclick={handleReset}
      title={$_("network.resetView") || "Reset"}
      aria-label={$_("network.resetView") || "Reset"}
    >
      <i class="fa-solid fa-expand" aria-hidden="true"></i>
    </button>
  </div>

  <!-- Focus mode indicator -->
  {#if focusedNodeId}
    <div class="focus-indicator">
      <span>{$_("network.focusMode") || "Fokus-Modus"}</span>
      <button
        onclick={exitFocusMode}
        class="exit-focus"
        aria-label={$_("network.exitFocus") || "Beenden"}
      >
        <i class="fa-solid fa-xmark" aria-hidden="true"></i>
        {$_("network.exitFocus") || "Beenden"}
      </button>
    </div>
  {/if}

  <!-- Legend -->
  <div class="graph-legend">
    <div class="legend-title">{$_("network.categories") || "Kategorien"}</div>
    <div class="legend-items">
      <div class="legend-item">
        <span class="legend-dot legend-dot--politik"></span>
        <span>Politik</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot legend-dot--technik"></span>
        <span>Technik</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot legend-dot--wirtschaft"></span>
        <span>Wirtschaft</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot legend-dot--kultur"></span>
        <span>Kultur</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot legend-dot--sicherheit"></span>
        <span>Sicherheit</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot legend-dot--andere"></span>
        <span>Andere</span>
      </div>
    </div>
  </div>

  <!-- Tooltip -->
  {#if showTooltip}
    <div class="graph-tooltip" style="left: {tooltipPosition.x}px; top: {tooltipPosition.y}px">
      <div class="tooltip-name">{tooltipContent.name}</div>
      <div class="tooltip-info">
        <span class="tooltip-category">{tooltipContent.category}</span>
        <span class="tooltip-articles"
          >{tooltipContent.articles} {$_("network.articles") || "Artikel"}</span
        >
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="graph-loading">
      <div class="spinner"></div>
      <span>{$_("network.graphLoading") || "Loading graph..."}</span>
    </div>
  {:else if !graphData || graphData.nodes.length === 0}
    <div class="graph-empty">
      <i class="fa-solid fa-diagram-project empty-icon"></i>
      <span>{$_("network.noData") || "No network data available"}</span>
    </div>
  {/if}

  <div
    bind:this={container}
    class="graph-canvas"
    class:hidden={loading || !graphData || graphData.nodes.length === 0}
  ></div>
</div>

<style>
  .network-graph-container {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 500px;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .graph-canvas {
    width: 100%;
    flex: 1;
    min-height: 0;
  }

  .graph-canvas.hidden {
    display: none;
  }

  /* Controls */
  .graph-controls {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    display: flex;
    gap: 0.375rem;
    z-index: 20;
  }

  .graph-controls button {
    width: 2.25rem;
    height: 2.25rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.875rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }

  .graph-controls button:hover {
    background-color: var(--bg-muted);
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  /* Focus mode indicator */
  .focus-indicator {
    position: absolute;
    top: 0.75rem;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 1rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--accent-primary);
    border-radius: 2rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    z-index: 20;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }

  .exit-focus {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.625rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 1rem;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .exit-focus:hover {
    background-color: var(--bg-muted);
    color: var(--text-primary);
    border-color: var(--accent-primary);
  }

  /* Legend */
  .graph-legend {
    position: absolute;
    bottom: 0.75rem;
    left: 0.75rem;
    padding: 0.625rem 0.875rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    z-index: 15;
    max-width: 200px;
  }

  .legend-title {
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 0.5rem;
  }

  .legend-items {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .legend-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .legend-dot--politik {
    background-color: #ef4444;
  }

  .legend-dot--technik {
    background-color: #3b82f6;
  }

  .legend-dot--wirtschaft {
    background-color: #10b981;
  }

  .legend-dot--kultur {
    background-color: #8b5cf6;
  }

  .legend-dot--sicherheit {
    background-color: #f59e0b;
  }

  .legend-dot--andere {
    background-color: #6b7280;
  }

  /* Tooltip */
  .graph-tooltip {
    position: absolute;
    transform: translate(-50%, -100%);
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
    z-index: 100;
    pointer-events: none;
    white-space: nowrap;
  }

  .tooltip-name {
    font-weight: 600;
    font-size: 0.875rem;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .tooltip-info {
    display: flex;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .tooltip-category {
    color: var(--accent-primary);
  }

  /* Loading & Empty states */
  .graph-loading,
  .graph-empty {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    color: var(--text-muted);
  }

  .empty-icon {
    font-size: 3rem;
    opacity: 0.3;
  }

  .spinner {
    width: 2.5rem;
    height: 2.5rem;
    border: 3px solid var(--border-default);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
