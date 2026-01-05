<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { networkStore, type Keyword, type KeywordNeighbor, type KeywordCategory } from '../stores/state.svelte';
  import Tooltip from './Tooltip.svelte';

  let searchInput = $state('');
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    await Promise.all([
      networkStore.loadKeywords(true),
      networkStore.loadTrendingKeywords(),
      networkStore.loadNetworkStats(),
    ]);
  });

  function handleSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      networkStore.searchKeywords(searchInput);
    }, 300);
  }

  function clearSearch() {
    searchInput = '';
    networkStore.clearSearch();
  }

  async function selectKeyword(keyword: Keyword) {
    await networkStore.selectKeyword(keyword.id);
  }

  async function navigateToNeighbor(neighbor: KeywordNeighbor) {
    await networkStore.navigateToNeighbor(neighbor.id);
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
  <!-- Header with Stats -->
  <div class="network-header">
    <h2 class="network-title">
      <Tooltip termKey="immanentize">{$_('network.title')}</Tooltip>
    </h2>
    {#if networkStore.networkStats}
      <div class="network-stats">
        <span class="stat">{$_('network.keywords')}: <strong>{networkStore.networkStats.total_keywords}</strong></span>
        <span class="stat">{$_('network.connections')}: <strong>{networkStore.networkStats.total_connections}</strong></span>
        <span class="stat">{$_('network.avgNeighbors')}: <strong>{networkStore.networkStats.avg_neighbors_per_keyword.toFixed(1)}</strong></span>
      </div>
    {/if}
  </div>

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
                onclick={() => selectKeyword(keyword)}
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
            {#each networkStore.keywords as keyword (keyword.id)}
              <button
                class="keyword-item {networkStore.selectedKeyword?.id === keyword.id ? 'active' : ''}"
                onclick={() => selectKeyword(keyword)}
              >
                <span class="keyword-name">{keyword.name}</span>
                <span class="keyword-count">{keyword.article_count}</span>
              </button>
            {/each}

            {#if networkStore.hasMore && !networkStore.loading}
              <button onclick={() => networkStore.loadMoreKeywords()} class="load-more">
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
    <div class="detail-panel">
      {#if networkStore.selectedKeyword}
        <div class="keyword-detail">
          <h3 class="detail-title">{networkStore.selectedKeyword.name}</h3>

          <div class="detail-meta">
            <span class="meta-item">
              <span class="meta-label">{$_('network.articleCount')}:</span>
              <span class="meta-value">{networkStore.selectedKeyword.article_count}</span>
            </span>
            <span class="meta-item">
              <span class="meta-label">{$_('network.firstSeen')}:</span>
              <span class="meta-value">{formatDate(networkStore.selectedKeyword.first_seen)}</span>
            </span>
            <span class="meta-item">
              <span class="meta-label">{$_('network.lastUsed')}:</span>
              <span class="meta-value">{formatDate(networkStore.selectedKeyword.last_used)}</span>
            </span>
          </div>

          <!-- Categories -->
          {#if networkStore.keywordCategories.length > 0}
            <div class="detail-section">
              <h4 class="section-title">
                <Tooltip termKey="sephiroth">{$_('network.categories')}</Tooltip>
              </h4>
              <div class="categories-list">
                {#each networkStore.keywordCategories as cat (cat.sephiroth_id)}
                  <div class="category-item" style="--cat-color: {cat.color || '#6366F1'}">
                    <span class="category-icon">{cat.icon || '📁'}</span>
                    <span class="category-name">{cat.name}</span>
                    <span class="category-weight {getWeightClass(cat.weight)}">{(cat.weight * 100).toFixed(0)}%</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Neighbors -->
          {#if networkStore.neighbors.length > 0}
            <div class="detail-section">
              <h4 class="section-title">{$_('network.neighbors')}</h4>
              <div class="neighbors-list">
                {#each networkStore.neighbors as neighbor (neighbor.id)}
                  <button
                    class="neighbor-item"
                    onclick={() => navigateToNeighbor(neighbor)}
                  >
                    <span class="neighbor-name">{neighbor.name}</span>
                    <div class="neighbor-stats">
                      <span class="cooccurrence" title={$_('network.cooccurrence')}>
                        {neighbor.cooccurrence}x
                      </span>
                      {#if neighbor.embedding_similarity !== null}
                        <span class="similarity" title={$_('network.similarity')}>
                          {(neighbor.embedding_similarity * 100).toFixed(0)}%
                        </span>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            </div>
          {:else if !networkStore.loading}
            <div class="no-neighbors">{$_('network.noNeighbors')}</div>
          {/if}
        </div>
      {:else}
        <div class="no-selection">
          <div class="no-selection-icon">&#128279;</div>
          <p>{$_('network.selectKeyword')}</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .keyword-network {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-default);
  }

  .network-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-default);
    background-color: var(--bg-surface);
  }

  .network-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 0.5rem 0;
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

  /* Neighbors */
  .neighbors-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .neighbor-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.625rem 0.75rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .neighbor-item:hover {
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .neighbor-name {
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .neighbor-stats {
    display: flex;
    gap: 0.5rem;
    font-size: 0.75rem;
  }

  .cooccurrence {
    color: var(--text-muted);
    background-color: var(--bg-overlay);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }

  .similarity {
    color: var(--accent-primary);
    background-color: rgba(99, 102, 241, 0.1);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }

  .no-neighbors {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
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
</style>
