<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { ReadingTrend } from "../types";
  import { getBiasColor } from "$lib/utils/articleFormat";

  let {
    readingTrends,
    loadingTrends,
    trendPeriod,
    onTrendPeriodChange,
  }: {
    readingTrends: ReadingTrend[];
    loadingTrends: boolean;
    trendPeriod: 7 | 30 | 90;
    onTrendPeriodChange: (days: 7 | 30 | 90) => void;
  } = $props();

  function getBiasLabel(bias: number | null): string {
    if (bias === null) return $_("articleView.notRated");
    if (bias <= -1.5) return $_("mindfuck.bias.strongLeft");
    if (bias <= -0.5) return $_("mindfuck.bias.left");
    if (bias <= 0.5) return $_("mindfuck.bias.neutral");
    if (bias <= 1.5) return $_("mindfuck.bias.right");
    return $_("mindfuck.bias.strongRight");
  }
</script>

<div class="section">
  <h3>{$_("mindfuck.trends.title")}</h3>
  <p class="section-description">{$_("mindfuck.trends.description")}</p>

  <!-- Period Selector -->
  <div class="trend-period-selector">
    <button
      type="button"
      class="period-btn {trendPeriod === 7 ? 'active' : ''}"
      onclick={() => onTrendPeriodChange(7)}
    >
      {$_("mindfuck.trends.last7Days")}
    </button>
    <button
      type="button"
      class="period-btn {trendPeriod === 30 ? 'active' : ''}"
      onclick={() => onTrendPeriodChange(30)}
    >
      {$_("mindfuck.trends.last30Days")}
    </button>
    <button
      type="button"
      class="period-btn {trendPeriod === 90 ? 'active' : ''}"
      onclick={() => onTrendPeriodChange(90)}
    >
      {$_("mindfuck.trends.last90Days")}
    </button>
  </div>

  {#if loadingTrends}
    <div class="loading">{$_("fnordView.loading")}</div>
  {:else if readingTrends.length === 0}
    <div class="no-data">
      <p>{$_("mindfuck.trends.noTrends")}</p>
    </div>
  {:else}
    <div class="trends-chart">
      <!-- Simple bar chart for reading trends -->
      <div class="trend-bars">
        {#each readingTrends as trend (trend.date)}
          <div class="trend-bar-column">
            <div class="trend-bar-wrapper">
              <div
                class="trend-bar"
                style="height: {Math.max(
                  4,
                  (trend.read_count / Math.max(...readingTrends.map((t) => t.read_count))) * 100,
                )}%"
                title="{trend.read_count} {$_('mindfuck.trends.articlesRead')}"
              ></div>
            </div>
            <span class="trend-date"
              >{new Date(trend.date).toLocaleDateString(undefined, {
                month: "short",
                day: "numeric",
              })}</span
            >
          </div>
        {/each}
      </div>

      <!-- Trend summary -->
      <div class="trend-summary">
        <div class="trend-stat">
          <span class="trend-stat-value">
            {readingTrends.reduce((sum, t) => sum + t.read_count, 0)}
          </span>
          <span class="trend-stat-label">{$_("mindfuck.trends.articlesRead")}</span>
        </div>
        <div class="trend-stat">
          <span
            class="trend-stat-value"
            style="color: {getBiasColor(
              readingTrends
                .filter((t) => t.avg_bias !== null)
                .reduce((sum, t, _, arr) => sum + (t.avg_bias || 0) / arr.length, 0) || null,
            )}"
          >
            {getBiasLabel(
              readingTrends.filter((t) => t.avg_bias !== null).length > 0
                ? readingTrends
                    .filter((t) => t.avg_bias !== null)
                    .reduce((sum, t, _, arr) => sum + (t.avg_bias || 0) / arr.length, 0)
                : null,
            )}
          </span>
          <span class="trend-stat-label">{$_("mindfuck.trends.avgBiasOverTime")}</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Sections */
  .section {
    margin-bottom: 2rem;
  }

  .section h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .section-description {
    margin: -0.5rem 0 1rem 0;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  /* Loading & No Data */
  .loading,
  .no-data {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: var(--text-muted);
  }

  .no-data p {
    text-align: center;
    max-width: 400px;
  }

  /* Trends */
  .trend-period-selector {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1.5rem;
  }

  .period-btn {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: none;
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .period-btn:hover {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .period-btn.active {
    border-color: var(--accent-primary);
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .trends-chart {
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
    padding: 1.5rem;
  }

  .trend-bars {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 150px;
    margin-bottom: 1.5rem;
    padding: 0 0.5rem;
    overflow-x: auto;
  }

  .trend-bar-column {
    display: flex;
    flex-direction: column;
    align-items: center;
    flex: 1;
    min-width: 24px;
    max-width: 40px;
  }

  .trend-bar-wrapper {
    flex: 1;
    width: 100%;
    display: flex;
    align-items: flex-end;
  }

  .trend-bar {
    width: 100%;
    background-color: var(--accent-primary);
    border-radius: 2px 2px 0 0;
    min-height: 4px;
    transition: height 0.3s ease;
  }

  .trend-bar:hover {
    filter: brightness(1.2);
  }

  .trend-date {
    margin-top: 0.5rem;
    font-size: 0.625rem;
    color: var(--text-muted);
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    white-space: nowrap;
  }

  .trend-summary {
    display: flex;
    justify-content: center;
    gap: 3rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-default);
  }

  .trend-stat {
    text-align: center;
  }

  .trend-stat-value {
    display: block;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--accent-primary);
  }

  .trend-stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Responsive */
  @media (max-width: 600px) {
    .trend-period-selector {
      flex-wrap: wrap;
    }
  }
</style>
