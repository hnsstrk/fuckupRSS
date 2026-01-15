<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import cytoscape from 'cytoscape';

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

  // Track previous graphData to prevent unnecessary re-renders
  let prevGraphDataKey = '';
  let mounted = false;

  // Get CSS variable values for Cytoscape (which doesn't support CSS vars)
  function getCssVar(name: string, fallback: string): string {
    if (typeof document === 'undefined') return fallback;
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return value || fallback;
  }

  function getThemeColors() {
    return {
      primary: getCssVar('--accent-primary', '#fab387'),
      primaryHover: getCssVar('--accent-secondary', '#f9e2af'),
      selected: getCssVar('--golden-apple-color', '#f9e2af'),
      text: getCssVar('--text-primary', '#cdd6f4'),
      textOutline: getCssVar('--bg-base', '#1e1e2e'),
      edge: getCssVar('--text-muted', '#585b70'),
      edgeSelected: getCssVar('--accent-primary', '#fab387'),
    };
  }

  function transformData(data: NetworkGraphData): cytoscape.ElementDefinition[] {
    const elements: cytoscape.ElementDefinition[] = [];

    // Find max article_count for scaling
    const maxArticleCount = Math.max(...data.nodes.map(n => n.article_count), 1);

    // Add nodes
    for (const node of data.nodes) {
      const size = 20 + (node.article_count / maxArticleCount) * 40;
      elements.push({
        group: 'nodes',
        data: {
          id: String(node.id),
          label: node.name,
          count: node.count,
          articleCount: node.article_count,
          clusterId: node.cluster_id,
          size: size,
        },
      });
    }

    // Find max weight for scaling
    const maxWeight = Math.max(...data.edges.map(e => e.weight), 0.1);

    // Add edges
    for (const edge of data.edges) {
      const width = 1 + (edge.weight / maxWeight) * 4;
      elements.push({
        group: 'edges',
        data: {
          id: `${edge.source}-${edge.target}`,
          source: String(edge.source),
          target: String(edge.target),
          weight: edge.weight,
          cooccurrence: edge.cooccurrence,
          width: width,
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

    // Get current theme colors
    const colors = getThemeColors();

    cy = cytoscape({
      container,
      elements: transformData(graphData),
      style: [
        {
          selector: 'node',
          style: {
            'background-color': colors.primary,
            'label': 'data(label)',
            'width': 'data(size)',
            'height': 'data(size)',
            'font-size': '10px',
            'color': colors.text,
            'text-outline-color': colors.textOutline,
            'text-outline-width': 2,
            'text-valign': 'bottom',
            'text-margin-y': 5,
          },
        },
        {
          selector: 'node:selected',
          style: {
            'background-color': colors.selected,
            'border-width': 3,
            'border-color': colors.primary,
          },
        },
        {
          selector: 'edge',
          style: {
            'width': 'data(width)',
            'line-color': colors.edge,
            'curve-style': 'bezier',
            'opacity': 0.6,
          },
        },
        {
          selector: 'edge:selected',
          style: {
            'line-color': colors.edgeSelected,
            'opacity': 1,
          },
        },
      ],
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
      },
      minZoom: 0.2,
      maxZoom: 3,
      wheelSensitivity: 0.3,
    });

    // Node click handler
    cy.on('tap', 'node', (evt: cytoscape.EventObject) => {
      const nodeId = parseInt(evt.target.id(), 10);
      onNodeClick?.(nodeId);
    });

    // Hover effects
    cy.on('mouseover', 'node', (evt: cytoscape.EventObject) => {
      evt.target.style('background-color', colors.primaryHover);
      container.style.cursor = 'pointer';
    });

    cy.on('mouseout', 'node', (evt: cytoscape.EventObject) => {
      if (!evt.target.selected()) {
        evt.target.style('background-color', colors.primary);
      }
      container.style.cursor = 'default';
    });

    // Fit graph to container after layout completes
    cy.on('layoutstop', () => {
      cy?.fit(undefined, 40);
    });
  }

  function handleZoomIn() {
    if (cy) {
      cy.zoom(cy.zoom() * 1.3);
    }
  }

  function handleZoomOut() {
    if (cy) {
      cy.zoom(cy.zoom() / 1.3);
    }
  }

  function handleReset() {
    if (cy) {
      cy.fit(undefined, 30);
    }
  }

  // Generate a stable key that includes node IDs
  function generateGraphKey(data: NetworkGraphData | null): string {
    if (!data || data.nodes.length === 0) return '';
    const nodeIds = data.nodes.map(n => n.id).sort((a, b) => a - b).join(',');
    return `${data.nodes.length}-${data.edges.length}-${nodeIds}`;
  }

  onMount(() => {
    mounted = true;
    // Initialize tracking value
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

  // Re-initialize when graphData changes (with stability check)
  $effect(() => {
    // Only run after mount
    if (!mounted) return;

    if (graphData && container) {
      // Create a stable key that includes node IDs
      const currentKey = generateGraphKey(graphData);

      // Only re-initialize if data actually changed
      if (currentKey !== prevGraphDataKey) {
        prevGraphDataKey = currentKey;
        initGraph();
      }
    }
  });
</script>

<div class="network-graph-container">
  <div class="graph-controls">
    <button onclick={handleZoomIn} title={$_('network.zoomIn') || 'Zoom In'}>+</button>
    <button onclick={handleZoomOut} title={$_('network.zoomOut') || 'Zoom Out'}>-</button>
    <button onclick={handleReset} title={$_('network.resetView') || 'Reset'}></button>
  </div>

  {#if loading}
    <div class="graph-loading">
      <div class="spinner"></div>
      <span>{$_('network.graphLoading') || 'Loading graph...'}</span>
    </div>
  {:else if !graphData || graphData.nodes.length === 0}
    <div class="graph-empty">
      <span>{$_('network.noData') || 'No network data available'}</span>
    </div>
  {/if}

  <div bind:this={container} class="graph-canvas" class:hidden={loading || !graphData || graphData.nodes.length === 0}></div>
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

  .graph-controls {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    display: flex;
    gap: 0.25rem;
    z-index: 10;
  }

  .graph-controls button {
    width: 2rem;
    height: 2rem;
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    font-size: 1rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }

  .graph-controls button:hover {
    background-color: var(--bg-muted);
    border-color: var(--accent-primary);
  }

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

  .spinner {
    width: 2rem;
    height: 2rem;
    border: 3px solid var(--border-default);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
