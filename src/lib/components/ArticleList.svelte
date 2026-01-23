<script lang="ts">
  import { _, locale } from "svelte-i18n";
  import { appState } from "../stores/state.svelte";
  import { ArticleItemCompact, ArticleItemSearch } from "./article";
  import type { SearchResult } from "../types";

  // Check if we're in search mode
  const isSearchMode = $derived(
    appState.searchQuery.length > 0 || appState.searchResults.length > 0,
  );

  function handleScroll(event: Event) {
    const target = event.target as HTMLDivElement;
    const scrollBottom =
      target.scrollHeight - target.scrollTop - target.clientHeight;

    // Load more when within 200px of bottom
    if (scrollBottom < 200 && appState.hasMoreFnords && !appState.loadingMore) {
      appState.loadMoreFnords();
    }
  }

  function handleSelectFnord(id: number) {
    appState.selectFnord(id);
  }

  function handleSelectSearchResult(result: SearchResult) {
    appState.selectFnord(result.fnord_id);
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
    {#if isSearchMode}
      <h2 class="list-title">{$_("search.results")}</h2>
      <p class="list-count">
        {appState.searchResults.length}
        {$locale?.startsWith("de") ? "Ergebnisse" : "results"}
        {#if appState.searchQuery}
          <span class="search-query">"{appState.searchQuery}"</span>
        {/if}
      </p>
    {:else}
      <h2 class="list-title">
        {#if appState.selectedPentacle}
          {appState.selectedPentacle.title || "Feed"}
        {:else}
          {$_("sidebar.allFeeds")}
        {/if}
      </h2>
      <p class="list-count">
        {appState.fnords
          .length}{#if appState.totalFnordsCount > appState.fnords.length}/{appState.totalFnordsCount}{/if}
        {$locale?.startsWith("de") ? "Artikel" : "articles"}
      </p>
    {/if}
  </div>

  <!-- Article List / Search Results -->
  <div class="list-content" onscroll={handleScroll}>
    {#if isSearchMode}
      <!-- Search Results -->
      {#each appState.searchResults as result (result.fnord_id)}
        <ArticleItemSearch
          fnord_id={result.fnord_id}
          title={result.title}
          pentacle_title={result.pentacle_title}
          published_at={result.published_at}
          similarity={result.similarity}
          summary={result.summary}
          active={appState.selectedFnordId === result.fnord_id}
          onclick={() => handleSelectSearchResult(result)}
        />
      {/each}

      {#if appState.searchResults.length === 0 && !appState.searching && appState.searchQuery}
        <div class="empty-state">
          {$_("search.noResults")}
        </div>
      {/if}

      {#if appState.searching}
        <div class="empty-state">
          <i class="loading-spinner fa-solid fa-rotate fa-spin"></i>
          {$_("search.searching")}
        </div>
      {/if}
    {:else}
      <!-- Normal Article List -->
      {#each appState.fnords as fnord (fnord.id)}
        <ArticleItemCompact
          id={fnord.id}
          title={fnord.title}
          status={fnord.status}
          pentacle_title={fnord.pentacle_title}
          published_at={fnord.published_at}
          categories={fnord.categories}
          revision_count={fnord.revision_count}
          quality_score={fnord.quality_score}
          political_bias={fnord.political_bias}
          active={appState.selectedFnordId === fnord.id}
          onclick={() => handleSelectFnord(fnord.id)}
        />
      {/each}

      {#if appState.loadingMore}
        <div class="loading-more">
          <i class="loading-spinner fa-solid fa-rotate fa-spin"></i>
          {$locale?.startsWith("de") ? "Lade mehr..." : "Loading more..."}
        </div>
      {:else if appState.hasMoreFnords && appState.fnords.length > 0}
        <div class="load-more-hint">
          {$locale?.startsWith("de") ? "Scrolle für mehr" : "Scroll for more"}
        </div>
      {/if}

      {#if appState.fnords.length === 0 && !appState.loading}
        <div class="empty-state">
          {$_("articleList.noArticles")}<br />
          {#if appState.pentacles.length === 0}
            {$_("sidebar.addFeed")}
          {:else}
            {$_("articleList.selectFeed")}
          {/if}
        </div>
      {/if}

      {#if appState.loading}
        <div class="empty-state">{$_("articleList.loading")}</div>
      {/if}
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
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .load-more-hint {
    text-align: center;
    padding: 0.5rem;
    color: var(--text-faint);
    font-size: 0.7rem;
  }

  .search-query {
    color: var(--accent-primary);
    font-style: italic;
  }
</style>
