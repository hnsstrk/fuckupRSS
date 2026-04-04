<script lang="ts">
  import { _ } from "svelte-i18n";
  import { formatChangedDate } from "$lib/utils/articleFormat";

  interface KeywordArticle {
    id: number;
    title: string;
    pentacle_title: string | null;
    published_at: string | null;
    status: string;
  }

  interface Props {
    keywordArticles: KeywordArticle[];
    hasMoreArticles: boolean;
    articlesLoading: boolean;
    loading: boolean;
    onOpenArticle: (id: number) => void;
    onLoadMoreArticles: () => void;
  }

  let {
    keywordArticles,
    hasMoreArticles,
    articlesLoading,
    loading,
    onOpenArticle,
    onLoadMoreArticles,
  }: Props = $props();

  function getStatusIconClass(status: string): string {
    switch (status) {
      case "concealed":
        return "fa-solid fa-eye-slash";
      case "illuminated":
        return "fa-solid fa-check";
      case "golden_apple":
        return "fa-solid fa-apple-whole";
      default:
        return "";
    }
  }
</script>

<div class="detail-section">
  <h4 class="section-title">{$_("network.linkedArticles") || "Verlinkte Artikel"}</h4>
  {#if keywordArticles.length > 0}
    <div class="articles-list">
      {#each keywordArticles as article (article.id)}
        <button class="article-item" onclick={() => onOpenArticle(article.id)}>
          <i
            class="article-status {getStatusIconClass(article.status)}"
            title={article.status}
          ></i>
          <div class="article-info">
            <span class="article-title">{article.title}</span>
            <span class="article-meta">
              {#if article.pentacle_title}
                <span class="article-source">{article.pentacle_title}</span>
              {/if}
              {#if article.published_at}
                <span class="article-date">{formatChangedDate(article.published_at)}</span>
              {/if}
            </span>
          </div>
        </button>
      {/each}
      {#if hasMoreArticles}
        <button
          class="load-more-articles"
          onclick={onLoadMoreArticles}
          disabled={articlesLoading}
        >
          {#if articlesLoading}
            {$_("network.loading") || "Laden..."}
          {:else}
            {$_("network.loadMore") || "Mehr laden"}
          {/if}
        </button>
      {/if}
    </div>
  {:else if !loading}
    <div class="no-articles">{$_("network.noArticles") || "Keine Artikel gefunden"}</div>
  {/if}
</div>

<style>
  .detail-section {
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  /* Articles List */
  .articles-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .article-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
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

  .article-status {
    font-size: 0.875rem;
    flex-shrink: 0;
    width: 1.25rem;
  }

  .article-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    min-width: 0;
    flex: 1;
  }

  .article-title {
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .article-meta {
    display: flex;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .article-source {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 150px;
  }

  .article-date {
    flex-shrink: 0;
  }

  .load-more-articles {
    padding: 0.5rem;
    background: none;
    border: 1px dashed var(--border-default);
    border-radius: 0.375rem;
    color: var(--accent-primary);
    cursor: pointer;
    font-size: 0.75rem;
    transition: all 0.2s;
  }

  .load-more-articles:hover:not(:disabled) {
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .load-more-articles:disabled {
    color: var(--text-muted);
    cursor: not-allowed;
  }

  .no-articles {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }
</style>
