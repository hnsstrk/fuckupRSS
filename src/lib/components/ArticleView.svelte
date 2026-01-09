<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { appState, toasts, type FnordRevision, type ArticleCategory, type Tag } from "../stores/state.svelte";
  import Tooltip from "./Tooltip.svelte";
  import RevisionView from "./RevisionView.svelte";

  let showRevisions = $state(false);
  let revisions = $state<FnordRevision[]>([]);
  let loadingRevisions = $state(false);

  // Categories and Tags
  let categories = $state<ArticleCategory[]>([]);
  let tags = $state<Tag[]>([]);

  // Track the last loaded article to prevent redundant fetches
  let lastLoadedFnordId = $state<number | null>(null);

  async function loadArticleData(fnordId: number, revisionCount: number) {
    // Load revisions if available
    if (revisionCount > 0) {
      loadingRevisions = true;
      try {
        revisions = await appState.getRevisions(fnordId);
      } catch {
        revisions = [];
      } finally {
        loadingRevisions = false;
      }
    } else {
      revisions = [];
    }

    // Load categories and tags
    try {
      const [cats, tgs] = await Promise.all([
        appState.getArticleCategories(fnordId),
        appState.getArticleTags(fnordId)
      ]);
      categories = cats;
      tags = tgs;
    } catch {
      categories = [];
      tags = [];
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
      loadArticleData(fnord.id, fnord.revision_count);
    }
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

  function getArticleTypeName(type: string | null): string {
    if (!type) return $_('articleView.unknown');
    return $_(`articleType.${type}`);
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

  function openInBrowser() {
    if (appState.selectedFnord) {
      window.open(appState.selectedFnord.url, "_blank");
    }
  }

  async function fetchFullContent() {
    if (appState.selectedFnord) {
      const result = await appState.fetchFullContent(appState.selectedFnord.id);
      if (result?.success) {
        toasts.success($_('toast.fetchSuccess'));
      } else if (result?.error) {
        toasts.error($_('toast.fetchError', { values: { error: result.error }}));
      } else if (appState.error) {
        toasts.error($_('toast.fetchError', { values: { error: appState.error }}));
      }
    }
  }

  async function analyzeWithAI() {
    if (appState.selectedFnord && appState.ollamaStatus.available) {
      const result = await appState.processArticleDiscordian(appState.selectedFnord.id);
      if (result?.success) {
        // Reload categories and tags after analysis
        categories = await appState.getArticleCategories(appState.selectedFnord.id);
        tags = await appState.getArticleTags(appState.selectedFnord.id);
        toasts.success($_('toast.analyzeSuccess'));
      } else if (result?.error) {
        toasts.error($_('toast.analyzeError', { values: { error: result.error }}));
      } else if (appState.error) {
        toasts.error($_('toast.analyzeError', { values: { error: appState.error }}));
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
        </div>

        <h1 class="article-title">{fnord.title}</h1>

        <div class="article-actions">
          <button
            onclick={() => appState.toggleGoldenApple(fnord.id)}
            class="btn {fnord.status === 'golden_apple' ? 'btn-golden' : 'btn-default'}"
          >
            <Tooltip termKey="golden_apple">
              <span class="btn-icon">{fnord.status === 'golden_apple' ? '✦' : '✧'}</span>
              <span>{fnord.status === "golden_apple" ? $_('terminology.golden_apple.term') : $_('actions.favorite')}</span>
            </Tooltip>
          </button>
          {#if !fnord.content_full}
            <button
              onclick={fetchFullContent}
              class="btn btn-default {appState.retrieving ? 'retrieving' : ''}"
              disabled={appState.retrieving}
              title="{$_('articleView.fetchFullText')} (r)"
            >
              {#if appState.retrieving}
                <span class="spinner">⟳</span>
              {/if}
              <Tooltip termKey="hagbard">
                <span>{$_('articleView.fullText')}</span>
              </Tooltip>
            </button>
          {/if}
          {#if appState.ollamaStatus.available}
            <button
              onclick={analyzeWithAI}
              class="btn btn-default {appState.analyzing ? 'retrieving' : ''}"
              disabled={appState.analyzing}
              title={$_('articleView.aiAnalysis')}
            >
              {#if appState.analyzing}
                <span class="spinner">⟳</span>
              {/if}
              <Tooltip termKey="discordian">
                <span>{fnord.summary ? $_('articleView.reanalyze') : $_('articleView.analyze')}</span>
              </Tooltip>
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
            <span class="revision-icon">{showRevisions ? '▼' : '▶'}</span>
            <span class="revision-title">
              <Tooltip termKey="fnord">{$_('articleView.changes.revisions')}</Tooltip> ({fnord.revision_count})
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

    <!-- Greyface Alert -->
    {#if fnord.political_bias !== null || fnord.sachlichkeit !== null || fnord.article_type}
      <div class="greyface-section">
        <div class="section-content">
          <div class="section-header">
            <Tooltip termKey="greyface">{$_('articleView.greyface.title')}</Tooltip>
          </div>
          <div class="greyface-grid">
            {#if fnord.article_type}
              <div class="greyface-item">
                <div class="item-label">{$_('articleView.articleType')}</div>
                <div class="item-value">{getArticleTypeName(fnord.article_type)}</div>
              </div>
            {/if}
            {#if fnord.political_bias !== null}
              <div class="greyface-item">
                <div class="item-label">{$_('articleView.greyface.bias')}</div>
                <div class="item-value">{getBiasLabel(fnord.political_bias)}</div>
              </div>
            {/if}
            {#if fnord.sachlichkeit !== null}
              <div class="greyface-item">
                <div class="item-label">{$_('articleView.greyface.sachlichkeit')}</div>
                <div class="item-value">{getSachlichkeitLabel(fnord.sachlichkeit)}</div>
              </div>
            {/if}
            {#if fnord.quality_score !== null}
              <div class="greyface-item">
                <div class="item-label">{$_('articleView.greyface.quality')}</div>
                <div class="item-value quality">{"★".repeat(fnord.quality_score)}{"☆".repeat(5 - fnord.quality_score)}</div>
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/if}

    <!-- Summary (Discordian Analysis) - only shown if AI-processed -->
    {#if fnord.processed_at && fnord.summary}
      <div class="summary-section">
        <div class="section-content">
          <div class="section-header">
            <Tooltip termKey="discordian">{$_('terminology.discordian.term')}</Tooltip>
          </div>
          <p class="summary-text">{stripHtml(fnord.summary)}</p>
        </div>
      </div>
    {/if}

    <!-- Sephiroth (Categories) & Immanentize (Tags) -->
    {#if categories.length > 0 || tags.length > 0}
      <div class="meta-section">
        <div class="section-content">
          {#if categories.length > 0}
            <div class="meta-row">
              <div class="meta-label">
                <Tooltip termKey="sephiroth">{$_('articleView.categories')}</Tooltip>
              </div>
              <div class="category-badges">
                {#each categories as cat}
                  <span
                    class="category-badge"
                    style="background-color: {cat.color || 'var(--bg-overlay)'}; color: {cat.color ? 'white' : 'var(--text-primary)'}"
                  >
                    {#if cat.icon}<span class="badge-icon">{cat.icon}</span>{/if}
                    {cat.name}
                  </span>
                {/each}
              </div>
            </div>
          {/if}

          {#if tags.length > 0}
            <div class="meta-row">
              <div class="meta-label">
                <Tooltip termKey="immanentize">{$_('articleView.keywords')}</Tooltip>
              </div>
              <div class="tag-list">
                {#each tags as tag}
                  <button
                    class="tag-badge clickable"
                    onclick={() => navigateToKeyword(tag.id)}
                    title={$_('network.title')}
                  >
                    {tag.name}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Content -->
    <div class="content-section">
      <div class="section-content">
        <article class="article-body">
          {#if fnord.content_full}
            {@html fnord.content_full}
          {:else if fnord.content_raw}
            {@html fnord.content_raw}
          {:else}
            <p class="no-content">
              {$_('articleView.noContent')}
            </p>
          {/if}
        </article>
      </div>
    </div>
  {:else}
    <!-- Empty State -->
    <div class="empty-state">
      <div class="empty-icon">▲</div>
      <h2 class="empty-title">
        <Tooltip termKey="fnord">{$_('articleView.noSelection')}</Tooltip>
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

  .greyface-section,
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

  .greyface-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
    gap: 1rem;
  }

  .greyface-item {
    font-size: 0.875rem;
  }

  .item-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-bottom: 0.25rem;
  }

  .item-value {
    color: var(--text-primary);
  }

  .item-value.quality {
    color: var(--golden-apple-color);
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

  .content-section {
    padding: 1.5rem;
  }

  .article-body {
    color: var(--text-primary);
    line-height: 1.7;
  }

  .article-body :global(h1),
  .article-body :global(h2),
  .article-body :global(h3),
  .article-body :global(h4) {
    color: var(--text-primary);
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
  }

  .article-body :global(a) {
    color: var(--accent-info);
  }

  .article-body :global(a:hover) {
    text-decoration: underline;
  }

  .article-body :global(code) {
    background-color: var(--bg-surface);
    padding: 0.125rem 0.25rem;
    border-radius: 0.25rem;
    font-size: 0.875em;
  }

  .article-body :global(pre) {
    background-color: var(--bg-surface);
    padding: 1rem;
    border-radius: 0.375rem;
    overflow-x: auto;
  }

  .article-body :global(blockquote) {
    border-left: 3px solid var(--accent-primary);
    padding-left: 1rem;
    color: var(--text-secondary);
    margin: 1rem 0;
  }

  .no-content {
    color: var(--text-muted);
    font-style: italic;
    margin: 0;
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
