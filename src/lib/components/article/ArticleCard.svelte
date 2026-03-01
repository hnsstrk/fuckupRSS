<script lang="ts">
  import { locale } from "svelte-i18n";
  import {
    formatFullDate,
    formatSimilarity,
    getStatusIcon,
    getStatusColorClass,
    getBiasLabel,
    getBiasColor,
    getCategoryColorVar,
  } from "../../utils/articleFormat";

  interface Category {
    id?: number;
    name: string;
    color: string | null;
    icon: string | null;
  }

  interface Tag {
    id?: number;
    name: string;
  }

  interface Props {
    fnord_id: number;
    title: string;
    pentacle_title?: string | null;
    published_at?: string | null;
    status?: string | null;
    categories?: Category[];
    tags?: Tag[];
    similarity?: number | null;
    political_bias?: number | null;
    reason?: string | null;
    showScore?: boolean;
    showStatus?: boolean;
    showCategories?: boolean;
    showTags?: boolean;
    showBias?: boolean;
    showReason?: boolean;
    showAction?: boolean;
    actionLabel?: string;
    maxTags?: number;
    onclick?: () => void;
    onaction?: () => void;
  }

  let {
    fnord_id: _fnord_id,
    title,
    pentacle_title = null,
    published_at = null,
    status = null,
    categories = [],
    tags = [],
    similarity = null,
    political_bias = null,
    reason = null,
    showScore = false,
    showStatus = false,
    showCategories = true,
    showTags = true,
    showBias = false,
    showReason = false,
    showAction = false,
    actionLabel = "Read",
    maxTags = 3,
    onclick,
    onaction,
  }: Props = $props();

  const currentLocale = $derived($locale || "de");
  const displayedTags = $derived(tags.slice(0, maxTags));
  const remainingTags = $derived(tags.length > maxTags ? tags.length - maxTags : 0);
  const hasMetaRow = $derived(
    (showCategories && categories.length > 0) || (showTags && tags.length > 0),
  );

  function handleAction(e: MouseEvent) {
    e.stopPropagation();
    onaction?.();
  }

  function handleCardClick() {
    onclick?.();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onclick?.();
    }
  }
</script>

<div
  class="article-card"
  class:clickable={!!onclick}
  onclick={handleCardClick}
  onkeydown={handleKeydown}
  role={onclick ? "button" : undefined}
  tabindex={onclick ? 0 : undefined}
>
  <div class="card-main">
    <span class="card-title">{title}</span>
    <div class="card-meta-row">
      {#if pentacle_title}
        <span class="card-source">{pentacle_title}</span>
      {/if}
      {#if published_at}
        <span class="card-date">{formatFullDate(published_at, currentLocale)}</span>
      {/if}
      {#if showStatus && status}
        <span class="card-status {getStatusColorClass(status)}">
          <i class={getStatusIcon(status)}></i>
        </span>
      {/if}
      {#if showBias && political_bias !== null}
        <span class="card-bias" style:color={getBiasColor(political_bias)}>
          {getBiasLabel(political_bias, currentLocale)}
        </span>
      {/if}
    </div>
    {#if hasMetaRow}
      <div class="card-tags-row">
        {#if showCategories}
          {#each categories as cat (cat.name)}
            <span
              class="card-category"
              style:background-color={getCategoryColorVar(cat.id, "var(--bg-overlay)")}
              title={cat.name}
            >
              {#if cat.icon}<i class={cat.icon}></i>{:else}{cat.name}{/if}
            </span>
          {/each}
        {/if}
        {#if showTags}
          {#each displayedTags as tag (tag.name)}
            <span class="card-tag">{tag.name}</span>
          {/each}
          {#if remainingTags > 0}
            <span class="card-tag-more">+{remainingTags}</span>
          {/if}
        {/if}
      </div>
    {/if}
    {#if showReason && reason}
      <p class="card-reason">{reason}</p>
    {/if}
    {#if showAction}
      <button type="button" class="card-action" onclick={handleAction}>
        {actionLabel}
      </button>
    {/if}
  </div>
  {#if showScore && similarity !== null}
    <span class="card-score" title="Ähnlichkeit">
      {formatSimilarity(similarity)}
    </span>
  {/if}
</div>

<style>
  .article-card {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding: 1rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    text-align: left;
    transition: all 0.2s;
    width: 100%;
  }

  .article-card.clickable {
    cursor: pointer;
  }

  .article-card.clickable:hover {
    background-color: var(--bg-muted);
    border-color: var(--accent-primary);
  }

  .card-main {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 0;
    flex: 1;
  }

  .card-title {
    font-size: 0.9375rem;
    color: var(--text-primary);
    font-weight: 600;
    line-height: 1.4;
  }

  .card-meta-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .card-source {
    font-weight: 500;
    color: var(--text-secondary);
  }

  .card-date {
    color: var(--text-muted);
  }

  .card-source + .card-date::before {
    content: "·";
    margin-right: 0.5rem;
  }

  .card-status {
    font-size: 0.75rem;
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

  .card-bias {
    font-weight: 500;
    font-size: 0.75rem;
  }

  .card-tags-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.375rem;
    margin-top: 0.25rem;
  }

  .card-category {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.5rem;
    height: 1.5rem;
    padding: 0.25rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    color: white;
  }

  .card-tag {
    display: inline-block;
    padding: 0.125rem 0.375rem;
    background-color: var(--bg-base);
    color: var(--text-secondary);
    border-radius: 0.25rem;
    font-size: 0.6875rem;
  }

  .card-tag-more {
    display: inline-block;
    padding: 0.125rem 0.375rem;
    color: var(--text-muted);
    font-size: 0.6875rem;
  }

  .card-reason {
    margin: 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-style: italic;
    line-height: 1.5;
  }

  .card-action {
    align-self: flex-start;
    margin-top: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid var(--accent-primary);
    border-radius: 0.375rem;
    background: none;
    color: var(--accent-primary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .card-action:hover {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .card-score {
    font-size: 0.875rem;
    font-weight: 700;
    color: var(--accent-primary);
    background-color: var(--bg-base);
    padding: 0.375rem 0.625rem;
    border-radius: 9999px;
    white-space: nowrap;
    flex-shrink: 0;
  }
</style>
