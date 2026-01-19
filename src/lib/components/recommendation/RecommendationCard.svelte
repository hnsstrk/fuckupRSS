<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { locale } from 'svelte-i18n';
  import type { Recommendation } from '../../types';
  import { formatRelativeDate } from '../../utils/articleFormat';
  import Tooltip from '../Tooltip.svelte';

  // Get the main category ID (1-6) from a category or subcategory ID
  function getMainCategoryId(id: number | undefined): number {
    if (!id) return 0;
    if (id <= 6) return id;
    return Math.floor(id / 100);
  }

  // Get CSS variable name for category color
  function getCategoryColorVar(id: number | undefined): string {
    const mainId = getMainCategoryId(id);
    if (mainId >= 1 && mainId <= 6) {
      return `var(--category-${mainId})`;
    }
    return 'var(--accent-primary)';
  }

  function getBiasColor(bias: number | null): string {
    if (bias === null) return 'var(--text-muted)';
    if (bias <= -1.5) return 'var(--ctp-red)';
    if (bias <= -0.5) return 'var(--ctp-maroon)';
    if (bias <= 0.5) return 'var(--ctp-green)';
    if (bias <= 1.5) return 'var(--ctp-blue)';
    return 'var(--ctp-sapphire)';
  }

  interface Props {
    recommendation: Recommendation;
    showBias?: boolean;
    compact?: boolean;
    onsave?: (fnordId: number) => void;
    onunsave?: (fnordId: number) => void;
    onclick?: (fnordId: number) => void;
  }

  let {
    recommendation,
    showBias = true,
    compact = false,
    onsave,
    onunsave,
    onclick
  }: Props = $props();

  let isSaving = $state(false);

  const currentLocale = $derived($locale || 'de');

  async function handleSave(e: MouseEvent) {
    e.stopPropagation();
    if (isSaving) return;
    isSaving = true;

    if (recommendation.is_saved) {
      onunsave?.(recommendation.fnord_id);
    } else {
      onsave?.(recommendation.fnord_id);
    }

    isSaving = false;
  }

  function handleClick() {
    onclick?.(recommendation.fnord_id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onclick?.(recommendation.fnord_id);
    }
  }
</script>

<article
  class="recommendation-card"
  class:compact
  class:saved={recommendation.is_saved}
  aria-label={recommendation.title}
>
  <!-- Clickable content area -->
  <button
    type="button"
    class="card-content"
    onclick={handleClick}
    onkeydown={handleKeydown}
  >
    {#if recommendation.image_url && !compact}
      <div class="card-image">
        <img
          src={recommendation.image_url}
          alt=""
          loading="lazy"
        />
      </div>
    {/if}

    <div class="card-body">
      <h3 class="card-title">{recommendation.title}</h3>

      <div class="card-meta">
        {#if recommendation.pentacle_icon}
          <img
            src={recommendation.pentacle_icon}
            alt=""
            class="source-icon"
          />
        {/if}
        <span class="source-name">{recommendation.pentacle_title || 'Unbekannt'}</span>
        <span class="separator">·</span>
        <time datetime={recommendation.published_at || ''}>
          {formatRelativeDate(recommendation.published_at, currentLocale)}
        </time>
      </div>

      {#if recommendation.summary && !compact}
        <p class="card-summary">{recommendation.summary}</p>
      {/if}
    </div>
  </button>

  <!-- Explanation -->
  <div class="card-explanation">
    <i class="fa-solid fa-lightbulb"></i>
    <span>{recommendation.explanation}</span>
  </div>

  <!-- Footer with categories and actions -->
  <footer class="card-footer">
    <div class="card-categories">
      {#each recommendation.categories.slice(0, 2) as category (category.sephiroth_id)}
        <span
          class="category-badge"
          style="--category-color: {getCategoryColorVar(category.sephiroth_id)}"
        >
          {#if category.icon}
            <i class={category.icon}></i>
          {/if}
          {category.name}
        </span>
      {/each}

      {#if showBias && recommendation.political_bias !== null}
        <span class="bias-indicator" style="color: {getBiasColor(recommendation.political_bias)}">
          <i class="fa-solid fa-scale-balanced"></i>
        </span>
      {/if}
    </div>

    <div class="card-actions">
      <Tooltip content={recommendation.is_saved ? $_('recommendations.unsave') : $_('recommendations.save')}>
        <button
          type="button"
          class="action-btn save-btn"
          class:saved={recommendation.is_saved}
          onclick={handleSave}
          disabled={isSaving}
          aria-label={recommendation.is_saved ? $_('recommendations.unsave') : $_('recommendations.save')}
        >
          <i class={recommendation.is_saved ? 'fa-solid fa-bookmark' : 'fa-regular fa-bookmark'}></i>
          {#if !compact}
            <span>{recommendation.is_saved ? $_('recommendations.saved') : $_('recommendations.save')}</span>
          {/if}
        </button>
      </Tooltip>
    </div>
  </footer>
</article>

<style>
  .recommendation-card {
    background: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.625rem;
    overflow: hidden;
    transition: all 0.2s ease;
  }

  .recommendation-card:hover {
    border-color: var(--border-subtle);
    box-shadow: 0 4px 12px var(--shadow-color);
  }

  .recommendation-card.saved {
    border-color: var(--accent-primary);
    border-left-width: 3px;
  }

  .card-content {
    display: block;
    width: 100%;
    padding: 0;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    color: inherit;
  }

  .card-content:hover .card-title {
    color: var(--accent-primary);
  }

  .card-image {
    width: 100%;
    height: 160px;
    overflow: hidden;
    background: var(--bg-base);
  }

  .card-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .card-body {
    padding: 1rem;
  }

  .card-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 0.5rem;
    line-height: 1.3;
    transition: color 0.15s ease;

    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .source-icon {
    width: 16px;
    height: 16px;
    border-radius: 0.25rem;
  }

  .source-name {
    font-weight: 500;
    color: var(--text-secondary);
  }

  .separator {
    color: var(--border-default);
  }

  .card-summary {
    margin: 0.75rem 0 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
    line-height: 1.5;

    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-explanation {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1rem;
    background: var(--bg-base);
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .card-explanation i {
    color: var(--ctp-yellow);
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.625rem 1rem;
    border-top: 1px solid var(--border-subtle);
  }

  .card-categories {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .category-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.5rem;
    background: color-mix(in srgb, var(--category-color) 20%, transparent);
    border: 1px solid color-mix(in srgb, var(--category-color) 40%, transparent);
    border-radius: 9999px;
    font-size: 0.6875rem;
    color: var(--category-color);
  }

  .category-badge i {
    font-size: 0.625rem;
  }

  .bias-indicator {
    display: inline-flex;
    align-items: center;
    font-size: 0.75rem;
    margin-left: 0.25rem;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.625rem;
    background: transparent;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--bg-base);
    color: var(--text-primary);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save-btn.saved {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  /* Compact Mode */
  .compact .card-body {
    padding: 0.75rem;
  }

  .compact .card-title {
    font-size: 0.9375rem;
  }

  .compact .card-explanation {
    padding: 0.375rem 0.75rem;
    font-size: 0.75rem;
  }

  .compact .card-footer {
    padding: 0.375rem 0.75rem;
  }
</style>
