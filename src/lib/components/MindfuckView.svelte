<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type {
    ReadingProfile,
    BlindSpot,
    CounterPerspective,
    ReadingTrend,
  } from "../types";
  import { appState } from "../stores/state.svelte";

  // Tab state
  let activeTab = $state<"overview" | "blindSpots" | "recommendations" | "trends">("overview");

  // Data state
  let readingProfile = $state<ReadingProfile | null>(null);
  let blindSpots = $state<BlindSpot[]>([]);
  let counterPerspectives = $state<CounterPerspective[]>([]);
  let readingTrends = $state<ReadingTrend[]>([]);

  // Loading states
  let loadingProfile = $state(true);
  let loadingBlindSpots = $state(false);
  let loadingPerspectives = $state(false);
  let loadingTrends = $state(false);

  // Trend period selection
  let trendPeriod = $state<7 | 30 | 90>(30);

  onMount(async () => {
    await loadReadingProfile();
  });

  async function loadReadingProfile() {
    loadingProfile = true;
    try {
      readingProfile = await invoke<ReadingProfile>("get_reading_profile");
    } catch (e) {
      console.error("Failed to load reading profile:", e);
      readingProfile = null;
    } finally {
      loadingProfile = false;
    }
  }

  async function loadBlindSpots() {
    loadingBlindSpots = true;
    try {
      blindSpots = await invoke<BlindSpot[]>("get_blind_spots");
    } catch (e) {
      console.error("Failed to load blind spots:", e);
      blindSpots = [];
    } finally {
      loadingBlindSpots = false;
    }
  }

  async function loadCounterPerspectives() {
    loadingPerspectives = true;
    try {
      counterPerspectives = await invoke<CounterPerspective[]>("get_counter_perspectives", {
        limit: 10,
      });
    } catch (e) {
      console.error("Failed to load counter perspectives:", e);
      counterPerspectives = [];
    } finally {
      loadingPerspectives = false;
    }
  }

  async function loadReadingTrends(days: number) {
    loadingTrends = true;
    try {
      readingTrends = await invoke<ReadingTrend[]>("get_reading_trends", { days });
    } catch (e) {
      console.error("Failed to load reading trends:", e);
      readingTrends = [];
    } finally {
      loadingTrends = false;
    }
  }

  function handleTabChange(tab: typeof activeTab) {
    activeTab = tab;

    // Load data for the tab if not already loaded
    if (tab === "blindSpots" && blindSpots.length === 0) {
      loadBlindSpots();
    } else if (tab === "recommendations" && counterPerspectives.length === 0) {
      loadCounterPerspectives();
    } else if (tab === "trends" && readingTrends.length === 0) {
      loadReadingTrends(trendPeriod);
    }
  }

  function handleTrendPeriodChange(days: 7 | 30 | 90) {
    trendPeriod = days;
    loadReadingTrends(days);
  }

  function getBiasLabel(bias: number | null): string {
    if (bias === null) return $_("articleView.notRated");
    if (bias <= -1.5) return $_("mindfuck.bias.strongLeft");
    if (bias <= -0.5) return $_("mindfuck.bias.left");
    if (bias <= 0.5) return $_("mindfuck.bias.neutral");
    if (bias <= 1.5) return $_("mindfuck.bias.right");
    return $_("mindfuck.bias.strongRight");
  }

  function getSachlichkeitLabel(sach: number | null): string {
    if (sach === null) return $_("articleView.notRated");
    if (sach <= 0.5) return $_("mindfuck.sachlichkeit.highlyEmotional");
    if (sach <= 1.5) return $_("mindfuck.sachlichkeit.emotional");
    if (sach <= 2.5) return $_("mindfuck.sachlichkeit.mixed");
    if (sach <= 3.5) return $_("mindfuck.sachlichkeit.mostlyObjective");
    return $_("mindfuck.sachlichkeit.objective");
  }

  function getBiasColor(bias: number | null): string {
    if (bias === null) return "var(--text-muted)";
    if (bias <= -1.5) return "var(--ctp-red)";
    if (bias <= -0.5) return "var(--ctp-maroon)";
    if (bias <= 0.5) return "var(--ctp-green)";
    if (bias <= 1.5) return "var(--ctp-blue)";
    return "var(--ctp-sapphire)";
  }

  function getSeverityColor(severity: string): string {
    switch (severity) {
      case "high":
        return "var(--status-error)";
      case "medium":
        return "var(--status-warning)";
      case "low":
        return "var(--ctp-yellow)";
      default:
        return "var(--text-muted)";
    }
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return "-";
    const date = new Date(dateStr);
    return date.toLocaleDateString();
  }

  function handleReadArticle(fnordId: number) {
    appState.selectedFnordId = fnordId;
    appState.currentView = "articles";
  }

  // Derived values
  let biasIndicator = $derived.by(() => {
    if (!readingProfile?.avg_political_bias) return $_("mindfuck.bias.balanced");
    const bias = readingProfile.avg_political_bias;
    if (bias < -0.3) return $_("mindfuck.bias.leaningLeft");
    if (bias > 0.3) return $_("mindfuck.bias.leaningRight");
    return $_("mindfuck.bias.balanced");
  });
