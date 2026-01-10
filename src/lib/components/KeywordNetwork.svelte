<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Keyword, KeywordNeighbor, KeywordCategory, TrendingKeyword, NetworkStats, NetworkGraph as NetworkGraphType } from '../stores/state.svelte';
  import Tooltip from './Tooltip.svelte';
  import NetworkGraph from './NetworkGraph.svelte';
  import KeywordTrendChart from './KeywordTrendChart.svelte';

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

  type TabType = 'list' | 'graph';
  let activeTab = $state<TabType>('list');
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

  function handleTabChange(tab: TabType) {
    activeTab = tab;
    if (tab === 'graph') {
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
    <div class="network-tabs">
      <button
        class="tab-btn {activeTab === 'list' ? 'active' : ''}"
        onclick={() => handleTabChange('list')}
      >
        {$_('network.listTab') || 'Keywords'}
      </button>
      <button
        class="tab-btn {activeTab === 'graph' ? 'active' : ''}"
        onclick={() => handleTabChange('graph')}
      >
        {$_('network.graphTab') || 'Graph'}
      </button>
    </div>
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
                {#each cooccurringKeywords.slice(0, 7) as coKw, idx}
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

  /* Tabs */
  .network-tabs {
    display: flex;
    gap: 0.25rem;
  }

  .tab-btn {
    padding: 0.5rem 1rem;
    background: none;
    border: 1px solid transparent;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
    border-radius: 0.25rem 0.25rem 0 0;
  }

  .tab-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-overlay);
  }

  .tab-btn.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
    background-color: var(--bg-overlay);
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
</style>
