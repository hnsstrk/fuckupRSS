<script lang="ts">
  import { _, locale } from "svelte-i18n";
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-shell";
  import {
    appState,
    toasts,
    type FnordRevision,
    type ArticleCategory,
    type Tag,
    type SimilarArticle,
  } from "../stores/state.svelte";
  import type { ArticleKeyword, ArticleCategoryDetailed } from "$lib/types";
  import RevisionView from "./RevisionView.svelte";
  import { ArticleCard } from "./article";
  import StatisticalPreview from "./article/StatisticalPreview.svelte";
  import { formatFullDate } from "$lib/utils/articleFormat";
  import ArticleGreyfaceAlert from "./ArticleGreyfaceAlert.svelte";
  import ArticleMetaSection from "./ArticleMetaSection.svelte";
  import ArticleContent from "./ArticleContent.svelte";

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

    if (!mounted || appState.selectedFnord?.id !== fnordId) return;

    // Load categories and tags (both old format and new detailed format)
    try {
      const [cats, tgs, kwds, catsDetailed] = await Promise.all([
        appState.getArticleCategories(fnordId),
        appState.getArticleTags(fnordId),
        invoke<ArticleKeyword[]>("get_article_keywords", { fnordId }),
        invoke<ArticleCategoryDetailed[]>("get_article_categories_detailed", { fnordId }),
      ]);
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

    if (!mounted || appState.selectedFnord?.id !== fnordId) return;

    // Load similar articles (only if article was processed and has embedding)
    if (hasEmbedding) {
      loadingSimilar = true;
      try {
        const similar = await appState.findSimilarArticles(fnordId, 5);
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

  // Combined effect for article changes
  $effect(() => {
    const fnord = appState.selectedFnord;

    if (!fnord) {
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

    if (fnord.has_changes) {
      appState.acknowledgeChanges(fnord.id);
    }

    if (fnord.id !== lastLoadedFnordId) {
      lastLoadedFnordId = fnord.id;
      const hasEmbedding = fnord.processed_at !== null;
      loadArticleData(fnord.id, fnord.revision_count, hasEmbedding);
    }
  });

  // Listen for batch-complete event to refresh similar articles
  async function handleBatchComplete() {
    const fnord = appState.selectedFnord;
    if (!fnord || !fnord.processed_at || !mounted) return;

    const fnordId = fnord.id;
    try {
      const [cats, tgs, similar] = await Promise.all([
        appState.getArticleCategories(fnordId),
        appState.getArticleTags(fnordId),
        appState.findSimilarArticles(fnordId, 5),
      ]);
      if (!mounted || appState.selectedFnord?.id !== fnordId) return;
      categories = cats;
      tags = tgs;
      similarArticles = similar;
    } catch {
      // Ignore errors during refresh
    }
  }

  // Handler for keywords update
  async function handleKeywordsUpdate(updatedKeywords: ArticleKeyword[]) {
    if (!mounted) return;
    articleKeywords = updatedKeywords;
    const fnord = appState.selectedFnord;
    if (fnord) {
      const fnordId = fnord.id;
      const newTags = await appState.getArticleTags(fnordId);
      if (!mounted || appState.selectedFnord?.id !== fnordId) return;
      tags = newTags;
    }
  }

  // Handler for categories update
  async function handleCategoriesUpdate(updatedCategories: ArticleCategoryDetailed[]) {
    if (!mounted) return;
    articleCategoriesDetailed = updatedCategories;
    const fnord = appState.selectedFnord;
    if (fnord) {
      const fnordId = fnord.id;
      const newCats = await appState.getArticleCategories(fnordId);
      if (!mounted || appState.selectedFnord?.id !== fnordId) return;
      categories = newCats;
    }
  }

  onMount(() => {
    window.addEventListener("batch-complete", handleBatchComplete);
  });

  onDestroy(() => {
    window.removeEventListener("batch-complete", handleBatchComplete);
  });

  // stripHtml uses DOMParser to safely extract text from HTML
  // DOMParser does not execute scripts and textContent is read-only extraction
  function stripHtml(html: string): string {
    const parser = new DOMParser();
    const doc = parser.parseFromString(html, "text/html");
    return doc.body.textContent || "";
  }

  async function openInBrowser() {
    if (appState.selectedFnord) {
      await open(appState.selectedFnord.url);
    }
  }

  // Get specific error message based on error content
  function getSpecificFetchError(error: string): string {
    const errorLower = error.toLowerCase();
    if (errorLower.includes("404") || errorLower.includes("not found")) {
      return $_("toast.fetchErrorNotFound");
    }
    if (errorLower.includes("403") || errorLower.includes("forbidden")) {
      return $_("toast.fetchErrorBlocked");
    }
    if (errorLower.includes("timeout") || errorLower.includes("timed out")) {
      return $_("toast.fetchErrorTimeout");
    }
    if (
      errorLower.includes("network") ||
      errorLower.includes("connection") ||
      errorLower.includes("unreachable")
    ) {
      return $_("toast.fetchErrorNetwork");
    }
    if (errorLower.includes("paywall") || errorLower.includes("subscription")) {
      return $_("toast.fetchErrorPaywall");
    }
    const serverErrorMatch = error.match(/5\d{2}/);
    if (serverErrorMatch) {
      return $_("toast.fetchErrorServerError", { values: { code: serverErrorMatch[0] } });
    }
    return $_("toast.fetchError", { values: { error } });
  }

  async function fetchFullContent() {
    const fnord = appState.selectedFnord;
    if (!fnord || !mounted) return;

    const fnordId = fnord.id;
    try {
      const result = await appState.fetchFullContent(fnordId);
      if (!mounted) return;

      if (result?.success) {
        toasts.success($_("toast.fetchSuccess"));
      } else if (result?.error) {
        toasts.error(getSpecificFetchError(result.error));
      } else if (appState.error) {
        toasts.error(getSpecificFetchError(appState.error));
      }
    } catch (e) {
      console.error("Fetch full content failed:", e);
      if (mounted) {
        toasts.error(getSpecificFetchError(String(e)));
      }
    }
  }

  async function analyzeWithAI() {
    const fnord = appState.selectedFnord;
    if (!fnord || !appState.ollamaStatus.available || !mounted) return;

    if (!appState.selectedModel) {
      toasts.error(
        $_("toast.analyzeError", {
          values: { error: "No AI model selected. Please configure a model in Settings." },
        }),
      );
      return;
    }

    const fnordId = fnord.id;
    try {
      const result = await appState.processArticleDiscordian(fnordId);
      if (!mounted) return;

      if (result?.success) {
        if (appState.selectedFnord?.id !== fnordId) return;

        const [cats, tgs, similar] = await Promise.all([
          appState.getArticleCategories(fnordId),
          appState.getArticleTags(fnordId),
          appState.findSimilarArticles(fnordId, 5),
        ]);

        if (!mounted || appState.selectedFnord?.id !== fnordId) return;

        categories = cats;
        tags = tgs;
        similarArticles = similar;
        toasts.success($_("toast.analyzeSuccess"));
      } else if (result?.error) {
        if (mounted) toasts.error($_("toast.analyzeError", { values: { error: result.error } }));
      } else if (result === null) {
        if (mounted)
          toasts.error(
            $_("toast.analyzeError", {
              values: { error: "Analysis failed to start. Please try again." },
            }),
          );
      } else if (appState.error) {
        if (mounted) toasts.error($_("toast.analyzeError", { values: { error: appState.error } }));
      }
    } catch (e) {
      console.error("AI analysis failed:", e);
      if (mounted) {
        toasts.error($_("toast.analyzeError", { values: { error: String(e) } }));
      }
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "v" && appState.selectedFnord) {
      e.preventDefault();
      openInBrowser();
    }
    if (e.key === "r" && appState.selectedFnord && !appState.selectedFnord.content_full) {
      e.preventDefault();
      fetchFullContent();
    }
  }

  function navigateToKeyword(tagId: number) {
    window.dispatchEvent(new CustomEvent("navigate-to-network", { detail: { keywordId: tagId } }));
  }

  function navigateToSimilarArticle(fnordId: number) {
    window.dispatchEvent(
      new CustomEvent("navigate-to-article", { detail: { articleId: fnordId } }),
    );
    const articleView = document.querySelector(".article-view");
    if (articleView) {
      articleView.scrollTo({ top: 0, behavior: "smooth" });
    }
  }

  function hasContentForAnalysis(fnord: typeof appState.selectedFnord): boolean {
    if (!fnord) return false;
    const content = fnord.content_full;
    return !!content && content.length >= 100;
  }

  type ContentStatus = "full" | "rss" | "missing";
  function getContentStatus(fnord: typeof appState.selectedFnord): ContentStatus {
    if (!fnord) return "missing";
    if (fnord.content_full && fnord.content_full.length > 500) {
      return "full";
    }
    if (fnord.content_raw) {
      return "rss";
    }
    return "missing";
  }

  function getContentStatusLabel(status: ContentStatus): string {
    switch (status) {
      case "full":
        return $_("articleView.contentStatus.full");
      case "rss":
        return $_("articleView.contentStatus.rss");
      case "missing":
        return $_("articleView.contentStatus.missing");
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
          <span>{formatFullDate(fnord.published_at, $locale || "de")}</span>
          {#if fnord.author}
            <span class="separator">·</span>
            <span>{$_("articleView.by")} {fnord.author}</span>
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
            <i
              class="btn-icon {fnord.status === 'golden_apple'
                ? 'fa-solid fa-apple-whole'
                : 'fa-regular fa-star'}"
            ></i>
            <span
              >{fnord.status === "golden_apple"
                ? $_("terminology.golden_apple.term")
                : $_("actions.favorite")}</span
            >
          </button>
          {#if !fnord.content_full}
            {@const hasFetchError = fnord.full_text_fetch_error}
            {#if appState.retrieving}
              <button
                class="btn btn-default retrieving"
                disabled
                title={$_("articleView.fetchFetching")}
              >
                <i class="spinner fa-solid fa-rotate fa-spin"></i>
                <span>{$_("articleView.fetchFetching")}</span>
              </button>
            {:else if hasFetchError}
              <div class="fetch-error-container">
                <span class="btn btn-error" title={hasFetchError}>
                  <i class="fa-solid fa-triangle-exclamation"></i>
                  <span>{$_("articleView.fetchError")}</span>
                </span>
                <button
                  onclick={fetchFullContent}
                  class="btn-retry"
                  title={$_("articleView.fetchFullText")}
                >
                  {$_("articleView.fetchRetry")}
                </button>
              </div>
            {:else}
              <button
                onclick={fetchFullContent}
                class="btn btn-default"
                title="{$_('articleView.fetchFullText')} (r)"
              >
                <span>{$_("articleView.fullText")}</span>
              </button>
            {/if}
          {/if}
          {#if appState.ollamaStatus.available}
            {@const canAnalyze = hasContentForAnalysis(fnord)}
            {@const hasModel = !!appState.selectedModel}
            {@const isDisabled = appState.analyzing || !canAnalyze || !hasModel}
            <button
              onclick={analyzeWithAI}
              class="btn btn-default {appState.analyzing ? 'retrieving' : ''} {!canAnalyze ||
              !hasModel
                ? 'btn-disabled-info'
                : ''}"
              disabled={isDisabled}
              title={!hasModel
                ? $_("articleView.analyzeNoModel")
                : canAnalyze
                  ? $_("articleView.aiAnalysis")
                  : $_("articleView.analyzeRequiresFulltext")}
            >
              {#if appState.analyzing}
                <i class="spinner fa-solid fa-rotate fa-spin"></i>
              {:else if !canAnalyze || !hasModel}
                <i class="fa-solid fa-circle-info btn-info-icon"></i>
              {/if}
              <span>{fnord.summary ? $_("articleView.reanalyze") : $_("articleView.analyze")}</span>
            </button>
          {/if}
          <button onclick={openInBrowser} class="btn btn-default">
            {$_("actions.openInBrowser")}
          </button>
        </div>
      </div>
    </div>

    <!-- Revision History Section with Diff -->
    {#if fnord.revision_count > 0 && revisions.length > 0}
      <div class="revision-section">
        <div class="section-content">
          <button class="revision-header" onclick={toggleRevisions}>
            <i class="revision-icon fa-solid {showRevisions ? 'fa-caret-down' : 'fa-caret-right'}"
            ></i>
            <span class="revision-title">
              {$_("articleView.changes.revisions")} ({fnord.revision_count})
            </span>
          </button>

          {#if showRevisions}
            <div class="revision-detail">
              {#if loadingRevisions}
                <div class="revision-loading">{$_("articleList.loading")}</div>
              {:else}
                <RevisionView {fnord} {revisions} />
              {/if}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Greyface Alert (compact) -->
    <ArticleGreyfaceAlert
      politicalBias={fnord.political_bias}
      sachlichkeit={fnord.sachlichkeit}
      qualityScore={fnord.quality_score}
    />

    <!-- Summary (Discordian Analysis) -->
    {#if fnord.processed_at && fnord.summary}
      <div class="summary-section">
        <div class="section-content">
          <div class="section-header">
            {$_("terminology.discordian.term")}
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

    <!-- Sephiroth (Categories) & Immanentize (Keywords) -->
    <ArticleMetaSection
      fnordId={fnord.id}
      {categories}
      {tags}
      {articleKeywords}
      {articleCategoriesDetailed}
      {editingKeywords}
      {editingCategories}
      onKeywordsUpdate={handleKeywordsUpdate}
      onCategoriesUpdate={handleCategoriesUpdate}
      onToggleEditingKeywords={() => (editingKeywords = !editingKeywords)}
      onToggleEditingCategories={() => (editingCategories = !editingCategories)}
      onNavigateToKeyword={navigateToKeyword}
    />

    <!-- Content -->
    <ArticleContent contentFull={fnord.content_full} contentRaw={fnord.content_raw} />

    <!-- Similar Articles (below content) -->
    {#if similarArticles.length > 0 || loadingSimilar}
      <div class="similar-section">
        <div class="section-content">
          <div class="section-header">{$_("articleView.similarArticles")}</div>
          {#if loadingSimilar}
            <div class="similar-loading">{$_("articleList.loading")}</div>
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
        {$_("articleView.noSelection")}
      </h2>
      <p class="empty-text">
        {$_("articleView.selectArticle")}<br />
        {$_("articleView.useKeys")} <kbd>j</kbd>
        {$_("articleView.and")}
        <kbd>k</kbd>
        {$_("articleView.toNavigate")}<br />
        <kbd>s</kbd>
        {$_("articleView.favoriteHint")}
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

  .spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
    margin-right: 0.25rem;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
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

  .summary-text {
    font-size: 0.875rem;
    color: var(--text-primary);
    line-height: 1.6;
    margin: 0;
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

  @media print {
    .article-actions,
    .revision-section,
    .similar-section {
      display: none !important;
    }

    .header-content,
    .section-content {
      max-width: 100%;
    }
  }
</style>
