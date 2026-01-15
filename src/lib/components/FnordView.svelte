<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import { appState, type Fnord, type FnordStats, type CategoryRevisionStats, type SourceRevisionStats } from '../stores/state.svelte';
  import Tooltip from './Tooltip.svelte';
  import Tabs, { type Tab } from './Tabs.svelte';
  import { ArticleItemCompact } from './article';

  let stats = $state<FnordStats | null>(null);
  let changedFnords = $state<Fnord[]>([]);
  let loading = $state(true);
  let selectedFnordId = $state<number | null>(null);

  // Tab state
  let activeTab = $state<string>('stats');

  // Expanded category state
  let expandedCategoryId = $state<number | null>(null);
  let subcategories = $state<CategoryRevisionStats[]>([]);
  let loadingSubcategories = $state(false);

  // Tabs definition - derived to include dynamic badge
  let tabs = $derived<Tab[]>([
    { id: 'stats', label: $_('fnordView.statsTab') || 'Statistiken' },
    { id: 'articles', label: $_('fnordView.articlesTab') || 'Geänderte Artikel', badge: changedFnords.length || undefined }
  ]);

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    try {
      const statsData = await appState.getFnordStats();
      stats = statsData;

      await appState.loadChangedFnords();
      changedFnords = appState.changedFnords;
    } catch (e) {
      console.error('[FnordView] Error loading data:', e);
    } finally {
      loading = false;
    }
  }

  function selectFnord(id: number) {
    selectedFnordId = id;
    appState.selectFnord(id);
    // Navigate to article view
    window.dispatchEvent(new CustomEvent('navigate-to-article', { detail: { articleId: id } }));
  }


  async function toggleCategory(categoryId: number) {
    if (expandedCategoryId === categoryId) {
      // Collapse
      expandedCategoryId = null;
      subcategories = [];
    } else {
      // Expand and load subcategories
      expandedCategoryId = categoryId;
      loadingSubcategories = true;
      try {
        subcategories = await invoke<CategoryRevisionStats[]>('get_subcategory_stats', {
          mainCategoryId: categoryId,
        });
      } catch (e) {
        console.error('Failed to load subcategories:', e);
        subcategories = [];
      } finally {
        loadingSubcategories = false;
      }
    }
  }
</script>

