<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { appState } from "../stores/state.svelte";
  import Tooltip from "./Tooltip.svelte";

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
    if (!type) return $locale?.startsWith('de') ? "Unbekannt" : "Unknown";
    return $_(`articleType.${type}`);
  }

  function getBiasLabel(bias: number | null): string {
    if (bias === null) return $locale?.startsWith('de') ? "Nicht bewertet" : "Not rated";
    const isGerman = $locale?.startsWith('de');
    switch (bias) {
      case -2: return isGerman ? "Stark links" : "Strong left";
      case -1: return isGerman ? "Leicht links" : "Lean left";
      case 0: return $_('articleView.greyface.biasCenter');
      case 1: return isGerman ? "Leicht rechts" : "Lean right";
      case 2: return isGerman ? "Stark rechts" : "Strong right";
      default: return isGerman ? "Unbekannt" : "Unknown";
    }
  }

  function getSachlichkeitLabel(s: number | null): string {
    if (s === null) return $locale?.startsWith('de') ? "Nicht bewertet" : "Not rated";
    const isGerman = $locale?.startsWith('de');
    switch (s) {
      case 0: return isGerman ? "Stark emotional" : "Highly emotional";
      case 1: return isGerman ? "Emotional" : "Emotional";
      case 2: return isGerman ? "Gemischt" : "Mixed";
      case 3: return isGerman ? "Überwiegend sachlich" : "Mostly objective";
      case 4: return isGerman ? "Sachlich" : "Objective";
      default: return isGerman ? "Unbekannt" : "Unknown";
    }
  }

  function openInBrowser() {
    if (appState.selectedFnord) {
      window.open(appState.selectedFnord.url, "_blank");
    }
  }

  async function fetchFullContent() {
    if (appState.selectedFnord) {
      await appState.fetchFullContent(appState.selectedFnord.id);
    }
  }

  async function analyzeWithAI() {
    if (appState.selectedFnord && appState.ollamaStatus.available) {
      await appState.processArticle(appState.selectedFnord.id);
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
              <span>{fnord.status === "golden_apple" ? $_('terminology.golden_apple.term') : $_('actions.favorite')}</span>
            </Tooltip>
          </button>
          {#if !fnord.content_full}
            <button
              onclick={fetchFullContent}
              class="btn btn-default {appState.retrieving ? 'retrieving' : ''}"
              disabled={appState.retrieving}
              title={$locale?.startsWith('de') ? 'Volltext abrufen (r)' : 'Fetch full text (r)'}
            >
              {#if appState.retrieving}
                <span class="spinner">⟳</span>
              {/if}
              <Tooltip termKey="hagbard">
                <span>{$locale?.startsWith('de') ? 'Volltext' : 'Full Text'}</span>
              </Tooltip>
            </button>
          {/if}
          {#if appState.ollamaStatus.available && !fnord.summary}
            <button
              onclick={analyzeWithAI}
              class="btn btn-default {appState.analyzing ? 'retrieving' : ''}"
              disabled={appState.analyzing}
              title={$locale?.startsWith('de') ? 'KI-Analyse' : 'AI Analysis'}
            >
              {#if appState.analyzing}
                <span class="spinner">⟳</span>
              {/if}
              <Tooltip termKey="discordian">
                <span>{$locale?.startsWith('de') ? 'Analysieren' : 'Analyze'}</span>
              </Tooltip>
            </button>
          {/if}
          <button onclick={openInBrowser} class="btn btn-default">
            {$_('actions.openInBrowser')}
          </button>
        </div>
      </div>
    </div>

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
                <div class="item-label">{$locale?.startsWith('de') ? 'Artikeltyp' : 'Article Type'}</div>
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

    <!-- Summary (Discordian Analysis) -->
    {#if fnord.summary}
      <div class="summary-section">
        <div class="section-content">
          <div class="section-header">
            <Tooltip termKey="discordian">{$_('terminology.discordian.term')}</Tooltip>
          </div>
          <p class="summary-text">{fnord.summary}</p>
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
            <p class="content-text">{fnord.content_raw}</p>
          {:else}
            <p class="no-content">
              {$locale?.startsWith('de')
                ? 'Kein Inhalt verfügbar. Klicke auf "Im Browser öffnen" um den vollständigen Artikel zu lesen.'
                : 'No content available. Click "Open in browser" to read the full article.'}
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
        {$locale?.startsWith('de') ? 'Benutze' : 'Use'} <kbd>j</kbd> {$locale?.startsWith('de') ? 'und' : 'and'}
        <kbd>k</kbd> {$locale?.startsWith('de') ? 'zum Navigieren.' : 'to navigate.'}
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

  .content-text {
    white-space: pre-wrap;
    margin: 0;
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
