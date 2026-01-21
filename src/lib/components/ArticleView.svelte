<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-shell';
  import { appState, toasts, type FnordRevision, type ArticleCategory, type Tag, type SimilarArticle } from "../stores/state.svelte";
  import type { ArticleKeyword, ArticleCategoryDetailed } from '$lib/types';
  import RevisionView from "./RevisionView.svelte";
  import { ArticleCard, ArticleKeywords, ArticleCategories } from "./article";
  import StatisticalPreview from "./article/StatisticalPreview.svelte";
  import { sanitizeArticleContent } from '$lib/utils/sanitizer';

  // Get the main category ID (1-6) from a category or subcategory ID
  function getMainCategoryId(id: number | undefined): number {
    if (!id) return 0;
    if (id <= 6) return id;
    return Math.floor(id / 100); // Subcategory IDs are 101, 102, 201, etc.
  }

  // Get CSS variable name for category color
  function getCategoryColorVar(id: number | undefined): string {
    const mainId = getMainCategoryId(id);
    if (mainId >= 1 && mainId <= 6) {
      return `var(--category-${mainId})`;
    }
    return 'var(--bg-overlay)';
  }

  // Track component mount state to prevent state updates after unmount
  let mounted = $state(true);

  // Cleanup effect to set mounted = false when component is destroyed
  $effect(() => {
    return () => {
      mounted = false;
    };
  });

  let showRevisions = $state(false);
  let revisions = $state<FnordRevision[]>([]);
  let loadingRevisions = $state(false);

  // Categories and Tags
  let categories = $state<ArticleCategory[]>([]);
  let tags = $state<Tag[]>([]);

  // Editable Keywords and Categories (with source tracking)
  let editingKeywords = $state(false);
  let editingCategories = $state(false);
  let articleKeywords = $state<ArticleKeyword[]>([]);
  let articleCategoriesDetailed = $state<ArticleCategoryDetailed[]>([]);

  // Similar Articles
  let similarArticles = $state<SimilarArticle[]>([]);
  let loadingSimilar = $state(false);

  // Track the last loaded article to prevent redundant fetches
  let lastLoadedFnordId = $state<number | null>(null);

  async function loadArticleData(fnordId: number, revisionCount: number, hasEmbedding: boolean) {
    // Load revisions if available
    if (revisionCount > 0) {
      loadingRevisions = true;
      try {
        const revs = await appState.getRevisions(fnordId);
        // Check if still mounted and same article before updating state
        if (!mounted || appState.selectedFnord?.id !== fnordId) return;
        revisions = revs;
      } catch {
        if (!mounted || appState.selectedFnord?.id !== fnordId) return;
        revisions = [];
      } finally {
        if (mounted) loadingRevisions = false;
      }
    } else {
      revisions = [];
    }

    // Check if still mounted before continuing
    if (!mounted || appState.selectedFnord?.id !== fnordId) return;

    // Load categories and tags (both old format and new detailed format)
    try {
      const [cats, tgs, kwds, catsDetailed] = await Promise.all([
        appState.getArticleCategories(fnordId),
        appState.getArticleTags(fnordId),
        invoke<ArticleKeyword[]>('get_article_keywords', { fnordId }),
        invoke<ArticleCategoryDetailed[]>('get_article_categories_detailed', { fnordId })
      ]);
      // Check if still mounted and same article before updating state
      if (!mounted || appState.selectedFnord?.id !== fnordId) return;
      categories = cats;
      tags = tgs;
      articleKeywords = kwds;
      articleCategoriesDetailed = catsDetailed;
    } catch {
      if (!mounted || appState.selectedFnord?.id !== fnordId) return;
      categories = [];
      tags = [];
      articleKeywords = [];
      articleCategoriesDetailed = [];
    }

    // Check if still mounted before continuing
    if (!mounted || appState.selectedFnord?.id !== fnordId) return;

    // Load similar articles (only if article was processed and has embedding)
    if (hasEmbedding) {
      loadingSimilar = true;
      try {
        const similar = await appState.findSimilarArticles(fnordId, 5);
        // Check if still mounted and same article before updating state
        if (!mounted || appState.selectedFnord?.id !== fnordId) return;
        similarArticles = similar;
      } catch {
        if (!mounted || appState.selectedFnord?.id !== fnordId) return;
        similarArticles = [];
      } finally {
        if (mounted) loadingSimilar = false;
      }
    } else {
      similarArticles = [];
    }
  }

  function toggleRevisions() {
    showRevisions = !showRevisions;
  }

  // Combined effect for article changes - handles all side effects
  $effect(() => {
    const fnord = appState.selectedFnord;

    if (!fnord) {
      // Reset state when no article selected
      if (lastLoadedFnordId !== null) {
        revisions = [];
        categories = [];
        tags = [];
        similarArticles = [];
        articleKeywords = [];
        articleCategoriesDetailed = [];
        editingKeywords = false;
        editingCategories = false;
        lastLoadedFnordId = null;
      }
      return;
    }

    // Auto-acknowledge changed articles
    if (fnord.has_changes) {
      appState.acknowledgeChanges(fnord.id);
    }

    // Load data only if article changed
    if (fnord.id !== lastLoadedFnordId) {
      lastLoadedFnordId = fnord.id;
      // Pass whether article was processed (likely has embedding)
      const hasEmbedding = fnord.processed_at !== null;
      loadArticleData(fnord.id, fnord.revision_count, hasEmbedding);
    }
  });

  // Listen for batch-complete event to refresh similar articles
  // (embeddings are regenerated during batch processing)
  async function handleBatchComplete() {
    const fnord = appState.selectedFnord;
    if (!fnord || !fnord.processed_at || !mounted) return;

    const fnordId = fnord.id;
    try {
      const [cats, tgs, similar] = await Promise.all([
        appState.getArticleCategories(fnordId),
        appState.getArticleTags(fnordId),
        appState.findSimilarArticles(fnordId, 5)
      ]);
      // Check if still mounted and same article before updating state
      if (!mounted || appState.selectedFnord?.id !== fnordId) return;
      categories = cats;
      tags = tgs;
      similarArticles = similar;
    } catch {
      // Ignore errors during refresh
    }
  }

  // Handler for keywords update (called by ArticleKeywords component)
  async function handleKeywordsUpdate(updatedKeywords: ArticleKeyword[]) {
    if (!mounted) return;
    articleKeywords = updatedKeywords;
    // Also refresh old tags for similar articles display
    const fnord = appState.selectedFnord;
    if (fnord) {
      const fnordId = fnord.id;
      const newTags = await appState.getArticleTags(fnordId);
      if (!mounted || appState.selectedFnord?.id !== fnordId) return;
      tags = newTags;
    }
  }

  // Handler for categories update (called by ArticleCategories component)
  async function handleCategoriesUpdate(updatedCategories: ArticleCategoryDetailed[]) {
    if (!mounted) return;
    articleCategoriesDetailed = updatedCategories;
    // Also refresh old categories for other displays
    const fnord = appState.selectedFnord;
    if (fnord) {
      const fnordId = fnord.id;
      const newCats = await appState.getArticleCategories(fnordId);
      if (!mounted || appState.selectedFnord?.id !== fnordId) return;
      categories = newCats;
    }
  }

  onMount(() => {
    window.addEventListener('batch-complete', handleBatchComplete);
  });

  onDestroy(() => {
    window.removeEventListener('batch-complete', handleBatchComplete);
  });

  function stripHtml(html: string): string {
    const div = document.createElement('div');
    div.innerHTML = html;
    return div.textContent || div.innerText || '';
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    const currentLocale = $locale || 'de';
    return date.toLocaleDateString(currentLocale.startsWith('de') ? "de-DE" : "en-US", {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function getBiasLabel(bias: number | null): string {
    if (bias === null) return $_('articleView.notRated');
    switch (bias) {
      case -2: return $_('articleView.biasStrongLeft');
      case -1: return $_('articleView.biasLeanLeft');
      case 0: return $_('articleView.greyface.biasCenter');
      case 1: return $_('articleView.biasLeanRight');
      case 2: return $_('articleView.biasStrongRight');
      default: return $_('articleView.unknown');
    }
  }

  function getSachlichkeitLabel(s: number | null): string {
    if (s === null) return $_('articleView.notRated');
    switch (s) {
      case 0: return $_('articleView.sachHighlyEmotional');
      case 1: return $_('articleView.sachEmotional');
      case 2: return $_('articleView.sachMixed');
      case 3: return $_('articleView.sachMostlyObjective');
      case 4: return $_('articleView.sachObjective');
      default: return $_('articleView.unknown');
    }
  }

  // Compact Greyface helpers
  function getBiasIcon(bias: number | null): string {
    if (bias === null) return 'fa-scale-balanced';
    if (bias < 0) return 'fa-scale-unbalanced';
    if (bias > 0) return 'fa-scale-unbalanced-flip';
    return 'fa-scale-balanced';
  }

  function getBiasColor(bias: number | null): string {
    if (bias === null) return 'neutral';
    if (bias <= -2) return 'strong-left';
    if (bias === -1) return 'lean-left';
    if (bias === 0) return 'center';
    if (bias === 1) return 'lean-right';
    return 'strong-right';
  }

  function getSachlichkeitIcon(s: number | null): string {
    if (s === null) return 'fa-face-meh';
    if (s <= 1) return 'fa-heart';
    if (s === 2) return 'fa-face-meh';
    return 'fa-brain';
  }

  function getSachlichkeitColor(s: number | null): string {
    if (s === null) return 'neutral';
    if (s <= 1) return 'emotional';
    if (s === 2) return 'mixed';
    return 'objective';
  }

  async function openInBrowser() {
    if (appState.selectedFnord) {
      await open(appState.selectedFnord.url);
    }
  }

  // Get specific error message based on error content
  function getSpecificFetchError(error: string): string {
    const errorLower = error.toLowerCase();
    // Check for specific HTTP status codes
    if (errorLower.includes('404') || errorLower.includes('not found')) {
      return $_('toast.fetchErrorNotFound');
    }
    if (errorLower.includes('403') || errorLower.includes('forbidden')) {
      return $_('toast.fetchErrorBlocked');
    }
    if (errorLower.includes('timeout') || errorLower.includes('timed out')) {
      return $_('toast.fetchErrorTimeout');
    }
    if (errorLower.includes('network') || errorLower.includes('connection') || errorLower.includes('unreachable')) {
      return $_('toast.fetchErrorNetwork');
    }
    if (errorLower.includes('paywall') || errorLower.includes('subscription')) {
      return $_('toast.fetchErrorPaywall');
    }
    // Check for 5xx server errors
    const serverErrorMatch = error.match(/5\d{2}/);
    if (serverErrorMatch) {
      return $_('toast.fetchErrorServerError', { values: { code: serverErrorMatch[0] }});
    }
    // Fallback to generic error
    return $_('toast.fetchError', { values: { error }});
  }

  async function fetchFullContent() {
    const fnord = appState.selectedFnord;
    if (!fnord || !mounted) return;

    const fnordId = fnord.id;
    try {
      const result = await appState.fetchFullContent(fnordId);

      // Check if still mounted after async operation
      if (!mounted) return;

      if (result?.success) {
        toasts.success($_('toast.fetchSuccess'));
      } else if (result?.error) {
        toasts.error(getSpecificFetchError(result.error));
      } else if (appState.error) {
        toasts.error(getSpecificFetchError(appState.error));
      }
    } catch (e) {
      console.error('Fetch full content failed:', e);
      if (mounted) {
        toasts.error(getSpecificFetchError(String(e)));
      }
    }
  }

  async function analyzeWithAI() {
    const fnord = appState.selectedFnord;
    if (!fnord || !appState.ollamaStatus.available || !mounted) return;

    const fnordId = fnord.id;
    try {
      const result = await appState.processArticleDiscordian(fnordId);

      // Check if still mounted and same article after async operation
      if (!mounted) return;

      if (result?.success) {
        // Check if same article is still selected before reloading data
        if (appState.selectedFnord?.id !== fnordId) return;

        // Reload categories, tags, and similar articles after analysis
        // (embedding is regenerated in backend, so similar articles may change)
        const [cats, tgs, similar] = await Promise.all([
          appState.getArticleCategories(fnordId),
          appState.getArticleTags(fnordId),
          appState.findSimilarArticles(fnordId, 5)
        ]);

        // Check again after Promise.all
        if (!mounted || appState.selectedFnord?.id !== fnordId) return;

        categories = cats;
        tags = tgs;
        similarArticles = similar;
        toasts.success($_('toast.analyzeSuccess'));
      } else if (result?.error) {
        if (mounted) toasts.error($_('toast.analyzeError', { values: { error: result.error }}));
      } else if (appState.error) {
        if (mounted) toasts.error($_('toast.analyzeError', { values: { error: appState.error }}));
      }
    } catch (e) {
      console.error('AI analysis failed:', e);
      if (mounted) {
        toasts.error($_('toast.analyzeError', { values: { error: String(e) }}));
      }
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "v" && appState.selectedFnord) {
      e.preventDefault();
      openInBrowser();
    }
    // 'r' for retrieve full text
    if (e.key === "r" && appState.selectedFnord && !appState.selectedFnord.content_full) {
      e.preventDefault();
      fetchFullContent();
    }
  }

  function navigateToKeyword(tagId: number) {
    window.dispatchEvent(new CustomEvent('navigate-to-network', { detail: { keywordId: tagId } }));
  }

  function navigateToSimilarArticle(fnordId: number) {
    // Use navigate event to ensure article is loaded even if not in current filter
    window.dispatchEvent(new CustomEvent('navigate-to-article', { detail: { articleId: fnordId } }));
    // Scroll to top of article view
    const articleView = document.querySelector('.article-view');
    if (articleView) {
      articleView.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }

  // Check if article has sufficient content for AI analysis
  function hasContentForAnalysis(fnord: typeof appState.selectedFnord): boolean {
    if (!fnord) return false;
    const content = fnord.content_full;
    return !!content && content.length >= 100;
  }

  // Determine content status for badge display
  type ContentStatus = 'full' | 'rss' | 'missing';
  function getContentStatus(fnord: typeof appState.selectedFnord): ContentStatus {
    if (!fnord) return 'missing';
    // Full text: content_full exists and is > 500 characters
    if (fnord.content_full && fnord.content_full.length > 500) {
      return 'full';
    }
    // RSS only: has content_raw but no or short content_full
    if (fnord.content_raw) {
      return 'rss';
    }
    // Missing: neither content_full nor content_raw
    return 'missing';
  }

  function getContentStatusLabel(status: ContentStatus): string {
    switch (status) {
      case 'full': return $_('articleView.contentStatus.full');
      case 'rss': return $_('articleView.contentStatus.rss');
      case 'missing': return $_('articleView.contentStatus.missing');
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="article-view">
  {#if appState.selectedFnord}
    {@const fnord = appState.selectedFnord}

    <!-- Article Header -->
    <div class="article-header">
      <div class="header-content">
        <div class="article-meta">
          <span class="source">{fnord.pentacle_title || "Unknown Source"}</span>
          <span class="separator">·</span>
          <span>{formatDate(fnord.published_at)}</span>
          {#if fnord.author}
            <span class="separator">·</span>
            <span>{$_('articleView.by')} {fnord.author}</span>
          {/if}
          <span class="separator">·</span>
          <span class="content-status-badge content-status-{getContentStatus(fnord)}">
            {getContentStatusLabel(getContentStatus(fnord))}
          </span>
        </div>

        <h1 class="article-title">{fnord.title}</h1>

        <div class="article-actions">
          <button
            onclick={() => appState.toggleGoldenApple(fnord.id)}
            class="btn {fnord.status === 'golden_apple' ? 'btn-golden' : 'btn-default'}"
          >
            <i class="btn-icon {fnord.status === 'golden_apple' ? 'fa-solid fa-apple-whole' : 'fa-regular fa-star'}"></i>
            <span>{fnord.status === "golden_apple" ? $_('terminology.golden_apple.term') : $_('actions.favorite')}</span>
          </button>
          {#if !fnord.content_full}
            {@const hasFetchError = fnord.full_text_fetch_error}
            {#if appState.retrieving}
              <!-- Loading state -->
              <button
                class="btn btn-default retrieving"
                disabled
                title={$_('articleView.fetchFetching')}
              >
                <i class="spinner fa-solid fa-rotate fa-spin"></i>
                <span>{$_('articleView.fetchFetching')}</span>
              </button>
            {:else if hasFetchError}
              <!-- Error state -->
              <div class="fetch-error-container">
                <span class="btn btn-error" title={hasFetchError}>
                  <i class="fa-solid fa-triangle-exclamation"></i>
                  <span>{$_('articleView.fetchError')}</span>
                </span>
                <button
                  onclick={fetchFullContent}
                  class="btn-retry"
                  title={$_('articleView.fetchFullText')}
                >
                  {$_('articleView.fetchRetry')}
                </button>
              </div>
            {:else}
              <!-- Normal state -->
              <button
                onclick={fetchFullContent}
                class="btn btn-default"
                title="{$_('articleView.fetchFullText')} (r)"
              >
                <span>{$_('articleView.fullText')}</span>
              </button>
            {/if}
          {/if}
          {#if appState.ollamaStatus.available}
            {@const canAnalyze = hasContentForAnalysis(fnord)}
            <button
              onclick={analyzeWithAI}
              class="btn btn-default {appState.analyzing ? 'retrieving' : ''} {!canAnalyze ? 'btn-disabled-info' : ''}"
              disabled={appState.analyzing || !canAnalyze}
              title={canAnalyze ? $_('articleView.aiAnalysis') : $_('articleView.analyzeRequiresFulltext')}
            >
              {#if appState.analyzing}
                <i class="spinner fa-solid fa-rotate fa-spin"></i>
              {:else if !canAnalyze}
                <i class="fa-solid fa-circle-info btn-info-icon"></i>
              {/if}
              <span>{fnord.summary ? $_('articleView.reanalyze') : $_('articleView.analyze')}</span>
            </button>
          {/if}
          <button onclick={openInBrowser} class="btn btn-default">
            {$_('actions.openInBrowser')}
          </button>
        </div>
      </div>
    </div>

    <!-- Revision History Section with Diff -->
    {#if fnord.revision_count > 0 && revisions.length > 0}
      <div class="revision-section">
        <div class="section-content">
          <button class="revision-header" onclick={toggleRevisions}>
            <i class="revision-icon fa-solid {showRevisions ? 'fa-caret-down' : 'fa-caret-right'}"></i>
            <span class="revision-title">
              {$_('articleView.changes.revisions')} ({fnord.revision_count})
            </span>
          </button>

          {#if showRevisions}
            <div class="revision-detail">
              {#if loadingRevisions}
                <div class="revision-loading">{$_('articleList.loading')}</div>
              {:else}
                <RevisionView {fnord} {revisions} />
              {/if}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Greyface Alert (compact) -->
    {#if fnord.political_bias !== null || fnord.sachlichkeit !== null || fnord.quality_score !== null}
      <div class="greyface-section">
        <div class="section-content">
          <div class="greyface-row">
            <div class="greyface-label">
              {$_('articleView.greyface.title')}
            </div>
            <div class="greyface-indicators">
              {#if fnord.political_bias !== null}
                <span class="indicator bias-{getBiasColor(fnord.political_bias)}" title="{$_('articleView.greyface.bias')}: {getBiasLabel(fnord.political_bias)}">
                  <i class="fa-solid {getBiasIcon(fnord.political_bias)}"></i>
                  <span class="indicator-text">{getBiasLabel(fnord.political_bias)}</span>
                </span>
              {/if}
              {#if fnord.sachlichkeit !== null}
                <span class="indicator sach-{getSachlichkeitColor(fnord.sachlichkeit)}" title="{$_('articleView.greyface.sachlichkeit')}: {getSachlichkeitLabel(fnord.sachlichkeit)}">
                  <i class="fa-solid {getSachlichkeitIcon(fnord.sachlichkeit)}"></i>
                  <span class="indicator-text">{getSachlichkeitLabel(fnord.sachlichkeit)}</span>
                </span>
              {/if}
              {#if fnord.quality_score !== null}
                <span class="indicator quality" title="{$_('articleView.greyface.quality')}">
                  {#each Array(fnord.quality_score) as _, i (i)}<i class="fa-solid fa-star"></i>{/each}{#each Array(5 - fnord.quality_score) as _, i (i)}<i class="fa-regular fa-star"></i>{/each}
                </span>
              {/if}
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Summary (Discordian Analysis) - only shown if AI-processed -->
    {#if fnord.processed_at && fnord.summary}
      <div class="summary-section">
        <div class="section-content">
          <div class="section-header">
            {$_('terminology.discordian.term')}
          </div>
          <p class="summary-text">{stripHtml(fnord.summary)}</p>
        </div>
      </div>
    {/if}

    <!-- Statistical Preview (before LLM processing) -->
    <div class="section-content">
      <StatisticalPreview
        fnordId={fnord.id}
        hasContent={!!(fnord.content_full || fnord.content_raw)}
        isProcessed={!!fnord.processed_at}
      />
    </div>

    <!-- Sephiroth (Categories) & Immanentize (Keywords) - always show for category editing -->
    <div class="meta-section">
        <div class="section-content">
          <!-- Categories - always show to allow adding when none exist -->
          <div class="meta-row">
            <div class="meta-label">
              {$_('articleView.categories')}
            </div>
            <div class="meta-content">
              {#if articleCategoriesDetailed.length > 0}
                <ArticleCategories
                  fnordId={fnord.id}
                  categories={articleCategoriesDetailed}
                  editing={editingCategories}
                  onUpdate={handleCategoriesUpdate}
                />
              {:else if categories.length > 0}
                <!-- Fallback to old display for articles not yet loaded with detailed info -->
                <div class="category-badges">
                  {#each categories as cat (cat.sephiroth_id)}
                    <span class="category-badge" style="background-color: {getCategoryColorVar(cat.sephiroth_id)}; color: white">
                      {#if cat.icon}<i class="{cat.icon} badge-icon"></i>{/if}
                      {cat.name}
                    </span>
                  {/each}
                </div>
              {:else}
                <!-- No categories - show add option in edit mode -->
                <ArticleCategories
                  fnordId={fnord.id}
                  categories={[]}
                  editing={editingCategories}
                  onUpdate={handleCategoriesUpdate}
                />
              {/if}
              <button
                class="edit-toggle"
                onclick={() => editingCategories = !editingCategories}
                title="Edit categories"
                aria-label={editingCategories ? 'Done editing categories' : 'Edit categories'}
              >
                <i class="fa-solid {editingCategories ? 'fa-check' : 'fa-pen'}"></i>
              </button>
            </div>
          </div>

          {#if articleKeywords.length > 0 || tags.length > 0}
            <div class="meta-row">
              <div class="meta-label">
                {$_('articleView.keywords')}
              </div>
              <div class="meta-content">
                {#if articleKeywords.length > 0}
                  <ArticleKeywords
                    fnordId={fnord.id}
                    keywords={articleKeywords}
                    editing={editingKeywords}
                    onUpdate={handleKeywordsUpdate}
                  />
                {:else}
                  <!-- Fallback to old display -->
                  <div class="tag-list">
                    {#each tags as tag (tag.id)}
                      <button class="tag-badge clickable" onclick={() => navigateToKeyword(tag.id)} title={$_('network.title')}>
                        {tag.name}
                      </button>
                    {/each}
                  </div>
                {/if}
                <button
                  class="edit-toggle"
                  onclick={() => editingKeywords = !editingKeywords}
                  title="Edit keywords"
                  aria-label={editingKeywords ? 'Done editing keywords' : 'Edit keywords'}
                >
                  <i class="fa-solid {editingKeywords ? 'fa-check' : 'fa-pen'}"></i>
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>

    <!-- Content -->
    <div class="content-section">
      <div class="section-content">
        <article class="article-body">
          {#if fnord.content_full}
            {@html sanitizeArticleContent(fnord.content_full)}
          {:else if fnord.content_raw}
            {@html sanitizeArticleContent(fnord.content_raw)}
          {:else}
            <p class="no-content">
              {$_('articleView.noContent')}
            </p>
          {/if}
        </article>
      </div>
    </div>

    <!-- Similar Articles (below content) -->
    {#if similarArticles.length > 0 || loadingSimilar}
      <div class="similar-section">
        <div class="section-content">
          <div class="section-header">{$_('articleView.similarArticles')}</div>
          {#if loadingSimilar}
            <div class="similar-loading">{$_('articleList.loading')}</div>
          {:else}
            <div class="similar-list">
              {#each similarArticles as article (article.fnord_id)}
                <ArticleCard
                  fnord_id={article.fnord_id}
                  title={article.title}
                  pentacle_title={article.pentacle_title}
                  published_at={article.published_at}
                  categories={article.categories}
                  tags={article.tags}
                  similarity={article.similarity}
                  showScore={true}
                  showCategories={true}
                  showTags={true}
                  onclick={() => navigateToSimilarArticle(article.fnord_id)}
                />
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}
  {:else}
    <!-- Empty State -->
    <div class="empty-state">
      <i class="empty-icon fa-solid fa-eye"></i>
      <h2 class="empty-title">
        {$_('articleView.noSelection')}
      </h2>
      <p class="empty-text">
        {$_('articleView.selectArticle')}<br />
        {$_('articleView.useKeys')} <kbd>j</kbd> {$_('articleView.and')}
        <kbd>k</kbd> {$_('articleView.toNavigate')}<br />
        <kbd>s</kbd> {$_('articleView.favoriteHint')}
      </p>
    </div>
  {/if}
</div>

<style>
  .article-view {
    flex: 1;
    background-color: var(--bg-base);
    overflow-y: auto;
  }

  .article-header {
    padding: 1.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .header-content,
  .section-content {
    max-width: 48rem;
    margin: 0 auto;
  }

  .article-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--text-muted);
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
  }

  .source {
    font-weight: 500;
    color: var(--text-secondary);
  }

  .separator {
    color: var(--text-faint);
  }

  /* Content Status Badge */
  .content-status-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .content-status-full {
    background-color: var(--accent-success);
    color: var(--text-on-accent);
  }

  .content-status-rss {
    background-color: var(--accent-warning);
    color: var(--text-on-accent);
  }

  .content-status-missing {
    background-color: var(--accent-error);
    color: var(--text-on-accent);
  }

  .article-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.3;
    margin: 0 0 1rem 0;
  }

  .article-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .btn {
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
  }

  .btn-default {
    background-color: var(--bg-surface);
    color: var(--text-secondary);
  }

  .btn-default:hover {
    background-color: var(--bg-overlay);
  }

  .btn-golden {
    background-color: var(--golden-apple-color);
    color: var(--text-on-accent);
  }

  .btn-golden:hover {
    filter: brightness(1.1);
  }

  .btn.retrieving {
    opacity: 0.7;
    cursor: wait;
  }

  .btn.btn-disabled-info {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn.btn-disabled-info:hover {
    background-color: var(--bg-surface);
  }

  .btn-info-icon {
    color: var(--accent-info);
    margin-right: 0.25rem;
  }

  /* Fetch error state */
  .fetch-error-container {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .btn-error {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    background-color: var(--bg-surface);
    color: var(--accent-error);
    border: 1px solid var(--accent-error);
    cursor: default;
  }

  .btn-retry {
    padding: 0.375rem 0.625rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    background-color: transparent;
    color: var(--accent-info);
    border: 1px solid var(--accent-info);
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-retry:hover {
    background-color: var(--accent-info);
    color: var(--text-on-accent);
  }

  /* Unavailable state */
  .btn-unavailable {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-unavailable i {
    color: var(--text-muted);
    margin-right: 0.25rem;
  }

  .spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
    margin-right: 0.25rem;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .revision-section {
    padding: 0.5rem 1.5rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .revision-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    padding: 0.25rem 0;
    width: 100%;
    text-align: left;
  }

  .revision-header:hover {
    color: var(--text-primary);
  }

  .revision-icon {
    font-size: 0.75rem;
  }

  .revision-title {
    font-weight: 500;
  }

  .revision-loading {
    font-size: 0.875rem;
    color: var(--text-muted);
    padding: 0.5rem 0;
  }

  .revision-detail {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
  }

  .summary-section {
    padding: 1rem 1.5rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .section-header {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 0.75rem;
  }

  /* Compact Greyface Alert */
  .greyface-section {
    padding: 1rem 1.5rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .greyface-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .greyface-label {
    color: var(--text-muted);
    font-size: 0.75rem;
    min-width: 5rem;
    padding-top: 0.25rem;
    flex-shrink: 0;
  }

  .greyface-indicators {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    flex: 1;
  }

  .greyface-indicators .indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.8125rem;
    background-color: var(--bg-overlay);
    cursor: default;
  }

  .greyface-indicators .indicator i {
    font-size: 0.875rem;
  }

  .indicator-text {
    color: var(--text-secondary);
  }

  /* Bias colors (Theme-aware via CSS variables) */
  .indicator.bias-strong-left { color: var(--bias-strong-left); }
  .indicator.bias-lean-left { color: var(--bias-lean-left); }
  .indicator.bias-center { color: var(--bias-center); }
  .indicator.bias-lean-right { color: var(--bias-lean-right); }
  .indicator.bias-strong-right { color: var(--bias-strong-right); }
  .indicator.bias-neutral { color: var(--text-muted); }

  /* Sachlichkeit colors (Theme-aware via CSS variables) */
  .indicator.sach-emotional { color: var(--sach-emotional); }
  .indicator.sach-mixed { color: var(--sach-mixed); }
  .indicator.sach-objective { color: var(--sach-objective); }
  .indicator.sach-neutral { color: var(--text-muted); }

  /* Quality stars */
  .indicator.quality {
    color: var(--golden-apple-color);
    gap: 0.125rem;
  }

  .summary-text {
    font-size: 0.875rem;
    color: var(--text-primary);
    line-height: 1.6;
    margin: 0;
  }

  /* Sephiroth (Categories) & Immanentize (Tags) */
  .meta-section {
    padding: 1rem 1.5rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .meta-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .meta-row:last-child {
    margin-bottom: 0;
  }

  .meta-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    min-width: 5rem;
    padding-top: 0.25rem;
  }

  .meta-content {
    flex: 1;
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .edit-toggle {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0.25rem;
    font-size: 0.75rem;
    opacity: 0.5;
    transition: opacity 0.2s;
    flex-shrink: 0;
  }

  .edit-toggle:hover {
    opacity: 1;
    color: var(--accent-primary);
  }

  .category-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .category-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.625rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .badge-icon {
    font-size: 0.875rem;
  }

  .tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .tag-badge {
    display: inline-block;
    padding: 0.125rem 0.5rem;
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
    border-radius: 0.25rem;
    font-size: 0.75rem;
    border: none;
  }

  .tag-badge.clickable {
    cursor: pointer;
    transition: all 0.2s;
  }

  .tag-badge.clickable:hover {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  /* Similar Articles Section */
  .similar-section {
    padding: 1.5rem;
    background-color: var(--bg-surface);
    border-top: 1px solid var(--border-default);
    margin-top: 1rem;
  }

  .similar-loading {
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  .similar-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  /* ===========================================
     Content Section - Mobile First
     =========================================== */
  .content-section {
    padding: 1rem;
  }

  @media (min-width: 640px) {
    .content-section {
      padding: 1.5rem;
    }
  }

  /* ===========================================
     Article Body - Base Typography
     =========================================== */
  .article-body {
    color: var(--text-primary);
    line-height: 1.75;
    font-size: 1rem;
    word-wrap: break-word;
    overflow-wrap: break-word;
    hyphens: auto;
  }

  @media (min-width: 640px) {
    .article-body {
      font-size: 1.0625rem;
      line-height: 1.8;
    }
  }

  /* First element should not have top margin */
  .article-body :global(> *:first-child) {
    margin-top: 0;
  }

  /* ===========================================
     Headings - h1 to h6
     =========================================== */
  .article-body :global(h1) {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-top: 2rem;
    margin-bottom: 1rem;
    line-height: 1.3;
    border-bottom: 1px solid var(--border-muted);
    padding-bottom: 0.5rem;
  }

  .article-body :global(h2) {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-top: 1.75rem;
    margin-bottom: 0.875rem;
    line-height: 1.35;
  }

  .article-body :global(h3) {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
    line-height: 1.4;
  }

  .article-body :global(h4) {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-top: 1.25rem;
    margin-bottom: 0.625rem;
    line-height: 1.4;
  }

  .article-body :global(h5) {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .article-body :global(h6) {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-muted);
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* ===========================================
     Text Formatting - Bold, Italic, etc.
     =========================================== */
  .article-body :global(strong),
  .article-body :global(b) {
    font-weight: 600;
    color: var(--text-primary);
  }

  .article-body :global(em),
  .article-body :global(i) {
    font-style: italic;
  }

  .article-body :global(u) {
    text-decoration: underline;
    text-decoration-color: var(--accent-primary);
    text-underline-offset: 2px;
  }

  .article-body :global(s),
  .article-body :global(del),
  .article-body :global(strike) {
    text-decoration: line-through;
    color: var(--text-muted);
  }

  .article-body :global(ins) {
    text-decoration: none;
    background-color: var(--diff-added-bg);
    padding: 0 0.125rem;
    border-radius: 2px;
  }

  .article-body :global(mark) {
    background-color: var(--golden-apple-color);
    color: var(--text-on-accent);
    padding: 0.0625rem 0.25rem;
    border-radius: 2px;
  }

  .article-body :global(small) {
    font-size: 0.875em;
    color: var(--text-secondary);
  }

  .article-body :global(sub),
  .article-body :global(sup) {
    font-size: 0.75em;
    line-height: 0;
    position: relative;
    vertical-align: baseline;
  }

  .article-body :global(sup) {
    top: -0.5em;
  }

  .article-body :global(sub) {
    bottom: -0.25em;
  }

  .article-body :global(abbr[title]) {
    text-decoration: underline dotted;
    text-decoration-color: var(--text-muted);
    cursor: help;
  }

  .article-body :global(cite) {
    font-style: italic;
    color: var(--text-secondary);
  }

  /* ===========================================
     Paragraphs
     =========================================== */
  .article-body :global(p) {
    margin: 0 0 1.25rem 0;
  }

  .article-body :global(p:last-child) {
    margin-bottom: 0;
  }

  /* ===========================================
     Links
     =========================================== */
  .article-body :global(a) {
    color: var(--accent-info);
    text-decoration: none;
    transition: color 0.15s ease;
  }

  .article-body :global(a:hover) {
    color: var(--accent-primary);
    text-decoration: underline;
  }

  .article-body :global(a:visited) {
    color: var(--accent-secondary);
  }

  /* External links indicator */
  .article-body :global(a[target="_blank"])::after {
    content: " \2197";
    font-size: 0.75em;
    color: var(--text-muted);
  }

  /* ===========================================
     Lists - Unordered, Ordered, Definition
     =========================================== */
  .article-body :global(ul),
  .article-body :global(ol) {
    margin: 1rem 0 1.25rem 0;
    padding-left: 1.5rem;
  }

  .article-body :global(ul) {
    list-style-type: disc;
  }

  .article-body :global(ol) {
    list-style-type: decimal;
  }

  .article-body :global(li) {
    margin: 0.375rem 0;
    padding-left: 0.25rem;
  }

  .article-body :global(li > p) {
    margin-bottom: 0.5rem;
  }

  /* Nested lists */
  .article-body :global(ul ul),
  .article-body :global(ol ul) {
    list-style-type: circle;
    margin: 0.375rem 0;
  }

  .article-body :global(ul ul ul),
  .article-body :global(ol ul ul) {
    list-style-type: square;
  }

  .article-body :global(ol ol),
  .article-body :global(ul ol) {
    list-style-type: lower-alpha;
    margin: 0.375rem 0;
  }

  /* Definition lists */
  .article-body :global(dl) {
    margin: 1rem 0 1.25rem 0;
  }

  .article-body :global(dt) {
    font-weight: 600;
    color: var(--text-primary);
    margin-top: 0.75rem;
  }

  .article-body :global(dt:first-child) {
    margin-top: 0;
  }

  .article-body :global(dd) {
    margin-left: 1.5rem;
    margin-top: 0.25rem;
    color: var(--text-secondary);
  }

  /* ===========================================
     Blockquotes
     =========================================== */
  .article-body :global(blockquote) {
    margin: 1.5rem 0;
    padding: 0.75rem 1rem 0.75rem 1.25rem;
    border-left: 4px solid var(--accent-primary);
    background-color: var(--bg-surface);
    border-radius: 0 0.375rem 0.375rem 0;
    color: var(--text-secondary);
    font-style: italic;
  }

  .article-body :global(blockquote p) {
    margin-bottom: 0.75rem;
  }

  .article-body :global(blockquote p:last-child) {
    margin-bottom: 0;
  }

  /* Nested blockquotes */
  .article-body :global(blockquote blockquote) {
    margin: 0.75rem 0;
    border-left-color: var(--accent-secondary);
  }

  .article-body :global(blockquote cite) {
    display: block;
    margin-top: 0.75rem;
    font-size: 0.875rem;
    color: var(--text-muted);
    font-style: normal;
  }

  .article-body :global(blockquote cite::before) {
    content: "\2014 ";
  }

  /* ===========================================
     Code - Inline and Blocks
     =========================================== */
  .article-body :global(code) {
    font-family: 'SF Mono', 'Fira Code', 'Consolas', 'Monaco', monospace;
    font-size: 0.875em;
    background-color: var(--bg-overlay);
    color: var(--accent-warning);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    word-break: break-word;
  }

  .article-body :global(pre) {
    margin: 1.25rem 0;
    padding: 1rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    border-radius: 0.5rem;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
  }

  .article-body :global(pre code) {
    background: none;
    padding: 0;
    font-size: 0.8125rem;
    color: var(--text-primary);
    line-height: 1.6;
    word-break: normal;
  }

  .article-body :global(kbd) {
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 0.8125em;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    padding: 0.125rem 0.375rem;
    box-shadow: 0 1px 0 var(--border-default);
  }

  .article-body :global(samp) {
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 0.875em;
    color: var(--accent-success);
  }

  .article-body :global(var) {
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-style: italic;
    color: var(--accent-info);
  }

  /* ===========================================
     Images and Figures
     =========================================== */
  .article-body :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 0.5rem;
    display: block;
    margin: 1.25rem auto;
    background-color: var(--bg-surface);
  }

  .article-body :global(figure) {
    margin: 1.5rem 0;
    padding: 0;
    text-align: center;
  }

  .article-body :global(figure img) {
    margin: 0 auto;
  }

  .article-body :global(figcaption) {
    margin-top: 0.75rem;
    font-size: 0.875rem;
    color: var(--text-muted);
    font-style: italic;
    line-height: 1.5;
    text-align: center;
    padding: 0 1rem;
  }

  /* ===========================================
     Tables
     =========================================== */
  .article-body :global(table) {
    width: 100%;
    margin: 1.25rem 0;
    border-collapse: collapse;
    font-size: 0.9375rem;
    overflow-x: auto;
    display: block;
  }

  @media (min-width: 640px) {
    .article-body :global(table) {
      display: table;
    }
  }

  .article-body :global(thead) {
    background-color: var(--bg-surface);
  }

  .article-body :global(th) {
    font-weight: 600;
    color: var(--text-primary);
    text-align: left;
    padding: 0.75rem;
    border-bottom: 2px solid var(--border-default);
  }

  .article-body :global(td) {
    padding: 0.625rem 0.75rem;
    border-bottom: 1px solid var(--border-muted);
    color: var(--text-secondary);
  }

  .article-body :global(tr:last-child td) {
    border-bottom: none;
  }

  .article-body :global(tbody tr:hover) {
    background-color: var(--bg-overlay);
  }

  .article-body :global(caption) {
    padding: 0.75rem;
    font-size: 0.875rem;
    color: var(--text-muted);
    caption-side: bottom;
    text-align: left;
  }

  /* ===========================================
     Horizontal Rules
     =========================================== */
  .article-body :global(hr) {
    margin: 2rem 0;
    border: none;
    border-top: 1px solid var(--border-default);
  }

  /* ===========================================
     Details/Summary (Collapsible)
     =========================================== */
  .article-body :global(details) {
    margin: 1rem 0;
    padding: 0.75rem 1rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border: 1px solid var(--border-muted);
  }

  .article-body :global(summary) {
    cursor: pointer;
    font-weight: 500;
    color: var(--text-primary);
    padding: 0.25rem 0;
  }

  .article-body :global(summary:hover) {
    color: var(--accent-primary);
  }

  .article-body :global(details[open] > summary) {
    margin-bottom: 0.75rem;
    border-bottom: 1px solid var(--border-muted);
    padding-bottom: 0.5rem;
  }

  /* ===========================================
     Time element
     =========================================== */
  .article-body :global(time) {
    color: var(--text-muted);
    font-size: 0.875em;
  }

  /* ===========================================
     No Content State
     =========================================== */
  .no-content {
    color: var(--text-muted);
    font-style: italic;
    margin: 0;
    padding: 2rem;
    text-align: center;
  }

  /* ===========================================
     Print Styles
     =========================================== */
  @media print {
    .article-view {
      background-color: white;
      color: black;
    }

    .article-header {
      border-bottom: 1px solid #ccc;
    }

    .article-actions,
    .greyface-section,
    .revision-section,
    .meta-section,
    .similar-section,
    .edit-toggle {
      display: none !important;
    }

    .header-content,
    .section-content {
      max-width: 100%;
    }

    .article-body {
      font-size: 11pt;
      line-height: 1.6;
      color: black;
    }

    .article-body :global(a) {
      color: black;
      text-decoration: underline;
    }

    .article-body :global(a[target="_blank"])::after {
      content: " (" attr(href) ")";
      font-size: 9pt;
      color: #666;
    }

    .article-body :global(img) {
      max-width: 100%;
      page-break-inside: avoid;
    }

    .article-body :global(pre),
    .article-body :global(blockquote) {
      page-break-inside: avoid;
      border-color: #ccc;
    }

    .article-body :global(h1),
    .article-body :global(h2),
    .article-body :global(h3),
    .article-body :global(h4) {
      page-break-after: avoid;
      color: black;
    }

    .article-body :global(table) {
      border: 1px solid #ccc;
    }

    .article-body :global(th),
    .article-body :global(td) {
      border: 1px solid #ccc;
    }
  }

  .empty-state {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    padding: 2rem;
  }

  .empty-icon {
    font-size: 4rem;
    color: var(--accent-primary);
    margin-bottom: 1rem;
  }

  .empty-title {
    font-size: 1.25rem;
    font-weight: 500;
    margin: 0 0 0.5rem 0;
  }

  .empty-text {
    font-size: 0.875rem;
    text-align: center;
    max-width: 20rem;
    margin: 0;
  }

  kbd {
    background-color: var(--bg-surface);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-family: inherit;
  }
</style>
