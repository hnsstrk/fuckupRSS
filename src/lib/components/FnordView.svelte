<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { appState, type Fnord, type FnordStats } from "../stores/state.svelte";
  import type {
    ArticleTimeline,
    GreyfaceIndex,
    KeywordStats,
    FeedActivity,
    BiasHeatmapEntry,
    KeywordCloudEntry,
  } from "../types";
  import Tooltip from "./Tooltip.svelte";
  import { formatError } from "$lib/utils/formatError";
  import Tabs, { type Tab } from "./Tabs.svelte";
  import { ArticleItemCompact } from "./article";
  import FnordStatsOverview from "./FnordStatsOverview.svelte";
  import FnordBiasHeatmap from "./FnordBiasHeatmap.svelte";
  import FnordCategoryCards from "./FnordCategoryCards.svelte";
  import FnordTrendsSection from "./FnordTrendsSection.svelte";
  import EntityExplorer from "./EntityExplorer.svelte";
  import { navigationStore } from "$lib/stores/navigation.svelte";
  import { createLogger } from "$lib/logger";

  const log = createLogger("FnordView");
  // State
  let stats = $state<FnordStats | null>(null);
  let changedFnords = $state<Fnord[]>([]);
  let loading = $state(true);
  let selectedFnordId = $state<number | null>(null);

  // Extended statistics state
  let timeline = $state<ArticleTimeline | null>(null);
  let greyfaceIndex = $state<GreyfaceIndex | null>(null);
  let topKeywords = $state<KeywordStats[]>([]);
  let feedActivity = $state<FeedActivity[]>([]);
  let biasHeatmap = $state<BiasHeatmapEntry[]>([]);
  let keywordCloud = $state<KeywordCloudEntry[]>([]);

  // Loading/error states for extended stats
  let extendedStatsLoading = $state(false);
  let extendedStatsError = $state<string | null>(null);

  // Period selector
  let selectedPeriod = $state<7 | 30 | 90>(7);

  // Tab state
  let activeTab = $state<string>("stats");

  // Easter egg state
  let show23EasterEgg = $state(false);

  // Tabs definition
  let tabs = $derived<Tab[]>([
    { id: "stats", label: $_("fnordView.statsTab") || "Statistiken" },
    {
      id: "articles",
      label: $_("fnordView.articlesTab") || "Geänderte Artikel",
      badge: changedFnords.length || undefined,
    },
    { id: "entities", label: $_("entities.title") || "Entitäten" },
  ]);

  async function handleBatchComplete() {
    await loadData();
  }

  async function handleKeywordsChanged() {
    await loadExtendedStats();
  }

  onMount(async () => {
    window.addEventListener("batch-complete", handleBatchComplete);
    window.addEventListener("keywords-changed", handleKeywordsChanged);
    await loadData();
  });

  onDestroy(() => {
    window.removeEventListener("batch-complete", handleBatchComplete);
    window.removeEventListener("keywords-changed", handleKeywordsChanged);
  });

  async function loadData() {
    loading = true;
    try {
      const statsData = await appState.getFnordStats();
      stats = statsData;

      await appState.loadChangedFnords();
      changedFnords = appState.changedFnords;

      await loadExtendedStats();
    } catch (e) {
      log.error("[FnordView] Error loading data:", e);
    } finally {
      loading = false;
    }
  }

  async function loadExtendedStats() {
    extendedStatsLoading = true;
    extendedStatsError = null;
    try {
      const [timelineData, greyfaceData, keywordsData, feedData, heatmapData, cloudData] =
        await Promise.all([
          invoke<ArticleTimeline>("get_article_timeline", { days: selectedPeriod }),
          invoke<GreyfaceIndex>("get_greyface_index"),
          invoke<KeywordStats[]>("get_top_keywords_stats", { days: selectedPeriod, limit: 5 }),
          invoke<FeedActivity[]>("get_feed_activity", { days: selectedPeriod, limit: 5 }),
          invoke<BiasHeatmapEntry[]>("get_bias_heatmap"),
          invoke<KeywordCloudEntry[]>("get_keyword_cloud", { days: selectedPeriod, limit: 50 }),
        ]);

      timeline = timelineData;
      greyfaceIndex = greyfaceData;
      topKeywords = keywordsData;
      feedActivity = feedData;
      biasHeatmap = heatmapData;
      keywordCloud = cloudData;

      // Easter egg
      if ((selectedPeriod as number) === 23) {
        show23EasterEgg = true;
      }
    } catch (e) {
      log.error("[FnordView] Error loading extended stats:", e);
      extendedStatsError = formatError(e);
    } finally {
      extendedStatsLoading = false;
    }
  }

  async function changePeriod(days: 7 | 30 | 90) {
    selectedPeriod = days;
    await loadExtendedStats();
  }

  function selectFnord(id: number) {
    selectedFnordId = id;
    appState.selectFnord(id);
    navigationStore.navigateToArticle(id);
  }
