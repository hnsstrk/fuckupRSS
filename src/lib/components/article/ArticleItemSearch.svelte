<script lang="ts">
  import { locale } from 'svelte-i18n';
  import {
    formatRelativeDate,
    formatSimilarity,
    truncateText
  } from '../../utils/articleFormat';

  interface Props {
    fnord_id: number;
    title: string;
    pentacle_title?: string | null;
    published_at?: string | null;
    similarity: number;
    summary?: string | null;
    active?: boolean;
    showSummary?: boolean;
    onclick?: () => void;
  }

  let {
    fnord_id: _fnord_id,
    title,
    pentacle_title = null,
    published_at = null,
    similarity,
    summary = null,
    active = false,
    showSummary = true,
    onclick
  }: Props = $props();

  const currentLocale = $derived($locale || 'de');
  const truncatedSummary = $derived(
    summary && showSummary ? truncateText(summary, 120) : null
  );
</script>

<button
  class="article-item-search"
  class:active
  type="button"
  {onclick}
>
  <div class="article-row">
    <div class="search-similarity">
      {formatSimilarity(similarity)}
    </div>
    <div class="article-content">
      <h3 class="article-title">{title}</h3>
      <div class="article-meta">
        <span class="source">{pentacle_title || "Unknown"}</span>
        <span class="separator">·</span>
        <span>{formatRelativeDate(published_at, currentLocale)}</span>
      </div>
      {#if truncatedSummary}
        <p class="search-summary">{truncatedSummary}</p>
      {/if}
    </div>
  </div>
</button>

<style>
  .article-item-search {
    width: 100%;
    padding: 1rem;
    text-align: left;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border-muted);
    border-left: 3px solid var(--accent-primary);
    cursor: pointer;
    transition: background-color 0.2s;
    color: var(--text-primary);
  }

  .article-item-search:hover {
    background-color: var(--bg-overlay);
  }

  .article-item-search.active {
    background-color: var(--bg-overlay);
  }

  .article-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .search-similarity {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 2.5rem;
    padding: 0.25rem 0.375rem;
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    border-radius: 0.25rem;
    font-size: 0.6875rem;
    font-weight: 600;
    margin-top: 0.125rem;
    flex-shrink: 0;
  }

  .article-content {
    flex: 1;
    min-width: 0;
  }

  .article-title {
    font-size: 0.875rem;
    font-weight: 500;
    line-height: 1.4;
    margin: 0;
    color: var(--text-primary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
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

  .search-summary {
    margin: 0.375rem 0 0 0;
    font-size: 0.75rem;
    color: var(--text-muted);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
