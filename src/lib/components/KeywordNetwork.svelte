<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Keyword, KeywordNeighbor, KeywordCategory, TrendingKeyword, NetworkStats, NetworkGraph as NetworkGraphType } from '../stores/state.svelte';
  import Tooltip from './Tooltip.svelte';
  import NetworkGraph from './NetworkGraph.svelte';
  import KeywordTrendChart from './KeywordTrendChart.svelte';
  import Tabs, { type Tab } from './Tabs.svelte';

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
    { id: 'graph', label: $_('network.graphTab') || 'Graph' },
    { id: 'synonyms', label: $_('network.synonymsTab') || 'Synonyme' },
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

  // Table view state
  let viewMode = $state<'list' | 'table'>('list');
  let sortColumn = $state<'name' | 'article_count' | 'first_seen' | 'last_used'>('article_count');
  let sortDirection = $state<'asc' | 'desc'>('desc');
  let minArticleFilter = $state(0);
  let showFilters = $state(false);

  // Similar keywords for detail panel
  let similarKeywords = $state<{ id: number; name: string; similarity: number; cooccurrence: number }[]>([]);
  let similarKeywordsLoading = $state(false);

  // Derived: sorted and filtered keywords for table view
  let filteredKeywords = $derived(() => {
    let result = searchInput && searchResults.length > 0 ? searchResults : keywords;

    // Apply min article filter
    if (minArticleFilter > 0) {
      result = result.filter(k => k.article_count >= minArticleFilter);
    }

    return result;
  });

  let sortedKeywords = $derived(() => {
    const data = [...filteredKeywords];

    data.sort((a, b) => {
      let aVal: string | number | null;
      let bVal: string | number | null;

      switch (sortColumn) {
        case 'name':
          aVal = a.name.toLowerCase();
          bVal = b.name.toLowerCase();
          break;
        case 'article_count':
          aVal = a.article_count;
          bVal = b.article_count;
          break;
        case 'first_seen':
          aVal = a.first_seen || '';
          bVal = b.first_seen || '';
          break;
        case 'last_used':
          aVal = a.last_used || '';
          bVal = b.last_used || '';
          break;
        default:
          return 0;
      }

      if (aVal < bVal) return sortDirection === 'asc' ? -1 : 1;
      if (aVal > bVal) return sortDirection === 'asc' ? 1 : -1;
      return 0;
    });

    return data;
  });

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

      // Load similar keywords (embedding-based)
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

  function handleSort(column: 'name' | 'article_count' | 'first_seen' | 'last_used') {
    if (sortColumn === column) {
      // Toggle direction
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortColumn = column;
      sortDirection = column === 'name' ? 'asc' : 'desc';
    }
  }

  function getSortIcon(column: string): string {
    if (sortColumn !== column) return 'fa-sort';
    return sortDirection === 'asc' ? 'fa-sort-up' : 'fa-sort-down';
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

  async function loadGraphDataAsync(forceRefresh = false) {
    if (graphData && !forceRefresh) return;
    graphLoading = true;
    try {
      graphData = await invoke<NetworkGraphType>('get_network_graph', { limit: 100, minWeight: 0.01 });
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

  function formatArticleDate(dateStr: string | null): string {
    if (!dateStr) return '';
    const date = new Date(dateStr);
    return date.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });
  }

  function getStatusIconClass(status: string): string {
    switch (status) {
      case 'concealed': return 'fa-solid fa-eye-slash';
      case 'illuminated': return 'fa-solid fa-check';
      case 'golden_apple': return 'fa-solid fa-apple-whole';
      default: return '';
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

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleDateString();
  }

  function getWeightClass(weight: number): string {
    if (weight >= 0.7) return 'weight-high';
    if (weight >= 0.4) return 'weight-medium';
    return 'weight-low';
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
        keepId,
        removeId,
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
  function handleKeepSearch() {
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
  function handleRemoveSearch() {
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

  function handleRenameKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleRename();
    } else if (e.key === 'Escape') {
      cancelRename();
    }
  }
</script>

<div class="keyword-network">
  <!-- Header with Stats and Tabs -->
  <div class="network-header">
    <div class="header-top">
      <h2 class="network-title">
        <Tooltip termKey="immanentize">{$_('network.title')}</Tooltip>
      </h2>
      {#if networkStats}
        <div class="network-stats">
          <span class="stat">{$_('network.keywords')}: <strong>{networkStats.total_keywords}</strong></span>
          <span class="stat">{$_('network.connections')}: <strong>{networkStats.total_connections}</strong></span>
          <span class="stat">{$_('network.avgNeighbors')}: <strong>{networkStats.avg_neighbors_per_keyword.toFixed(1)}</strong></span>
        </div>
      {/if}
    </div>

    <!-- Tabs -->
    <Tabs {tabs} bind:activeTab onchange={handleTabChange} />
  </div>

  <!-- Tab Content -->
  {#if activeTab === 'list'}
  <div class="network-content">
    <!-- Left Panel: Search & Keywords List/Table -->
    <div class="keywords-panel" class:table-mode={viewMode === 'table'}>
      <!-- Toolbar: Search + View Toggle + Filters -->
      <div class="panel-toolbar">
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

        <div class="toolbar-actions">
          <!-- Filter Toggle -->
          <button
            class="toolbar-btn"
            class:active={showFilters}
            onclick={() => showFilters = !showFilters}
            title={$_('network.filters') || 'Filter'}
          >
            <i class="fa-solid fa-filter"></i>
          </button>

          <!-- View Mode Toggle -->
          <div class="view-toggle">
            <button
              class="toggle-btn"
              class:active={viewMode === 'list'}
              onclick={() => viewMode = 'list'}
              title={$_('network.listView') || 'Liste'}
            >
              <i class="fa-solid fa-list"></i>
            </button>
            <button
              class="toggle-btn"
              class:active={viewMode === 'table'}
              onclick={() => viewMode = 'table'}
              title={$_('network.tableView') || 'Tabelle'}
            >
              <i class="fa-solid fa-table"></i>
            </button>
          </div>
        </div>
      </div>

      <!-- Filter Bar -->
      {#if showFilters}
        <div class="filter-bar">
          <div class="filter-item">
            <label class="filter-label">{$_('network.minArticles') || 'Min. Artikel'}:</label>
            <input
              type="number"
              class="filter-input"
              bind:value={minArticleFilter}
              min="0"
              placeholder="0"
            />
          </div>
          <div class="filter-stats">
            {filteredKeywords.length} / {keywords.length} {$_('network.keywords') || 'Keywords'}
          </div>
          {#if minArticleFilter > 0}
            <button class="filter-clear" onclick={() => minArticleFilter = 0}>
              <i class="fa-solid fa-xmark"></i>
              {$_('network.clearFilter') || 'Filter löschen'}
            </button>
          {/if}
        </div>
      {/if}

      <!-- TABLE VIEW -->
      {#if viewMode === 'table'}
        <div class="keywords-table-wrapper">
          <table class="keywords-table">
            <thead>
              <tr>
                <th class="sortable" onclick={() => handleSort('name')}>
                  {$_('network.name') || 'Name'}
                  <i class="fa-solid {getSortIcon('name')} sort-icon"></i>
                </th>
                <th class="sortable num" onclick={() => handleSort('article_count')}>
                  {$_('network.articles') || 'Artikel'}
                  <i class="fa-solid {getSortIcon('article_count')} sort-icon"></i>
                </th>
                <th class="sortable" onclick={() => handleSort('first_seen')}>
                  {$_('network.firstSeen') || 'Erstellt'}
                  <i class="fa-solid {getSortIcon('first_seen')} sort-icon"></i>
                </th>
                <th class="sortable" onclick={() => handleSort('last_used')}>
                  {$_('network.lastUsed') || 'Zuletzt'}
                  <i class="fa-solid {getSortIcon('last_used')} sort-icon"></i>
                </th>
              </tr>
            </thead>
            <tbody>
              {#each sortedKeywords as keyword (keyword.id)}
                <tr
                  class="keyword-row"
                  class:active={selectedKeyword?.id === keyword.id}
                  onclick={() => selectKeywordById(keyword.id)}
                >
                  <td class="name-cell">
                    {keyword.name}
                    {#if keyword.is_canonical}
                      <i class="fa-solid fa-crown canonical-icon" title={$_('network.canonical') || 'Kanonisch'}></i>
                    {/if}
                  </td>
                  <td class="num">{keyword.article_count}</td>
                  <td class="date-cell">{keyword.first_seen ? formatArticleDate(keyword.first_seen) : '-'}</td>
                  <td class="date-cell">{keyword.last_used ? formatArticleDate(keyword.last_used) : '-'}</td>
                </tr>
              {/each}
            </tbody>
          </table>

          {#if hasMore && !loading && !searchInput}
            <button onclick={() => loadKeywords(false)} class="load-more table-load-more">
              {$_('network.loadMore')}
            </button>
          {/if}
        </div>

      <!-- LIST VIEW -->
      {:else}
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
            {#if trendingKeywords.length > 0 && minArticleFilter === 0}
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

            <!-- All Keywords (filtered) -->
            <div class="list-section">
              <div class="section-label">{$_('network.allKeywords')}</div>
              {#each filteredKeywords as keyword (keyword.id)}
                <button
                  class="keyword-item {selectedKeyword?.id === keyword.id ? 'active' : ''}"
                  onclick={() => selectKeywordById(keyword.id)}
                >
                  <span class="keyword-name">{keyword.name}</span>
                  <span class="keyword-count">{keyword.article_count}</span>
                </button>
              {/each}

              {#if hasMore && !loading && minArticleFilter === 0}
                <button onclick={() => loadKeywords(false)} class="load-more">
                  {$_('network.loadMore')}
                </button>
              {/if}
            </div>
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

    <!-- Right Panel: Keyword Details -->
    <div class="detail-panel">
      {#if selectedKeyword}
        <div class="keyword-detail">
          <div class="detail-title-row">
            {#if isRenaming}
              <div class="rename-form">
                <input
                  type="text"
                  class="rename-input"
                  bind:value={renameInput}
                  onkeydown={handleRenameKeydown}
                  disabled={renameLoading}
                  autofocus
                />
                <button
                  class="rename-btn save"
                  onclick={handleRename}
                  disabled={renameLoading || !renameInput.trim()}
                  title={$_('common.save') || 'Speichern'}
                >
                  {#if renameLoading}
                    <i class="fa-solid fa-spinner fa-spin"></i>
                  {:else}
                    <i class="fa-solid fa-check"></i>
                  {/if}
                </button>
                <button
                  class="rename-btn cancel"
                  onclick={cancelRename}
                  disabled={renameLoading}
                  title={$_('common.cancel') || 'Abbrechen'}
                >
                  <i class="fa-solid fa-times"></i>
                </button>
              </div>
              {#if renameError}
                <div class="rename-error">{renameError}</div>
              {/if}
            {:else}
              <h3 class="detail-title">{selectedKeyword.name}</h3>
              <button
                class="edit-btn"
                onclick={startRename}
                title={$_('network.renameKeyword') || 'Umbenennen'}
              >
                <i class="fa-solid fa-pen"></i>
              </button>
            {/if}
          </div>

          <div class="detail-meta">
            <span class="meta-item">
              <span class="meta-label">{$_('network.articleCount')}:</span>
              <span class="meta-value">{selectedKeyword.article_count}</span>
            </span>
            <span class="meta-item">
              <span class="meta-label">{$_('network.firstSeen')}:</span>
              <span class="meta-value">{formatDate(selectedKeyword.first_seen)}</span>
            </span>
            <span class="meta-item">
              <span class="meta-label">{$_('network.lastUsed')}:</span>
              <span class="meta-value">{formatDate(selectedKeyword.last_used)}</span>
            </span>
          </div>

          <!-- Categories grouped by main category -->
          {#if keywordCategories.length > 0}
            {@const groupedCategories = (() => {
              // Separate main categories (parent_id is null) from subcategories
              const mainCats = keywordCategories.filter(c => c.parent_id === null);
              const subCats = keywordCategories.filter(c => c.parent_id !== null);

              // Group subcategories by their parent
              const subsByParent = subCats.reduce((acc, cat) => {
                const key = cat.parent_name || 'Sonstige';
                if (!acc[key]) acc[key] = [];
                acc[key].push(cat);
                return acc;
              }, {} as Record<string, typeof keywordCategories>);

              // Build result: main categories with their subcategories
              const result: Array<{
                id: number;
                name: string;
                icon: string | null;
                color: string | null;
                weight: number;
                subcategories: typeof keywordCategories;
              }> = [];

              // Add main categories that are directly assigned
              for (const main of mainCats) {
                const subs = subsByParent[main.name] || [];
                delete subsByParent[main.name];
                result.push({
                  id: main.sephiroth_id,
                  name: main.name,
                  icon: main.icon,
                  color: main.color,
                  weight: main.weight + subs.reduce((sum, s) => sum + s.weight, 0),
                  subcategories: subs
                });
              }

              // Add subcategories whose main category is not directly assigned
              for (const [parentName, subs] of Object.entries(subsByParent)) {
                const firstSub = subs[0];
                result.push({
                  id: firstSub.parent_id || 0,
                  name: parentName,
                  icon: firstSub.parent_icon, // Use parent's icon from first sub
                  color: firstSub.color, // Use first sub's color (inherited from parent)
                  weight: subs.reduce((sum, s) => sum + s.weight, 0),
                  subcategories: subs
                });
              }

              return result.sort((a, b) => b.weight - a.weight);
            })()}
            {@const maxWeight = Math.max(...groupedCategories.map(c => c.weight), 0.01)}
            <div class="detail-section">
              <h4 class="section-title">
                <Tooltip termKey="sephiroth">{$_('network.categories')}</Tooltip>
                <span class="help-icon" title={$_('network.categoriesHelp')}>?</span>
              </h4>
              <div class="category-cards">
                {#each groupedCategories as group (group.id)}
                  {@const barWidth = (group.weight / maxWeight) * 100}
                  <div class="category-card" style="--cat-color: {group.color || '#6366F1'}">
                    <div class="card-header">
                      <div class="card-icon-wrapper">
                        <i class="{group.icon || 'fa-solid fa-folder'}"></i>
                      </div>
                      <span class="card-title">{group.name}</span>
                    </div>
                    <div class="card-stats">
                      <div class="stat-row">
                        <span class="stat-label">{$_('network.weight') || 'Gewicht'}</span>
                        <span class="stat-value">{(group.weight * 100).toFixed(0)}%</span>
                      </div>
                      <div class="progress-bar">
                        <div class="progress-fill" style="width: {barWidth}%"></div>
                      </div>
                    </div>
                    {#if group.subcategories.length > 0}
                      <div class="subcategories">
                        {#each group.subcategories as cat (cat.sephiroth_id)}
                          <div class="subcategory-item">
                            <div class="subcategory-info">
                              <i class="{cat.icon || 'fa-solid fa-folder'} subcategory-icon"></i>
                              <span class="subcategory-name">{cat.name}</span>
                            </div>
                            <span class="subcategory-weight {getWeightClass(cat.weight)}">{(cat.weight * 100).toFixed(0)}%</span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Similar Keywords (Embedding-based) -->
          <div class="detail-section">
            <h4 class="section-title">
              <i class="fa-solid fa-diagram-project section-icon"></i>
              {$_('network.similarKeywords') || 'Ähnliche Keywords'}
            </h4>
            {#if similarKeywordsLoading}
              <div class="loading-similar">
                <i class="fa-solid fa-spinner fa-spin"></i>
                {$_('network.loading') || 'Laden...'}
              </div>
            {:else if similarKeywords.length > 0}
              <div class="similar-keywords-grid">
                {#each similarKeywords as simKw (simKw.id)}
                  <button
                    class="similar-keyword-item"
                    onclick={() => selectKeywordById(simKw.id)}
                    title={`${$_('network.similarity') || 'Ähnlichkeit'}: ${Math.round(simKw.similarity * 100)}% | ${$_('network.cooccurrence') || 'Kookkurrenz'}: ${simKw.cooccurrence}`}
                  >
                    <span class="similar-name">{simKw.name}</span>
                    <div class="similar-scores">
                      {#if simKw.similarity > 0}
                        <span class="similar-score semantic" title={$_('network.embeddingSimilarity') || 'Embedding-Ähnlichkeit'}>
                          <i class="fa-solid fa-brain"></i>
                          {Math.round(simKw.similarity * 100)}%
                        </span>
                      {/if}
                      {#if simKw.cooccurrence > 0}
                        <span class="similar-score cooccur" title={$_('network.cooccurrence') || 'Kookkurrenz'}>
                          <i class="fa-solid fa-link"></i>
                          {simKw.cooccurrence}
                        </span>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            {:else}
              <div class="no-similar">{$_('network.noSimilarKeywords') || 'Keine ähnlichen Keywords gefunden'}</div>
            {/if}
          </div>

          <!-- Trend Chart with Co-occurring Keywords -->
          <div class="detail-section">
            <h4 class="section-title">{$_('network.trendComparison')}</h4>
            <KeywordTrendChart
              keywordId={selectedKeyword.id}
              keywordName={selectedKeyword.name}
              neighborIds={cooccurringKeywords.slice(0, 7).map(k => k.id)}
              ondayschange={handleDaysChange}
            />
            {#if cooccurringKeywords.length > 0}
              <div class="neighbor-legend">
                <span class="legend-label">{$_('network.comparedWith')}:</span>
                {#each cooccurringKeywords.slice(0, 7) as coKw, idx (coKw.id)}
                  <button
                    class="neighbor-tag"
                    style="--neighbor-color: {['#f9e2af', '#a6e3a1', '#89b4fa', '#f5c2e7', '#94e2d5', '#fab387', '#89dceb'][idx]}"
                    onclick={() => selectKeywordById(coKw.id)}
                    title="{coKw.cooccurrence_count} {$_('network.articleCount')}"
                  >
                    {coKw.name}
                  </button>
                {/each}
                {#if cooccurringKeywords.length > 7}
                  <span
                    class="more-count"
                    title={cooccurringKeywords.slice(7).map(k => k.name).join(', ')}
                  >
                    +{cooccurringKeywords.length - 7}
                  </span>
                {/if}
              </div>
            {/if}
          </div>

          <!-- Linked Articles -->
          <div class="detail-section">
            <h4 class="section-title">{$_('network.linkedArticles') || 'Verlinkte Artikel'}</h4>
            {#if keywordArticles.length > 0}
              <div class="articles-list">
                {#each keywordArticles as article (article.id)}
                  <button class="article-item" onclick={() => openArticle(article.id)}>
                    <i class="article-status {getStatusIconClass(article.status)}" title={article.status}></i>
                    <div class="article-info">
                      <span class="article-title">{article.title}</span>
                      <span class="article-meta">
                        {#if article.pentacle_title}
                          <span class="article-source">{article.pentacle_title}</span>
                        {/if}
                        {#if article.published_at}
                          <span class="article-date">{formatArticleDate(article.published_at)}</span>
                        {/if}
                      </span>
                    </div>
                  </button>
                {/each}
                {#if hasMoreArticles}
                  <button
                    class="load-more-articles"
                    onclick={loadMoreArticles}
                    disabled={articlesLoading}
                  >
                    {#if articlesLoading}
                      {$_('network.loading') || 'Laden...'}
                    {:else}
                      {$_('network.loadMore') || 'Mehr laden'}
                    {/if}
                  </button>
                {/if}
              </div>
            {:else if !loading}
              <div class="no-articles">{$_('network.noArticles') || 'Keine Artikel gefunden'}</div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="no-selection">
          <i class="no-selection-icon fa-solid fa-link"></i>
          <p>{$_('network.selectKeyword')}</p>
        </div>
      {/if}
    </div>
  </div>
  {:else if activeTab === 'graph'}
  <!-- Graph View -->
  <div class="graph-view">
    <NetworkGraph
      graphData={graphData || emptyGraphData}
      onNodeClick={handleGraphNodeClick}
      loading={graphLoading}
    />
  </div>
  {:else if activeTab === 'synonyms'}
  <!-- Synonyms View -->
  <div class="synonyms-view">
    <!-- Manual Merge - Full Width at Top -->
    <div class="synonyms-section full-width">
      <h3 class="section-heading">{$_('network.manualMerge') || 'Manuelles Zusammenführen'}</h3>
      <p class="section-description">{$_('network.manualMergeDescription') || 'Wähle zwei Keywords aus: Das erste bleibt erhalten, das zweite wird gelöscht und alle Verknüpfungen werden übertragen.'}</p>

      <div class="merge-form">
        <!-- Keep Keyword (Target) -->
        <div class="merge-field">
          <label class="merge-label">
            <i class="fa-solid fa-check merge-label-icon keep"></i>
            {$_('network.keepKeyword') || 'Behalten'}
          </label>
          <div class="merge-search-box">
            <input
              type="text"
              bind:value={keepSearchInput}
              oninput={handleKeepSearch}
              placeholder={$_('network.searchKeywordPlaceholder') || 'Keyword suchen...'}
              class="merge-search-input"
            />
            {#if keepSearchInput}
              <button onclick={clearKeepSearch} class="clear-btn" aria-label="Clear"><i class="fa-solid fa-xmark"></i></button>
            {/if}
            {#if keepSearchResults.length > 0 && !selectedKeepKeyword}
              <div class="merge-search-results">
                {#each keepSearchResults as keyword (keyword.id)}
                  <button
                    class="merge-search-item"
                    onclick={() => selectKeepKeyword(keyword)}
                  >
                    <span class="item-name">{keyword.name}</span>
                    <span class="item-count">{keyword.article_count}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          {#if selectedKeepKeyword}
            <div class="selected-chip keep">
              <i class="fa-solid fa-check"></i>
              <span>{selectedKeepKeyword.name}</span>
              <span class="chip-count">({selectedKeepKeyword.article_count} Artikel)</span>
            </div>
          {/if}
        </div>

        <!-- Visual Arrow -->
        <div class="merge-arrow">
          <i class="fa-solid fa-arrow-right"></i>
          <span class="arrow-label">{$_('network.replacesLabel') || 'ersetzt'}</span>
        </div>

        <!-- Remove Keyword (Source) -->
        <div class="merge-field">
          <label class="merge-label">
            <i class="fa-solid fa-trash merge-label-icon remove"></i>
            {$_('network.removeKeyword') || 'Löschen'}
          </label>
          <div class="merge-search-box">
            <input
              type="text"
              bind:value={removeSearchInput}
              oninput={handleRemoveSearch}
              placeholder={$_('network.searchKeywordPlaceholder') || 'Keyword suchen...'}
              class="merge-search-input"
            />
            {#if removeSearchInput}
              <button onclick={clearRemoveSearch} class="clear-btn" aria-label="Clear"><i class="fa-solid fa-xmark"></i></button>
            {/if}
            {#if removeSearchResults.length > 0 && !selectedRemoveKeyword}
              <div class="merge-search-results">
                {#each removeSearchResults as keyword (keyword.id)}
                  {#if keyword.id !== selectedKeepKeyword?.id}
                    <button
                      class="merge-search-item"
                      onclick={() => selectRemoveKeyword(keyword)}
                    >
                      <span class="item-name">{keyword.name}</span>
                      <span class="item-count">{keyword.article_count}</span>
                    </button>
                  {/if}
                {/each}
              </div>
            {/if}
          </div>
          {#if selectedRemoveKeyword}
            <div class="selected-chip remove">
              <i class="fa-solid fa-trash"></i>
              <span>{selectedRemoveKeyword.name}</span>
              <span class="chip-count">({selectedRemoveKeyword.article_count} Artikel)</span>
            </div>
          {/if}
        </div>
      </div>

      <!-- Merge Preview & Action -->
      {#if selectedKeepKeyword && selectedRemoveKeyword}
        <div class="merge-preview">
          <div class="preview-text">
            <i class="fa-solid fa-info-circle"></i>
            <span>
              <strong>"{selectedRemoveKeyword.name}"</strong> wird gelöscht.
              Alle {selectedRemoveKeyword.article_count} Artikel werden zu <strong>"{selectedKeepKeyword.name}"</strong> übertragen.
            </span>
          </div>
          <button
            class="action-btn danger"
            onclick={executeManualMerge}
            disabled={synonymsLoading || selectedKeepKeyword.id === selectedRemoveKeyword.id}
          >
            {#if synonymsLoading}
              <i class="fa-solid fa-rotate fa-spin"></i>
            {:else}
              <i class="fa-solid fa-code-merge"></i>
            {/if}
            {$_('network.executeMerge') || 'Zusammenführen'}
          </button>
        </div>
      {:else}
        <div class="merge-hint">
          <i class="fa-solid fa-hand-pointer"></i>
          {$_('network.selectBothKeywords') || 'Wähle beide Keywords aus, um sie zusammenzuführen.'}
        </div>
      {/if}

      {#if selectedKeepKeyword && selectedRemoveKeyword && selectedKeepKeyword.id === selectedRemoveKeyword.id}
        <div class="merge-error">
          <i class="fa-solid fa-triangle-exclamation"></i>
          {$_('network.sameKeywordError') || 'Die beiden Keywords müssen unterschiedlich sein.'}
        </div>
      {/if}
    </div>

    <!-- Two columns below -->
    <div class="synonyms-columns">
      <!-- AI Synonym Suggestions -->
      <div class="synonyms-section">
        <h3 class="section-heading">{$_('network.synonymCandidates') || 'KI-Synonym-Vorschläge'}</h3>
        <button
          class="action-btn primary"
          onclick={findSynonymCandidates}
          disabled={synonymsLoading}
        >
          {#if synonymsLoading}
            {$_('network.loading') || 'Lade...'}
          {:else}
            {$_('network.findSynonyms') || 'Synonyme finden'}
          {/if}
        </button>

        {#if synonymsError}
          <div class="feedback-message error">{synonymsError}</div>
        {/if}
        {#if synonymSuccess}
          <div class="feedback-message success">{synonymSuccess}</div>
        {/if}

        {#if synonymCandidates.length > 0}
          <div class="synonym-list">
            {#each synonymCandidates as candidate (candidate.keyword_a_id + '-' + candidate.keyword_b_id)}
              <div class="synonym-item">
                <div class="synonym-pair">
                  <span class="synonym-keyword">{candidate.keyword_a_name}</span>
                  <span class="synonym-similarity">{(candidate.similarity * 100).toFixed(0)}%</span>
                  <span class="synonym-keyword">{candidate.keyword_b_name}</span>
                </div>
                <div class="synonym-actions">
                  <button
                    class="merge-btn left"
                    onclick={() => mergeKeywords(candidate.keyword_a_id, candidate.keyword_b_id, candidate.keyword_a_name, candidate.keyword_b_name)}
                    title="{candidate.keyword_b_name} -> {candidate.keyword_a_name}"
                    disabled={synonymsLoading}
                  >
                    &#8592;
                  </button>
                  <button
                    class="merge-btn right"
                    onclick={() => mergeKeywords(candidate.keyword_b_id, candidate.keyword_a_id, candidate.keyword_b_name, candidate.keyword_a_name)}
                    title="{candidate.keyword_a_name} -> {candidate.keyword_b_name}"
                    disabled={synonymsLoading}
                  >
                    &#8594;
                  </button>
                  <button
                    class="dismiss-btn"
                    onclick={() => dismissSynonymPair(candidate.keyword_a_id, candidate.keyword_b_id)}
                    title={$_('network.dismissSynonym') || 'Ignorieren'}
                  >
                    &#10005;
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {:else if !synonymsLoading && synonymCandidates.length === 0}
          <div class="empty-hint">{$_('network.clickFindSynonyms') || 'Klicke auf "Synonyme finden" um KI-Vorschläge zu laden'}</div>
        {/if}
      </div>

      <!-- Create New Keyword -->
      <div class="synonyms-section">
        <h3 class="section-heading">{$_('network.createKeyword') || 'Neues Keyword erstellen'}</h3>
        <div class="create-keyword-form">
          <input
            type="text"
            bind:value={newKeywordInput}
            placeholder={$_('network.newKeywordPlaceholder') || 'Keyword eingeben...'}
            class="create-keyword-input"
            onkeydown={(e) => e.key === 'Enter' && createNewKeyword()}
          />
          <button
            class="action-btn primary"
            onclick={createNewKeyword}
            disabled={createKeywordLoading || !newKeywordInput.trim()}
          >
            {#if createKeywordLoading}
              {$_('network.loading') || 'Lade...'}
            {:else}
              {$_('network.create') || 'Erstellen'}
            {/if}
          </button>
        </div>
        {#if createKeywordError}
          <div class="feedback-message error">{createKeywordError}</div>
        {/if}
        {#if createKeywordSuccess}
          <div class="feedback-message success">{createKeywordSuccess}</div>
        {/if}
      </div>
    </div>
  </div>
  {/if}
</div>

<style>
  .keyword-network {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1;
    background-color: var(--bg-default);
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

  .network-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .network-stats {
    display: flex;
    gap: 1.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .network-stats .stat strong {
    color: var(--text-secondary);
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

  .graph-view :global(.network-graph-container) {
    flex: 1;
    width: 100%;
    min-height: 0;
  }

  /* Trends View */
  .trends-view {
    flex: 1;
    padding: 1rem;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .trends-hint {
    text-align: center;
    color: var(--text-muted);
    padding: 2rem;
    font-size: 0.875rem;
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
    min-width: 280px;
    transition: width 0.2s;
  }

  .keywords-panel.table-mode {
    width: 50%;
    max-width: 600px;
  }

  /* Panel Toolbar */
  .panel-toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border-bottom: 1px solid var(--border-default);
  }

  .panel-toolbar .search-box {
    flex: 1;
    padding: 0;
    border-bottom: none;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .toolbar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    background: none;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .toolbar-btn:hover {
    color: var(--text-primary);
    border-color: var(--accent-primary);
  }

  .toolbar-btn.active {
    color: var(--accent-primary);
    border-color: var(--accent-primary);
    background-color: rgba(var(--accent-primary-rgb), 0.1);
  }

  .view-toggle {
    display: flex;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    overflow: hidden;
  }

  .toggle-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-btn:hover {
    color: var(--text-primary);
  }

  .toggle-btn.active {
    color: var(--accent-primary);
    background-color: rgba(var(--accent-primary-rgb), 0.1);
  }

  /* Filter Bar */
  .filter-bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border-bottom: 1px solid var(--border-default);
    flex-wrap: wrap;
  }

  .filter-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .filter-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .filter-input {
    width: 60px;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.75rem;
  }

  .filter-stats {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-left: auto;
  }

  .filter-clear {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: none;
    border: 1px solid var(--status-error);
    border-radius: 0.25rem;
    color: var(--status-error);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .filter-clear:hover {
    background-color: var(--status-error);
    color: white;
  }

  /* Table View */
  .keywords-table-wrapper {
    flex: 1;
    overflow: auto;
  }

  .keywords-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8125rem;
  }

  .keywords-table thead {
    position: sticky;
    top: 0;
    background-color: var(--bg-surface);
    z-index: 1;
  }

  .keywords-table th {
    padding: 0.5rem 0.75rem;
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    border-bottom: 2px solid var(--border-default);
    white-space: nowrap;
  }

  .keywords-table th.sortable {
    cursor: pointer;
    user-select: none;
  }

  .keywords-table th.sortable:hover {
    color: var(--text-primary);
  }

  .keywords-table th.num,
  .keywords-table td.num {
    text-align: right;
  }

  .sort-icon {
    margin-left: 0.25rem;
    font-size: 0.625rem;
    opacity: 0.5;
  }

  .keywords-table th.sortable:hover .sort-icon,
  .keywords-table th.sortable .sort-icon.fa-sort-up,
  .keywords-table th.sortable .sort-icon.fa-sort-down {
    opacity: 1;
  }

  .keywords-table td {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border-default);
  }

  .keyword-row {
    cursor: pointer;
    transition: background-color 0.15s;
  }

  .keyword-row:hover {
    background-color: var(--bg-overlay);
  }

  .keyword-row.active {
    background-color: rgba(var(--accent-primary-rgb), 0.1);
  }

  .name-cell {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .canonical-icon {
    font-size: 0.625rem;
    color: var(--accent-warning);
    margin-left: 0.25rem;
  }

  .date-cell {
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .table-load-more {
    width: 100%;
    padding: 0.75rem;
    margin-top: 0;
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

  /* Detail Panel */
  .detail-panel {
    flex: 1;
    overflow-y: auto;
    background-color: var(--bg-default);
  }

  .keyword-detail {
    padding: 1.5rem;
  }

  .detail-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .detail-title {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .edit-btn {
    padding: 0.375rem 0.5rem;
    background: none;
    border: 1px solid transparent;
    border-radius: 0.25rem;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .edit-btn:hover {
    color: var(--accent-primary);
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .rename-form {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
  }

  .rename-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    font-size: 1.25rem;
    font-weight: 600;
    border: 2px solid var(--accent-primary);
    border-radius: 0.375rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    outline: none;
  }

  .rename-input:focus {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-primary) 25%, transparent);
  }

  .rename-btn {
    padding: 0.5rem 0.625rem;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.875rem;
  }

  .rename-btn.save {
    background-color: var(--accent-success);
    color: white;
  }

  .rename-btn.save:hover:not(:disabled) {
    background-color: color-mix(in srgb, var(--accent-success) 80%, black);
  }

  .rename-btn.cancel {
    background-color: var(--bg-overlay);
    color: var(--text-muted);
  }

  .rename-btn.cancel:hover:not(:disabled) {
    background-color: var(--bg-muted);
    color: var(--text-primary);
  }

  .rename-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .rename-error {
    margin-top: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--accent-error) 15%, transparent);
    border-radius: 0.375rem;
    color: var(--accent-error);
    font-size: 0.8125rem;
  }

  .detail-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 1.5rem;
    padding: 0.75rem;
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
  }

  .meta-item {
    font-size: 0.875rem;
  }

  .meta-label {
    color: var(--text-muted);
  }

  .meta-value {
    color: var(--text-primary);
    font-weight: 500;
  }

  .detail-section {
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .help-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1rem;
    height: 1rem;
    font-size: 0.625rem;
    font-weight: 700;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    border-radius: 50%;
    cursor: help;
    text-transform: none;
  }

  .help-icon:hover {
    color: var(--text-primary);
    border-color: var(--accent-primary);
  }

  /* Category Cards (matching FnordView) */
  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 0.75rem;
  }

  .category-card {
    background: linear-gradient(135deg, color-mix(in srgb, var(--cat-color) 15%, var(--bg-default)) 0%, var(--bg-default) 100%);
    border: 1px solid color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-radius: 0.5rem;
    padding: 0.75rem;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.625rem;
  }

  .card-icon-wrapper {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 70%, black));
    border-radius: 0.375rem;
    color: white;
    font-size: 0.8125rem;
    box-shadow: 0 2px 6px color-mix(in srgb, var(--cat-color) 40%, transparent);
  }

  .card-title {
    font-size: 0.8125rem;
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
    gap: 0.25rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .stat-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .stat-value {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .progress-bar {
    height: 4px;
    background-color: color-mix(in srgb, var(--cat-color) 20%, transparent);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 80%, white));
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  /* Subcategories in cards */
  .subcategories {
    margin-top: 0.625rem;
    padding-top: 0.625rem;
    border-top: 1px solid color-mix(in srgb, var(--cat-color) 20%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.25rem 0.5rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.25rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    min-width: 0;
    flex: 1;
  }

  .subcategory-icon {
    font-size: 0.625rem;
    color: var(--cat-color);
    flex-shrink: 0;
  }

  .subcategory-name {
    font-size: 0.6875rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .subcategory-weight {
    font-size: 0.5625rem;
    padding: 0.125rem 0.25rem;
    border-radius: 0.1875rem;
    font-weight: 600;
    flex-shrink: 0;
  }

  .subcategory-weight.weight-high {
    background-color: rgba(34, 197, 94, 0.2);
    color: var(--accent-success);
  }

  .subcategory-weight.weight-medium {
    background-color: rgba(251, 191, 36, 0.2);
    color: var(--accent-warning);
  }

  .subcategory-weight.weight-low {
    background-color: var(--bg-overlay);
    color: var(--text-muted);
  }

  /* No Selection State */
  .no-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    padding: 2rem;
    text-align: center;
  }

  .no-selection-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .no-selection p {
    font-size: 0.875rem;
    margin: 0;
  }

  /* Neighbor Legend */
  .neighbor-legend {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding: 0.5rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }

  .legend-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .neighbor-tag {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--neighbor-color);
    border-radius: 0.25rem;
    color: var(--neighbor-color);
    cursor: pointer;
    transition: all 0.2s;
  }

  .neighbor-tag:hover {
    background-color: var(--neighbor-color);
    color: var(--bg-default);
  }

  .more-count {
    font-size: 0.75rem;
    color: var(--text-muted);
    padding: 0.25rem 0.5rem;
    cursor: help;
  }

  .more-count:hover {
    color: var(--text-primary);
  }

  /* Articles List */
  .articles-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .article-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-surface);
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    text-align: left;
    transition: background-color 0.15s;
    width: 100%;
  }

  .article-item:hover {
    background-color: var(--bg-overlay);
  }

  .article-status {
    font-size: 0.875rem;
    flex-shrink: 0;
    width: 1.25rem;
  }

  .article-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    min-width: 0;
    flex: 1;
  }

  .article-title {
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .article-meta {
    display: flex;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .article-source {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 150px;
  }

  .article-date {
    flex-shrink: 0;
  }

  .load-more-articles {
    padding: 0.5rem;
    background: none;
    border: 1px dashed var(--border-default);
    border-radius: 0.375rem;
    color: var(--accent-primary);
    cursor: pointer;
    font-size: 0.75rem;
    transition: all 0.2s;
  }

  .load-more-articles:hover:not(:disabled) {
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .load-more-articles:disabled {
    color: var(--text-muted);
    cursor: not-allowed;
  }

  .no-articles {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }

  /* Synonyms View */
  .synonyms-view {
    flex: 1;
    padding: 1rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .synonyms-columns {
    display: flex;
    gap: 1rem;
  }

  .synonyms-columns .synonyms-section {
    flex: 1;
  }

  .synonyms-section {
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--border-default);
  }

  .synonyms-section.full-width {
    width: 100%;
  }

  .section-heading {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .action-btn {
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid var(--border-default);
    background-color: var(--bg-overlay);
    color: var(--text-primary);
  }

  .action-btn:hover:not(:disabled) {
    background-color: var(--bg-tertiary);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-btn.primary {
    background-color: var(--accent-primary);
    border-color: var(--accent-primary);
    color: var(--bg-default);
  }

  .action-btn.primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .feedback-message {
    margin-top: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.75rem;
  }

  .feedback-message.error {
    background-color: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--accent-error);
    color: var(--accent-error);
  }

  .feedback-message.success {
    background-color: rgba(34, 197, 94, 0.1);
    border: 1px solid var(--accent-success);
    color: var(--accent-success);
  }

  .synonym-list {
    margin-top: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 400px;
    overflow-y: auto;
  }

  .synonym-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    gap: 0.5rem;
  }

  .synonym-pair {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
  }

  .synonym-keyword {
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .synonym-similarity {
    font-size: 0.625rem;
    font-weight: 600;
    padding: 0.125rem 0.375rem;
    background-color: var(--bg-surface);
    border-radius: 0.25rem;
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  .synonym-actions {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .merge-btn {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0.25rem;
    border: 1px solid var(--border-default);
    background-color: var(--bg-surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.875rem;
  }

  .merge-btn:hover:not(:disabled) {
    background-color: var(--accent-success);
    border-color: var(--accent-success);
    color: var(--bg-default);
  }

  .merge-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dismiss-btn {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0.25rem;
    border: 1px solid var(--border-default);
    background-color: var(--bg-surface);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.75rem;
  }

  .dismiss-btn:hover {
    background-color: var(--accent-error);
    border-color: var(--accent-error);
    color: var(--bg-default);
  }

  .empty-hint {
    margin-top: 0.75rem;
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
  }

  .create-keyword-form {
    display: flex;
    gap: 0.5rem;
  }

  .create-keyword-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .create-keyword-input::placeholder {
    color: var(--text-faint);
  }

  .create-keyword-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .selected-keyword-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    margin-bottom: 0.75rem;
  }

  .selected-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .selected-name {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--accent-primary);
  }

  .selected-count {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .manual-search-box {
    position: relative;
    margin-bottom: 0.75rem;
  }

  .manual-search-input {
    width: 100%;
    padding: 0.5rem 2rem 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .manual-search-input::placeholder {
    color: var(--text-faint);
  }

  .manual-search-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .manual-search-results {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    max-height: 300px;
    overflow-y: auto;
  }

  .manual-search-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
  }

  .manual-keyword-name {
    flex: 1;
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .manual-keyword-count {
    font-size: 0.75rem;
    color: var(--text-muted);
    background-color: var(--bg-surface);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    flex-shrink: 0;
  }

  .replace-btn {
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    border: 1px solid var(--accent-warning);
    background-color: transparent;
    color: var(--accent-warning);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
    flex-shrink: 0;
  }

  .replace-btn:hover:not(:disabled) {
    background-color: var(--accent-warning);
    color: var(--bg-default);
  }

  .replace-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .no-keyword-selected {
    padding: 2rem;
    text-align: center;
    color: var(--text-muted);
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
  }

  .no-keyword-selected p {
    margin: 0;
    font-size: 0.875rem;
    line-height: 1.5;
  }

  /* Manual Merge Form */
  .section-description {
    font-size: 0.8125rem;
    color: var(--text-muted);
    margin-bottom: 1rem;
    line-height: 1.5;
  }

  .merge-form {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .merge-field {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .merge-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .merge-label-icon {
    font-size: 0.875rem;
  }

  .merge-label-icon.keep {
    color: var(--accent-success);
  }

  .merge-label-icon.remove {
    color: var(--accent-error);
  }

  .merge-search-box {
    position: relative;
  }

  .merge-search-input {
    width: 100%;
    padding: 0.625rem 2rem 0.625rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .merge-search-input::placeholder {
    color: var(--text-faint);
  }

  .merge-search-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .merge-search-results {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 10;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-top: none;
    border-radius: 0 0 0.375rem 0.375rem;
    max-height: 200px;
    overflow-y: auto;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .merge-search-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    transition: background-color 0.15s;
  }

  .merge-search-item:hover {
    background-color: var(--bg-overlay);
  }

  .merge-search-item .item-name {
    font-size: 0.875rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .merge-search-item .item-count {
    font-size: 0.75rem;
    color: var(--text-muted);
    background-color: var(--bg-overlay);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    flex-shrink: 0;
  }

  .selected-chip {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .selected-chip.keep {
    background-color: rgba(34, 197, 94, 0.15);
    border: 1px solid var(--accent-success);
    color: var(--accent-success);
  }

  .selected-chip.remove {
    background-color: rgba(239, 68, 68, 0.15);
    border: 1px solid var(--accent-error);
    color: var(--accent-error);
  }

  .selected-chip i {
    font-size: 0.75rem;
  }

  .chip-count {
    font-size: 0.75rem;
    opacity: 0.8;
    font-weight: 400;
  }

  .merge-arrow {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    padding-top: 1.5rem;
    color: var(--text-muted);
  }

  .merge-arrow i {
    font-size: 1.25rem;
  }

  .arrow-label {
    font-size: 0.625rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .merge-preview {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border-left: 3px solid var(--accent-warning);
  }

  .preview-text {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .preview-text i {
    color: var(--accent-warning);
    margin-top: 0.125rem;
    flex-shrink: 0;
  }

  .preview-text strong {
    color: var(--text-primary);
  }

  .merge-hint {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--text-muted);
    font-size: 0.8125rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px dashed var(--border-default);
  }

  .merge-error {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    margin-top: 0.5rem;
    background-color: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--accent-error);
    border-radius: 0.375rem;
    color: var(--accent-error);
    font-size: 0.8125rem;
  }

  .action-btn.danger {
    background-color: var(--accent-error);
    border-color: var(--accent-error);
    color: white;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .action-btn.danger:hover:not(:disabled) {
    opacity: 0.9;
  }

  /* Similar Keywords Section */
  .section-icon {
    font-size: 0.875rem;
    color: var(--accent-primary);
  }

  .similar-keywords-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 0.5rem;
  }

  .similar-keyword-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    cursor: pointer;
    text-align: left;
    transition: all 0.2s;
  }

  .similar-keyword-item:hover {
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .similar-name {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .similar-scores {
    display: flex;
    gap: 0.5rem;
  }

  .similar-score {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-weight: 500;
  }

  .similar-score i {
    font-size: 0.5rem;
  }

  .similar-score.semantic {
    background-color: rgba(139, 92, 246, 0.15);
    color: var(--accent-purple, #a78bfa);
  }

  .similar-score.cooccur {
    background-color: rgba(34, 197, 94, 0.15);
    color: var(--accent-success);
  }

  .loading-similar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .no-similar {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }
</style>