</script>

<div class="fnord-view">
  <!-- Header -->
  <div class="fnord-header">
    <div class="header-top">
      <h2 class="view-title">
        <i class="fa-solid fa-clipboard-list nav-icon"></i>
        {$_("fnordView.title") || "Fnord-Statistiken"}
        <Tooltip termKey="fnord_stats">
          <i class="fa-solid fa-circle-info info-icon"></i>
        </Tooltip>
      </h2>
      {#if stats}
        <div class="fnord-summary">
          <span class="summary-item">
            <span class="summary-value">{stats.total_revisions}</span>
            <span class="summary-label">{$_("fnordView.totalRevisions") || "Revisionen"}</span>
          </span>
          <span class="summary-item">
            <span class="summary-value">{stats.articles_with_changes}</span>
            <span class="summary-label"
              >{$_("fnordView.articlesWithChanges") || "Geänderte Artikel"}</span
            >
          </span>
        </div>
      {/if}
    </div>

    <Tabs {tabs} bind:activeTab />
  </div>

  <!-- Content -->
  <div class="fnord-content">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <span>{$_("fnordView.loading") || "Laden..."}</span>
      </div>
    {:else if activeTab === "stats" && stats}
      <div class="stats-view">
        <!-- GESAMT-ÜBERSICHT (ohne Zeitfilter) -->
        <div class="stats-section-header">
          <h3 class="section-header-title">
            <i class="fa-solid fa-chart-pie"></i>
            {$_("fnordView.overallStats") || "Gesamt-Übersicht"}
          </h3>
        </div>

        <FnordStatsOverview {stats} {greyfaceIndex} />

        <FnordBiasHeatmap {biasHeatmap} />

        <FnordCategoryCards byCategory={stats.by_category} />

        <FnordTrendsSection
          {timeline}
          {topKeywords}
          {feedActivity}
          {keywordCloud}
          {selectedPeriod}
          {extendedStatsLoading}
          {extendedStatsError}
          onChangePeriod={changePeriod}
          onRetry={() => loadExtendedStats()}
        />
      </div>
    {:else if activeTab === "articles"}
      <div class="articles-view">
        {#if changedFnords.length === 0}
          <div class="empty-state">
            <i class="empty-icon fa-solid fa-check"></i>
            <p>{$_("fnordView.noChangedArticles") || "Keine geänderten Artikel"}</p>
          </div>
        {:else}
          <div class="articles-list">
            {#each changedFnords as fnord (fnord.id)}
              <ArticleItemCompact
                id={fnord.id}
                title={fnord.title}
                status={fnord.status}
                pentacle_title={fnord.pentacle_title}
                changed_at={fnord.changed_at}
                revision_count={fnord.revision_count}
                categories={fnord.categories}
                active={selectedFnordId === fnord.id}
                showIndicators={false}
                onclick={() => selectFnord(fnord.id)}
              />
            {/each}
          </div>
        {/if}
      </div>
    {:else if activeTab === "entities"}
      <div class="entities-view">
        <EntityExplorer />
      </div>
    {/if}
  </div>

  <!-- Easter Egg -->
  {#if show23EasterEgg}
    <div class="easter-egg-23">
      <span>{$_("fnordView.easterEgg23") || "Du hast das Geheimnis der 23 entdeckt!"}</span>
    </div>
  {/if}
</div>

<style>
  .fnord-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1;
    background-color: var(--bg-base);
  }

  .fnord-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-default);
    background-color: var(--bg-surface);
  }

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .fnord-summary {
    display: flex;
    gap: 1.5rem;
  }

  .summary-item {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .summary-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .summary-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Section Headers */
  .stats-section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 0;
    margin-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-default);
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

  .fnord-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.5rem;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 1rem;
  }

  .empty-icon {
    font-size: 3rem;
    opacity: 0.5;
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
    to {
      transform: rotate(360deg);
    }
  }

  /* Stats View */
  .stats-view {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  /* Articles View */
  .articles-view {
    height: 100%;
  }

  /* Entities View */
  .entities-view {
    height: 100%;
  }

  .articles-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  /* Easter Egg */
  .easter-egg-23 {
    position: fixed;
    bottom: 2rem;
    left: 50%;
    transform: translateX(-50%);
    background: var(--category-3);
    color: var(--bg-base);
    padding: 0.75rem 1.5rem;
    border-radius: 2rem;
    font-weight: 600;
    animation: pulse 2s ease-in-out infinite;
    z-index: 1000;
  }

  @keyframes pulse {
    0%,
    100% {
      transform: translateX(-50%) scale(1);
    }
    50% {
      transform: translateX(-50%) scale(1.05);
    }
  }
</style>
