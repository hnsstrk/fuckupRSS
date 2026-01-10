<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { appState } from "../stores/state.svelte";
  import Tooltip from "./Tooltip.svelte";

  let listContainer: HTMLDivElement;

  function handleScroll(event: Event) {
    const target = event.target as HTMLDivElement;
    const scrollBottom = target.scrollHeight - target.scrollTop - target.clientHeight;

    // Load more when within 200px of bottom
    if (scrollBottom < 200 && appState.hasMoreFnords && !appState.loadingMore) {
      appState.loadMoreFnords();
    }
  }

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


  function getBiasIndicator(bias: number): string {
    switch (bias) {
      case -2: return "◀◀";
      case -1: return "◀";
      case 0: return "●";
      case 1: return "▶";
      case 2: return "▶▶";
      default: return "●";
    }
  }

  function getBiasLabel(bias: number): string {
    const currentLocale = $locale || 'de';
    const isGerman = currentLocale.startsWith('de');
    switch (bias) {
      case -2: return isGerman ? "Stark links" : "Strong left";
      case -1: return isGerman ? "Leicht links" : "Lean left";
      case 0: return isGerman ? "Neutral" : "Neutral";
      case 1: return isGerman ? "Leicht rechts" : "Lean right";
      case 2: return isGerman ? "Stark rechts" : "Strong right";
      default: return "";
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
        {$_('sidebar.allFeeds')}
      {/if}
    </h2>
    <p class="list-count">
      {appState.fnords.length}{#if appState.totalFnordsCount > appState.fnords.length}/{appState.totalFnordsCount}{/if} {$locale?.startsWith('de') ? 'Artikel' : 'articles'}
    </p>
  </div>

  <!-- Article List -->
  <div class="list-content" bind:this={listContainer} onscroll={handleScroll}>
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
            {#if fnord.quality_score || fnord.categories.length > 0 || fnord.revision_count > 0}
              <div class="article-indicators">
                {#if fnord.categories.length > 0}
                  <span class="category-dots" title={fnord.categories.map(c => c.name).join(', ')}>
                    {#each fnord.categories.slice(0, 3) as cat (cat.name)}
                      <span class="category-dot" style="background-color: {cat.color || 'var(--text-muted)'}"></span>
                    {/each}
                  </span>
                {/if}
                {#if fnord.revision_count > 0}
                  <span class="revision-count" title="{$_('articleView.changes.revisions')}: {fnord.revision_count}">
                    ✎{fnord.revision_count}
                  </span>
                {/if}
                {#if fnord.quality_score}
                  <span class="quality" title={$_('articleView.greyface.quality')}>
                    {"★".repeat(fnord.quality_score)}{"☆".repeat(5 - fnord.quality_score)}
                  </span>
                {/if}
                {#if fnord.political_bias !== null && fnord.political_bias !== 0}
                  <span class="bias bias-{fnord.political_bias < 0 ? 'left' : 'right'}" title="{getBiasLabel(fnord.political_bias)}">
                    {getBiasIndicator(fnord.political_bias)}
                  </span>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      </button>
    {/each}

    {#if appState.loadingMore}
      <div class="loading-more">
        <span class="loading-spinner">↻</span>
        {$locale?.startsWith('de') ? 'Lade mehr...' : 'Loading more...'}
      </div>
    {:else if appState.hasMoreFnords && appState.fnords.length > 0}
      <div class="load-more-hint">
        {$locale?.startsWith('de') ? 'Scrolle für mehr' : 'Scroll for more'}
      </div>
    {/if}

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
    font-size: 0.65rem;
    padding: 0.1rem 0.25rem;
    border-radius: 0.2rem;
    background-color: var(--bg-overlay);
  }

  .bias-left {
    color: #89b4fa;
  }

  .bias-right {
    color: #f38ba8;
  }

  .category-dots {
    display: flex;
    gap: 0.2rem;
    align-items: center;
  }

  .category-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
  }

  .revision-count {
    color: var(--accent-secondary);
    font-size: 0.7rem;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .loading-more {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .loading-spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .load-more-hint {
    text-align: center;
    padding: 0.5rem;
    color: var(--text-faint);
    font-size: 0.7rem;
  }
</style>
