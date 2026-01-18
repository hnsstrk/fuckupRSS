<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import { appState, type Fnord } from '../stores/state.svelte';
  import Tabs, { type Tab } from './Tabs.svelte';
  import Tooltip from './Tooltip.svelte';
  import { ArticleItemCompact } from './article';

  // Type for analysis status articles from backend
  interface AnalysisStatusArticle {
    id: number;
    title: string;
    pentacle_id: number;
    pentacle_title: string | null;
    summary: string | null;
    published_at: string | null;
    status: string;
    analysis_attempts: number;
    last_error: string | null;
  }

  // State
  let loading = $state(false);
  let articles = $state<Fnord[]>([]);
  let activeTab = $state<string>('articles');
  let selectedArticleId = $state<number | null>(null);

  // Stats
  let totalCount = $state(0);
  let unreadCount = $state(0);
  let favoritesCount = $state(0);
  let failedCount = $state(0);
  let hopelessCount = $state(0);

  // Tabs definition with badges
  let tabs = $derived<Tab[]>([
    { id: 'articles', label: $_('erisianArchives.tabs.articles') || 'Artikel' },
    { id: 'unread', label: $_('erisianArchives.tabs.unread') || 'Ungelesen', badge: unreadCount || undefined },
    { id: 'goldenApple', label: $_('erisianArchives.tabs.goldenApple') || 'Golden Apple', badge: favoritesCount || undefined },
    { id: 'failed', label: $_('erisianArchives.tabs.failed') || 'Fehlgeschlagen', badge: failedCount || undefined },
    { id: 'hopeless', label: $_('erisianArchives.tabs.hopeless') || 'Hoffnungslos', badge: hopelessCount || undefined },
  ]);

  onMount(async () => {
    await loadStats();
    await loadArticles();
  });

  async function loadStats() {
    try {
      // Get total count
      totalCount = await invoke<number>('get_fnords_count', { filter: null });

      // Get unread count
      unreadCount = await invoke<number>('get_fnords_count', { filter: { status: 'concealed' } });

      // Get favorites count
      favoritesCount = await invoke<number>('get_fnords_count', { filter: { status: 'golden_apple' } });

      // Get failed count
      const failedResult = await invoke<{ count: number }>('get_failed_count');
      failedCount = failedResult.count;

      // Get hopeless count
      const hopelessResult = await invoke<{ count: number }>('get_hopeless_count');
      hopelessCount = hopelessResult.count;
    } catch (e) {
      console.error('[ErisianArchives] Error loading stats:', e);
    }
  }

  async function loadArticles() {
    loading = true;
    try {
      let filter: { status?: string; limit?: number } | null = null;

      switch (activeTab) {
        case 'articles':
          filter = { limit: 100 };
          break;
        case 'unread':
          filter = { status: 'concealed', limit: 100 };
          break;
        case 'goldenApple':
          filter = { status: 'golden_apple', limit: 100 };
          break;
        case 'failed':
          {
            const failedArticles = await invoke<AnalysisStatusArticle[]>('get_failed_articles', { limit: 100 });
            // Map to Fnord-like structure for ArticleItemCompact
            articles = failedArticles.map(a => ({
              id: a.id,
              pentacle_id: a.pentacle_id,
              pentacle_title: a.pentacle_title,
              guid: '',
              url: '',
              title: a.title,
              author: null,
              content_raw: null,
              content_full: null,
              summary: a.summary,
              image_url: null,
              published_at: a.published_at,
              processed_at: null,
              status: a.status,
              political_bias: null,
              sachlichkeit: null,
              quality_score: null,
              has_changes: false,
              changed_at: null,
              revision_count: 0,
              categories: [],
            } as Fnord));
          }
          loading = false;
          return;
        case 'hopeless':
          {
            const hopelessArticles = await invoke<AnalysisStatusArticle[]>('get_hopeless_articles', { limit: 100 });
            // Map to Fnord-like structure for ArticleItemCompact
            articles = hopelessArticles.map(a => ({
              id: a.id,
              pentacle_id: a.pentacle_id,
              pentacle_title: a.pentacle_title,
              guid: '',
              url: '',
              title: a.title,
              author: null,
              content_raw: null,
              content_full: null,
              summary: a.summary,
              image_url: null,
              published_at: a.published_at,
              processed_at: null,
              status: a.status,
              political_bias: null,
              sachlichkeit: null,
              quality_score: null,
              has_changes: false,
              changed_at: null,
              revision_count: 0,
              categories: [],
            } as Fnord));
          }
          loading = false;
          return;
      }

      articles = await invoke<Fnord[]>('get_fnords', { filter });
    } catch (e) {
      console.error('[ErisianArchives] Error loading articles:', e);
      articles = [];
    } finally {
      loading = false;
    }
  }

  function handleTabChange(tabId: string) {
    activeTab = tabId;
    loadArticles();
  }

  function selectArticle(id: number) {
    selectedArticleId = id;
    appState.selectFnord(id);
    window.dispatchEvent(new CustomEvent('navigate-to-article', { detail: { articleId: id } }));
  }

  // Empty state messages based on active tab
  let emptyMessage = $derived.by(() => {
    switch (activeTab) {
      case 'articles':
        return $_('erisianArchives.noArticles') || 'Keine Artikel vorhanden';
      case 'unread':
        return $_('erisianArchives.noUnread') || 'Alle Artikel wurden gelesen - Erleuchtung erreicht!';
      case 'goldenApple':
        return $_('erisianArchives.noFavorites') || 'Keine Golden Apples - markiere Artikel als Favorit!';
      case 'failed':
        return $_('erisianArchives.noFailed') || 'Keine fehlgeschlagenen Analysen';
      case 'hopeless':
        return $_('erisianArchives.noHopeless') || 'Keine hoffnungslosen Faelle - die KI hat alles gemeistert!';
      default:
        return '';
    }
  });

  // Check if current tab is a special analysis tab (not yet supported for listing)
  let isAnalysisTab = $derived(activeTab === 'failed' || activeTab === 'hopeless');
  let analysisTabCount = $derived(activeTab === 'failed' ? failedCount : hopelessCount);
