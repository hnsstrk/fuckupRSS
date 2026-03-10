<script lang="ts">
  import { _ } from "svelte-i18n";
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import EntityBadge from "./EntityBadge.svelte";

  interface EntityInfo {
    id: number;
    name: string;
    entity_type: string;
    normalized_name: string;
    article_count: number;
    mention_count: number | null;
    confidence: number | null;
  }

  interface EntityArticle {
    fnord_id: number;
    title: string;
    url: string;
    published_at: string | null;
    summary: string | null;
    mention_count: number;
  }

  let searchQuery = $state("");
  let activeTab = $state("all");
  let entities = $state<EntityInfo[]>([]);
  let loading = $state(false);
  let selectedEntity = $state<EntityInfo | null>(null);
  let entityArticles = $state<EntityArticle[]>([]);
  let loadingArticles = $state(false);

  const tabs = ["all", "person", "organization", "location", "event"];

  async function loadTopEntities() {
    loading = true;
    try {
      const typeFilter = activeTab === "all" ? null : activeTab;
      entities = await invoke<EntityInfo[]>("get_top_entities", {
        entityType: typeFilter,
        limit: 50,
      });
    } catch (e) {
      console.error("Failed to load entities:", e);
      entities = [];
    } finally {
      loading = false;
    }
  }

  async function searchEntities() {
    if (!searchQuery.trim()) {
      await loadTopEntities();
      return;
    }
    loading = true;
    try {
      const typeFilter = activeTab === "all" ? null : activeTab;
      entities = await invoke<EntityInfo[]>("search_entities", {
        query: searchQuery,
        entityType: typeFilter,
      });
    } catch (e) {
      console.error("Failed to search entities:", e);
      entities = [];
    } finally {
      loading = false;
    }
  }

  async function selectEntity(entity: EntityInfo) {
    selectedEntity = entity;
    loadingArticles = true;
    try {
      entityArticles = await invoke<EntityArticle[]>("get_entity_articles", {
        entityId: entity.id,
      });
    } catch (e) {
      console.error("Failed to load entity articles:", e);
      entityArticles = [];
    } finally {
      loadingArticles = false;
    }
  }

  function clearSelection() {
    selectedEntity = null;
    entityArticles = [];
  }

  function getTabLabel(tab: string): string {
    if (tab === "all") return $_("entities.all");
    return $_(`entities.${tab}`);
  }

  // Reload when tab changes
  $effect(() => {
    // Track activeTab
    const _tab = activeTab;
    if (searchQuery.trim()) {
      searchEntities();
    } else {
      loadTopEntities();
    }
  });

  // Debounced search
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  function onSearchInput() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      searchEntities();
    }, 300);
  }

  // Event listener for batch-complete to refresh
  function handleBatchComplete() {
    loadTopEntities();
  }

  onMount(() => {
    window.addEventListener("batch-complete", handleBatchComplete);
  });

  onDestroy(() => {
    window.removeEventListener("batch-complete", handleBatchComplete);
    if (searchTimeout) clearTimeout(searchTimeout);
  });
</script>

<div class="entity-explorer">
  <div class="explorer-header">
    <h3 class="explorer-title">
      <i class="fa-solid fa-diagram-project"></i>
      {$_("entities.title")}
    </h3>
  </div>

  <!-- Search -->
  <div class="search-bar">
    <i class="fa-solid fa-search search-icon"></i>
    <input
      type="text"
      bind:value={searchQuery}
      oninput={onSearchInput}
      placeholder={$_("entities.search")}
      class="search-input"
    />
  </div>

  <!-- Tabs -->
  <div class="tab-bar">
    {#each tabs as tab (tab)}
      <button
        class="tab-btn"
        class:active={activeTab === tab}
        onclick={() => (activeTab = tab)}
      >
        {getTabLabel(tab)}
      </button>
    {/each}
  </div>

  <!-- Content -->
  {#if selectedEntity}
    <!-- Entity Detail View -->
    <div class="entity-detail">
      <button class="back-btn" onclick={clearSelection}>
        <i class="fa-solid fa-arrow-left"></i>
        {$_("entities.topEntities")}
      </button>

      <div class="detail-header">
        <EntityBadge
          name={selectedEntity.name}
          entityType={selectedEntity.entity_type}
          articleCount={selectedEntity.article_count}
        />
        <span class="detail-count">
          {selectedEntity.article_count}
          {$_("entities.articles")}
        </span>
      </div>

      <h4 class="detail-section-title">{$_("entities.relatedArticles")}</h4>

      {#if loadingArticles}
        <div class="loading-text">...</div>
      {:else if entityArticles.length === 0}
        <div class="empty-text">{$_("entities.noEntities")}</div>
      {:else}
        <div class="article-list">
          {#each entityArticles as article (article.fnord_id)}
            <div class="article-item">
              <div class="article-title">{article.title}</div>
              {#if article.summary}
                <div class="article-summary">{article.summary}</div>
              {/if}
              <div class="article-meta">
                {#if article.published_at}
                  <span class="article-date">{article.published_at}</span>
                {/if}
                <span class="article-mentions">
                  {article.mention_count}
                  {$_("entities.mentions")}
                </span>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Entity List -->
    {#if loading}
      <div class="loading-text">...</div>
    {:else if entities.length === 0}
      <div class="empty-text">{$_("entities.noEntities")}</div>
    {:else}
      <div class="entity-list">
        {#each entities as entity (entity.id)}
          <button class="entity-row" onclick={() => selectEntity(entity)}>
            <EntityBadge
              name={entity.name}
              entityType={entity.entity_type}
            />
            <span class="entity-count">
              {entity.article_count}
            </span>
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .entity-explorer {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .explorer-header {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-default);
  }

  .explorer-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .search-bar {
    padding: 0.5rem 1rem;
    position: relative;
    border-bottom: 1px solid var(--border-default);
  }

  .search-icon {
    position: absolute;
    left: 1.5rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .search-input {
    width: 100%;
    padding: 0.375rem 0.5rem 0.375rem 1.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.8rem;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .tab-bar {
    display: flex;
    gap: 0;
    padding: 0 0.5rem;
    border-bottom: 1px solid var(--border-default);
    overflow-x: auto;
  }

  .tab-btn {
    padding: 0.5rem 0.75rem;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .tab-btn:hover {
    color: var(--text-primary);
  }

  .tab-btn.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
  }

  .entity-list {
    overflow-y: auto;
    flex: 1;
    padding: 0.5rem;
  }

  .entity-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.375rem 0.5rem;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: 0.375rem;
    transition: background-color 0.15s;
  }

  .entity-row:hover {
    background-color: var(--bg-overlay);
  }

  .entity-count {
    font-size: 0.7rem;
    color: var(--text-muted);
    min-width: 1.5rem;
    text-align: right;
  }

  .loading-text,
  .empty-text {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  /* Detail view */
  .entity-detail {
    overflow-y: auto;
    flex: 1;
    padding: 0.75rem;
  }

  .back-btn {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    border: none;
    background: none;
    color: var(--accent-primary);
    font-size: 0.75rem;
    cursor: pointer;
    margin-bottom: 0.75rem;
  }

  .back-btn:hover {
    text-decoration: underline;
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .detail-count {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .detail-section-title {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.5rem 0;
  }

  .article-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .article-item {
    padding: 0.5rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: var(--bg-surface);
  }

  .article-title {
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .article-summary {
    font-size: 0.7rem;
    color: var(--text-secondary);
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    margin-bottom: 0.25rem;
  }

  .article-meta {
    display: flex;
    gap: 0.75rem;
    font-size: 0.65rem;
    color: var(--text-muted);
  }
</style>
