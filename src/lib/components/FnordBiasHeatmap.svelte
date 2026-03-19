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
            <th class="bias-col" title="-2">
              <i class="fa-solid fa-angles-left"></i>
              <span class="bias-label">{$_("articleView.biasStrongLeft") || "Stark links"}</span>
            </th>
            <th class="bias-col" title="-1">
              <i class="fa-solid fa-angle-left"></i>
              <span class="bias-label">{$_("articleView.biasLeanLeft") || "Leicht links"}</span>
            </th>
            <th class="bias-col neutral" title="0">
              <i class="fa-solid fa-minus"></i>
              <span class="bias-label">{$_("articleView.biasNeutral") || "Neutral"}</span>
            </th>
            <th class="bias-col" title="+1">
              <span class="bias-label">{$_("articleView.biasLeanRight") || "Leicht rechts"}</span>
              <i class="fa-solid fa-angle-right"></i>
            </th>
            <th class="bias-col" title="+2">
              <span class="bias-label">{$_("articleView.biasStrongRight") || "Stark rechts"}</span>
              <i class="fa-solid fa-angles-right"></i>
            </th>
            <th class="avg-col">{$_("fnordView.avgBias") || "Ø Tendenz"}</th>
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
    width: auto;
    min-width: 4rem;
    font-size: 0.7rem;
    line-height: 1.2;
  }

  .bias-col i {
    display: block;
    font-size: 0.75rem;
    margin-bottom: 0.15rem;
    color: var(--text-muted);
  }

  .bias-col.neutral i {
    color: var(--accent-primary);
  }

  .bias-label {
    display: block;
    font-size: 0.6rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .avg-col {
    min-width: 5rem;
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
