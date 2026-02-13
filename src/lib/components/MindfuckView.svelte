<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { ReadingProfile, BlindSpot, CounterPerspective, ReadingTrend } from "../types";
  import Tabs, { type Tab } from "./Tabs.svelte";
  import Tooltip from "./Tooltip.svelte";
  import { RecommendationList } from "./recommendation";
  import { getCategoryColorVar, getBiasColor } from "$lib/utils/articleFormat";

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
    try {
      counterPerspectives = await invoke<CounterPerspective[]>("get_counter_perspectives", {
        limit: 10,
      });
    } catch (e) {
      console.error("Failed to load counter perspectives:", e);
      counterPerspectives = [];
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

  function handleReadArticle(fnordId: number) {
    window.dispatchEvent(
      new CustomEvent("navigate-to-article", { detail: { articleId: fnordId } }),
    );
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
    <div class="header-top">
      <h2 class="view-title">
        <i class="fa-solid fa-brain nav-icon"></i>
        {$_("mindfuck.title")}
        <Tooltip termKey="mindfuck">
          <i class="fa-solid fa-circle-info info-icon"></i>
        </Tooltip>
      </h2>
      {#if readingProfile}
        <div class="mindfuck-stats">
          <span class="stat">
            <span class="stat-value">{readingProfile.total_read}</span>
            <span class="stat-label">{$_("mindfuck.profile.totalRead")}</span>
          </span>
          <span class="stat">
            <span class="stat-value">{readingProfile.total_articles}</span>
            <span class="stat-label">{$_("mindfuck.profile.totalArticles")}</span>
          </span>
        </div>
      {/if}
    </div>
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
        {@const maxRead = Math.max(...readingProfile.by_category.map((c) => c.read_count), 1)}
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
              <span
                class="stat-value bias-indicator"
                style="color: {getBiasColor(readingProfile.avg_political_bias)}"
              >
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
                style="--cat-color: {getCategoryColorVar(cat.sephiroth_id)}"
                data-category-id={cat.sephiroth_id}
                onclick={() => toggleCategoryExpand(cat.sephiroth_id)}
              >
                <div class="card-header">
                  <div class="card-icon-wrapper">
                    {#if cat.icon}
                      <i class={cat.icon}></i>
                    {:else}
                      <i class="fa-solid fa-folder"></i>
                    {/if}
                  </div>
                  <span class="card-title">{cat.name}</span>
                  <i class="fa-solid fa-chevron-down expand-icon {isExpanded ? 'rotated' : ''}"></i>
                </div>
                <div class="card-stats">
                  <div class="stat-row">
                    <span class="stat-label">{$_("mindfuck.categories.read")}</span>
                    <span class="stat-value">{cat.read_count}</span>
                  </div>
                  <div class="progress-bar">
                    <div class="progress-fill" style="width: {barWidth}%"></div>
                  </div>
                  <div class="stat-row secondary">
                    <span class="stat-label">{$_("mindfuck.categories.available")}</span>
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
                            <span
                              class="warning-badge"
                              title={$_("mindfuck.blindSpots.lowReadRate")}>!</span
                            >
                          {/if}
                        </div>
                        <div class="subcategory-stats">
                          <span class="subcategory-count" title={$_("mindfuck.categories.read")}>
                            {sub.read_count}
                          </span>
                          <span class="subcategory-divider">/</span>
                          <span
                            class="subcategory-count"
                            title={$_("mindfuck.categories.available")}
                          >
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

        <!-- Political Bias Distribution - Spectrum View -->
        <div class="section">
          <h3>{$_("mindfuck.bias.title")}</h3>
          <div class="political-spectrum-card">
            <!-- Spectrum Header -->
            <div class="spectrum-header">
              <span class="spectrum-label left">{$_("mindfuck.bias.left")}</span>
              <span class="spectrum-label center">{$_("mindfuck.bias.neutral")}</span>
              <span class="spectrum-label right">{$_("mindfuck.bias.right")}</span>
            </div>

            <!-- Main Spectrum Bar -->
            {#if readingProfile.by_bias.reduce((sum, b) => sum + b.read_count, 0) > 0}
              {@const totalBiasReads = readingProfile.by_bias.reduce(
                (sum, b) => sum + b.read_count,
                0,
              )}
              <div class="spectrum-bar-container">
                <div class="spectrum-bar">
                  {#each readingProfile.by_bias as bias (bias.bias_value)}
                    {@const segmentClass =
                      bias.bias_value <= -1.5
                        ? "left-extreme"
                        : bias.bias_value <= -0.5
                          ? "left-leaning"
                          : bias.bias_value <= 0.5
                            ? "neutral"
                            : bias.bias_value <= 1.5
                              ? "right-leaning"
                              : "right-extreme"}
                    {#if bias.read_count > 0}
                      <div
                        class="spectrum-segment {segmentClass}"
                        style="width: {bias.percentage}%"
                        title="{bias.label}: {bias.read_count} ({bias.percentage.toFixed(1)}%)"
                      >
                        {#if bias.percentage > 10}
                          <span class="segment-label">{bias.percentage.toFixed(0)}%</span>
                        {/if}
                      </div>
                    {/if}
                  {/each}
                </div>

                <!-- Position Indicator (based on average bias) -->
                {#if readingProfile.avg_political_bias !== null}
                  {@const avgBias = readingProfile.avg_political_bias}
                  {@const indicatorPosition = ((avgBias + 2) / 4) * 100}
                  <div
                    class="position-indicator"
                    style="left: {indicatorPosition}%"
                    title="{$_('mindfuck.bias.yourPosition')}: {avgBias.toFixed(2)}"
                  >
                    <i class="fa-solid fa-caret-down"></i>
                  </div>
                {/if}
              </div>

              <!-- Scale Markers -->
              <div class="spectrum-scale">
                <span class="scale-mark">-2</span>
                <span class="scale-mark">-1</span>
                <span class="scale-mark">0</span>
                <span class="scale-mark">+1</span>
                <span class="scale-mark">+2</span>
              </div>

              <!-- Detailed Breakdown -->
              <div class="spectrum-details">
                {#each readingProfile.by_bias as bias (bias.bias_value)}
                  {@const segmentClass =
                    bias.bias_value <= -1.5
                      ? "left-extreme"
                      : bias.bias_value <= -0.5
                        ? "left-leaning"
                        : bias.bias_value <= 0.5
                          ? "neutral"
                          : bias.bias_value <= 1.5
                            ? "right-leaning"
                            : "right-extreme"}
                  <div class="detail-item">
                    <span class="detail-dot {segmentClass}"></span>
                    <span class="detail-label">{bias.label}</span>
                    <span class="detail-count">{bias.read_count}</span>
                    <span class="detail-percent">({bias.percentage.toFixed(1)}%)</span>
                  </div>
                {/each}
              </div>

              <!-- Summary -->
              {#if readingProfile.avg_political_bias !== null}
                <div class="spectrum-summary">
                  <div
                    class="summary-indicator"
                    style="color: {getBiasColor(readingProfile.avg_political_bias)}"
                  >
                    <i class="fa-solid fa-compass"></i>
                    <span>{biasIndicator}</span>
                  </div>
                  <div class="summary-stat">
                    <span class="summary-value">{totalBiasReads}</span>
                    <span class="summary-label">{$_("mindfuck.bias.articlesAnalyzed")}</span>
                  </div>
                </div>
              {/if}
            {:else}
              <div class="spectrum-empty">
                <i class="fa-solid fa-scale-balanced"></i>
                <p>{$_("mindfuck.bias.noData")}</p>
              </div>
            {/if}
          </div>
        </div>

        <!-- Sachlichkeit Distribution -->
        <div class="section">
          <h3>{$_("mindfuck.sachlichkeit.title")}</h3>
          <div class="sachlichkeit-spectrum-card">
            <!-- Spectrum Header -->
            <div class="sachlichkeit-header">
              <span class="sachlichkeit-label-header emotional"
                >{$_("mindfuck.sachlichkeit.emotional")}</span
              >
              <span class="sachlichkeit-label-header mixed"
                >{$_("mindfuck.sachlichkeit.mixed")}</span
              >
              <span class="sachlichkeit-label-header objective"
                >{$_("mindfuck.sachlichkeit.objective")}</span
              >
            </div>

            <!-- Main Spectrum Bar -->
            {#if readingProfile.by_sachlichkeit.reduce((sum, s) => sum + s.read_count, 0) > 0}
              {@const totalSachReads = readingProfile.by_sachlichkeit.reduce(
                (sum, s) => sum + s.read_count,
                0,
              )}
              <div class="sachlichkeit-bar-container">
                <div class="sachlichkeit-spectrum-bar">
                  {#each readingProfile.by_sachlichkeit as sach (sach.sachlichkeit_value)}
                    {@const segmentClass =
                      sach.sachlichkeit_value <= 0.5
                        ? "highly-emotional"
                        : sach.sachlichkeit_value <= 1.5
                          ? "emotional"
                          : sach.sachlichkeit_value <= 2.5
                            ? "mixed"
                            : sach.sachlichkeit_value <= 3.5
                              ? "mostly-objective"
                              : "objective"}
                    {#if sach.read_count > 0}
                      <div
                        class="sachlichkeit-segment {segmentClass}"
                        style="width: {sach.percentage}%"
                        title="{sach.label}: {sach.read_count} ({sach.percentage.toFixed(1)}%)"
                      >
                        {#if sach.percentage > 10}
                          <span class="segment-label">{sach.percentage.toFixed(0)}%</span>
                        {/if}
                      </div>
                    {/if}
                  {/each}
                </div>
              </div>

              <!-- Scale Markers -->
              <div class="sachlichkeit-scale">
                <span class="scale-mark">0</span>
                <span class="scale-mark">1</span>
                <span class="scale-mark">2</span>
                <span class="scale-mark">3</span>
                <span class="scale-mark">4</span>
              </div>

              <!-- Detailed Breakdown -->
              <div class="sachlichkeit-details">
                {#each readingProfile.by_sachlichkeit as sach (sach.sachlichkeit_value)}
                  {@const segmentClass =
                    sach.sachlichkeit_value <= 0.5
                      ? "highly-emotional"
                      : sach.sachlichkeit_value <= 1.5
                        ? "emotional"
                        : sach.sachlichkeit_value <= 2.5
                          ? "mixed"
                          : sach.sachlichkeit_value <= 3.5
                            ? "mostly-objective"
                            : "objective"}
                  <div class="detail-item">
                    <span class="detail-dot {segmentClass}"></span>
                    <span class="detail-label">{sach.label}</span>
                    <span class="detail-count">{sach.read_count}</span>
                    <span class="detail-percent">({sach.percentage.toFixed(1)}%)</span>
                  </div>
                {/each}
              </div>

              <!-- Summary -->
              <div class="sachlichkeit-summary">
                <div class="summary-indicator" style="color: var(--sach-objective)">
                  <i class="fa-solid fa-gauge-high"></i>
                  <span>{$_("mindfuck.sachlichkeit.title")}</span>
                </div>
                <div class="summary-stat">
                  <span class="summary-value">{totalSachReads}</span>
                  <span class="summary-label">{$_("mindfuck.bias.articlesAnalyzed")}</span>
                </div>
              </div>
            {:else}
              <div class="sachlichkeit-empty">
                <i class="fa-solid fa-gauge-high"></i>
                <p>{$_("mindfuck.sachlichkeit.noData")}</p>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    {:else if activeTab === "blindSpots"}
      {#if loadingBlindSpots}
        <div class="loading">{$_("fnordView.loading")}</div>
      {:else if blindSpots.length === 0}
        <!-- No blind spots - positive message -->
        <div class="no-blind-spots-container">
          <div class="no-blind-spots-card">
            <div class="success-icon-wrapper">
              <i class="fa-solid fa-check-circle"></i>
            </div>
            <h3>{$_("mindfuck.blindSpots.noBlindSpots")}</h3>
            <p>{$_("mindfuck.blindSpots.noBlindSpotsSubtitle")}</p>
            <div class="balance-indicator">
              <div class="balance-bar">
                <div class="balance-fill"></div>
              </div>
              <span class="balance-label">100%</span>
            </div>
          </div>
        </div>
      {:else}
        <div class="section">
          <h3>{$_("mindfuck.blindSpots.title")}</h3>
          <p class="section-description">{$_("mindfuck.blindSpots.description")}</p>

          <!-- Blind Spots Cards Grid -->
          <div class="blind-spots-grid">
            {#each blindSpots as spot (spot.name)}
              {@const readPercentage =
                spot.available_count > 0
                  ? Math.round((spot.read_count / spot.available_count) * 100)
                  : 0}
              {@const severityColor = getSeverityColor(spot.severity)}
              {@const categoryColor = spot.main_category_color || severityColor}
              <div
                class="blind-spot-card severity-{spot.severity}"
                style="--severity-color: {severityColor}; --category-color: {categoryColor}"
              >
                <!-- Severity indicator bar at top -->
                <div class="severity-bar"></div>

                <!-- Card Header with Icon and Category -->
                <div class="blind-spot-card-header">
                  <div class="blind-spot-icon-wrapper">
                    {#if spot.icon}
                      <i class={spot.icon}></i>
                    {:else}
                      <i class="fa-solid fa-eye-slash"></i>
                    {/if}
                  </div>
                  <div class="blind-spot-badge" style="background-color: {severityColor}">
                    {$_(`mindfuck.blindSpots.severity.${spot.severity}`)}
                  </div>
                </div>

                <!-- Title and Category -->
                <div class="blind-spot-title-section">
                  <h4 class="blind-spot-card-title">{spot.name}</h4>
                  {#if spot.main_category}
                    <span
                      class="blind-spot-category-tag"
                      style="color: {categoryColor}; border-color: {categoryColor}"
                    >
                      <i class="fa-solid fa-folder-tree"></i>
                      {spot.main_category}
                    </span>
                  {/if}
                </div>

                <!-- Progress visualization -->
                <div class="blind-spot-progress-section">
                  <div class="blind-spot-progress-header">
                    <span class="blind-spot-progress-label"
                      >{$_("mindfuck.blindSpots.readPercentage", {
                        values: { percent: readPercentage },
                      })}</span
                    >
                    <span class="blind-spot-progress-ratio"
                      >{spot.read_count} / {spot.available_count}</span
                    >
                  </div>
                  <div class="blind-spot-progress-bar">
                    <div
                      class="blind-spot-progress-fill"
                      style="width: {readPercentage}%; background-color: {severityColor}"
                    ></div>
                    <div
                      class="blind-spot-progress-remaining"
                      style="width: {100 - readPercentage}%"
                    ></div>
                  </div>
                </div>

                <!-- Severity explanation -->
                <div class="blind-spot-explanation">
                  <i class="fa-solid fa-circle-info"></i>
                  <span>{$_(`mindfuck.blindSpots.severityDescription.${spot.severity}`)}</span>
                </div>

                <!-- Action button -->
                <button
                  class="blind-spot-action-btn"
                  onclick={() => {
                    const event = new CustomEvent("filter-by-category", {
                      detail: { categoryName: spot.name },
                    });
                    window.dispatchEvent(event);
                  }}
                  type="button"
                >
                  <i class="fa-solid fa-arrow-right"></i>
                  {$_("mindfuck.blindSpots.exploreCategory")}
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {:else if activeTab === "recommendations"}
      <RecommendationList onArticleClick={handleReadArticle} />
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
                      style="height: {Math.max(
                        4,
                        (trend.read_count / Math.max(...readingTrends.map((t) => t.read_count))) *
                          100,
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

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .mindfuck-stats {
    display: flex;
    gap: 1.5rem;
    align-items: flex-end;
  }

  .mindfuck-stats .stat {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .mindfuck-stats .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .mindfuck-stats .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
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

  /* Category colors are set via inline style using getCategoryColorVar() for theme-awareness */

  .category-card {
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--cat-color) 25%, var(--bg-base)) 0%,
      color-mix(in srgb, var(--cat-color) 8%, var(--bg-base)) 100%
    );
    border: 1px solid color-mix(in srgb, var(--cat-color) 50%, transparent);
    border-left: 3px solid var(--cat-color);
    border-radius: 0.625rem;
    padding: 1rem;
    transition:
      transform 0.15s ease,
      box-shadow 0.15s ease,
      border-color 0.15s ease;
    cursor: pointer;
    text-align: left;
    width: 100%;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 15%, transparent);
  }

  .category-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-color: color-mix(in srgb, var(--cat-color) 70%, transparent);
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
    background: linear-gradient(
      135deg,
      var(--cat-color),
      color-mix(in srgb, var(--cat-color) 70%, black)
    );
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
    background: linear-gradient(
      90deg,
      var(--cat-color),
      color-mix(in srgb, var(--cat-color) 80%, white)
    );
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

  /* Sachlichkeit Spectrum Card */
  .sachlichkeit-spectrum-card {
    background-color: var(--bg-overlay);
    border-radius: 0.75rem;
    border: 1px solid var(--border-default);
    padding: 1.25rem;
  }

  .sachlichkeit-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }

  .sachlichkeit-label-header {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .sachlichkeit-label-header.emotional {
    color: var(--sach-emotional);
  }

  .sachlichkeit-label-header.mixed {
    color: var(--sach-mixed);
  }

  .sachlichkeit-label-header.objective {
    color: var(--sach-objective);
  }

  .sachlichkeit-bar-container {
    position: relative;
    margin-bottom: 0.5rem;
  }

  .sachlichkeit-spectrum-bar {
    display: flex;
    height: 2rem;
    border-radius: 0.5rem;
    overflow: hidden;
    background: linear-gradient(
      90deg,
      var(--sach-emotional) 0%,
      color-mix(in srgb, var(--sach-emotional) 50%, var(--bg-surface)) 25%,
      var(--sach-mixed) 50%,
      color-mix(in srgb, var(--sach-objective) 50%, var(--bg-surface)) 75%,
      var(--sach-objective) 100%
    );
    opacity: 0.15;
  }

  .sachlichkeit-spectrum-bar:has(.sachlichkeit-segment) {
    background: var(--bg-base);
    opacity: 1;
  }

  .sachlichkeit-segment {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      width 0.3s ease,
      opacity 0.2s ease;
    min-width: 2px;
    position: relative;
  }

  .sachlichkeit-segment:hover {
    filter: brightness(1.1);
  }

  .sachlichkeit-segment.highly-emotional {
    background: var(--sach-emotional);
  }

  .sachlichkeit-segment.emotional {
    background: color-mix(in srgb, var(--sach-emotional) 70%, var(--sach-mixed));
  }

  .sachlichkeit-segment.mixed {
    background: var(--sach-mixed);
  }

  .sachlichkeit-segment.mostly-objective {
    background: color-mix(in srgb, var(--sach-objective) 60%, var(--sach-mixed));
  }

  .sachlichkeit-segment.objective {
    background: var(--sach-objective);
  }

  .sachlichkeit-scale {
    display: flex;
    justify-content: space-between;
    padding: 0 0.25rem;
    margin-bottom: 1rem;
  }

  .sachlichkeit-details {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    margin-bottom: 1rem;
    padding: 0.75rem;
    background-color: var(--bg-base);
    border-radius: 0.5rem;
  }

  .sachlichkeit-details .detail-dot.highly-emotional {
    background: var(--sach-emotional);
  }

  .sachlichkeit-details .detail-dot.emotional {
    background: color-mix(in srgb, var(--sach-emotional) 70%, var(--sach-mixed));
  }

  .sachlichkeit-details .detail-dot.mixed {
    background: var(--sach-mixed);
  }

  .sachlichkeit-details .detail-dot.mostly-objective {
    background: color-mix(in srgb, var(--sach-objective) 60%, var(--sach-mixed));
  }

  .sachlichkeit-details .detail-dot.objective {
    background: var(--sach-objective);
  }

  .sachlichkeit-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-default);
  }

  .sachlichkeit-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    color: var(--text-muted);
    gap: 0.75rem;
  }

  .sachlichkeit-empty i {
    font-size: 2rem;
    opacity: 0.5;
  }

  .sachlichkeit-empty p {
    margin: 0;
    font-size: 0.875rem;
  }

  /* Political Spectrum Card */
  .political-spectrum-card {
    background-color: var(--bg-overlay);
    border-radius: 0.75rem;
    border: 1px solid var(--border-default);
    padding: 1.25rem;
  }

  .spectrum-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }

  .spectrum-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .spectrum-label.left {
    color: var(--category-2);
  }

  .spectrum-label.center {
    color: var(--text-muted);
  }

  .spectrum-label.right {
    color: var(--category-5);
  }

  .spectrum-bar-container {
    position: relative;
    margin-bottom: 0.5rem;
  }

  .spectrum-bar {
    display: flex;
    height: 2rem;
    border-radius: 0.5rem;
    overflow: hidden;
    background: linear-gradient(
      90deg,
      var(--category-2) 0%,
      color-mix(in srgb, var(--category-2) 50%, var(--bg-surface)) 25%,
      var(--text-muted) 50%,
      color-mix(in srgb, var(--category-5) 50%, var(--bg-surface)) 75%,
      var(--category-5) 100%
    );
    opacity: 0.15;
  }

  .spectrum-bar:has(.spectrum-segment) {
    background: var(--bg-base);
    opacity: 1;
  }

  .spectrum-segment {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      width 0.3s ease,
      opacity 0.2s ease;
    min-width: 2px;
    position: relative;
  }

  .spectrum-segment:hover {
    filter: brightness(1.1);
  }

  .spectrum-segment.left-extreme {
    background: var(--category-2);
  }

  .spectrum-segment.left-leaning {
    background: color-mix(in srgb, var(--category-2) 60%, var(--bg-surface));
  }

  .spectrum-segment.neutral {
    background: var(--text-muted);
  }

  .spectrum-segment.right-leaning {
    background: color-mix(in srgb, var(--category-5) 60%, var(--bg-surface));
  }

  .spectrum-segment.right-extreme {
    background: var(--category-5);
  }

  .segment-label {
    font-size: 0.6875rem;
    font-weight: 600;
    color: white;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .position-indicator {
    position: absolute;
    top: -0.25rem;
    transform: translateX(-50%);
    color: var(--accent-primary);
    font-size: 1.25rem;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.3));
    z-index: 10;
    transition: left 0.3s ease;
  }

  .spectrum-scale {
    display: flex;
    justify-content: space-between;
    padding: 0 0.25rem;
    margin-bottom: 1rem;
  }

  .scale-mark {
    font-size: 0.625rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .spectrum-details {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    margin-bottom: 1rem;
    padding: 0.75rem;
    background-color: var(--bg-base);
    border-radius: 0.5rem;
  }

  .detail-item {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.8125rem;
  }

  .detail-dot {
    width: 0.625rem;
    height: 0.625rem;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .detail-dot.left-extreme {
    background: var(--category-2);
  }

  .detail-dot.left-leaning {
    background: color-mix(in srgb, var(--category-2) 60%, var(--bg-surface));
  }

  .detail-dot.neutral {
    background: var(--text-muted);
  }

  .detail-dot.right-leaning {
    background: color-mix(in srgb, var(--category-5) 60%, var(--bg-surface));
  }

  .detail-dot.right-extreme {
    background: var(--category-5);
  }

  .detail-label {
    color: var(--text-secondary);
  }

  .detail-count {
    font-weight: 600;
    color: var(--text-primary);
  }

  .detail-percent {
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .spectrum-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-default);
  }

  .summary-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9375rem;
    font-weight: 600;
  }

  .summary-indicator i {
    font-size: 1rem;
  }

  .summary-stat {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .summary-value {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .summary-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .spectrum-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    color: var(--text-muted);
    gap: 0.75rem;
  }

  .spectrum-empty i {
    font-size: 2rem;
    opacity: 0.5;
  }

  .spectrum-empty p {
    margin: 0;
    font-size: 0.875rem;
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

  /* Blind Spots - No Blind Spots State */
  .no-blind-spots-container {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 3rem 1rem;
  }

  .no-blind-spots-card {
    text-align: center;
    max-width: 400px;
    padding: 2.5rem 2rem;
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--status-success) 15%, var(--bg-overlay)) 0%,
      color-mix(in srgb, var(--status-success) 5%, var(--bg-overlay)) 100%
    );
    border: 1px solid color-mix(in srgb, var(--status-success) 30%, var(--border-default));
    border-radius: 1rem;
    box-shadow: 0 4px 24px color-mix(in srgb, var(--status-success) 10%, transparent);
  }

  .success-icon-wrapper {
    width: 4.5rem;
    height: 4.5rem;
    margin: 0 auto 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(
      135deg,
      var(--status-success),
      color-mix(in srgb, var(--status-success) 70%, var(--ctp-green))
    );
    border-radius: 50%;
    box-shadow: 0 8px 24px color-mix(in srgb, var(--status-success) 35%, transparent);
  }

  .success-icon-wrapper i {
    font-size: 2.25rem;
    color: white;
  }

  .no-blind-spots-card h3 {
    margin: 0 0 0.75rem;
    font-size: 1.25rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .no-blind-spots-card p {
    margin: 0 0 1.5rem;
    font-size: 0.9375rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .balance-indicator {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    justify-content: center;
  }

  .balance-bar {
    width: 120px;
    height: 8px;
    background: var(--border-subtle);
    border-radius: 4px;
    overflow: hidden;
  }

  .balance-fill {
    width: 100%;
    height: 100%;
    background: linear-gradient(
      90deg,
      var(--status-success),
      color-mix(in srgb, var(--status-success) 80%, var(--ctp-green))
    );
    border-radius: 4px;
  }

  .balance-label {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--status-success);
  }

  /* Blind Spots - Cards Grid */
  .blind-spots-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1.25rem;
  }

  .blind-spot-card {
    position: relative;
    background: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.75rem;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease,
      border-color 0.2s ease;
    overflow: hidden;
  }

  .blind-spot-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px color-mix(in srgb, var(--severity-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--severity-color) 50%, var(--border-default));
  }

  /* Severity indicator bar at top */
  .severity-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 4px;
    background: var(--severity-color);
  }

  .blind-spot-card.severity-high .severity-bar {
    background: linear-gradient(
      90deg,
      var(--status-error),
      color-mix(in srgb, var(--status-error) 70%, var(--ctp-red))
    );
  }

  .blind-spot-card.severity-medium .severity-bar {
    background: linear-gradient(
      90deg,
      var(--status-warning),
      color-mix(in srgb, var(--status-warning) 70%, var(--ctp-peach))
    );
  }

  .blind-spot-card.severity-low .severity-bar {
    background: linear-gradient(
      90deg,
      var(--ctp-yellow),
      color-mix(in srgb, var(--ctp-yellow) 70%, var(--ctp-rosewater))
    );
  }

  /* Card Header */
  .blind-spot-card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding-top: 0.25rem;
  }

  .blind-spot-icon-wrapper {
    width: 2.5rem;
    height: 2.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--severity-color) 25%, var(--bg-surface)),
      color-mix(in srgb, var(--severity-color) 10%, var(--bg-surface))
    );
    border: 1px solid color-mix(in srgb, var(--severity-color) 30%, transparent);
    border-radius: 0.5rem;
    color: var(--severity-color);
    font-size: 1.125rem;
  }

  .blind-spot-badge {
    padding: 0.25rem 0.625rem;
    border-radius: 1rem;
    font-size: 0.6875rem;
    font-weight: 600;
    color: white;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  /* Title Section */
  .blind-spot-title-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .blind-spot-card-title {
    margin: 0;
    font-size: 1.0625rem;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.3;
  }

  .blind-spot-category-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid;
    border-radius: 0.25rem;
    font-size: 0.6875rem;
    font-weight: 500;
    width: fit-content;
  }

  .blind-spot-category-tag i {
    font-size: 0.625rem;
  }

  /* Progress Section */
  .blind-spot-progress-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .blind-spot-progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .blind-spot-progress-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .blind-spot-progress-ratio {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .blind-spot-progress-bar {
    display: flex;
    height: 8px;
    border-radius: 4px;
    overflow: hidden;
    background: var(--border-subtle);
  }

  .blind-spot-progress-fill {
    height: 100%;
    border-radius: 4px 0 0 4px;
    transition: width 0.3s ease;
  }

  .blind-spot-progress-remaining {
    height: 100%;
    background: repeating-linear-gradient(
      -45deg,
      color-mix(in srgb, var(--severity-color) 8%, transparent),
      color-mix(in srgb, var(--severity-color) 8%, transparent) 4px,
      color-mix(in srgb, var(--severity-color) 15%, transparent) 4px,
      color-mix(in srgb, var(--severity-color) 15%, transparent) 8px
    );
  }

  /* Severity Explanation */
  .blind-spot-explanation {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.625rem;
    background: color-mix(in srgb, var(--severity-color) 8%, transparent);
    border-radius: 0.375rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .blind-spot-explanation i {
    color: var(--severity-color);
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  /* Action Button */
  .blind-spot-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.625rem 1rem;
    background: transparent;
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    margin-top: auto;
  }

  .blind-spot-action-btn:hover {
    background: color-mix(in srgb, var(--severity-color) 10%, transparent);
    border-color: var(--severity-color);
    color: var(--severity-color);
  }

  .blind-spot-action-btn i {
    font-size: 0.75rem;
    transition: transform 0.2s ease;
  }

  .blind-spot-action-btn:hover i {
    transform: translateX(3px);
  }

  /* Recommendations */
  .recommendations-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  /* Recommendations Empty State */
  .recommendations-empty-state {
    display: flex;
    flex-direction: column;
    gap: 2rem;
    max-width: 900px;
    margin: 0 auto;
  }

  .empty-state-header {
    text-align: center;
    padding: 2rem 1rem;
  }

  .empty-state-icon {
    width: 5rem;
    height: 5rem;
    margin: 0 auto 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--accent-primary), var(--ctp-mauve));
    border-radius: 50%;
    box-shadow: 0 8px 32px color-mix(in srgb, var(--accent-primary) 30%, transparent);
  }

  .empty-state-icon i {
    font-size: 2.5rem;
    color: white;
  }

  .empty-state-header h3 {
    margin: 0 0 0.75rem;
    font-size: 1.5rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .empty-state-description {
    margin: 0;
    font-size: 1rem;
    color: var(--text-secondary);
    max-width: 500px;
    margin: 0 auto;
    line-height: 1.5;
  }

  .empty-state-section {
    background: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.75rem;
    padding: 1.5rem;
  }

  .empty-state-section h4 {
    margin: 0 0 1.25rem;
    font-size: 1rem;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .empty-state-section h4 i {
    color: var(--accent-primary);
  }

  /* Steps Grid */
  .steps-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }

  .step-card {
    position: relative;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 0.5rem;
    padding: 1.25rem;
    padding-left: 3rem;
  }

  .step-number {
    position: absolute;
    left: -0.75rem;
    top: 1rem;
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-primary);
    color: white;
    border-radius: 50%;
    font-weight: 700;
    font-size: 0.875rem;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent-primary) 40%, transparent);
  }

  .step-content {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .step-icon {
    margin-bottom: 0.5rem;
  }

  .step-icon i {
    font-size: 1.25rem;
    color: var(--accent-primary);
  }

  .step-content h5 {
    margin: 0;
    font-size: 0.9375rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .step-content p {
    margin: 0;
    font-size: 0.8125rem;
    color: var(--text-muted);
    line-height: 1.4;
  }

  /* Progress Checklist */
  .progress-checklist {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .progress-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.875rem 1rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 0.5rem;
    transition: all 0.2s ease;
  }

  .progress-item.completed {
    border-color: var(--status-success);
    background: color-mix(in srgb, var(--status-success) 8%, var(--bg-surface));
  }

  .progress-check {
    flex-shrink: 0;
    font-size: 1.25rem;
  }

  .progress-item:not(.completed) .progress-check {
    color: var(--text-faint);
  }

  .progress-item.completed .progress-check {
    color: var(--status-success);
  }

  .progress-details {
    flex: 1;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .progress-label {
    font-size: 0.875rem;
    color: var(--text-primary);
    font-weight: 500;
  }

  .progress-value {
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .progress-value.status-ok {
    color: var(--status-success);
  }

  .progress-value.status-warn {
    color: var(--status-warning);
  }

  .progress-bar-mini {
    width: 80px;
    height: 6px;
    background: var(--border-subtle);
    border-radius: 3px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .progress-fill-mini {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-primary), var(--ctp-mauve));
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  /* Placeholder Cards */
  .placeholder-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1rem;
  }

  .placeholder-card {
    background: var(--bg-surface);
    border: 1px dashed var(--border-default);
    border-radius: 0.5rem;
    padding: 1rem;
    opacity: 0.75;
    position: relative;
    overflow: hidden;
  }

  .placeholder-card::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(
      135deg,
      transparent 60%,
      color-mix(in srgb, var(--accent-primary) 5%, transparent)
    );
    pointer-events: none;
  }

  .placeholder-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .placeholder-icon {
    width: 2rem;
    height: 2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--border-subtle);
    border-radius: 0.25rem;
  }

  .placeholder-icon i {
    font-size: 1rem;
    color: var(--text-faint);
  }

  .placeholder-meta {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .placeholder-source {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .placeholder-date {
    font-size: 0.6875rem;
    color: var(--text-faint);
  }

  .placeholder-title {
    margin: 0 0 0.75rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-weight: 500;
    line-height: 1.4;
  }

  .placeholder-reason {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
    padding: 0.5rem;
    background: color-mix(in srgb, var(--ctp-yellow) 10%, transparent);
    border-radius: 0.25rem;
  }

  .placeholder-reason i {
    font-size: 0.75rem;
    color: var(--ctp-yellow);
    flex-shrink: 0;
    margin-top: 0.125rem;
  }

  .placeholder-reason span {
    font-size: 0.75rem;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .placeholder-bias {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .bias-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .bias-dot.left {
    background: var(--ctp-red);
  }

  .bias-dot.neutral {
    background: var(--ctp-green);
  }

  .bias-dot.right {
    background: var(--ctp-blue);
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

    /* Political spectrum responsive */
    .spectrum-details {
      flex-direction: column;
      gap: 0.5rem;
    }

    .spectrum-summary {
      flex-direction: column;
      gap: 1rem;
      align-items: flex-start;
    }

    .summary-stat {
      align-items: flex-start;
    }

    .political-spectrum-card {
      padding: 1rem;
    }

    /* Empty state responsive */
    .empty-state-header {
      padding: 1.5rem 0.5rem;
    }

    .empty-state-icon {
      width: 4rem;
      height: 4rem;
    }

    .empty-state-icon i {
      font-size: 2rem;
    }

    .empty-state-header h3 {
      font-size: 1.25rem;
    }

    .steps-grid {
      grid-template-columns: 1fr;
    }

    .step-card {
      padding-left: 2.5rem;
    }

    .step-number {
      left: -0.5rem;
      width: 1.5rem;
      height: 1.5rem;
      font-size: 0.75rem;
    }

    .progress-details {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.25rem;
    }

    .progress-bar-mini {
      width: 100%;
      margin-top: 0.5rem;
    }

    .placeholder-cards {
      grid-template-columns: 1fr;
    }

    /* Blind spots responsive */
    .blind-spots-grid {
      grid-template-columns: 1fr;
    }

    .no-blind-spots-container {
      padding: 2rem 0.5rem;
    }

    .no-blind-spots-card {
      padding: 2rem 1.5rem;
    }

    .success-icon-wrapper {
      width: 3.5rem;
      height: 3.5rem;
    }

    .success-icon-wrapper i {
      font-size: 1.75rem;
    }

    .no-blind-spots-card h3 {
      font-size: 1.125rem;
    }

    .blind-spot-card {
      padding: 1rem;
    }

    .blind-spot-icon-wrapper {
      width: 2rem;
      height: 2rem;
      font-size: 1rem;
    }

    .blind-spot-badge {
      padding: 0.2rem 0.5rem;
      font-size: 0.625rem;
    }

    .blind-spot-card-title {
      font-size: 1rem;
    }
  }
</style>
