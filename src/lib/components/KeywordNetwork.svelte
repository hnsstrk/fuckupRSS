<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount, onDestroy, untrack } from 'svelte';
  import type { NetworkGraph as NetworkGraphType } from '../types';
  import NetworkGraph from './NetworkGraph.svelte';
  import KeywordTable from './KeywordTable.svelte';
  import Tabs, { type Tab } from './Tabs.svelte';
  import Tooltip from './Tooltip.svelte';
  import KeywordNetworkDetail from './network/KeywordNetworkDetail.svelte';
  import KeywordNetworkSynonyms from './network/KeywordNetworkSynonyms.svelte';
  import CompoundKeywordManager from './CompoundKeywordManager.svelte';
  import { networkStore } from '../stores/network.svelte';

  let activeTab = $state<string>('list');

  // Tabs definition
  let tabs = $derived<Tab[]>([
    { id: 'list', label: $_('network.listTab') || 'Keywords' },
    { id: 'table', label: $_('network.tableTab') || 'Tabelle' },
    { id: 'graph', label: $_('network.graphTab') || 'Graph' },
    { id: 'synonyms', label: $_('network.synonymsTab') || 'Synonyme' },
    { id: 'compounds', label: $_('network.compoundsTab') || 'Compounds' },
  ]);
  let searchInput = $state('');
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  // Manual merge search timeouts (kept local since they are UI-only)
  let keepSearchTimeout: ReturnType<typeof setTimeout> | null = null;
  let removeSearchTimeout: ReturnType<typeof setTimeout> | null = null;

  // Stable empty graph data to prevent re-renders
  const emptyGraphData: NetworkGraphType = { nodes: [], edges: [] };

  // Graph filter settings (local UI state)
  let graphMinArticleCount = $state(3);
  let graphMinWeight = $state(0.1);

  function openArticle(articleId: number) {
    window.dispatchEvent(new CustomEvent('navigate-to-article', { detail: { articleId } }));
  }

  async function loadGraphDataAsync(forceRefresh = false) {
    if (networkStore.graphData && !forceRefresh) return;
    await networkStore.loadNetworkGraph(100, graphMinWeight, graphMinArticleCount);
  }

  // Track previous keyword to detect actual navigation changes
  let prevSelectedKeywordId = $state<number | null>(null);

  // React to navigation: when selectedKeyword changes externally (e.g. from App.svelte navigate-to-network),
  // ensure we're on the list tab to show the detail panel.
  // Uses untrack for activeTab so tab switches don't re-trigger this effect.
  $effect(() => {
    const kw = networkStore.selectedKeyword;
    const kwId = kw?.id ?? null;
    // Only switch tab when the keyword actually changes to a new value
    if (kwId !== null && kwId !== untrack(() => prevSelectedKeywordId)) {
      untrack(() => {
        if (activeTab !== 'list') {
          activeTab = 'list';
        }
      });
    }
    prevSelectedKeywordId = kwId;
  });

  onMount(async () => {
    networkStore.setupEventListeners();
    await networkStore.refreshAll();
  });

  onDestroy(() => {
    networkStore.teardownEventListeners();
  });

  function handleTabChange(tabId: string) {
    if (tabId === 'graph') {
      loadGraphDataAsync(true);
    }
  }

  function handleGraphNodeClick(nodeId: number) {
    networkStore.selectKeyword(nodeId);
  }

  function handleSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);

    if (!searchInput.trim()) {
      networkStore.searchResults = [];
      return;
    }

    searchTimeout = setTimeout(() => {
      networkStore.searchKeywords(searchInput);
    }, 300);
  }

  function clearSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchInput = '';
    networkStore.clearSearch();
  }

  // === Synonyms Tab Functions (delegating to store) ===

  function handleKeepSearch(value: string) {
    networkStore.keepSearchInput = value;
    if (keepSearchTimeout) clearTimeout(keepSearchTimeout);

    if (!value.trim()) {
      networkStore.keepSearchResults = [];
      return;
    }

    keepSearchTimeout = setTimeout(() => {
      networkStore.searchKeepKeywords(value);
    }, 300);
  }

  function handleRemoveSearch(value: string) {
    networkStore.removeSearchInput = value;
    if (removeSearchTimeout) clearTimeout(removeSearchTimeout);

    if (!value.trim()) {
      networkStore.removeSearchResults = [];
      return;
    }

    removeSearchTimeout = setTimeout(() => {
      networkStore.searchRemoveKeywords(value);
    }, 300);
  }

  function handleNewKeywordInput(value: string) {
    networkStore.newKeywordInput = value;
  }

  function handleRenameInputChange(value: string) {
    networkStore.renameInput = value;
  }

  async function handleShowKeywordArticles(keywordId: number, _keywordName: string) {
    await networkStore.selectKeyword(keywordId);
    activeTab = 'list';
  }
