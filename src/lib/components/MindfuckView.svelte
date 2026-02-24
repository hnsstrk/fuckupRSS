<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { ReadingProfile, BlindSpot, CounterPerspective, ReadingTrend } from "../types";
  import Tabs, { type Tab } from "./Tabs.svelte";
  import Tooltip from "./Tooltip.svelte";
  import { RecommendationList } from "./recommendation";
  import MindfuckOverviewTab from "./MindfuckOverviewTab.svelte";
  import MindfuckBlindSpotsTab from "./MindfuckBlindSpotsTab.svelte";
  import MindfuckTrendsTab from "./MindfuckTrendsTab.svelte";

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
      <MindfuckOverviewTab
        {readingProfile}
        {loadingProfile}
        {biasIndicator}
      />
    {:else if activeTab === "blindSpots"}
      <MindfuckBlindSpotsTab
        {blindSpots}
        {loadingBlindSpots}
      />
    {:else if activeTab === "recommendations"}
      <RecommendationList onArticleClick={handleReadArticle} />
    {:else if activeTab === "trends"}
      <MindfuckTrendsTab
        {readingTrends}
        {loadingTrends}
        {trendPeriod}
        onTrendPeriodChange={handleTrendPeriodChange}
      />
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
</style>
