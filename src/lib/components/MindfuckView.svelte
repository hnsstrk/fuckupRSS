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
  import Tabs, { type Tab } from "./Tabs.svelte";
  import { ArticleCard } from "./article";

  // Tab state
  let activeTab = $state<string>("overview");

  // Tabs definition
  let tabs = $derived<Tab[]>([
    { id: "overview", label: $_("mindfuck.tabs.overview") },
    { id: "blindSpots", label: $_("mindfuck.tabs.blindSpots") },
    { id: "recommendations", label: $_("mindfuck.tabs.recommendations") },
    { id: "trends", label: $_("mindfuck.tabs.trends") },
  ]);

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

  // Expanded category for subcategory view
  let expandedCategoryId = $state<number | null>(null);

  function toggleCategoryExpand(categoryId: number) {
    if (expandedCategoryId === categoryId) {
      expandedCategoryId = null;
    } else {
      expandedCategoryId = categoryId;
    }
  }

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

  function handleTabChange(tabId: string) {
    // Load data for the tab if not already loaded
    if (tabId === "blindSpots" && blindSpots.length === 0) {
      loadBlindSpots();
    } else if (tabId === "recommendations" && counterPerspectives.length === 0) {
      loadCounterPerspectives();
    } else if (tabId === "trends" && readingTrends.length === 0) {
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
    window.dispatchEvent(new CustomEvent('navigate-to-article', { detail: { articleId: fnordId } }));
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
  <div class="tabs-wrapper">
    <Tabs {tabs} bind:activeTab onchange={handleTabChange} />
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
        {@const maxRead = Math.max(...readingProfile.by_category.map(c => c.read_count), 1)}
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

        <!-- Category Distribution - Card Layout (like FnordView) -->
        <div class="section">
          <h3>{$_("mindfuck.categories.title")}</h3>
          <div class="category-cards">
            {#each readingProfile.by_category as cat (cat.sephiroth_id)}
              {@const barWidth = (cat.read_count / maxRead) * 100}
              {@const isExpanded = expandedCategoryId === cat.sephiroth_id}
              <button
                class="category-card {isExpanded ? 'expanded' : ''}"
                style="--cat-color: {cat.color || '#6366F1'}"
                onclick={() => toggleCategoryExpand(cat.sephiroth_id)}
              >
                <div class="card-header">
                  <div class="card-icon-wrapper">
                    {#if cat.icon}
                      <i class="{cat.icon}"></i>
                    {:else}
                      <i class="fa-solid fa-folder"></i>
                    {/if}
                  </div>
                  <span class="card-title">{cat.name}</span>
                  <i class="fa-solid fa-chevron-down expand-icon {isExpanded ? 'rotated' : ''}"></i>
                </div>
                <div class="card-stats">
                  <div class="stat-row">
                    <span class="stat-label">{$_("mindfuck.categories.read") || 'Gelesen'}</span>
                    <span class="stat-value">{cat.read_count}</span>
                  </div>
                  <div class="progress-bar">
                    <div class="progress-fill" style="width: {barWidth}%"></div>
                  </div>
                  <div class="stat-row secondary">
                    <span class="stat-label">{$_("mindfuck.categories.available") || 'Verfügbar'}</span>
                    <span class="stat-value">{cat.total_count}</span>
                  </div>
                </div>

                <!-- Subcategories (expanded view) -->
                {#if isExpanded && cat.subcategories && cat.subcategories.length > 0}
                  <div class="subcategories">
                    {#each cat.subcategories as sub (sub.sephiroth_id)}
                      <div class="subcategory-item">
                        <div class="subcategory-info">
                          {#if sub.icon}
                            <i class="{sub.icon} subcategory-icon"></i>
                          {/if}
                          <span class="subcategory-name">{sub.name}</span>
                          {#if sub.percentage < 30 && sub.total_count > 5}
                            <span class="warning-badge" title={$_("mindfuck.blindSpots.lowReadRate")}>!</span>
                          {/if}
                        </div>
                        <div class="subcategory-stats">
                          <span class="subcategory-count" title="{$_('mindfuck.categories.read') || 'Gelesen'}">
                            {sub.read_count}
                          </span>
                          <span class="subcategory-divider">/</span>
                          <span class="subcategory-count" title="{$_('mindfuck.categories.available') || 'Verfügbar'}">
                            {sub.total_count}
                          </span>
                        </div>
                      </div>
                    {/each}
                  </div>
                {/if}
              </button>
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
              <div class="blind-spot-item" style="border-left-color: {spot.main_category_color || getSeverityColor(spot.severity)}">
                <div class="blind-spot-header">
                  <div class="blind-spot-name-wrapper">
                    <span class="blind-spot-name">{spot.name}</span>
                    {#if spot.main_category}
                      <span class="blind-spot-main-category" style="color: {spot.main_category_color || 'var(--text-muted)'}">
                        {spot.main_category}
                      </span>
                    {/if}
                  </div>
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
              <ArticleCard
                fnord_id={article.fnord_id}
                title={article.title}
                pentacle_title={article.pentacle_title}
                published_at={article.published_at}
                political_bias={article.political_bias}
                reason={article.reason}
                showBias={true}
                showReason={true}
                showAction={true}
                showCategories={false}
                showTags={false}
                actionLabel={$_("mindfuck.counterPerspectives.readArticle")}
                onclick={() => handleReadArticle(article.fnord_id)}
                onaction={() => handleReadArticle(article.fnord_id)}
              />
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
  .tabs-wrapper {
    padding: 0 1.5rem;
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

  /* Category Cards (matching FnordView) */
  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  .category-card {
    background: linear-gradient(135deg, color-mix(in srgb, var(--cat-color) 15%, var(--bg-base)) 0%, var(--bg-base) 100%);
    border: 1px solid color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-radius: 0.625rem;
    padding: 1rem;
    transition: transform 0.15s ease, box-shadow 0.15s ease;
    cursor: pointer;
    text-align: left;
    width: 100%;
  }

  .category-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px color-mix(in srgb, var(--cat-color) 20%, transparent);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.875rem;
  }

  .card-icon-wrapper {
    width: 2.25rem;
    height: 2.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 70%, black));
    border-radius: 0.5rem;
    color: white;
    font-size: 1rem;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 40%, transparent);
  }

  .card-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-stats {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .stat-row.secondary {
    margin-top: 0.25rem;
  }

  .stat-row.secondary .stat-label,
  .stat-row.secondary .stat-value {
    font-size: 0.6875rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .progress-bar {
    height: 6px;
    background-color: color-mix(in srgb, var(--cat-color) 20%, transparent);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 80%, white));
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  /* Category Card expanded state */
  .category-card.expanded {
    grid-column: 1 / -1;
  }

  .expand-icon {
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: transform 0.2s ease;
    flex-shrink: 0;
  }

  .expand-icon.rotated {
    transform: rotate(180deg);
  }

  /* Subcategories */
  .subcategories {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid color-mix(in srgb, var(--cat-color) 20%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.375rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    flex: 1;
  }

  .subcategory-icon {
    font-size: 0.75rem;
    color: var(--cat-color);
    flex-shrink: 0;
  }

  .subcategory-name {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .warning-badge {
    font-size: 0.625rem;
    width: 1rem;
    height: 1rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background-color: var(--accent-warning);
    color: var(--bg-base);
    border-radius: 50%;
    font-weight: 700;
    flex-shrink: 0;
  }

  .subcategory-stats {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .subcategory-count {
    font-weight: 500;
  }

  .subcategory-divider {
    color: var(--text-faint);
  }

  /* Bias Distribution - using bar styles */
  .bar-fill.sachlichkeit {
    background-color: var(--ctp-teal);
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
    align-items: flex-start;
    margin-bottom: 0.5rem;
    gap: 1rem;
  }

  .blind-spot-name-wrapper {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .blind-spot-name {
    font-weight: 600;
    color: var(--text-primary);
  }

  .blind-spot-main-category {
    font-size: 0.75rem;
    font-weight: 500;
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
