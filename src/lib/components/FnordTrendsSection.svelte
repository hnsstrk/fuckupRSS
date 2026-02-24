<script lang="ts">
  import { _ } from "svelte-i18n";
  import type {
    ArticleTimeline,
    KeywordStats,
    FeedActivity,
    KeywordCloudEntry,
  } from "../types";

  let {
    timeline,
    topKeywords,
    feedActivity,
    keywordCloud,
    selectedPeriod,
    extendedStatsLoading,
    extendedStatsError,
    onChangePeriod,
    onRetry,
  }: {
    timeline: ArticleTimeline | null;
    topKeywords: KeywordStats[];
    feedActivity: FeedActivity[];
    keywordCloud: KeywordCloudEntry[];
    selectedPeriod: 7 | 30 | 90;
    extendedStatsLoading: boolean;
    extendedStatsError: string | null;
    onChangePeriod: (days: 7 | 30 | 90) => void;
    onRetry: () => void;
  } = $props();

  function getTrendIcon(trend: number): string {
    if (trend > 10) return "fa-solid fa-arrow-trend-up";
    if (trend < -10) return "fa-solid fa-arrow-trend-down";
    if (trend === 100) return "fa-solid fa-sparkles";
    return "fa-solid fa-minus";
  }

  function getTrendClass(trend: number): string {
    if (trend > 10) return "trend-up";
    if (trend < -10) return "trend-down";
    if (trend === 100) return "trend-new";
    return "trend-stable";
  }

  function getKeywordTypeColor(type: string | null): string {
    switch (type) {
      case "person":
        return "var(--category-2)";
      case "organization":
        return "var(--category-3)";
      case "location":
        return "var(--category-4)";
      case "acronym":
        return "var(--category-5)";
      default:
        return "var(--category-1)";
    }
  }

  function hasTimelineData(data: ArticleTimeline | null): boolean {
    if (!data || data.data.length === 0) return false;
    return data.data.some((d) => d.articles > 0 || d.revisions > 0);
  }

  let hasTrendData = $derived(
    hasTimelineData(timeline) ||
      topKeywords.length > 0 ||
      feedActivity.length > 0 ||
      keywordCloud.length > 0,
  );
</script>

<!-- TRENDS & AKTIVITÄT Header with Period Selector -->
<div class="stats-section-header trends-header">
  <h3 class="section-header-title">
    <i class="fa-solid fa-chart-line"></i>
    {$_("fnordView.trendsAndActivity") || "Trends & Aktivität"}
  </h3>
  <div class="period-selector">
    <span class="period-label">{$_("fnordView.period") || "Zeitraum"}:</span>
    <div class="period-buttons">
      <button
        class="period-btn"
        class:active={selectedPeriod === 7}
        onclick={() => onChangePeriod(7)}
        disabled={extendedStatsLoading}
      >
        {$_("fnordView.days7") || "7 Tage"}
      </button>
      <button
        class="period-btn"
        class:active={selectedPeriod === 30}
        onclick={() => onChangePeriod(30)}
        disabled={extendedStatsLoading}
      >
        {$_("fnordView.days30") || "30 Tage"}
      </button>
      <button
        class="period-btn"
        class:active={selectedPeriod === 90}
        onclick={() => onChangePeriod(90)}
        disabled={extendedStatsLoading}
      >
        {$_("fnordView.days90") || "90 Tage"}
      </button>
    </div>
  </div>
</div>

