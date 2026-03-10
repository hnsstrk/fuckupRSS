<script lang="ts">
  import { _ } from "svelte-i18n";
  import { getBiasColor } from "$lib/utils/articleFormat";
  import type { BiasHeatmapEntry } from "../types";

  let {
    biasHeatmap,
  }: {
    biasHeatmap: BiasHeatmapEntry[];
  } = $props();
</script>

{#if biasHeatmap.length > 0}
  <div class="stats-card full-width">
    <h3 class="card-title">
      <i class="fa-solid fa-table-cells"></i>
      {$_("fnordView.biasHeatmap") || "Bias-Heatmap"}
    </h3>
    <div class="heatmap-container">
      <table class="heatmap-table">
        <thead>
          <tr>
            <th>{$_("fnordView.source") || "Quelle"}</th>
            <th class="bias-col">-2</th>
            <th class="bias-col">-1</th>
            <th class="bias-col">0</th>
            <th class="bias-col">+1</th>
            <th class="bias-col">+2</th>
            <th>{$_("fnordView.avgBias") || "Avg"}</th>
          </tr>
        </thead>
        <tbody>
          {#each biasHeatmap.slice(0, 10) as entry (entry.pentacle_id)}
            {@const maxCell = Math.max(
              entry.bias_minus_2,
              entry.bias_minus_1,
              entry.bias_0,
              entry.bias_plus_1,
              entry.bias_plus_2,
              1,
            )}
            <tr>
              <td class="source-cell">{entry.pentacle_title || `Feed #${entry.pentacle_id}`}</td>
              <td class="heatmap-cell" style="--intensity: {entry.bias_minus_2 / maxCell}">
                {entry.bias_minus_2 || ""}
              </td>
              <td class="heatmap-cell" style="--intensity: {entry.bias_minus_1 / maxCell}">
                {entry.bias_minus_1 || ""}
              </td>
              <td class="heatmap-cell neutral-cell" style="--intensity: {entry.bias_0 / maxCell}">
                {entry.bias_0 || ""}
              </td>
              <td class="heatmap-cell" style="--intensity: {entry.bias_plus_1 / maxCell}">
                {entry.bias_plus_1 || ""}
              </td>
              <td class="heatmap-cell" style="--intensity: {entry.bias_plus_2 / maxCell}">
                {entry.bias_plus_2 || ""}
              </td>
              <td class="avg-cell" style="color: {getBiasColor(entry.avg_bias)}">
                {entry.avg_bias.toFixed(2)}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
{/if}

<style>
  .stats-card {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    padding: 1rem;
    border: 1px solid var(--border-default);
  }

  .stats-card.full-width {
    grid-column: 1 / -1;
  }

  .card-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 1rem 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .card-title i {
    color: var(--accent-primary);
  }

  /* Heatmap */
  .heatmap-container {
    overflow-x: auto;
  }

  .heatmap-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.75rem;
  }

  .heatmap-table th {
    padding: 0.5rem;
    text-align: center;
    color: var(--text-muted);
    font-weight: 500;
    border-bottom: 1px solid var(--border-default);
  }

  .heatmap-table th:first-child {
    text-align: left;
  }

  .bias-col {
    width: 3rem;
  }

  .heatmap-table td {
    padding: 0.375rem;
    text-align: center;
  }

  .source-cell {
    text-align: left;
    color: var(--text-primary);
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .heatmap-cell {
    background: color-mix(in srgb, var(--accent-primary) calc(var(--intensity) * 50%), transparent);
    color: var(--text-primary);
    font-weight: 500;
  }

  .heatmap-cell.neutral-cell {
    background: color-mix(in srgb, var(--text-muted) calc(var(--intensity) * 30%), transparent);
  }

  .avg-cell {
    font-weight: 600;
  }
</style>
