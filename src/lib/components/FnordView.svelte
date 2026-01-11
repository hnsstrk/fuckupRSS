<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { appState, type Fnord, type FnordStats, type CategoryRevisionStats, type SourceRevisionStats } from '../stores/state.svelte';
  import Tooltip from './Tooltip.svelte';
  import Tabs, { type Tab } from './Tabs.svelte';

  let stats = $state<FnordStats | null>(null);
  let changedFnords = $state<Fnord[]>([]);
  let loading = $state(true);
  let selectedFnordId = $state<number | null>(null);

  // Tab state
  let activeTab = $state<string>('stats');

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
      const [statsData, fnordsData] = await Promise.all([
        appState.getFnordStats(),
        appState.loadChangedFnords().then(() => appState.changedFnords),
      ]);
      stats = statsData;
      changedFnords = appState.changedFnords;
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

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '-';
    const date = new Date(dateStr);
    return date.toLocaleDateString('de-DE', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
    });
  }

  function getStatusIconClass(status: string): string {
    switch (status) {
      case 'concealed':
        return 'fa-solid fa-eye-slash';
      case 'illuminated':
        return 'fa-solid fa-check';
      case 'golden_apple':
        return 'fa-solid fa-apple-whole';
      default:
        return '';
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
      <div class="stats-view">
        <!-- By Category -->
        <div class="stats-section">
          <h3 class="section-title">
            <Tooltip termKey="sephiroth">{$_('fnordView.byCategory') || 'Nach Kategorie'}</Tooltip>
          </h3>
          <div class="stats-table-container">
            <table class="stats-table">
              <thead>
                <tr>
                  <th>{$_('fnordView.category') || 'Kategorie'}</th>
                  <th class="num">{$_('fnordView.revisions') || 'Revisionen'}</th>
                  <th class="num">{$_('fnordView.articles') || 'Artikel'}</th>
                </tr>
              </thead>
              <tbody>
                {#each stats.by_category.filter((c) => c.revision_count > 0) as cat}
                  <tr>
                    <td class="category-cell">
                      {#if cat.icon}
                        <span class="category-icon">{cat.icon}</span>
                      {/if}
                      <span class="category-name" style="--cat-color: {cat.color || '#6366F1'}">{cat.name}</span>
                    </td>
                    <td class="num">{cat.revision_count}</td>
                    <td class="num">{cat.article_count}</td>
                  </tr>
                {:else}
                  <tr>
                    <td colspan="3" class="empty-row">{$_('fnordView.noData') || 'Keine Daten'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>

        <!-- By Source -->
        <div class="stats-section">
          <h3 class="section-title">
            <Tooltip termKey="pentacle">{$_('fnordView.bySource') || 'Nach Quelle'}</Tooltip>
          </h3>
          <div class="stats-table-container">
            <table class="stats-table">
              <thead>
                <tr>
                  <th>{$_('fnordView.source') || 'Quelle'}</th>
                  <th class="num">{$_('fnordView.revisions') || 'Revisionen'}</th>
                  <th class="num">{$_('fnordView.articles') || 'Artikel'}</th>
                </tr>
              </thead>
              <tbody>
                {#each stats.by_source.filter((s) => s.revision_count > 0) as source}
                  <tr>
                    <td class="source-cell">{source.title || `Feed #${source.pentacle_id}`}</td>
                    <td class="num">{source.revision_count}</td>
                    <td class="num">{source.article_count}</td>
                  </tr>
                {:else}
                  <tr>
                    <td colspan="3" class="empty-row">{$_('fnordView.noData') || 'Keine Daten'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
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
            {#each changedFnords as fnord}
              <button
                class="article-item {selectedFnordId === fnord.id ? 'active' : ''}"
                onclick={() => selectFnord(fnord.id)}
              >
                <div class="article-status" title={fnord.status}>
                  <i class={getStatusIconClass(fnord.status)}></i>
                </div>
                <div class="article-info">
                  <span class="article-title">{fnord.title}</span>
                  <span class="article-meta">
                    {#if fnord.pentacle_title}
                      <span class="article-source">{fnord.pentacle_title}</span>
                    {/if}
                    {#if fnord.changed_at}
                      <span class="article-changed">
                        {$_('fnordView.changedAt') || 'Geändert'}: {formatDate(fnord.changed_at)}
                      </span>
                    {/if}
                    <span class="article-revisions">
                      {fnord.revision_count} {$_('fnordView.revisions') || 'Revisionen'}
                    </span>
                  </span>
                </div>
              </button>
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
    border-radius: 0.5rem;
    padding: 1rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .stats-table-container {
    overflow-x: auto;
  }

  .stats-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  .stats-table th,
  .stats-table td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--border-default);
  }

  .stats-table th {
    font-weight: 600;
    color: var(--text-muted);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .stats-table th.num,
  .stats-table td.num {
    text-align: right;
    width: 80px;
  }

  .stats-table tbody tr:hover {
    background-color: var(--bg-overlay);
  }

  .category-cell {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .category-icon {
    font-size: 1rem;
  }

  .category-name {
    color: var(--text-primary);
    border-left: 3px solid var(--cat-color);
    padding-left: 0.5rem;
  }

  .source-cell {
    color: var(--text-primary);
    max-width: 250px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-row {
    text-align: center;
    color: var(--text-muted);
    font-style: italic;
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

  .article-item {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.75rem;
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

  .article-item.active {
    background-color: var(--bg-overlay);
    border-left: 3px solid var(--accent-primary);
  }

  .article-status {
    font-size: 1rem;
    flex-shrink: 0;
    width: 1.5rem;
  }

  .article-info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
    flex: 1;
  }

  .article-title {
    font-size: 0.9375rem;
    color: var(--text-primary);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .article-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .article-source {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .article-changed {
    color: var(--accent-warning);
  }

  .article-revisions {
    background-color: var(--bg-overlay);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }
</style>
