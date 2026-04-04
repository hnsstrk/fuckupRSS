<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { FnordStats } from "../stores/state.svelte";
  import {
    getBiasColor,
    getBiasRangeLabel,
    getSachlichkeitRangeLabel,
  } from "$lib/utils/articleFormat";
  import type { GreyfaceIndex } from "../types";
  import Tooltip from "./Tooltip.svelte";

  let {
    stats,
    greyfaceIndex,
  }: {
    stats: FnordStats;
    greyfaceIndex: GreyfaceIndex | null;
  } = $props();

  function getGreyfaceLevel(index: number): string {
    if (index < 20) return "excellent";
    if (index < 40) return "good";
    if (index < 60) return "moderate";
    if (index < 80) return "concerning";
    return "critical";
  }
</script>

<!-- Top Row: Greyface Index + By Source -->
<div class="stats-row top-row">
  <!-- Greyface Index Card -->
  {#if greyfaceIndex && greyfaceIndex.articles_with_bias > 0}
    <div class="stats-card greyface-card">
      <h3 class="card-title">
        <i class="fa-solid fa-triangle-exclamation"></i>
        {$_("fnordView.greyface.title") || "Greyface-Index"}
        <Tooltip
          content={$_("fnordView.greyface.help") ||
            "Misst die durchschnittliche politische Tendenz und Sachlichkeit deiner gelesenen Artikel."}
        >
          <i class="fa-solid fa-circle-info help-icon"></i>
        </Tooltip>
      </h3>
      <div class="greyface-content">
        <div class="greyface-gauge">
          <div
            class="gauge-fill {getGreyfaceLevel(greyfaceIndex.index)}"
            style="--gauge-value: {greyfaceIndex.index}%"
          ></div>
          <span class="gauge-value">{greyfaceIndex.index.toFixed(0)}</span>
        </div>
        <div class="greyface-details">
          <div class="detail-row">
            <span class="detail-label">{$_("fnordView.greyface.avgBias")}</span>
            <span
              class="detail-value bias-{getBiasColor(greyfaceIndex.avg_political_bias, 'class')}"
              title="{greyfaceIndex.avg_political_bias.toFixed(2)}"
            >
              {getBiasRangeLabel(greyfaceIndex.avg_political_bias, $_)}
            </span>
          </div>
          <div class="detail-row">
            <span class="detail-label">{$_("fnordView.greyface.avgSachlichkeit")}</span>
            <span
              class="detail-value"
              title="{greyfaceIndex.avg_sachlichkeit.toFixed(2)}"
            >
              {getSachlichkeitRangeLabel(greyfaceIndex.avg_sachlichkeit, $_)}
            </span>
          </div>
          <div class="detail-row">
            <span class="detail-label"
              >{$_("fnordView.greyface.articlesWithBias") || "Mit Bias-Daten"}</span
            >
            <span class="detail-value"
              >{greyfaceIndex.articles_with_bias} / {greyfaceIndex.total_articles}</span
            >
          </div>
        </div>
        <!-- Bias Distribution Bar -->
        <div class="bias-distribution">
          <div class="distribution-label">
            {$_("fnordView.greyface.distribution") || "Bias-Verteilung"}
          </div>
          <div class="distribution-bar">
            {#if greyfaceIndex.articles_with_bias > 0}
              {@const total = greyfaceIndex.articles_with_bias}
              <div
                class="dist-segment left-extreme"
                style="width: {(greyfaceIndex.bias_distribution.left_extreme / total) * 100}%"
                title="{$_('fnordView.greyface.leftExtreme')}: {greyfaceIndex.bias_distribution
                  .left_extreme}"
              ></div>
              <div
                class="dist-segment left-leaning"
                style="width: {(greyfaceIndex.bias_distribution.left_leaning / total) * 100}%"
                title="{$_('fnordView.greyface.leftLeaning')}: {greyfaceIndex.bias_distribution
                  .left_leaning}"
              ></div>
              <div
                class="dist-segment neutral"
                style="width: {(greyfaceIndex.bias_distribution.neutral / total) * 100}%"
                title="{$_('fnordView.greyface.neutral')}: {greyfaceIndex.bias_distribution
                  .neutral}"
              ></div>
              <div
                class="dist-segment right-leaning"
                style="width: {(greyfaceIndex.bias_distribution.right_leaning / total) * 100}%"
                title="{$_('fnordView.greyface.rightLeaning')}: {greyfaceIndex.bias_distribution
                  .right_leaning}"
              ></div>
              <div
                class="dist-segment right-extreme"
                style="width: {(greyfaceIndex.bias_distribution.right_extreme / total) * 100}%"
                title="{$_('fnordView.greyface.rightExtreme')}: {greyfaceIndex.bias_distribution
                  .right_extreme}"
              ></div>
            {/if}
          </div>
          <div class="distribution-legend">
            <span class="legend-item"><span class="legend-dot left-extreme"></span> -2</span>
            <span class="legend-item"><span class="legend-dot left-leaning"></span> -1</span>
            <span class="legend-item"><span class="legend-dot neutral"></span> 0</span>
            <span class="legend-item"><span class="legend-dot right-leaning"></span> +1</span>
            <span class="legend-item"><span class="legend-dot right-extreme"></span> +2</span>
          </div>
        </div>
      </div>
    </div>
  {:else}
    <!-- Greyface Placeholder when no data -->
    <div class="stats-card greyface-card greyface-empty">
      <h3 class="card-title">
        <i class="fa-solid fa-triangle-exclamation"></i>
        {$_("fnordView.greyface.title") || "Greyface-Index"}
        <Tooltip
          content={$_("fnordView.greyface.help") ||
            "Misst die durchschnittliche politische Tendenz und Sachlichkeit deiner gelesenen Artikel."}
        >
          <i class="fa-solid fa-circle-info help-icon"></i>
        </Tooltip>
      </h3>
      <div class="empty-placeholder">
        <i class="fa-solid fa-chart-pie empty-icon"></i>
        <p>{$_("fnordView.greyface.noData") || "Keine Bias-Daten vorhanden"}</p>
        <span class="empty-hint"
          >{$_("fnordView.greyface.noDataHint") ||
            "Führe die KI-Analyse durch, um Bias-Werte zu erhalten."}</span
        >
      </div>
    </div>
  {/if}

  <!-- By Source Card -->
  {#if stats.by_source.length > 0}
    {@const maxSourceRevisions = Math.max(...stats.by_source.map((s) => s.revision_count), 1)}
    <div class="stats-card source-card">
      <h3 class="card-title">
        <i class="fa-solid fa-rss"></i>
        {$_("fnordView.bySource") || "Nach Quelle"}
        <Tooltip
          content={$_("fnordView.bySource.help") ||
            "Zeigt die Anzahl der Revisionen und Artikel pro Feed-Quelle."}
        >
          <i class="fa-solid fa-circle-info help-icon"></i>
        </Tooltip>
      </h3>
      <div class="source-list">
        {#each stats.by_source.slice(0, 6) as source (source.pentacle_id)}
          {@const barWidth = (source.revision_count / maxSourceRevisions) * 100}
          <div class="source-item">
            <span class="source-name" title={source.title}
              >{source.title || `Feed #${source.pentacle_id}`}</span
            >
            <div class="source-bar-wrapper">
              <div class="source-progress">
                <div class="source-progress-fill" style="width: {barWidth}%"></div>
              </div>
            </div>
            <span class="source-stats">
              <span class="source-count">{source.revision_count}</span>
              <span class="source-articles">({source.article_count})</span>
            </span>
          </div>
        {/each}
        {#if stats.by_source.length > 6}
          <div class="source-more">
            +{stats.by_source.length - 6}
            {$_("fnordView.moreSources") || "weitere"}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
  }

  .stats-row.top-row {
    grid-template-columns: 1fr 2fr;
  }

  @media (max-width: 900px) {
    .stats-row.top-row {
      grid-template-columns: 1fr;
    }
  }

  .stats-card {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    padding: 1rem;
    border: 1px solid var(--border-default);
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

  /* Source Card */
  .source-card {
    overflow: hidden;
  }

  .source-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .source-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .source-name {
    font-size: 0.75rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 140px;
    flex-shrink: 0;
  }

  .source-bar-wrapper {
    flex: 1;
    min-width: 80px;
  }

  .source-progress {
    height: 6px;
    background-color: var(--bg-base);
    border-radius: 3px;
    overflow: hidden;
  }

  .source-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-primary), var(--category-3));
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .source-stats {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
    min-width: 50px;
    justify-content: flex-end;
  }

  .source-count {
    font-size: 0.8125rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .source-articles {
    font-size: 0.625rem;
    color: var(--text-muted);
  }

  .source-more {
    text-align: center;
    font-size: 0.6875rem;
    color: var(--text-muted);
    padding-top: 0.375rem;
    border-top: 1px solid var(--border-default);
    margin-top: 0.25rem;
  }

  /* Empty Placeholder */
  .empty-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 1.5rem 1rem;
    text-align: center;
    color: var(--text-muted);
  }

  .empty-placeholder .empty-icon {
    font-size: 2rem;
    opacity: 0.3;
    margin-bottom: 0.75rem;
  }

  .empty-placeholder p {
    margin: 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .empty-placeholder .empty-hint {
    font-size: 0.75rem;
    margin-top: 0.25rem;
    opacity: 0.7;
  }

  .greyface-empty {
    display: flex;
    flex-direction: column;
  }

  /* Greyface Index Card */
  .greyface-content {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .greyface-gauge {
    position: relative;
    height: 1rem;
    background: linear-gradient(
      90deg,
      var(--category-4) 0%,
      var(--category-3) 40%,
      var(--category-5) 70%,
      var(--category-2) 100%
    );
    border-radius: 0.5rem;
    overflow: hidden;
  }

  .gauge-fill {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    width: var(--gauge-value);
    background: var(--bg-base);
    opacity: 0.3;
    transition: width 0.3s ease;
  }

  .gauge-value {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.75rem;
    font-weight: 700;
    color: white;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .greyface-details {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
  }

  .detail-label {
    color: var(--text-muted);
  }

  .detail-value {
    font-weight: 600;
    color: var(--text-primary);
  }

  /* Bias Distribution */
  .bias-distribution {
    margin-top: 0.5rem;
  }

  .distribution-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
    margin-bottom: 0.375rem;
  }

  .distribution-bar {
    display: flex;
    height: 0.5rem;
    border-radius: 0.25rem;
    overflow: hidden;
    background: var(--bg-base);
  }

  .dist-segment {
    height: 100%;
    min-width: 2px;
    transition: width 0.3s ease;
  }

  .dist-segment.left-extreme {
    background: var(--category-2);
  }
  .dist-segment.left-leaning {
    background: color-mix(in srgb, var(--category-2) 60%, var(--bg-surface));
  }
  .dist-segment.neutral {
    background: var(--text-muted);
  }
  .dist-segment.right-leaning {
    background: color-mix(in srgb, var(--category-5) 60%, var(--bg-surface));
  }
  .dist-segment.right-extreme {
    background: var(--category-5);
  }

  .distribution-legend {
    display: flex;
    justify-content: space-between;
    margin-top: 0.25rem;
    font-size: 0.625rem;
    color: var(--text-muted);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .legend-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
  }

  .legend-dot.left-extreme {
    background: var(--category-2);
  }
  .legend-dot.left-leaning {
    background: color-mix(in srgb, var(--category-2) 60%, var(--bg-surface));
  }
  .legend-dot.neutral {
    background: var(--text-muted);
  }
  .legend-dot.right-leaning {
    background: color-mix(in srgb, var(--category-5) 60%, var(--bg-surface));
  }
  .legend-dot.right-extreme {
    background: var(--category-5);
  }
</style>