<div class="fnord-view">
  <!-- Header -->
  <div class="fnord-header">
    <div class="header-top">
      <h2 class="fnord-title">
        <Tooltip termKey="fnord">{$_('fnordView.title') || 'Fnord'}</Tooltip>
      </h2>
      {#if stats}
        <div class="fnord-summary">
          <span class="summary-item">
            <span class="summary-value">{stats.total_revisions}</span>
            <span class="summary-label">{$_('fnordView.totalRevisions') || 'Revisionen'}</span>
          </span>
          <span class="summary-item">
            <span class="summary-value">{stats.articles_with_changes}</span>
            <span class="summary-label">{$_('fnordView.articlesWithChanges') || 'Geänderte Artikel'}</span>
          </span>
        </div>
      {/if}
    </div>

    <!-- Tabs -->
    <Tabs {tabs} bind:activeTab />
  </div>

  <!-- Content -->
  <div class="fnord-content">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <span>{$_('fnordView.loading') || 'Laden...'}</span>
      </div>
    {:else if activeTab === 'stats' && stats}
      {@const maxRevisions = Math.max(...stats.by_category.map(c => c.revision_count), 1)}
      {@const maxSourceRevisions = Math.max(...stats.by_source.map(s => s.revision_count), 1)}
      <div class="stats-view">
        <!-- By Category -->
        <div class="stats-section">
          <h3 class="section-title">
            <Tooltip termKey="sephiroth">{$_('fnordView.byCategory') || 'Nach Kategorie'}</Tooltip>
          </h3>
          <div class="category-cards">
            {#each stats.by_category as cat (cat.sephiroth_id)}
              {@const barWidth = (cat.revision_count / maxRevisions) * 100}
              {@const isExpanded = expandedCategoryId === cat.sephiroth_id}
              <button
                class="category-card {isExpanded ? 'expanded' : ''}"
                style="--cat-color: {cat.color || '#6366F1'}"
                onclick={() => toggleCategory(cat.sephiroth_id)}
              >
                <div class="card-header">
                  <div class="card-icon-wrapper">
                    {#if cat.icon}
                      <i class="{cat.icon}"></i>
                    {:else}
                      <i class="fa-solid fa-folder"></i>
                    {/if}
                  </div>
                  <span class="card-title">{cat.name}</span>
                  <i class="fa-solid fa-chevron-down expand-icon {isExpanded ? 'rotated' : ''}"></i>
                </div>
                <div class="card-stats">
                  <div class="stat-row">
                    <span class="stat-label">{$_('fnordView.revisions') || 'Revisionen'}</span>
                    <span class="stat-value">{cat.revision_count}</span>
                  </div>
                  <div class="progress-bar">
                    <div class="progress-fill" style="width: {barWidth}%"></div>
                  </div>
                  <div class="stat-row secondary">
                    <span class="stat-label">{$_('fnordView.articles') || 'Artikel'}</span>
                    <span class="stat-value">{cat.article_count}</span>
                  </div>
                </div>

                <!-- Subcategories (expanded view) -->
                {#if isExpanded}
                  <div class="subcategories">
                    {#if loadingSubcategories}
                      <div class="subcategory-loading">
                        <div class="spinner small"></div>
                      </div>
                    {:else if subcategories.length > 0}
                      {#each subcategories as sub (sub.sephiroth_id)}
                        <div class="subcategory-item">
                          <div class="subcategory-info">
                            {#if sub.icon}
                              <i class="{sub.icon} subcategory-icon"></i>
                            {/if}
                            <span class="subcategory-name">{sub.name}</span>
                          </div>
                          <div class="subcategory-stats">
                            <span class="subcategory-count" title="{$_('fnordView.revisions') || 'Revisionen'}">
                              {sub.revision_count}
                            </span>
                            <span class="subcategory-divider">/</span>
                            <span class="subcategory-count" title="{$_('fnordView.articles') || 'Artikel'}">
                              {sub.article_count}
                            </span>
                          </div>
                        </div>
                      {/each}
                    {:else}
                      <div class="subcategory-empty">
                        {$_('fnordView.noSubcategories') || 'Keine Unterkategorien'}
                      </div>
                    {/if}
                  </div>
                {/if}
              </button>
            {:else}
              <div class="empty-cards">
                <i class="fa-light fa-chart-bar empty-icon"></i>
                <p>{$_('fnordView.noData') || 'Keine Daten'}</p>
              </div>
            {/each}
          </div>
        </div>

        <!-- By Source -->
        <div class="stats-section">
          <h3 class="section-title">
            <Tooltip termKey="pentacle">{$_('fnordView.bySource') || 'Nach Quelle'}</Tooltip>
          </h3>
          <div class="source-list">
            {#each stats.by_source.filter((s) => s.revision_count > 0) as source (source.pentacle_id)}
              {@const barWidth = (source.revision_count / maxSourceRevisions) * 100}
              <div class="source-item">
                <div class="source-info">
                  <i class="fa-solid fa-rss source-icon"></i>
                  <span class="source-name">{source.title || `Feed #${source.pentacle_id}`}</span>
                </div>
                <div class="source-stats">
                  <div class="source-bar-container">
                    <div class="source-bar" style="width: {barWidth}%"></div>
                  </div>
                  <div class="source-numbers">
                    <span class="source-revisions" title="{$_('fnordView.revisions') || 'Revisionen'}">
                      <i class="fa-solid fa-code-compare"></i>
                      {source.revision_count}
                    </span>
                    <span class="source-articles" title="{$_('fnordView.articles') || 'Artikel'}">
                      <i class="fa-solid fa-newspaper"></i>
                      {source.article_count}
                    </span>
                  </div>
                </div>
              </div>
            {:else}
              <div class="empty-cards">
                <i class="fa-light fa-rss empty-icon"></i>
                <p>{$_('fnordView.noData') || 'Keine Daten'}</p>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {:else if activeTab === 'articles'}
      <div class="articles-view">
        {#if changedFnords.length === 0}
          <div class="empty-state">
            <i class="empty-icon fa-solid fa-check"></i>
            <p>{$_('fnordView.noChangedArticles') || 'Keine geänderten Artikel'}</p>
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
    {/if}
  </div>
</div>

<style>
  .fnord-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1;
    background-color: var(--bg-default);
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

  .fnord-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
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
    gap: 2rem;
  }

  .stats-section {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    padding: 1.25rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 1rem 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Category Cards */
  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  .category-card {
    background: linear-gradient(135deg, color-mix(in srgb, var(--cat-color) 15%, var(--bg-default)) 0%, var(--bg-default) 100%);
    border: 1px solid color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-radius: 0.625rem;
    padding: 1rem;
    transition: transform 0.15s ease, box-shadow 0.15s ease;
    cursor: pointer;
    text-align: left;
    width: 100%;
  }

  .category-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px color-mix(in srgb, var(--cat-color) 20%, transparent);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.875rem;
  }

  .card-icon-wrapper {
    width: 2.25rem;
    height: 2.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 70%, black));
    border-radius: 0.5rem;
    color: white;
    font-size: 1rem;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 40%, transparent);
  }

  .card-title {
    font-size: 0.9375rem;
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
    gap: 0.375rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .stat-row.secondary {
    margin-top: 0.25rem;
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .stat-value {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .stat-row.secondary .stat-label,
  .stat-row.secondary .stat-value {
    font-size: 0.6875rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .progress-bar {
    height: 6px;
    background-color: color-mix(in srgb, var(--cat-color) 20%, transparent);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 80%, white));
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  /* Category Card expanded state */
  .category-card.expanded {
    grid-column: 1 / -1;
  }

  .expand-icon {
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: transform 0.2s ease;
    flex-shrink: 0;
  }

  .expand-icon.rotated {
    transform: rotate(180deg);
  }

  /* Subcategories */
  .subcategories {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid color-mix(in srgb, var(--cat-color) 20%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .subcategory-loading {
    display: flex;
    justify-content: center;
    padding: 0.5rem;
  }

  .spinner.small {
    width: 1rem;
    height: 1rem;
    border-width: 2px;
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.375rem;
    font-size: 0.8125rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .subcategory-icon {
    font-size: 0.75rem;
    color: var(--cat-color);
    opacity: 0.8;
  }

  .subcategory-name {
    color: var(--text-primary);
  }

  .subcategory-stats {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .subcategory-count {
    font-weight: 500;
  }

  .subcategory-divider {
    opacity: 0.5;
  }

  .subcategory-empty {
    text-align: center;
    color: var(--text-muted);
    font-size: 0.75rem;
    padding: 0.5rem;
  }

  /* Source List */
  .source-list {
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .source-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background-color: var(--bg-default);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    transition: background-color 0.15s ease;
  }

  .source-item:hover {
    background-color: var(--bg-overlay);
  }

  .source-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1;
    min-width: 0;
  }

  .source-icon {
    font-size: 0.875rem;
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  .source-name {
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source-stats {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-shrink: 0;
  }

  .source-bar-container {
    width: 80px;
    height: 4px;
    background-color: var(--border-default);
    border-radius: 2px;
    overflow: hidden;
  }

  .source-bar {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-primary), var(--accent-secondary, var(--accent-primary)));
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  .source-numbers {
    display: flex;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .source-revisions,
  .source-articles {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .source-revisions i,
  .source-articles i {
    font-size: 0.625rem;
    opacity: 0.7;
  }

  /* Empty state for cards */
  .empty-cards {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    color: var(--text-muted);
    gap: 0.5rem;
  }

  .empty-cards .empty-icon {
    font-size: 2rem;
    opacity: 0.4;
  }

  .empty-cards p {
    margin: 0;
    font-size: 0.875rem;
  }

  /* Articles View */
  .articles-view {
    height: 100%;
  }

  .articles-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
</style>