</script>

<div class="keyword-network">
  <!-- Header with Stats and Tabs -->
  <div class="network-header">
    <div class="header-top">
      <h2 class="view-title">
        <i class="fa-solid fa-circle-nodes nav-icon"></i>
        {$_('network.title')}
        <Tooltip termKey="immanentize_network">
          <i class="fa-solid fa-circle-info info-icon"></i>
        </Tooltip>
      </h2>
      {#if networkStore.networkStats}
        <div class="network-stats">
          <span class="stat-item">
            <span class="stat-value">{networkStore.networkStats.total_keywords}</span>
            <span class="stat-label">{$_('network.keywords')}</span>
          </span>
          <span class="stat-item">
            <span class="stat-value">{networkStore.networkStats.total_connections}</span>
            <span class="stat-label">{$_('network.connections')}</span>
          </span>
          <span class="stat-item">
            <span class="stat-value">{networkStore.networkStats.avg_neighbors_per_keyword.toFixed(1)}</span>
            <span class="stat-label">{$_('network.avgNeighbors')}</span>
          </span>
        </div>
      {/if}
    </div>

    <!-- Tabs -->
    <Tabs {tabs} bind:activeTab onchange={handleTabChange} />
  </div>

  <!-- Tab Content -->
  {#if activeTab === 'list'}
  <div class="network-content">
    <!-- Left Panel: Search & Keywords List -->
    <div class="keywords-panel">
      <!-- Search -->
      <div class="search-box">
        <input
          type="text"
          bind:value={searchInput}
          oninput={handleSearch}
          placeholder={$_('network.searchPlaceholder')}
          class="search-input"
        />
        {#if searchInput}
          <button onclick={clearSearch} class="clear-btn">&times;</button>
        {/if}
      </div>

      <!-- Search Results or Keywords List -->
      <div class="keywords-list">
        {#if searchInput && networkStore.searchResults.length > 0}
          <div class="list-section">
            <div class="section-label">{$_('network.searchResults')}</div>
            {#each networkStore.searchResults as keyword (keyword.id)}
              <button
                class="keyword-item {networkStore.selectedKeyword?.id === keyword.id ? 'active' : ''}"
                onclick={() => networkStore.selectKeyword(keyword.id)}
              >
                <span class="keyword-name">{keyword.name}</span>
                <span class="keyword-count">{keyword.article_count}</span>
              </button>
            {/each}
          </div>
        {:else if searchInput && networkStore.searchResults.length === 0 && !networkStore.loading}
          <div class="empty-search">{$_('network.noResults')}</div>
        {:else}
          <!-- Trending Keywords -->
          {#if networkStore.trendingKeywords.length > 0}
            <div class="list-section">
              <div class="section-label">{$_('network.trending')}</div>
              {#each networkStore.trendingKeywords.slice(0, 5) as keyword (keyword.id)}
                <button
                  class="keyword-item trending {networkStore.selectedKeyword?.id === keyword.id ? 'active' : ''}"
                  onclick={() => networkStore.selectKeyword(keyword.id)}
                >
                  <span class="keyword-name">
                    <i class="trend-icon fa-solid fa-caret-up"></i>
                    {keyword.name}
                  </span>
                  <span class="keyword-count">{keyword.recent_count}</span>
                </button>
              {/each}
            </div>
          {/if}

          <!-- All Keywords -->
          <div class="list-section">
            <div class="section-label">{$_('network.allKeywords')}</div>
            {#each networkStore.keywords as keyword (keyword.id)}
              <button
                class="keyword-item {networkStore.selectedKeyword?.id === keyword.id ? 'active' : ''}"
                onclick={() => networkStore.selectKeyword(keyword.id)}
              >
                <span class="keyword-name">{keyword.name}</span>
                <span class="keyword-count">{keyword.article_count}</span>
              </button>
            {/each}

            {#if networkStore.hasMore && !networkStore.loading}
              <button onclick={() => networkStore.loadKeywords(false)} class="load-more">
                {$_('network.loadMore')}
              </button>
            {/if}
          </div>
        {/if}

        {#if networkStore.loading}
          <div class="loading-indicator">{$_('network.loading')}</div>
        {/if}

        {#if networkStore.error}
          <div class="error-message">{networkStore.error}</div>
        {/if}
      </div>
    </div>

    <!-- Right Panel: Keyword Details -->
    <KeywordNetworkDetail
      selectedKeyword={networkStore.selectedKeyword}
      neighbors={networkStore.neighbors}
      keywordCategories={networkStore.keywordCategories}
      keywordArticles={networkStore.keywordArticles}
      cooccurringKeywords={networkStore.cooccurringKeywords}
      similarKeywords={networkStore.similarKeywords}
      similarKeywordsLoading={networkStore.similarKeywordsLoading}
      hasMoreArticles={networkStore.hasMoreArticles}
      articlesLoading={networkStore.articlesLoading}
      loading={networkStore.loading}
      trendDays={networkStore.trendDays}
      isRenaming={networkStore.isRenaming}
      renameInput={networkStore.renameInput}
      renameLoading={networkStore.renameLoading}
      renameError={networkStore.renameError}
      onKeywordSelect={(id) => networkStore.selectKeyword(id)}
      onOpenArticle={openArticle}
      onLoadMoreArticles={() => networkStore.loadMoreArticles()}
      onDaysChange={(days) => networkStore.handleDaysChange(days)}
      onStartRename={() => networkStore.startRename()}
      onCancelRename={() => networkStore.cancelRename()}
      onHandleRename={() => networkStore.handleRename()}
      onRenameInputChange={handleRenameInputChange}
      onSynonymAssigned={() => { networkStore.loadKeywords(true); networkStore.loadNetworkStats(); if (networkStore.selectedKeyword) networkStore.loadSimilarKeywords(networkStore.selectedKeyword.id); }}
    />
  </div>
  {:else if activeTab === 'table'}
  <!-- Table View -->
  <div class="table-view">
    <KeywordTable onKeywordSelect={(id) => networkStore.selectKeyword(id)} onShowKeywordArticles={handleShowKeywordArticles} />
  </div>
  {:else if activeTab === 'graph'}
  <!-- Graph View -->
  <div class="graph-view">
    <div class="graph-filters">
      <label class="filter-item">
        <span class="filter-label">{$_('network.minArticles') || 'Min. Artikel'}:</span>
        <input
          type="range"
          min="1"
          max="10"
          bind:value={graphMinArticleCount}
          onchange={() => loadGraphDataAsync(true)}
        />
        <span class="filter-value">{graphMinArticleCount}</span>
      </label>
      <label class="filter-item">
        <span class="filter-label">{$_('network.minWeight') || 'Min. Gewicht'}:</span>
        <input
          type="range"
          min="0.05"
          max="0.5"
          step="0.05"
          bind:value={graphMinWeight}
          onchange={() => loadGraphDataAsync(true)}
        />
        <span class="filter-value">{graphMinWeight.toFixed(2)}</span>
      </label>
      {#if networkStore.graphData}
        <span class="graph-info">
          {networkStore.graphData.nodes.length} {$_('network.nodes') || 'Knoten'}, {networkStore.graphData.edges.length} {$_('network.edges') || 'Kanten'}
        </span>
      {/if}
    </div>
    <NetworkGraph
      graphData={networkStore.graphData || emptyGraphData}
      onNodeClick={handleGraphNodeClick}
      loading={networkStore.graphLoading}
    />
  </div>
  {:else if activeTab === 'synonyms'}
  <!-- Synonyms View -->
  <KeywordNetworkSynonyms
    synonymCandidates={networkStore.synonymCandidates}
    synonymsLoading={networkStore.synonymsLoading}
    synonymsError={networkStore.synonymsError}
    synonymSuccess={networkStore.synonymSuccess}
    keepSearchInput={networkStore.keepSearchInput}
    keepSearchResults={networkStore.keepSearchResults}
    selectedKeepKeyword={networkStore.selectedKeepKeyword}
    removeSearchInput={networkStore.removeSearchInput}
    removeSearchResults={networkStore.removeSearchResults}
    selectedRemoveKeyword={networkStore.selectedRemoveKeyword}
    newKeywordInput={networkStore.newKeywordInput}
    createKeywordLoading={networkStore.createKeywordLoading}
    createKeywordSuccess={networkStore.createKeywordSuccess}
    createKeywordError={networkStore.createKeywordError}
    onFindSynonyms={() => networkStore.findSynonymCandidates()}
    onMergeKeywords={(keepId, removeId, keepName, removeName) => networkStore.mergeKeywords(keepId, removeId, keepName, removeName)}
    onDismissSynonymPair={(a, b) => networkStore.dismissSynonymPair(a, b)}
    onKeepSearchInput={handleKeepSearch}
    onSelectKeepKeyword={(kw) => networkStore.selectKeepKeyword(kw)}
    onClearKeepSearch={() => networkStore.clearKeepSearch()}
    onRemoveSearchInput={handleRemoveSearch}
    onSelectRemoveKeyword={(kw) => networkStore.selectRemoveKeyword(kw)}
    onClearRemoveSearch={() => networkStore.clearRemoveSearch()}
    onExecuteManualMerge={() => networkStore.executeManualMerge()}
    onNewKeywordInput={handleNewKeywordInput}
    onCreateNewKeyword={() => networkStore.createNewKeyword()}
  />
  {:else if activeTab === 'compounds'}
  <!-- Compound Keywords View -->
  <CompoundKeywordManager loadKeywords={() => networkStore.loadKeywords(true)} />
  {/if}
</div>

<style>
  .keyword-network {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1;
    background-color: var(--bg-base);
  }

  .network-header {
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

  /* .network-title removed - now uses global .view-title class */

  .network-stats {
    display: flex;
    gap: 1.5rem;
    align-items: flex-end;
  }

  .network-stats .stat-item {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .network-stats .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .network-stats .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Table View */
  .table-view {
    flex: 1;
    width: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Graph View */
  .graph-view {
    flex: 1;
    width: 100%;
    padding: 1rem;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .graph-filters {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 0.75rem 1rem;
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
  }

  .filter-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .filter-label {
    white-space: nowrap;
  }

  .filter-item input[type="range"] {
    width: 100px;
    accent-color: var(--accent-primary);
  }

  .filter-value {
    min-width: 2.5rem;
    text-align: right;
    font-weight: 500;
    color: var(--text-primary);
  }

  .graph-info {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--text-muted);
    background-color: var(--bg-overlay);
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
  }

  .graph-view :global(.network-graph-container) {
    flex: 1;
    width: 100%;
    min-height: 0;
  }

  .network-content {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  /* Keywords Panel */
  .keywords-panel {
    width: 280px;
    border-right: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    background-color: var(--bg-surface);
  }

  .search-box {
    padding: 0.75rem;
    border-bottom: 1px solid var(--border-default);
    position: relative;
  }

  .search-input {
    width: 100%;
    padding: 0.5rem 2rem 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .search-input::placeholder {
    color: var(--text-faint);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .clear-btn {
    position: absolute;
    right: 1rem;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1.25rem;
    line-height: 1;
    padding: 0.25rem;
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }

  .keywords-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem 0;
  }

  .list-section {
    margin-bottom: 1rem;
  }

  .section-label {
    padding: 0.25rem 0.75rem;
    font-size: 0.625rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-faint);
  }

  .keyword-item {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background-color 0.15s;
    color: var(--text-primary);
  }

  .keyword-item:hover {
    background-color: var(--bg-overlay);
  }

  .keyword-item.active {
    background-color: var(--bg-overlay);
    border-left: 2px solid var(--accent-primary);
  }

  .keyword-item.trending .trend-icon {
    color: var(--accent-success);
    font-size: 0.625rem;
    margin-right: 0.25rem;
  }

  .keyword-name {
    font-size: 0.875rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .keyword-count {
    font-size: 0.75rem;
    color: var(--text-muted);
    background-color: var(--bg-overlay);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    flex-shrink: 0;
  }

  .load-more {
    width: 100%;
    padding: 0.5rem;
    background: none;
    border: none;
    color: var(--accent-primary);
    cursor: pointer;
    font-size: 0.75rem;
  }

  .load-more:hover {
    text-decoration: underline;
  }

  .loading-indicator,
  .empty-search {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .error-message {
    padding: 0.75rem;
    margin: 0.5rem;
    background-color: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--accent-error);
    border-radius: 0.375rem;
    color: var(--accent-error);
    font-size: 0.75rem;
  }
</style>
