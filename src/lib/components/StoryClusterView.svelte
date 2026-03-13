<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { SvelteSet } from "svelte/reactivity";
  import { getBiasColor } from "$lib/utils/articleFormat";
  import { renderMarkdown } from "$lib/utils/sanitizer";

  // Types matching backend structs
  interface StoryCluster {
    id: number;
    title: string;
    summary: string | null;
    perspective_comparison: string | null;
    article_count: number;
    source_names: string[];
    created_at: string | null;
    updated_at: string | null;
  }

  interface StoryClusterArticle {
    fnord_id: number;
    title: string;
    summary: string | null;
    political_bias: number | null;
    sachlichkeit: number | null;
    source_name: string;
    published_at: string | null;
    similarity_score: number;
  }

  interface StoryClusterDetail {
    cluster: StoryCluster;
    articles: StoryClusterArticle[];
  }

  interface DiscoverResult {
    clusters_found: number;
    clusters: StoryCluster[];
  }

  // State
  let clusters = $state<StoryCluster[]>([]);
  let selectedClusterId = $state<number | null>(null);
  let clusterDetail = $state<StoryClusterDetail | null>(null);
  let loading = $state(false);
  let discovering = $state(false);
  let comparing = $state(false);
  let detailLoading = $state(false);

  // Settings
  let showSettings = $state(false);
  let minArticles = $state(3);
  let days = $state(7);

  // Expanded summaries
  let expandedArticles = new SvelteSet<number>();

  onMount(async () => {
    window.addEventListener("batch-complete", handleBatchComplete);
    await loadClusters();
  });

  onDestroy(() => {
    window.removeEventListener("batch-complete", handleBatchComplete);
  });

  async function handleBatchComplete() {
    await loadClusters();
  }

  async function loadClusters() {
    loading = true;
    try {
      clusters = await invoke<StoryCluster[]>("get_story_clusters", { limit: 50 });
    } catch (e) {
      console.error("[StoryClusterView] Error loading clusters:", e);
      clusters = [];
    } finally {
      loading = false;
    }
  }

  async function discoverClusters() {
    discovering = true;
    try {
      const result = await invoke<DiscoverResult>("discover_story_clusters", {
        minArticles,
        days,
      });
      if (result.clusters_found > 0) {
        await loadClusters();
      }
    } catch (e) {
      console.error("[StoryClusterView] Error discovering clusters:", e);
    } finally {
      discovering = false;
    }
  }

  async function selectCluster(clusterId: number) {
    selectedClusterId = clusterId;
    detailLoading = true;
    try {
      clusterDetail = await invoke<StoryClusterDetail>("get_story_cluster_detail", {
        clusterId,
      });
    } catch (e) {
      console.error("[StoryClusterView] Error loading cluster detail:", e);
      clusterDetail = null;
    } finally {
      detailLoading = false;
    }
  }

  async function comparePerspectives() {
    if (!selectedClusterId) return;
    comparing = true;
    try {
      const result = await invoke<string>("compare_perspectives", {
        clusterId: selectedClusterId,
      });
      // Update detail with comparison
      if (clusterDetail) {
        clusterDetail = {
          ...clusterDetail,
          cluster: {
            ...clusterDetail.cluster,
            perspective_comparison: result,
          },
        };
      }
      // Also update cluster in list
      clusters = clusters.map((c) =>
        c.id === selectedClusterId ? { ...c, perspective_comparison: result } : c,
      );
    } catch (e) {
      console.error("[StoryClusterView] Error comparing perspectives:", e);
    } finally {
      comparing = false;
    }
  }

  async function deleteCluster(clusterId: number) {
    if (!confirm($_("storyClusters.deleteConfirm"))) return;
    try {
      await invoke("delete_story_cluster", { clusterId });
      clusters = clusters.filter((c) => c.id !== clusterId);
      if (selectedClusterId === clusterId) {
        selectedClusterId = null;
        clusterDetail = null;
      }
    } catch (e) {
      console.error("[StoryClusterView] Error deleting cluster:", e);
    }
  }

  function toggleArticleSummary(fnordId: number) {
    if (expandedArticles.has(fnordId)) {
      expandedArticles.delete(fnordId);
    } else {
      expandedArticles.add(fnordId);
    }
  }

  function biasIndicator(bias: number | null): string {
    if (bias === null) return "";
    const labels: Record<number, string> = {
      "-2": "<<",
      "-1": "<",
      0: "=",
      1: ">",
      2: ">>",
    };
    return labels[bias] ?? "";
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return "";
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString("de-DE", {
        day: "2-digit",
        month: "2-digit",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return dateStr;
    }
  }
</script>

<div class="story-cluster-view">
  <!-- Header -->
  <div class="sc-header">
    <div class="sc-header-left">
      <h2>
        <i class="fa-solid fa-layer-group"></i>
        {$_("storyClusters.title")}
      </h2>
      {#if clusters.length > 0}
        <span class="cluster-count">{clusters.length}</span>
      {/if}
    </div>
    <div class="sc-header-actions">
      <button
        class="btn-icon"
        onclick={() => (showSettings = !showSettings)}
        title={$_("storyClusters.settings")}
      >
        <i class="fa-solid fa-sliders"></i>
      </button>
      <button class="btn-primary" onclick={discoverClusters} disabled={discovering}>
        {#if discovering}
          <i class="fa-solid fa-spinner fa-spin"></i>
          {$_("storyClusters.discovering")}
        {:else}
          <i class="fa-solid fa-magnifying-glass-chart"></i>
          {$_("storyClusters.discover")}
        {/if}
      </button>
    </div>
  </div>

  <!-- Settings panel -->
  {#if showSettings}
    <div class="sc-settings">
      <div class="setting-row">
        <label for="sc-min-articles">{$_("storyClusters.minArticles")}</label>
        <input id="sc-min-articles" type="number" min="2" max="20" bind:value={minArticles} />
      </div>
      <div class="setting-row">
        <label for="sc-days">{$_("storyClusters.days")}</label>
        <input id="sc-days" type="number" min="1" max="90" bind:value={days} />
      </div>
    </div>
  {/if}

  <!-- Main content: two-panel layout -->
  <div class="sc-panels">
    <!-- Left: Cluster list -->
    <div class="sc-list-panel">
      {#if loading}
        <div class="sc-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
        </div>
      {:else if clusters.length === 0}
        <div class="sc-empty">
          <i class="fa-light fa-layer-group"></i>
          <p>{$_("storyClusters.noClusters")}</p>
        </div>
      {:else}
        <div class="sc-cluster-list">
          {#each clusters as cluster (cluster.id)}
            <button
              class="sc-cluster-card"
              class:selected={selectedClusterId === cluster.id}
              onclick={() => selectCluster(cluster.id)}
            >
              <div class="sc-card-title">{cluster.title}</div>
              <div class="sc-card-meta">
                <span class="sc-meta-item">
                  <i class="fa-solid fa-newspaper"></i>
                  {cluster.article_count}
                  {$_("storyClusters.articles")}
                </span>
                <span class="sc-meta-item">
                  <i class="fa-solid fa-rss"></i>
                  {cluster.source_names.length}
                  {$_("storyClusters.sources")}
                </span>
              </div>
              <div class="sc-card-sources">
                {#each cluster.source_names as source, i (i)}
                  <span class="sc-source-badge">{source}</span>
                {/each}
              </div>
              {#if cluster.perspective_comparison}
                <div class="sc-card-has-comparison">
                  <i class="fa-solid fa-check-circle"></i>
                  {$_("storyClusters.comparison")}
                </div>
              {/if}
              {#if cluster.updated_at}
                <div class="sc-card-date">{formatDate(cluster.updated_at)}</div>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Right: Detail panel -->
    <div class="sc-detail-panel">
      {#if !selectedClusterId}
        <div class="sc-empty">
          <i class="fa-light fa-magnifying-glass-chart"></i>
          <p>{$_("storyClusters.selectCluster")}</p>
        </div>
      {:else if detailLoading}
        <div class="sc-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
        </div>
      {:else if clusterDetail}
        <div class="sc-detail">
          <!-- Detail header -->
          <div class="sc-detail-header">
            <h3>{clusterDetail.cluster.title}</h3>
            <div class="sc-detail-actions">
              <button class="btn-primary" onclick={comparePerspectives} disabled={comparing}>
                {#if comparing}
                  <i class="fa-solid fa-spinner fa-spin"></i>
                  {$_("storyClusters.comparing")}
                {:else}
                  <i class="fa-solid fa-scale-balanced"></i>
                  {$_("storyClusters.perspectives")}
                {/if}
              </button>
              <button
                class="btn-danger"
                onclick={() => deleteCluster(clusterDetail!.cluster.id)}
                title={$_("storyClusters.delete")}
              >
                <i class="fa-solid fa-trash"></i>
              </button>
            </div>
          </div>

          <!-- Perspective Comparison -->
          {#if clusterDetail.cluster.perspective_comparison}
            <div class="sc-comparison">
              <h4>
                <i class="fa-solid fa-scale-balanced"></i>
                {$_("storyClusters.comparison")}
              </h4>
              <div class="sc-comparison-text markdown-content">
                {@html renderMarkdown(clusterDetail.cluster.perspective_comparison)}
              </div>
            </div>
          {/if}

          <!-- Articles list -->
          <div class="sc-articles">
            <h4>
              <i class="fa-solid fa-newspaper"></i>
              {clusterDetail.articles.length}
              {$_("storyClusters.articles")}
              &mdash;
              {clusterDetail.cluster.source_names.length}
              {$_("storyClusters.sources")}
            </h4>

            {#each clusterDetail.articles as article (article.fnord_id)}
              <div class="sc-article-card">
                <div class="sc-article-header">
                  <div class="sc-article-source">
                    <span class="sc-source-badge">{article.source_name}</span>
                    {#if article.political_bias !== null}
                      <span
                        class="sc-bias-indicator"
                        style="color: {getBiasColor(article.political_bias)}"
                        title="{$_('storyClusters.bias')}: {biasIndicator(article.political_bias)}"
                      >
                        {biasIndicator(article.political_bias)}
                      </span>
                    {/if}
                    <span class="sc-similarity">
                      {Math.round(article.similarity_score * 100)}%
                    </span>
                  </div>
                  {#if article.published_at}
                    <span class="sc-article-date">
                      {formatDate(article.published_at)}
                    </span>
                  {/if}
                </div>

                <button
                  class="sc-article-title"
                  onclick={() => toggleArticleSummary(article.fnord_id)}
                >
                  {article.title}
                  <i
                    class="fa-solid"
                    class:fa-chevron-down={!expandedArticles.has(article.fnord_id)}
                    class:fa-chevron-up={expandedArticles.has(article.fnord_id)}
                  ></i>
                </button>

                {#if expandedArticles.has(article.fnord_id) && article.summary}
                  <div class="sc-article-summary markdown-content">
                    {@html renderMarkdown(article.summary)}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .story-cluster-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
  }

  /* Header */
  .sc-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .sc-header-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .sc-header-left h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .cluster-count {
    background: var(--accent-color);
    color: var(--accent-text);
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.1rem 0.5rem;
    border-radius: 10px;
  }

  .sc-header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  /* Buttons */
  .btn-primary {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.8rem;
    background: var(--accent-color);
    color: var(--accent-text);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
    transition: opacity 0.15s;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-icon:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .btn-danger {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.6rem;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.15s;
  }

  .btn-danger:hover {
    background: var(--red, #e06c75);
    color: white;
    border-color: var(--red, #e06c75);
  }

  /* Settings panel */
  .sc-settings {
    display: flex;
    gap: 1rem;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .setting-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .setting-row label {
    font-size: 0.8rem;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .setting-row input {
    width: 60px;
    padding: 0.25rem 0.4rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.85rem;
  }

  /* Two-panel layout */
  .sc-panels {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .sc-list-panel {
    width: 340px;
    min-width: 280px;
    border-right: 1px solid var(--border-color);
    overflow-y: auto;
    flex-shrink: 0;
  }

  .sc-detail-panel {
    flex: 1;
    overflow-y: auto;
    min-width: 0;
  }

  /* Loading / Empty states */
  .sc-loading,
  .sc-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: var(--text-muted);
    gap: 0.75rem;
    height: 100%;
  }

  .sc-empty i {
    font-size: 2.5rem;
    opacity: 0.4;
  }

  .sc-empty p {
    margin: 0;
    font-size: 0.9rem;
  }

  /* Cluster list */
  .sc-cluster-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
  }

  .sc-cluster-card {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.6rem 0.75rem;
    background: var(--bg-primary);
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
    width: 100%;
    font-family: inherit;
    color: inherit;
  }

  .sc-cluster-card:hover {
    background: var(--bg-hover);
    border-color: var(--border-color);
  }

  .sc-cluster-card.selected {
    background: var(--bg-active, var(--bg-secondary));
    border-color: var(--accent-color);
  }

  .sc-card-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.3;
  }

  .sc-card-meta {
    display: flex;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .sc-meta-item {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .sc-card-sources {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .sc-source-badge {
    display: inline-block;
    padding: 0.1rem 0.4rem;
    background: var(--bg-tertiary, var(--bg-secondary));
    color: var(--text-secondary);
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 500;
    white-space: nowrap;
  }

  .sc-card-has-comparison {
    font-size: 0.72rem;
    color: var(--green, #98c379);
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .sc-card-date {
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  /* Detail panel */
  .sc-detail {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .sc-detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }

  .sc-detail-header h3 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.4;
  }

  .sc-detail-actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  /* Comparison section */
  .sc-comparison {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1rem;
  }

  .sc-comparison h4 {
    margin: 0 0 0.75rem 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--accent-color);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .sc-comparison-text {
    font-size: 0.88rem;
    line-height: 1.6;
    color: var(--text-primary);
  }

  .sc-comparison-text h4 {
    margin: 1rem 0 0.4rem 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .sc-comparison-text h5 {
    margin: 0.75rem 0 0.3rem 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .sc-comparison-text p {
    margin: 0 0 0.5rem 0;
  }

  /* Articles section */
  .sc-articles h4 {
    margin: 0 0 0.5rem 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .sc-article-card {
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    margin-bottom: 0.5rem;
    overflow: hidden;
  }

  .sc-article-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.75rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .sc-article-source {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .sc-bias-indicator {
    font-size: 0.8rem;
    font-weight: 700;
    font-family: monospace;
  }

  .sc-similarity {
    font-size: 0.72rem;
    color: var(--text-muted);
    background: var(--bg-tertiary, var(--bg-primary));
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
  }

  .sc-article-date {
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  .sc-article-title {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.5rem;
    width: 100%;
    padding: 0.6rem 0.75rem;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    font-size: 0.88rem;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.4;
    font-family: inherit;
    transition: background 0.1s;
  }

  .sc-article-title:hover {
    background: var(--bg-hover);
  }

  .sc-article-title i {
    flex-shrink: 0;
    font-size: 0.7rem;
    color: var(--text-muted);
    margin-top: 0.2rem;
  }

  .sc-article-summary {
    padding: 0.5rem 0.75rem 0.75rem;
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--text-secondary);
    border-top: 1px solid var(--border-color);
  }
</style>
