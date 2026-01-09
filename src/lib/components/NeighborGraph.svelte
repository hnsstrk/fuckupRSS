<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import cytoscape from 'cytoscape';
  import type { KeywordNeighbor } from '../stores/state.svelte';

  interface Props {
    centerId: number;
    centerName: string;
    neighbors: KeywordNeighbor[];
    onNodeClick?: (id: number) => void;
  }

  let { centerId, centerName, neighbors, onNodeClick }: Props = $props();

  let container: HTMLDivElement;
  // Note: cy is NOT $state() to avoid triggering effects when it changes
  let cy: cytoscape.Core | null = null;

  function buildGraphData(): cytoscape.ElementDefinition[] {
    const elements: cytoscape.ElementDefinition[] = [];

    // Add center node (larger, different color)
    elements.push({
      group: 'nodes',
      data: {
        id: String(centerId),
        label: centerName,
        isCenter: true,
        size: 50,
      },
    });

    // Find max weight for scaling
    const maxWeight = Math.max(...neighbors.map(n => n.combined_weight), 0.1);

    // Add neighbor nodes
    for (const neighbor of neighbors) {
      const size = 20 + (neighbor.combined_weight / maxWeight) * 25;
      elements.push({
        group: 'nodes',
        data: {
          id: String(neighbor.id),
          label: neighbor.name,
          isCenter: false,
          size: size,
          cooccurrence: neighbor.cooccurrence,
          similarity: neighbor.embedding_similarity,
        },
      });

      // Add edge
      const width = 1 + (neighbor.combined_weight / maxWeight) * 4;
      elements.push({
        group: 'edges',
        data: {
          id: `${centerId}-${neighbor.id}`,
          source: String(centerId),
          target: String(neighbor.id),
          weight: neighbor.combined_weight,
          width: width,
        },
      });
    }

    return elements;
  }

  function initGraph() {
    if (!container || neighbors.length === 0) return;

    // Destroy existing instance
    if (cy) {
      cy.destroy();
      cy = null;
    }

    cy = cytoscape({
      container,
      elements: buildGraphData(),
      style: [
        {
          selector: 'node[?isCenter]',
          style: {
            'background-color': '#f9e2af',
            'label': 'data(label)',
            'width': 'data(size)',
            'height': 'data(size)',
            'font-size': '12px',
            'font-weight': 'bold',
            'color': '#cdd6f4',
            'text-outline-color': '#1e1e2e',
            'text-outline-width': 2,
            'text-valign': 'center',
            'text-halign': 'center',
            'border-width': 3,
            'border-color': '#fab387',
          },
        },
        {
          selector: 'node[!isCenter]',
          style: {
            'background-color': '#cba6f7',
            'label': 'data(label)',
            'width': 'data(size)',
            'height': 'data(size)',
            'font-size': '10px',
            'color': '#cdd6f4',
            'text-outline-color': '#1e1e2e',
            'text-outline-width': 2,
            'text-valign': 'bottom',
            'text-margin-y': 5,
          },
        },
        {
          selector: 'node:selected',
          style: {
            'background-color': '#a6e3a1',
            'border-width': 2,
            'border-color': '#94e2d5',
          },
        },
        {
          selector: 'edge',
          style: {
            'width': 'data(width)',
            'line-color': '#585b70',
            'curve-style': 'bezier',
            'opacity': 0.7,
          },
        },
      ],
      layout: {
        name: 'concentric',
        concentric: (node: cytoscape.NodeSingular) => {
          return node.data('isCenter') ? 2 : 1;
        },
        levelWidth: () => 1,
        minNodeSpacing: 50,
        animate: true,
        animationDuration: 300,
      },
      minZoom: 0.5,
      maxZoom: 2,
      wheelSensitivity: 0.3,
      userPanningEnabled: true,
      userZoomingEnabled: true,
      boxSelectionEnabled: false,
    });

    // Node click handler (only for non-center nodes)
    cy.on('tap', 'node[!isCenter]', (evt: cytoscape.EventObject) => {
      const nodeId = parseInt(evt.target.id(), 10);
      onNodeClick?.(nodeId);
    });

    // Hover effects
    cy.on('mouseover', 'node[!isCenter]', (evt: cytoscape.EventObject) => {
      evt.target.style('background-color', '#f5c2e7');
      container.style.cursor = 'pointer';
    });

    cy.on('mouseout', 'node[!isCenter]', (evt: cytoscape.EventObject) => {
      if (!evt.target.selected()) {
        evt.target.style('background-color', '#cba6f7');
      }
      container.style.cursor = 'default';
    });

    // Fit to view after layout
    cy.on('layoutstop', () => {
      cy?.fit(undefined, 20);
    });
  }

  onMount(() => {
    initGraph();
  });

  onDestroy(() => {
    if (cy) {
      cy.destroy();
      cy = null;
    }
  });

  // Re-initialize when data changes
  $effect(() => {
    if (centerId && neighbors && container) {
      initGraph();
    }
  });
</script>

<div class="neighbor-graph-container">
  {#if neighbors.length === 0}
    <div class="graph-empty">
      <span>{$_('network.noNeighbors') || 'No related keywords found'}</span>
    </div>
  {:else}
    <div bind:this={container} class="graph-canvas"></div>
  {/if}
</div>

<style>
  .neighbor-graph-container {
    width: 100%;
    height: 280px;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    overflow: hidden;
  }

  .graph-canvas {
    width: 100%;
    height: 100%;
  }

  .graph-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 0.875rem;
  }
</style>
