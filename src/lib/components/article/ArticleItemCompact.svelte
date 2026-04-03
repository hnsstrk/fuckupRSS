<script lang="ts">
  import { locale } from "svelte-i18n";
  import {
    formatRelativeDate,
    formatChangedDate,
    getStatusIcon,
    getStatusColorClass,
    getBiasIcon,
    getBiasLabel,
    getBiasDirectionClass,
    getCategoryColorVar,
  } from "../../utils/articleFormat";

  interface Category {
    id?: number;
    name: string;
    color: string | null;
  }

  interface Props {
    id: number;
    title: string;
    status: string;
    pentacle_title?: string | null;
    published_at?: string | null;
    changed_at?: string | null;
    categories?: Category[];
    revision_count?: number;
    quality_score?: number | null;
    political_bias?: number | null;
    active?: boolean;
    showStatus?: boolean;
    showIndicators?: boolean;
    onclick?: () => void;
  }

  let {
    id: _id,
    title,
    status,
    pentacle_title = null,
    published_at = null,
    changed_at = null,
    categories = [],
    revision_count = 0,
    quality_score = null,
    political_bias = null,
    active = false,
    showStatus = true,
    showIndicators = true,
    onclick,
  }: Props = $props();

  const currentLocale = $derived($locale || "de");
  const isUnread = $derived(status === "concealed");
  const hasIndicators = $derived(showIndicators);
</script>

<button class="article-item-compact" class:active type="button" {onclick}>
  <div class="article-row">
    {#if showStatus}
      <i class="status-icon {getStatusIcon(status)} {getStatusColorClass(status)}"></i>
    {/if}
    <div class="article-content">
      <h3 class="article-title" class:unread={isUnread}>{title}</h3>

      <div class="article-meta">
        {#if pentacle_title}
          <span class="source" title={pentacle_title}>{pentacle_title}</span>
          <span class="separator">·</span>
        {/if}
        {#if changed_at}
          <span class="changed-date">{formatChangedDate(changed_at)}</span>
        {:else if published_at}
          <span class="date">{formatRelativeDate(published_at, currentLocale)}</span>
        {/if}
        {#if revision_count > 0}
          <span class="revision-badge" title="Revisionen">
            <i class="fa-solid fa-pen-to-square"></i>{revision_count}
          </span>
        {/if}
      </div>

      {#if hasIndicators}
        <div class="article-indicators">
          {#if categories.length > 0}
            <div class="category-list" title={categories.map((c) => c.name).join(", ")}>
              {#each categories.slice(0, 3) as cat (cat.name)}
                <span
                  class="category-dot"
                  style="background-color: {cat.color ||
                    getCategoryColorVar(cat.id, 'var(--text-muted)')}"
                ></span>
              {/each}
            </div>
          {/if}

          <div class="indicators-right">
            {#if quality_score}
              <span class="quality">
                {#each {length: quality_score} as _, i (i)}<i class="fa-solid fa-star"></i>{/each}
              </span>
            {/if}
            {#if political_bias !== null && political_bias !== 0}
              <span
                class="bias {getBiasDirectionClass(political_bias)}"
                title={getBiasLabel(political_bias, currentLocale)}
              >
                <i class={getBiasIcon(political_bias)}></i>
              </span>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
</button>

<style>
  .article-item-compact {
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

  .article-item-compact:hover {
    background-color: var(--bg-overlay);
  }

  .article-item-compact.active {
    background-color: var(--bg-overlay);
  }

  .article-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .status-icon {
    font-size: 1rem;
    margin-top: 0.125rem;
    flex-shrink: 0;
  }

  .status-concealed {
    color: var(--fnord-color);
  }
  .status-illuminated {
    color: var(--illuminated-color);
  }
  .status-golden_apple {
    color: var(--golden-apple-color);
  }

  .article-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .article-title {
    font-size: 0.875rem;
    font-weight: 500;
    line-height: 1.3;
    margin: 0;
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .article-title.unread {
    color: var(--text-primary);
    font-weight: 600;
  }

  .article-meta {
    display: flex;
    align-items: baseline;
    font-size: 0.75rem;
    color: var(--text-muted);
    line-height: 1.2;
    gap: 0.375rem;
    margin-top: 0.25rem;
    width: 100%;
  }

  .source {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
    flex: 0 1 auto;
  }

  .separator {
    color: var(--text-faint);
    flex-shrink: 0;
  }

  .date,
  .changed-date {
    white-space: nowrap;
    flex-shrink: 0;
  }

  .changed-date {
    color: var(--accent-warning);
  }

  .revision-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.125rem;
    padding: 0 0.25rem;
    background-color: var(--fnord-color);
    color: var(--bg-base);
    border-radius: 0.25rem;
    font-size: 0.625rem;
    font-weight: 600;
  }

  .revision-badge i {
    font-size: 0.5rem;
  }

  .article-indicators {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-top: 0.25rem;
    height: 1.25rem;
  }

  .category-list {
    display: flex;
    gap: 0.25rem;
    overflow: hidden;
    align-items: center;
  }

  .category-dot {
    display: inline-block;
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .indicators-right {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin-left: auto;
  }

  .quality {
    color: var(--golden-apple-color);
    font-size: 0.625rem;
    display: flex;
    gap: 1px;
  }

  .bias {
    font-size: 0.65rem;
    padding: 0.1rem 0.25rem;
    border-radius: 0.2rem;
    background-color: var(--bg-overlay);
  }

  .bias-left {
    color: var(--bias-lean-left);
  }

  .bias-right {
    color: var(--bias-lean-right);
  }
</style>