</script>

<div class="erisian-archives">
  <!-- Header -->
  <div class="erisian-header">
    <div class="header-top">
      <h2 class="view-title">
        <i class="fa-solid fa-newspaper nav-icon"></i>
        {$_('erisianArchives.title') || 'Erisian Archives'}
        <Tooltip termKey="erisian_archives">
          <i class="fa-solid fa-circle-info info-icon"></i>
        </Tooltip>
      </h2>
      <div class="erisian-summary">
        <span class="summary-item">
          <span class="summary-value">{totalCount}</span>
          <span class="summary-label">{$_('erisianArchives.stats.total') || 'Gesamt'}</span>
        </span>
        <span class="summary-item">
          <span class="summary-value">{unreadCount}</span>
          <span class="summary-label">{$_('erisianArchives.stats.unread') || 'Ungelesen'}</span>
        </span>
        <span class="summary-item">
          <span class="summary-value">{favoritesCount}</span>
          <span class="summary-label">{$_('erisianArchives.stats.favorites') || 'Favoriten'}</span>
        </span>
      </div>
    </div>

    <!-- Tabs -->
    <Tabs {tabs} bind:activeTab onchange={handleTabChange} />
  </div>

  <!-- Content -->
  <div class="erisian-content">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <span>{$_('fnordView.loading') || 'Laden...'}</span>
      </div>
    {:else if isAnalysisTab}
      <!-- Special handling for failed/hopeless tabs -->
      {#if analysisTabCount === 0}
        <div class="empty-state">
          <i class="empty-icon fa-solid fa-check-circle"></i>
          <p>{emptyMessage}</p>
        </div>
      {:else}
        <div class="info-state">
          <i class="info-icon fa-solid fa-info-circle"></i>
          <p class="info-count">{analysisTabCount} {activeTab === 'failed' ? $_('statusBar.failed') : $_('statusBar.hopeless')}</p>
          <p class="info-description">
            {#if activeTab === 'failed'}
              {$_('statusBar.failedTooltip') || 'Artikel, bei denen die KI-Analyse fehlgeschlagen ist, aber noch weitere Versuche ausstehen.'}
            {:else}
              {$_('statusBar.hopelessTooltip') || 'Artikel, deren KI-Analyse nach mehreren Versuchen fehlgeschlagen ist. Diese werden nicht mehr automatisch analysiert.'}
            {/if}
          </p>
        </div>
      {/if}
    {:else if articles.length === 0}
      <div class="empty-state">
        <i class="empty-icon fa-solid fa-box-open"></i>
        <p>{emptyMessage}</p>
      </div>
    {:else}
      <div class="articles-list">
        {#each articles as article (article.id)}
          <ArticleItemCompact
            id={article.id}
            title={article.title}
            status={article.status}
            pentacle_title={article.pentacle_title}
            published_at={article.published_at}
            categories={article.categories}
            revision_count={article.revision_count}
            quality_score={article.quality_score}
            political_bias={article.political_bias}
            active={selectedArticleId === article.id}
            onclick={() => selectArticle(article.id)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .erisian-archives {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1;
    background-color: var(--bg-base);
  }

  .erisian-header {
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

  .erisian-summary {
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

  .erisian-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.5rem;
  }

  .loading-state,
  .empty-state,
  .info-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 1rem;
    text-align: center;
  }

  .empty-icon,
  .info-state .info-icon {
    font-size: 3rem;
    opacity: 0.5;
  }

  .info-state .info-icon {
    color: var(--accent-primary);
    opacity: 0.7;
  }

  .info-count {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .info-description {
    max-width: 400px;
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin: 0;
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
    to { transform: rotate(360deg); }
  }

  .articles-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
</style>
