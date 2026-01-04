<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { appState, type Fnord } from "../stores/state.svelte";
  import Tooltip from "./Tooltip.svelte";

  function getStatusIcon(status: string): string {
    switch (status) {
      case "concealed": return "●";
      case "illuminated": return "○";
      case "golden_apple": return "✦";
      default: return "○";
    }
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    const currentLocale = $locale || 'de';
    const isGerman = currentLocale.startsWith('de');

    if (diffMins < 60) {
      return isGerman ? `vor ${diffMins} Min` : `${diffMins} min ago`;
    } else if (diffHours < 24) {
      return isGerman ? `vor ${diffHours} Std` : `${diffHours}h ago`;
    } else if (diffDays < 7) {
      return isGerman ? `vor ${diffDays} Tagen` : `${diffDays}d ago`;
    } else {
      return date.toLocaleDateString(isGerman ? "de-DE" : "en-US", {
        day: "numeric",
        month: "short",
      });
    }
  }

  function getArticleTypeIcon(type: string | null): string {
    switch (type) {
      case "news": return "📰";
      case "analysis": return "🔍";
      case "opinion": return "💭";
      case "satire": return "🎭";
      case "ad": return "📢";
      default: return "❓";
    }
  }

  function handleSelectFnord(id: number) {
    appState.selectFnord(id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "j") {
      e.preventDefault();
      appState.selectNextFnord();
    } else if (e.key === "k") {
      e.preventDefault();
      appState.selectPrevFnord();
    } else if (e.key === "s" && appState.selectedFnordId) {
      e.preventDefault();
      appState.toggleGoldenApple(appState.selectedFnordId);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="article-list">
  <!-- Header -->
  <div class="list-header">
    <h2 class="list-title">
      {#if appState.selectedPentacle}
        {appState.selectedPentacle.title || "Feed"}
      {:else}
        {$_('sidebar.allFeeds')} (<Tooltip termKey="fnord"><span class="text-fnord">{$_('terminology.fnord.term')}</span></Tooltip>)
      {/if}
    </h2>
    <p class="list-count">
      {appState.fnords.length} {$locale?.startsWith('de') ? 'Artikel' : 'articles'}
    </p>
  </div>

  <!-- Article List -->
  <div class="list-content">
    {#each appState.fnords as fnord (fnord.id)}
      <button
        class="article-item {appState.selectedFnordId === fnord.id ? 'active' : ''}"
        onclick={() => handleSelectFnord(fnord.id)}
      >
        <div class="article-row">
          <span class="status-icon status-{fnord.status}">{getStatusIcon(fnord.status)}</span>
          <div class="article-content">
            <h3 class="article-title {fnord.status === 'concealed' ? 'unread' : ''}">{fnord.title}</h3>
            <div class="article-meta">
              <span class="source">{fnord.pentacle_title || "Unknown"}</span>
              <span class="separator">·</span>
              <span>{formatDate(fnord.published_at)}</span>
            </div>
            {#if fnord.article_type || fnord.quality_score}
              <div class="article-indicators">
                {#if fnord.article_type}
                  <span title={$_(`articleType.${fnord.article_type}`)}>{getArticleTypeIcon(fnord.article_type)}</span>
                {/if}
                {#if fnord.quality_score}
                  <span class="quality" title={$_('articleView.greyface.quality')}>
                    {"★".repeat(fnord.quality_score)}{"☆".repeat(5 - fnord.quality_score)}
                  </span>
                {/if}
                {#if fnord.political_bias !== null && fnord.political_bias !== 0}
                  <span class="bias" title="{$_('articleView.greyface.bias')}: {fnord.political_bias}">
                    {fnord.political_bias < 0 ? "←" : "→"}
                  </span>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      </button>
    {/each}

    {#if appState.fnords.length === 0 && !appState.loading}
      <div class="empty-state">
        {$_('articleList.noArticles')}<br />
        {#if appState.pentacles.length === 0}
          <Tooltip termKey="pentacle">{$_('sidebar.addFeed')}</Tooltip>
        {:else}
          {$_('articleList.selectFeed')}
        {/if}
      </div>
    {/if}

    {#if appState.loading}
      <div class="empty-state">{$_('articleList.loading')}</div>
    {/if}
  </div>

  <!-- Keyboard Hints -->
  <div class="keyboard-hints">
    <span><kbd>j</kbd> {$locale?.startsWith('de') ? 'weiter' : 'next'}</span>
    <span><kbd>k</kbd> {$locale?.startsWith('de') ? 'zurück' : 'prev'}</span>
    <span><kbd>s</kbd> {$locale?.startsWith('de') ? 'Favorit' : 'star'}</span>
  </div>
</div>

<style>
  .article-list {
    width: 20rem;
    background-color: var(--bg-surface);
    border-right: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    height: 100%;
    flex-shrink: 0;
    overflow: hidden;
  }

  .list-header {
    padding: 1rem;
    border-bottom: 1px solid var(--border-default);
  }

  .list-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .list-count {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0.25rem 0 0 0;
  }

  .list-content {
    flex: 1;
    overflow-y: auto;
  }

  .article-item {
    width: 100%;
    padding: 1rem;
    text-align: left;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border-muted);
    cursor: pointer;
    transition: background-color 0.2s;
    color: var(--text-primary);
  }

  .article-item:hover {
    background-color: var(--bg-overlay);
  }

  .article-item.active {
    background-color: var(--bg-overlay);
  }

  .article-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .status-icon {
    font-size: 1.125rem;
    margin-top: 0.125rem;
  }

  .status-concealed { color: var(--fnord-color); }
  .status-illuminated { color: var(--illuminated-color); }
  .status-golden_apple { color: var(--golden-apple-color); }

  .article-content {
    flex: 1;
    min-width: 0;
  }

  .article-title {
    font-size: 0.875rem;
    font-weight: 500;
    line-height: 1.4;
    margin: 0;
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .article-title.unread {
    color: var(--text-primary);
  }

  .article-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .source {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .separator {
    color: var(--text-faint);
  }

  .article-indicators {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.375rem;
    font-size: 0.75rem;
  }

  .quality {
    color: var(--golden-apple-color);
  }

  .bias {
    color: var(--text-muted);
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .keyboard-hints {
    border-top: 1px solid var(--border-default);
    padding: 0.5rem;
    display: flex;
    justify-content: center;
    gap: 1rem;
    font-size: 0.75rem;
    color: var(--text-faint);
  }

  kbd {
    background-color: var(--bg-overlay);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-family: inherit;
  }

  .text-fnord {
    color: var(--fnord-color);
  }
</style>
