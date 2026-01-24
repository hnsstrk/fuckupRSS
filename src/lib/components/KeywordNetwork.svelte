<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Keyword, KeywordNeighbor, KeywordCategory, TrendingKeyword, NetworkStats, NetworkGraph as NetworkGraphType } from '../stores/state.svelte.ts';
  import NetworkGraph from './NetworkGraph.svelte';
  import KeywordTable from './KeywordTable.svelte';
  import Tabs, { type Tab } from './Tabs.svelte';
  import Tooltip from './Tooltip.svelte';
  import KeywordNetworkDetail from './network/KeywordNetworkDetail.svelte';
  import KeywordNetworkSynonyms from './network/KeywordNetworkSynonyms.svelte';
  import CompoundKeywordManager from './CompoundKeywordManager.svelte';

  // Type for keyword articles
  interface KeywordArticle {
    id: number;
    title: string;
    pentacle_title: string | null;
    published_at: string | null;
    status: string;
  }

  // Type for co-occurring keywords
  interface CooccurringKeyword {
    id: number;
    name: string;
    cooccurrence_count: number;
  }

  // Type for synonym candidate
  interface SynonymCandidate {
    keyword_a_id: number;
    keyword_a_name: string;
    keyword_b_id: number;
    keyword_b_name: string;
    similarity: number;
  }

  // Type for merge result
  interface MergeSynonymsResult {
    merged_pairs: number;
    affected_articles: number;
  }

  // Type for create keyword result
  interface CreateKeywordResult {
    id: number;
    name: string;
    created: boolean;
  }

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

  // Local reactive state
  let keywords = $state<Keyword[]>([]);
  let trendingKeywords = $state<TrendingKeyword[]>([]);
  let networkStats = $state<NetworkStats | null>(null);
  let selectedKeyword = $state<Keyword | null>(null);
  let neighbors = $state<KeywordNeighbor[]>([]);
  let keywordCategories = $state<KeywordCategory[]>([]);
  let keywordArticles = $state<KeywordArticle[]>([]);
  let searchResults = $state<Keyword[]>([]);
  let graphData = $state<NetworkGraphType | null>(null);
  let cooccurringKeywords = $state<CooccurringKeyword[]>([]);
  let trendDays = $state(30);
  let loading = $state(false);
  let graphLoading = $state(false);
  let articlesLoading = $state(false);
  let error = $state<string | null>(null);
  let hasMore = $state(true);
  let hasMoreArticles = $state(true);
  let offset = $state(0);
  let articlesOffset = $state(0);
  const limit = 50;
  const articlesLimit = 10;

  // Synonyms tab state
  let synonymCandidates = $state<SynonymCandidate[]>([]);
  let synonymsLoading = $state(false);
  let synonymsError = $state<string | null>(null);
  let synonymSuccess = $state<string | null>(null);

  // Manual merge - two search fields
  let keepSearchInput = $state('');
  let keepSearchResults = $state<Keyword[]>([]);
  let keepSearchTimeout: ReturnType<typeof setTimeout> | null = null;
  let selectedKeepKeyword = $state<Keyword | null>(null);

  let removeSearchInput = $state('');
  let removeSearchResults = $state<Keyword[]>([]);
  let removeSearchTimeout: ReturnType<typeof setTimeout> | null = null;
  let selectedRemoveKeyword = $state<Keyword | null>(null);

  let newKeywordInput = $state('');
  let createKeywordLoading = $state(false);
  let createKeywordSuccess = $state<string | null>(null);
  let createKeywordError = $state<string | null>(null);

  // Rename keyword state
  let isRenaming = $state(false);
  let renameInput = $state('');
  let renameLoading = $state(false);
  let renameError = $state<string | null>(null);

  // Similar keywords for detail panel
  let similarKeywords = $state<{ id: number; name: string; similarity: number; cooccurrence: number }[]>([]);
  let similarKeywordsLoading = $state(false);

  // Stable empty graph data to prevent re-renders
  const emptyGraphData: NetworkGraphType = { nodes: [], edges: [] };

  async function loadKeywords(reset = false) {
    if (loading) return;
    loading = true;
    error = null;

    if (reset) {
      offset = 0;
      keywords = [];
      hasMore = true;
    }

    try {
      const newKeywords = await invoke<Keyword[]>('get_keywords', { limit, offset });
      if (newKeywords.length < limit) {
        hasMore = false;
      }
      keywords = reset ? newKeywords : [...keywords, ...newKeywords];
      offset += newKeywords.length;
    } catch (e) {
      error = String(e);
      console.error('Failed to load keywords:', e);
    } finally {
      loading = false;
    }
  }

  async function loadTrendingKeywords() {
    try {
      trendingKeywords = await invoke<TrendingKeyword[]>('get_trending_keywords', { days: 7, limit: 20 });
    } catch (e) {
      console.error('Failed to load trending keywords:', e);
    }
  }

  async function loadNetworkStats() {
    try {
      networkStats = await invoke<NetworkStats>('get_network_stats');
    } catch (e) {
      console.error('Failed to load network stats:', e);
    }
  }

  async function selectKeywordById(id: number) {
    loading = true;
    error = null;
    // Reset articles, co-occurring keywords, and similar keywords when selecting new keyword
    keywordArticles = [];
    cooccurringKeywords = [];
    similarKeywords = [];
    articlesOffset = 0;
    hasMoreArticles = true;

    try {
      const [kw, nbrs, cats, articles, cooccurring] = await Promise.all([
        invoke<Keyword | null>('get_keyword', { id }),
        invoke<KeywordNeighbor[]>('get_keyword_neighbors', { id, limit: 10 }),
        invoke<KeywordCategory[]>('get_keyword_categories', { id }),
        invoke<KeywordArticle[]>('get_keyword_articles', { id, limit: articlesLimit, offset: 0 }),
        invoke<CooccurringKeyword[]>('get_cooccurring_keywords', { keywordId: id, days: trendDays, limit: 20 }),
      ]);
      selectedKeyword = kw;
      neighbors = nbrs;
      keywordCategories = cats;
      keywordArticles = articles;
      cooccurringKeywords = cooccurring;
      articlesOffset = articles.length;
      hasMoreArticles = articles.length >= articlesLimit;
      // Load similar keywords after main data is loaded
      loadSimilarKeywords(id);
    } catch (e) {
      error = String(e);
      console.error('Failed to load keyword details:', e);
    } finally {
      loading = false;
    }
  }

  async function loadSimilarKeywords(keywordId: number) {
    similarKeywordsLoading = true;
    try {
      const similar = await invoke<{ id: number; name: string; similarity: number; cooccurrence: number }[]>(
        'get_similar_keywords',
        { keywordId, limit: 8 }
      );
      similarKeywords = similar;
    } catch (e) {
      console.error('Failed to load similar keywords:', e);
      similarKeywords = [];
    } finally {
      similarKeywordsLoading = false;
    }
  }

  async function loadMoreArticles() {
    if (!selectedKeyword || articlesLoading || !hasMoreArticles) return;
    articlesLoading = true;
    try {
      const articles = await invoke<KeywordArticle[]>('get_keyword_articles', {
        id: selectedKeyword.id,
        limit: articlesLimit,
        offset: articlesOffset,
      });
      keywordArticles = [...keywordArticles, ...articles];
      articlesOffset += articles.length;
      hasMoreArticles = articles.length >= articlesLimit;
    } catch (e) {
      console.error('Failed to load more articles:', e);
    } finally {
      articlesLoading = false;
    }
  }

  function openArticle(articleId: number) {
    // Dispatch event to navigate to article view
    window.dispatchEvent(new CustomEvent('navigate-to-article', { detail: { articleId } }));
  }

  async function loadCooccurringKeywords(keywordId: number, days: number) {
    try {
      cooccurringKeywords = await invoke<CooccurringKeyword[]>('get_cooccurring_keywords', {
        keywordId,
        days,
        limit: 20,
      });
    } catch (e) {
      console.error('Failed to load co-occurring keywords:', e);
      cooccurringKeywords = [];
    }
  }

  function handleDaysChange(days: number) {
    trendDays = days;
    if (selectedKeyword) {
      loadCooccurringKeywords(selectedKeyword.id, days);
    }
  }

  // Graph filter settings
  let graphMinArticleCount = $state(3);
  let graphMinWeight = $state(0.1);

  async function loadGraphDataAsync(forceRefresh = false) {
    if (graphData && !forceRefresh) return;
    graphLoading = true;
    try {
      graphData = await invoke<NetworkGraphType>('get_network_graph', {
        limit: 100,
        minWeight: graphMinWeight,
        minArticleCount: graphMinArticleCount,
      });
    } catch (e) {
      console.error('Failed to load graph:', e);
    } finally {
      graphLoading = false;
    }
  }

  async function searchKeywordsLocal(query: string) {
    if (!query.trim()) {
      searchResults = [];
      return;
    }
    try {
      searchResults = await invoke<Keyword[]>('search_keywords', { query, limit: 20 });
    } catch (e) {
      console.error('Failed to search keywords:', e);
    }
  }

  onMount(async () => {
    await Promise.all([
      loadKeywords(true),
      loadTrendingKeywords(),
      loadNetworkStats(),
    ]);
  });

  function handleTabChange(tabId: string) {
    if (tabId === 'graph') {
      // Always refresh graph when switching to tab
      loadGraphDataAsync(true);
    }
  }

  function handleGraphNodeClick(nodeId: number) {
    selectKeywordById(nodeId);
  }

  function handleSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);

    // If input is empty, clear search immediately
    if (!searchInput.trim()) {
      searchResults = [];
      return;
    }

    searchTimeout = setTimeout(() => {
      searchKeywordsLocal(searchInput);
    }, 300);
  }

  function clearSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchInput = '';
    searchResults = [];
  }

  // === Synonyms Tab Functions ===

  async function findSynonymCandidates() {
    synonymsLoading = true;
    synonymsError = null;
    synonymSuccess = null;

    try {
      synonymCandidates = await invoke<SynonymCandidate[]>('find_synonym_candidates', {
        threshold: 0.85,
        limit: 20,
      });
      if (synonymCandidates.length === 0) {
        synonymSuccess = $_('network.noSynonymCandidates') || 'Keine Synonym-Kandidaten gefunden';
      }
    } catch (e) {
      synonymsError = String(e);
      console.error('Failed to find synonym candidates:', e);
    } finally {
      synonymsLoading = false;
    }
  }

  async function mergeKeywords(keepId: number, removeId: number, keepName: string, removeName: string) {
    synonymsLoading = true;
    synonymsError = null;
    synonymSuccess = null;

    try {
      const result = await invoke<MergeSynonymsResult>('merge_keyword_pair', {
        keep_id: keepId,
        remove_id: removeId,
      });
      synonymSuccess = `"${removeName}" -> "${keepName}" (${result.affected_articles} ${$_('network.articleCount') || 'Artikel'})`;
      // Remove the merged candidate from the list
      synonymCandidates = synonymCandidates.filter(
        (c) =>
          !(
            (c.keyword_a_id === keepId && c.keyword_b_id === removeId) ||
            (c.keyword_a_id === removeId && c.keyword_b_id === keepId)
          )
      );
      // Refresh keywords list
      await loadKeywords(true);
      await loadNetworkStats();
    } catch (e) {
      synonymsError = String(e);
      console.error('Failed to merge keywords:', e);
    } finally {
      synonymsLoading = false;
    }
  }

  async function dismissSynonymPair(keywordAId: number, keywordBId: number) {
    synonymsError = null;
    synonymSuccess = null;

    try {
      await invoke('dismiss_synonym_pair', { keywordAId, keywordBId });
      // Remove from list
      synonymCandidates = synonymCandidates.filter(
        (c) =>
          !(
            (c.keyword_a_id === keywordAId && c.keyword_b_id === keywordBId) ||
            (c.keyword_a_id === keywordBId && c.keyword_b_id === keywordAId)
          )
      );
      synonymSuccess = $_('network.synonymDismissed') || 'Synonym-Vorschlag ignoriert';
    } catch (e) {
      synonymsError = String(e);
      console.error('Failed to dismiss synonym pair:', e);
    }
  }

  // Search for "keep" keyword (the one that stays)
  function handleKeepSearch(value: string) {
    keepSearchInput = value;
    if (keepSearchTimeout) clearTimeout(keepSearchTimeout);

    if (!keepSearchInput.trim()) {
      keepSearchResults = [];
      return;
    }

    keepSearchTimeout = setTimeout(async () => {
      try {
        keepSearchResults = await invoke<Keyword[]>('search_keywords', {
          query: keepSearchInput,
          limit: 10,
        });
      } catch (e) {
        console.error('Failed to search keywords:', e);
      }
    }, 300);
  }

  function selectKeepKeyword(keyword: Keyword) {
    selectedKeepKeyword = keyword;
    keepSearchInput = keyword.name;
    keepSearchResults = [];
  }

  function clearKeepSearch() {
    if (keepSearchTimeout) clearTimeout(keepSearchTimeout);
    keepSearchInput = '';
    keepSearchResults = [];
    selectedKeepKeyword = null;
  }

  // Search for "remove" keyword (the one that will be replaced)
  function handleRemoveSearch(value: string) {
    removeSearchInput = value;
    if (removeSearchTimeout) clearTimeout(removeSearchTimeout);

    if (!removeSearchInput.trim()) {
      removeSearchResults = [];
      return;
    }

    removeSearchTimeout = setTimeout(async () => {
      try {
        removeSearchResults = await invoke<Keyword[]>('search_keywords', {
          query: removeSearchInput,
          limit: 10,
        });
      } catch (e) {
        console.error('Failed to search keywords:', e);
      }
    }, 300);
  }

  function selectRemoveKeyword(keyword: Keyword) {
    selectedRemoveKeyword = keyword;
    removeSearchInput = keyword.name;
    removeSearchResults = [];
  }

  function clearRemoveSearch() {
    if (removeSearchTimeout) clearTimeout(removeSearchTimeout);
    removeSearchInput = '';
    removeSearchResults = [];
    selectedRemoveKeyword = null;
  }

  async function executeManualMerge() {
    if (!selectedKeepKeyword || !selectedRemoveKeyword) return;
    if (selectedKeepKeyword.id === selectedRemoveKeyword.id) return;

    await mergeKeywords(
      selectedKeepKeyword.id,
      selectedRemoveKeyword.id,
      selectedKeepKeyword.name,
      selectedRemoveKeyword.name
    );

    // Clear both search fields after successful merge
    clearKeepSearch();
    clearRemoveSearch();
  }

  function handleNewKeywordInput(value: string) {
    newKeywordInput = value;
  }

  async function handleShowKeywordArticles(keywordId: number, _keywordName: string) {
    // Select the keyword and switch to list view to show its articles
    await selectKeywordById(keywordId);
    activeTab = 'list';
  }

  async function createNewKeyword() {
    if (!newKeywordInput.trim()) return;

    createKeywordLoading = true;
    createKeywordError = null;
    createKeywordSuccess = null;

    try {
      const result = await invoke<CreateKeywordResult>('create_keyword', {
        name: newKeywordInput.trim(),
      });

      if (result.created) {
        createKeywordSuccess = `"${result.name}" ${$_('network.keywordCreated') || 'erstellt'}`;
      } else {
        createKeywordSuccess = `"${result.name}" ${$_('network.keywordExists') || 'existiert bereits'}`;
      }
      newKeywordInput = '';
      // Refresh keywords list
      await loadKeywords(true);
      await loadNetworkStats();
    } catch (e) {
      createKeywordError = String(e);
      console.error('Failed to create keyword:', e);
    } finally {
      createKeywordLoading = false;
    }
  }

  function startRename() {
    if (selectedKeyword) {
      renameInput = selectedKeyword.name;
      isRenaming = true;
      renameError = null;
    }
  }

  function cancelRename() {
    isRenaming = false;
    renameInput = '';
    renameError = null;
  }

  function handleRenameInputChange(value: string) {
    renameInput = value;
  }

  async function handleRename() {
    if (!selectedKeyword || !renameInput.trim()) return;
    if (renameInput.trim() === selectedKeyword.name) {
      cancelRename();
      return;
    }

    renameLoading = true;
    renameError = null;

    try {
      const newName = await invoke<string>('rename_keyword', {
        id: selectedKeyword.id,
        newName: renameInput.trim(),
      });

      // Update local state
      selectedKeyword = { ...selectedKeyword, name: newName };

      // Update in keywords list
      keywords = keywords.map(k =>
        k.id === selectedKeyword!.id ? { ...k, name: newName } : k
      );

      // Update in trending if present
      trendingKeywords = trendingKeywords.map(k =>
        k.id === selectedKeyword!.id ? { ...k, name: newName } : k
      );

      isRenaming = false;
      renameInput = '';
    } catch (e) {
      renameError = String(e);
      console.error('Failed to rename keyword:', e);
    } finally {
      renameLoading = false;
    }
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
      {#if networkStats}
        <div class="network-stats">
          <span class="stat-item">
            <span class="stat-value">{networkStats.total_keywords}</span>
            <span class="stat-label">{$_('network.keywords')}</span>
          </span>
          <span class="stat-item">
            <span class="stat-value">{networkStats.total_connections}</span>
            <span class="stat-label">{$_('network.connections')}</span>
          </span>
          <span class="stat-item">
            <span class="stat-value">{networkStats.avg_neighbors_per_keyword.toFixed(1)}</span>
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
        {#if searchInput && searchResults.length > 0}
          <div class="list-section">
            <div class="section-label">{$_('network.searchResults')}</div>
            {#each searchResults as keyword (keyword.id)}
              <button
                class="keyword-item {selectedKeyword?.id === keyword.id ? 'active' : ''}"
                onclick={() => selectKeywordById(keyword.id)}
              >
                <span class="keyword-name">{keyword.name}</span>
                <span class="keyword-count">{keyword.article_count}</span>
              </button>
            {/each}
          </div>
        {:else if searchInput && searchResults.length === 0 && !loading}
          <div class="empty-search">{$_('network.noResults')}</div>
        {:else}
          <!-- Trending Keywords -->
          {#if trendingKeywords.length > 0}
            <div class="list-section">
              <div class="section-label">{$_('network.trending')}</div>
              {#each trendingKeywords.slice(0, 5) as keyword (keyword.id)}
                <button
                  class="keyword-item trending {selectedKeyword?.id === keyword.id ? 'active' : ''}"
                  onclick={() => selectKeywordById(keyword.id)}
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
            {#each keywords as keyword (keyword.id)}
              <button
                class="keyword-item {selectedKeyword?.id === keyword.id ? 'active' : ''}"
                onclick={() => selectKeywordById(keyword.id)}
              >
                <span class="keyword-name">{keyword.name}</span>
                <span class="keyword-count">{keyword.article_count}</span>
              </button>
            {/each}

            {#if hasMore && !loading}
              <button onclick={() => loadKeywords(false)} class="load-more">
                {$_('network.loadMore')}
              </button>
            {/if}
          </div>
        {/if}

        {#if loading}
          <div class="loading-indicator">{$_('network.loading')}</div>
        {/if}

        {#if error}
          <div class="error-message">{error}</div>
        {/if}
      </div>
    </div>

    <!-- Right Panel: Keyword Details -->
    <KeywordNetworkDetail
      {selectedKeyword}
      {neighbors}
      {keywordCategories}
      {keywordArticles}
      {cooccurringKeywords}
      {similarKeywords}
      {similarKeywordsLoading}
      {hasMoreArticles}
      {articlesLoading}
      {loading}
      {trendDays}
      {isRenaming}
      {renameInput}
      {renameLoading}
      {renameError}
      onKeywordSelect={selectKeywordById}
      onOpenArticle={openArticle}
      onLoadMoreArticles={loadMoreArticles}
      onDaysChange={handleDaysChange}
      onStartRename={startRename}
      onCancelRename={cancelRename}
      onHandleRename={handleRename}
      onRenameInputChange={handleRenameInputChange}
      onSynonymAssigned={() => { loadKeywords(); if (selectedKeywordId) loadSimilarKeywords(selectedKeywordId); }}
    />
  </div>
  {:else if activeTab === 'table'}
  <!-- Table View -->
  <div class="table-view">
    <KeywordTable onKeywordSelect={selectKeywordById} onShowKeywordArticles={handleShowKeywordArticles} />
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
      {#if graphData}
        <span class="graph-info">
          {graphData.nodes.length} {$_('network.nodes') || 'Knoten'}, {graphData.edges.length} {$_('network.edges') || 'Kanten'}
        </span>
      {/if}
    </div>
    <NetworkGraph
      graphData={graphData || emptyGraphData}
      onNodeClick={handleGraphNodeClick}
      loading={graphLoading}
    />
  </div>
  {:else if activeTab === 'synonyms'}
  <!-- Synonyms View -->
  <KeywordNetworkSynonyms
    {synonymCandidates}
    {synonymsLoading}
    {synonymsError}
    {synonymSuccess}
    {keepSearchInput}
    {keepSearchResults}
    {selectedKeepKeyword}
    {removeSearchInput}
    {removeSearchResults}
    {selectedRemoveKeyword}
    {newKeywordInput}
    {createKeywordLoading}
    {createKeywordSuccess}
    {createKeywordError}
    onFindSynonyms={findSynonymCandidates}
    onMergeKeywords={mergeKeywords}
    onDismissSynonymPair={dismissSynonymPair}
    onKeepSearchInput={handleKeepSearch}
    onSelectKeepKeyword={selectKeepKeyword}
    onClearKeepSearch={clearKeepSearch}
    onRemoveSearchInput={handleRemoveSearch}
    onSelectRemoveKeyword={selectRemoveKeyword}
    onClearRemoveSearch={clearRemoveSearch}
    onExecuteManualMerge={executeManualMerge}
    onNewKeywordInput={handleNewKeywordInput}
    onCreateNewKeyword={createNewKeyword}
  />
  {:else if activeTab === 'compounds'}
  <!-- Compound Keywords View -->
  <CompoundKeywordManager loadKeywords={() => loadKeywords(true)} />
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
