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
  let manualSearchInput = $state('');
  let manualSearchResults = $state<Keyword[]>([]);
  let manualSearchTimeout: ReturnType<typeof setTimeout> | null = null;
  let newKeywordInput = $state('');
  let createKeywordLoading = $state(false);
  let createKeywordSuccess = $state<string | null>(null);
  let createKeywordError = $state<string | null>(null);

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
    // Reset articles and co-occurring keywords when selecting new keyword
    keywordArticles = [];
    cooccurringKeywords = [];
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
    } catch (e) {
      error = String(e);
      console.error('Failed to load keyword details:', e);
    } finally {
      loading = false;
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

  function getStatusIcon(status: string): string {
    switch (status) {
      case 'concealed': return '👁';
      case 'illuminated': return '✓';
      case 'golden_apple': return '🍎';
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

  function handleManualSearch() {
    if (manualSearchTimeout) clearTimeout(manualSearchTimeout);

    if (!manualSearchInput.trim()) {
      manualSearchResults = [];
      return;
    }

    manualSearchTimeout = setTimeout(async () => {
      try {
        manualSearchResults = await invoke<Keyword[]>('search_keywords', {
          query: manualSearchInput,
          limit: 10,
        });
      } catch (e) {
        console.error('Failed to search keywords:', e);
      }
    }, 300);
  }

  function clearManualSearch() {
    if (manualSearchTimeout) clearTimeout(manualSearchTimeout);
    manualSearchInput = '';
    manualSearchResults = [];
  }

  async function manualMergeKeyword(targetKeyword: Keyword) {
    if (!selectedKeyword) return;

    // Selected keyword replaces the target (searched) keyword
    await mergeKeywords(
      selectedKeyword.id,
      targetKeyword.id,
      selectedKeyword.name,
      targetKeyword.name
    );
    clearManualSearch();
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
                    <span class="trend-icon">&#9650;</span>
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
    <div class="detail-panel">
      {#if selectedKeyword}
        <div class="keyword-detail">
          <h3 class="detail-title">{selectedKeyword.name}</h3>

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

          <!-- Categories -->
          {#if keywordCategories.length > 0}
            <div class="detail-section">
              <h4 class="section-title">
                <Tooltip termKey="sephiroth">{$_('network.categories')}</Tooltip>
                <span class="help-icon" title={$_('network.categoriesHelp')}>?</span>
              </h4>
              <div class="categories-list">
                {#each keywordCategories as cat (cat.sephiroth_id)}
                  <div class="category-item" style="--cat-color: {cat.color || '#6366F1'}">
                    <span class="category-icon">{cat.icon || '📁'}</span>
                    <span class="category-name">{cat.name}</span>
                    <span class="category-weight {getWeightClass(cat.weight)}">{(cat.weight * 100).toFixed(0)}%</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

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
                    <span class="article-status" title={article.status}>{getStatusIcon(article.status)}</span>
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
          <div class="no-selection-icon">&#128279;</div>
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
    <div class="synonyms-content">
      <!-- Left Panel: AI Suggestions & Create Keyword -->
      <div class="synonyms-left-panel">
        <!-- AI Synonym Suggestions -->
        <div class="synonyms-section">
          <h3 class="section-heading">{$_('network.synonymCandidates') || 'KI-Synonym-Vorschlaege'}</h3>
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
            <div class="empty-hint">{$_('network.clickFindSynonyms') || 'Klicke auf "Synonyme finden" um KI-Vorschlaege zu laden'}</div>
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

      <!-- Right Panel: Manual Keyword Linking -->
      <div class="synonyms-right-panel">
        <div class="synonyms-section">
          <h3 class="section-heading">{$_('network.manualMerge') || 'Manuelles Zusammenfuehren'}</h3>

          {#if selectedKeyword}
            <div class="selected-keyword-info">
              <span class="selected-label">{$_('network.selectedKeyword') || 'Ausgewaehlt'}:</span>
              <span class="selected-name">{selectedKeyword.name}</span>
              <span class="selected-count">({selectedKeyword.article_count} {$_('network.articleCount') || 'Artikel'})</span>
            </div>

            <div class="manual-search-box">
              <input
                type="text"
                bind:value={manualSearchInput}
                oninput={handleManualSearch}
                placeholder={$_('network.searchToReplace') || 'Keyword zum Ersetzen suchen...'}
                class="manual-search-input"
              />
              {#if manualSearchInput}
                <button onclick={clearManualSearch} class="clear-btn">&times;</button>
              {/if}
            </div>

            {#if manualSearchResults.length > 0}
              <div class="manual-search-results">
                {#each manualSearchResults as keyword (keyword.id)}
                  {#if keyword.id !== selectedKeyword.id}
                    <div class="manual-search-item">
                      <span class="manual-keyword-name">{keyword.name}</span>
                      <span class="manual-keyword-count">{keyword.article_count}</span>
                      <button
                        class="replace-btn"
                        onclick={() => manualMergeKeyword(keyword)}
                        disabled={synonymsLoading}
                        title="{keyword.name} wird durch {selectedKeyword.name} ersetzt"
                      >
                        {$_('network.replace') || 'Ersetzen'}
                      </button>
                    </div>
                  {/if}
                {/each}
              </div>
            {:else if manualSearchInput && !manualSearchResults.length}
              <div class="empty-hint">{$_('network.noResults') || 'Keine Ergebnisse gefunden'}</div>
            {/if}
          {:else}
            <div class="no-keyword-selected">
              <p>{$_('network.noKeywordSelected') || 'Waehle zuerst ein Keyword im "Keywords"-Tab aus, um es mit anderen Keywords zusammenzufuehren.'}</p>
            </div>
          {/if}
        </div>
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

  .detail-title {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 1rem 0;
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

  /* Categories */
  .categories-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .category-item {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.625rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border-left: 3px solid var(--cat-color);
  }

  .category-icon {
    font-size: 0.875rem;
  }

  .category-name {
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .category-weight {
    font-size: 0.625rem;
    padding: 0.125rem 0.25rem;
    border-radius: 0.25rem;
    font-weight: 600;
  }

  .category-weight.weight-high {
    background-color: rgba(34, 197, 94, 0.2);
    color: var(--accent-success);
  }

  .category-weight.weight-medium {
    background-color: rgba(251, 191, 36, 0.2);
    color: var(--accent-warning);
  }

  .category-weight.weight-low {
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
  }

  .synonyms-content {
    display: flex;
    gap: 1.5rem;
    height: 100%;
  }

  .synonyms-left-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .synonyms-right-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .synonyms-section {
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--border-default);
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
</style>