<!-- Loading state -->
{#if extendedStatsLoading}
  <div class="trends-loading-state">
    <div class="spinner"></div>
    <span>{$_("fnordView.loadingTrends") || "Trend-Daten werden geladen..."}</span>
  </div>
{:else if extendedStatsError}
  <!-- Error state -->
  <div class="trends-error-state">
    <i class="fa-solid fa-exclamation-triangle error-icon"></i>
    <p>{$_("fnordView.trendsError") || "Fehler beim Laden der Trend-Daten"}</p>
    <span class="error-message">{extendedStatsError}</span>
    <button class="retry-btn" onclick={onRetry}>
      <i class="fa-solid fa-refresh"></i>
      {$_("fnordView.retry") || "Erneut versuchen"}
    </button>
  </div>
{:else if !hasTrendData}
  <!-- Empty state -->
  <div class="trends-empty-state">
    <i class="fa-solid fa-chart-line-down empty-icon"></i>
    <p>{$_("fnordView.noTrendData") || "Keine Trend-Daten vorhanden"}</p>
    <span class="empty-hint"
      >{$_("fnordView.noTrendDataHint") ||
        "Trend-Daten werden verfügbar, sobald Artikel synchronisiert und analysiert wurden."}</span
    >
  </div>
{:else}
  <!-- Timeline Card -->
  {#if timeline && hasTimelineData(timeline)}
    {@const timelineData = timeline.data}
    {@const maxArticles = Math.max(...timelineData.map((d) => d.articles), 1)}
    {@const maxRevisions = Math.max(...timelineData.map((d) => d.revisions), 1)}
    {@const maxVal = Math.max(maxArticles, maxRevisions)}
    <div class="stats-card full-width timeline-card">
      <h3 class="card-title">
        <i class="fa-solid fa-chart-area"></i>
        {$_("fnordView.timeline") || "Eris-Chronik"}
      </h3>
      <div class="timeline-chart">
        <div class="chart-bars">
          {#each timelineData as day, i (day.date)}
            <div class="chart-day" title={day.date}>
              <div class="bar-container">
                <div
                  class="bar articles-bar"
                  style="height: {(day.articles / maxVal) * 100}%"
                  title="{$_('fnordView.articles')}: {day.articles}"
                ></div>
                <div
                  class="bar revisions-bar"
                  style="height: {(day.revisions / maxVal) * 100}%"
                  title="{$_('fnordView.revisions')}: {day.revisions}"
                ></div>
              </div>
              {#if i === 0 || i === timelineData.length - 1 || i === Math.floor(timelineData.length / 2)}
                <span class="day-label">{day.date.slice(5)}</span>
              {/if}
            </div>
          {/each}
        </div>
        <div class="chart-legend">
          <span class="legend-item"
            ><span class="legend-bar articles"></span> {$_("fnordView.articles")}</span
          >
          <span class="legend-item"
            ><span class="legend-bar revisions"></span> {$_("fnordView.revisions")}</span
          >
        </div>
      </div>
    </div>
  {/if}

  <!-- Keywords + Feed Activity Row -->
  <div class="stats-row">
    <!-- Top Keywords -->
    {#if topKeywords.length > 0}
      <div class="stats-card">
        <h3 class="card-title">
          <i class="fa-solid fa-hashtag"></i>
          {$_("fnordView.topKeywords") || "Top Keywords"}
        </h3>
        <div class="keyword-list">
          {#each topKeywords as kw, i (kw.id)}
            <div class="keyword-item">
              <span class="keyword-rank">#{i + 1}</span>
              <span
                class="keyword-name"
                style="border-left: 3px solid {getKeywordTypeColor(kw.keyword_type)}; padding-left: 0.5rem;"
              >
                {kw.name}
              </span>
              <span class="keyword-count">{kw.count}</span>
              <span class="keyword-trend {getTrendClass(kw.trend)}">
                <i class={getTrendIcon(kw.trend)}></i>
                {#if kw.trend !== 0 && kw.trend !== 100}
                  {Math.abs(kw.trend).toFixed(0)}%
                {/if}
              </span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Feed Activity -->
    {#if feedActivity.length > 0}
      <div class="stats-card">
        <h3 class="card-title">
          <i class="fa-solid fa-bolt"></i>
          {$_("fnordView.feedActivity") || "Feed-Aktivität"}
        </h3>
        <div class="feed-list">
          {#each feedActivity as feed (feed.pentacle_id)}
            <div class="feed-item">
              <span class="feed-name">{feed.title || `Feed #${feed.pentacle_id}`}</span>
              <div class="feed-stats">
                <span class="feed-stat" title={$_("fnordView.articlesInPeriod")}>
                  <i class="fa-solid fa-newspaper"></i>
                  {feed.articles_period}
                </span>
                <span class="feed-stat" title={$_("fnordView.revisions")}>
                  <i class="fa-solid fa-code-compare"></i>
                  {feed.revisions_period}
                </span>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <!-- Keyword Cloud -->
  {#if keywordCloud.length > 0}
    <div class="stats-card full-width">
      <h3 class="card-title">
        <i class="fa-solid fa-cloud"></i>
        {$_("fnordView.keywordCloud") || "Keyword-Wolke"}
      </h3>
      <div class="keyword-cloud">
        {#each keywordCloud as kw (kw.id)}
          <span
            class="cloud-word"
            style="
            font-size: {0.75 + kw.weight * 1.25}rem;
            opacity: {0.5 + kw.weight * 0.5};
            color: {getKeywordTypeColor(kw.keyword_type)};
          "
            title="{kw.name}: {kw.count} {$_('fnordView.articles')}"
          >
            {kw.name}
          </span>
        {/each}
        <!-- Hidden fnord Easter Egg -->
        <span class="cloud-word fnord-hidden" title="fnord">fnord</span>
      </div>
    </div>
  {/if}
{/if}

<style>
  /* Section Headers */
  .stats-section-header.trends-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 1.5rem;
    padding-top: 1.5rem;
    border-top: 2px solid var(--border-default);
    padding-bottom: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .section-header-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .section-header-title i {
    color: var(--accent-primary);
  }

  /* Period Selector */
  .period-selector {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .period-label {
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }

  .period-buttons {
    display: flex;
    gap: 0.25rem;
  }

  .period-btn {
    padding: 0.25rem 0.75rem;
    font-size: 0.75rem;
    background: var(--bg-base);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .period-btn:hover {
    background: var(--bg-overlay);
    border-color: var(--border-active);
  }

  .period-btn.active {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }

  .period-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Stats Cards */
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

  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
  }

  /* Spinner */
  .spinner {
    width: 2rem;
    height: 2rem;
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

  /* Timeline Chart */
  .timeline-chart {
    height: 150px;
    display: flex;
    flex-direction: column;
  }

  .chart-bars {
    flex: 1;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding-bottom: 1.5rem;
  }

  .chart-day {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
    position: relative;
  }

  .bar-container {
    flex: 1;
    width: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 1px;
  }

  .bar {
    width: 45%;
    border-radius: 2px 2px 0 0;
    transition: height 0.3s ease;
    min-height: 2px;
  }

  .bar.articles-bar {
    background: var(--accent-primary);
  }

  .bar.revisions-bar {
    background: var(--category-5);
  }

  .day-label {
    position: absolute;
    bottom: 0;
    font-size: 0.625rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .chart-legend {
    display: flex;
    justify-content: center;
    gap: 1rem;
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .legend-item {
    display: flex;
    align-items: center;
  }

  .legend-bar {
    display: inline-block;
    width: 1rem;
    height: 0.5rem;
    border-radius: 2px;
    margin-right: 0.25rem;
  }

  .legend-bar.articles {
    background: var(--accent-primary);
  }
  .legend-bar.revisions {
    background: var(--category-5);
  }

  /* Keyword List */
  .keyword-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .keyword-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--bg-base);
    border-radius: 0.375rem;
  }

  .keyword-rank {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    width: 1.5rem;
  }

  .keyword-name {
    flex: 1;
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .keyword-count {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .keyword-trend {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.6875rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }

  .keyword-trend.trend-up {
    color: var(--category-4);
    background: color-mix(in srgb, var(--category-4) 15%, transparent);
  }

  .keyword-trend.trend-down {
    color: var(--category-5);
    background: color-mix(in srgb, var(--category-5) 15%, transparent);
  }

  .keyword-trend.trend-new {
    color: var(--category-3);
    background: color-mix(in srgb, var(--category-3) 15%, transparent);
  }

  .keyword-trend.trend-stable {
    color: var(--text-muted);
  }

  /* Feed List */
  .feed-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .feed-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    background: var(--bg-base);
    border-radius: 0.375rem;
  }

  .feed-name {
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    margin-right: 0.5rem;
  }

  .feed-stats {
    display: flex;
    gap: 0.75rem;
  }

  .feed-stat {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .feed-stat i {
    font-size: 0.625rem;
  }

  /* Keyword Cloud */
  .keyword-cloud {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    justify-content: center;
    padding: 1rem;
  }

  .cloud-word {
    cursor: default;
    transition: transform 0.15s ease;
  }

  .cloud-word:hover {
    transform: scale(1.1);
  }

  .cloud-word.fnord-hidden {
    font-size: 0.5rem !important;
    opacity: 0.05 !important;
    color: var(--text-muted) !important;
    cursor: help;
  }

  .cloud-word.fnord-hidden:hover {
    opacity: 0.3 !important;
  }

  /* Trends Section States */
  .trends-loading-state,
  .trends-error-state,
  .trends-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1.5rem;
    text-align: center;
    color: var(--text-muted);
    gap: 0.75rem;
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    border: 1px dashed var(--border-default);
    margin-top: 1rem;
  }

  .trends-loading-state span,
  .trends-empty-state p,
  .trends-error-state p {
    font-size: 0.9375rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .trends-empty-state .empty-icon,
  .trends-error-state .error-icon {
    font-size: 2.5rem;
    opacity: 0.4;
  }

  .trends-error-state .error-icon {
    color: var(--category-5);
    opacity: 0.7;
  }

  .trends-empty-state .empty-hint,
  .trends-error-state .error-message {
    font-size: 0.8125rem;
    opacity: 0.7;
    max-width: 350px;
  }

  .trends-error-state .error-message {
    font-family: monospace;
    font-size: 0.75rem;
    padding: 0.5rem 1rem;
    background-color: var(--bg-base);
    border-radius: 0.375rem;
    color: var(--text-muted);
  }

  .retry-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    margin-top: 0.5rem;
    font-size: 0.875rem;
    background: var(--accent-primary);
    border: none;
    border-radius: 0.5rem;
    color: white;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .retry-btn:hover {
    background: color-mix(in srgb, var(--accent-primary) 85%, black);
    transform: translateY(-1px);
  }
</style>