</script>

<div class="mindfuck-view">
  <div class="mindfuck-header">
    <h2>{$_("mindfuck.title")}</h2>
    <p class="subtitle">{$_("mindfuck.subtitle")}</p>
  </div>

  <!-- Tabs -->
  <div class="tabs">
    <button
      type="button"
      class="tab {activeTab === 'overview' ? 'active' : ''}"
      onclick={() => handleTabChange("overview")}
    >
      {$_("mindfuck.tabs.overview")}
    </button>
    <button
      type="button"
      class="tab {activeTab === 'blindSpots' ? 'active' : ''}"
      onclick={() => handleTabChange("blindSpots")}
    >
      {$_("mindfuck.tabs.blindSpots")}
    </button>
    <button
      type="button"
      class="tab {activeTab === 'recommendations' ? 'active' : ''}"
      onclick={() => handleTabChange("recommendations")}
    >
      {$_("mindfuck.tabs.recommendations")}
    </button>
    <button
      type="button"
      class="tab {activeTab === 'trends' ? 'active' : ''}"
      onclick={() => handleTabChange("trends")}
    >
      {$_("mindfuck.tabs.trends")}
    </button>
  </div>

  <div class="tab-content">
    {#if activeTab === "overview"}
      {#if loadingProfile}
        <div class="loading">{$_("fnordView.loading")}</div>
      {:else if !readingProfile || readingProfile.total_read === 0}
        <div class="no-data">
          <p>{$_("mindfuck.profile.noData")}</p>
        </div>
      {:else}
        <!-- Profile Overview -->
        <div class="section">
          <h3>{$_("mindfuck.profile.title")}</h3>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="stat-value">{readingProfile.total_read}</span>
              <span class="stat-label">{$_("mindfuck.profile.totalRead")}</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{readingProfile.total_articles}</span>
              <span class="stat-label">{$_("mindfuck.profile.totalArticles")}</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{readingProfile.read_percentage.toFixed(1)}%</span>
              <span class="stat-label">{$_("mindfuck.profile.readPercentage")}</span>
            </div>
            <div class="stat-item">
              <span class="stat-value bias-indicator" style="color: {getBiasColor(readingProfile.avg_political_bias)}">
                {biasIndicator}
              </span>
              <span class="stat-label">{$_("mindfuck.profile.avgBias")}</span>
            </div>
          </div>
        </div>

        <!-- Category Distribution -->
        <div class="section">
          <h3>{$_("mindfuck.categories.title")}</h3>
          <div class="category-bars">
            {#each readingProfile.by_category as cat (cat.sephiroth_id)}
              <div class="category-bar-row">
                <div class="category-label">
                  {#if cat.icon}
                    <span class="category-icon">{cat.icon}</span>
                  {/if}
                  <span class="category-name">{cat.name}</span>
                </div>
                <div class="bar-container">
                  <div class="bar-background">
                    <div
                      class="bar-fill read"
                      style="width: {cat.percentage}%; background-color: {cat.color || 'var(--accent-primary)'}"
                    ></div>
                  </div>
                  <span class="bar-value">{cat.read_count} / {cat.total_count}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Political Bias Distribution -->
        <div class="section">
          <h3>{$_("mindfuck.bias.title")}</h3>
          <div class="bias-distribution">
            {#each readingProfile.by_bias as bias (bias.bias_value)}
              <div class="bias-bar-row">
                <div class="bias-label">{bias.label}</div>
                <div class="bar-container">
                  <div class="bar-background">
                    <div
                      class="bar-fill"
                      style="width: {bias.percentage}%; background-color: {getBiasColor(bias.bias_value)}"
                    ></div>
                  </div>
                  <span class="bar-value">{bias.read_count} ({bias.percentage.toFixed(1)}%)</span>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Sachlichkeit Distribution -->
        <div class="section">
          <h3>{$_("mindfuck.sachlichkeit.title")}</h3>
          <div class="sachlichkeit-distribution">
            {#each readingProfile.by_sachlichkeit as sach (sach.sachlichkeit_value)}
              <div class="sach-bar-row">
                <div class="sach-label">{sach.label}</div>
                <div class="bar-container">
                  <div class="bar-background">
                    <div
                      class="bar-fill sachlichkeit"
                      style="width: {sach.percentage}%"
                    ></div>
                  </div>
                  <span class="bar-value">{sach.read_count} ({sach.percentage.toFixed(1)}%)</span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

    {:else if activeTab === "blindSpots"}
      {#if loadingBlindSpots}
        <div class="loading">{$_("fnordView.loading")}</div>
      {:else if blindSpots.length === 0}
        <div class="no-data">
          <p>{$_("mindfuck.blindSpots.noBlindSpots")}</p>
        </div>
      {:else}
        <div class="section">
          <h3>{$_("mindfuck.blindSpots.title")}</h3>
          <p class="section-description">{$_("mindfuck.blindSpots.description")}</p>
          <div class="blind-spots-list">
            {#each blindSpots as spot (spot.name)}
              <div class="blind-spot-item" style="border-left-color: {getSeverityColor(spot.severity)}">
                <div class="blind-spot-header">
                  <span class="blind-spot-name">{spot.name}</span>
                  <span
                    class="blind-spot-severity"
                    style="color: {getSeverityColor(spot.severity)}"
                  >
                    {$_(`mindfuck.blindSpots.severity.${spot.severity}`)}
                  </span>
                </div>
                <p class="blind-spot-description">{spot.description}</p>
                <div class="blind-spot-stats">
                  <span>{spot.read_count} {$_("mindfuck.categories.read")}</span>
                  <span>/</span>
                  <span>{spot.available_count} {$_("mindfuck.categories.available")}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

    {:else if activeTab === "recommendations"}
      {#if loadingPerspectives}
        <div class="loading">{$_("fnordView.loading")}</div>
      {:else if counterPerspectives.length === 0}
        <div class="no-data">
          <p>{$_("mindfuck.counterPerspectives.noRecommendations")}</p>
        </div>
      {:else}
        <div class="section">
          <h3>{$_("mindfuck.counterPerspectives.title")}</h3>
          <p class="section-description">{$_("mindfuck.counterPerspectives.description")}</p>
          <div class="recommendations-list">
            {#each counterPerspectives as article (article.fnord_id)}
              <div class="recommendation-item">
                <div class="recommendation-content">
                  <h4 class="recommendation-title">{article.title}</h4>
                  <div class="recommendation-meta">
                    {#if article.pentacle_title}
                      <span class="recommendation-source">{article.pentacle_title}</span>
                    {/if}
                    {#if article.published_at}
                      <span class="recommendation-date">{formatDate(article.published_at)}</span>
                    {/if}
                    {#if article.political_bias !== null}
                      <span
                        class="recommendation-bias"
                        style="color: {getBiasColor(article.political_bias)}"
                      >
                        {getBiasLabel(article.political_bias)}
                      </span>
                    {/if}
                  </div>
                  <p class="recommendation-reason">{article.reason}</p>
                </div>
                <button
                  type="button"
                  class="btn-read"
                  onclick={() => handleReadArticle(article.fnord_id)}
                >
                  {$_("mindfuck.counterPerspectives.readArticle")}
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}

    {:else if activeTab === "trends"}
      <div class="section">
        <h3>{$_("mindfuck.trends.title")}</h3>
        <p class="section-description">{$_("mindfuck.trends.description")}</p>

        <!-- Period Selector -->
        <div class="trend-period-selector">
          <button
            type="button"
            class="period-btn {trendPeriod === 7 ? 'active' : ''}"
            onclick={() => handleTrendPeriodChange(7)}
          >
            {$_("mindfuck.trends.last7Days")}
          </button>
          <button
            type="button"
            class="period-btn {trendPeriod === 30 ? 'active' : ''}"
            onclick={() => handleTrendPeriodChange(30)}
          >
            {$_("mindfuck.trends.last30Days")}
          </button>
          <button
            type="button"
            class="period-btn {trendPeriod === 90 ? 'active' : ''}"
            onclick={() => handleTrendPeriodChange(90)}
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
                      style="height: {Math.max(4, (trend.read_count / Math.max(...readingTrends.map(t => t.read_count))) * 100)}%"
                      title="{trend.read_count} {$_('mindfuck.trends.articlesRead')}"
                    ></div>
                  </div>
                  <span class="trend-date">{new Date(trend.date).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}</span>
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
                <span class="trend-stat-value" style="color: {getBiasColor(readingTrends.filter(t => t.avg_bias !== null).reduce((sum, t, _, arr) => sum + (t.avg_bias || 0) / arr.length, 0) || null)}">
                  {getBiasLabel(readingTrends.filter(t => t.avg_bias !== null).length > 0
                    ? readingTrends.filter(t => t.avg_bias !== null).reduce((sum, t, _, arr) => sum + (t.avg_bias || 0) / arr.length, 0)
                    : null)}
                </span>
                <span class="trend-stat-label">{$_("mindfuck.trends.avgBiasOverTime")}</span>
              </div>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .mindfuck-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    background-color: var(--bg-surface);
    overflow: hidden;
  }

  .mindfuck-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .mindfuck-header h2 {
    margin: 0;
    font-size: 1.25rem;
    color: var(--accent-primary);
  }

  .subtitle {
    margin: 0.25rem 0 0 0;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  /* Tabs */
  .tabs {
    display: flex;
    gap: 0.25rem;
    padding: 0 1.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .tab {
    padding: 0.75rem 1rem;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 0.875rem;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: all 0.2s;
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
  }

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

  /* Stats Grid */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 1rem;
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
  }

  .stat-item {
    text-align: center;
  }

  .stat-value {
    display: block;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--accent-primary);
  }

  .stat-value.bias-indicator {
    font-size: 1rem;
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Category Bars */
  .category-bars {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .category-bar-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .category-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 140px;
  }

  .category-icon {
    font-size: 1rem;
  }

  .category-name {
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .bar-container {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .bar-background {
    flex: 1;
    height: 8px;
    background-color: var(--bg-overlay);
    border-radius: 4px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .bar-fill.read {
    opacity: 0.8;
  }

  .bar-fill.sachlichkeit {
    background-color: var(--ctp-teal);
  }

  .bar-value {
    min-width: 80px;
    text-align: right;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Bias Distribution */
  .bias-distribution,
  .sachlichkeit-distribution {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .bias-bar-row,
  .sach-bar-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .bias-label,
  .sach-label {
    min-width: 140px;
    font-size: 0.875rem;
    color: var(--text-primary);
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

  /* Blind Spots */
  .blind-spots-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .blind-spot-item {
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
    border-left-width: 4px;
  }

  .blind-spot-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .blind-spot-name {
    font-weight: 600;
    color: var(--text-primary);
  }

  .blind-spot-severity {
    font-size: 0.75rem;
    font-weight: 500;
  }

  .blind-spot-description {
    margin: 0 0 0.5rem 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .blind-spot-stats {
    display: flex;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Recommendations */
  .recommendations-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .recommendation-item {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
  }

  .recommendation-content {
    flex: 1;
    min-width: 0;
  }

  .recommendation-title {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .recommendation-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .recommendation-source {
    color: var(--accent-secondary);
  }

  .recommendation-bias {
    font-weight: 500;
  }

  .recommendation-reason {
    margin: 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-style: italic;
  }

  .btn-read {
    padding: 0.5rem 1rem;
    border: 1px solid var(--accent-primary);
    border-radius: 0.375rem;
    background: none;
    color: var(--accent-primary);
    font-size: 0.875rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .btn-read:hover {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
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
    .stats-grid {
      grid-template-columns: repeat(2, 1fr);
    }

    .category-bar-row,
    .bias-bar-row,
    .sach-bar-row {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.5rem;
    }

    .category-label,
    .bias-label,
    .sach-label {
      min-width: auto;
    }

    .bar-container {
      width: 100%;
    }

    .recommendation-item {
      flex-direction: column;
    }

    .btn-read {
      align-self: flex-start;
    }

    .trend-period-selector {
      flex-wrap: wrap;
    }
  }
</style>
