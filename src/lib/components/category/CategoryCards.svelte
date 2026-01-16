<script lang="ts">
  import { _ } from 'svelte-i18n';

  // Generic category structure that all variants can map to
  interface CategoryData {
    id: number;
    name: string;
    icon?: string | null;
    color?: string | null;
    // Stats - use what's available
    primaryValue: number;
    primaryLabel: string;
    secondaryValue?: number;
    secondaryLabel?: string;
    // For subcategories
    subcategories?: SubcategoryData[];
  }

  interface SubcategoryData {
    id: number;
    name: string;
    icon?: string | null;
    primaryValue: number;
    secondaryValue?: number;
    // For weight display
    weight?: number;
    // For warning indicators
    showWarning?: boolean;
  }

  interface Props {
    categories: CategoryData[];
    title?: string;
    tooltipKey?: string;
    expandable?: boolean;
    expandedId?: number | null;
    loadingExpanded?: boolean;
    emptyMessage?: string;
    onExpand?: (id: number) => void;
  }

  let {
    categories,
    title = '',
    tooltipKey,
    expandable = true,
    expandedId = null,
    loadingExpanded = false,
    emptyMessage,
    onExpand
  }: Props = $props();

  const maxPrimary = $derived(Math.max(...categories.map(c => c.primaryValue), 1));

  function handleExpand(id: number) {
    if (expandable && onExpand) {
      onExpand(id);
    }
  }

  function getWeightClass(weight: number | undefined): string {
    if (!weight) return 'weight-low';
    if (weight >= 0.7) return 'weight-high';
    if (weight >= 0.4) return 'weight-medium';
    return 'weight-low';
  }
</script>

{#if title}
  <h4 class="cards-title">
    {#if tooltipKey}
      <span class="title-tooltip" title={$_(tooltipKey)}>{title}</span>
    {:else}
      {title}
    {/if}
  </h4>
{/if}

<div class="category-cards">
  {#each categories as cat (cat.id)}
    {@const barWidth = (cat.primaryValue / maxPrimary) * 100}
    {@const isExpanded = expandedId === cat.id}
    <button
      class="category-card"
      class:expanded={isExpanded}
      class:expandable
      style="--cat-color: {cat.color || '#6366F1'}"
      onclick={() => handleExpand(cat.id)}
      disabled={!expandable}
    >
      <div class="card-header">
        <div class="card-icon-wrapper">
          <i class="{cat.icon || 'fa-solid fa-folder'}"></i>
        </div>
        <span class="card-title">{cat.name}</span>
        {#if expandable}
          <i class="fa-solid fa-chevron-down expand-icon" class:rotated={isExpanded}></i>
        {/if}
      </div>
      <div class="card-stats">
        <div class="stat-row">
          <span class="stat-label">{cat.primaryLabel}</span>
          <span class="stat-value">{cat.primaryValue}{typeof cat.primaryValue === 'number' && cat.primaryLabel.includes('%') ? '' : ''}</span>
        </div>
        <div class="progress-bar">
          <div class="progress-fill" style="width: {barWidth}%"></div>
        </div>
        {#if cat.secondaryValue !== undefined && cat.secondaryLabel}
          <div class="stat-row secondary">
            <span class="stat-label">{cat.secondaryLabel}</span>
            <span class="stat-value">{cat.secondaryValue}</span>
          </div>
        {/if}
      </div>

      <!-- Subcategories (expanded view) -->
      {#if isExpanded && expandable}
        <div class="subcategories">
          {#if loadingExpanded}
            <div class="subcategory-loading">
              <div class="spinner small"></div>
            </div>
          {:else if cat.subcategories && cat.subcategories.length > 0}
            {#each cat.subcategories as sub (sub.id)}
              <div class="subcategory-item">
                <div class="subcategory-info">
                  {#if sub.icon}
                    <i class="{sub.icon} subcategory-icon"></i>
                  {/if}
                  <span class="subcategory-name">{sub.name}</span>
                  {#if sub.showWarning}
                    <span class="warning-badge" title={$_('mindfuck.blindSpots.lowReadRate') || 'Niedrige Leserate'}>!</span>
                  {/if}
                </div>
                <div class="subcategory-stats">
                  {#if sub.weight !== undefined}
                    <span class="subcategory-weight {getWeightClass(sub.weight)}">{(sub.weight * 100).toFixed(0)}%</span>
                  {:else}
                    <span class="subcategory-count">{sub.primaryValue}</span>
                    {#if sub.secondaryValue !== undefined}
                      <span class="subcategory-divider">/</span>
                      <span class="subcategory-count">{sub.secondaryValue}</span>
                    {/if}
                  {/if}
                </div>
              </div>
            {/each}
          {:else}
            <div class="subcategory-empty">
              {emptyMessage || $_('fnordView.noSubcategories') || 'Keine Unterkategorien'}
            </div>
          {/if}
        </div>
      {/if}
    </button>
  {:else}
    <div class="empty-cards">
      <i class="fa-light fa-chart-bar empty-icon"></i>
      <p>{emptyMessage || $_('fnordView.noData') || 'Keine Daten'}</p>
    </div>
  {/each}
</div>

<style>
  .cards-title {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .title-tooltip {
    cursor: help;
    border-bottom: 1px dotted var(--text-muted);
  }

  /* Category Cards Grid */
  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  .category-card {
    background: linear-gradient(135deg, color-mix(in srgb, var(--cat-color) 25%, var(--bg-base)) 0%, color-mix(in srgb, var(--cat-color) 8%, var(--bg-base)) 100%);
    border: 1px solid color-mix(in srgb, var(--cat-color) 50%, transparent);
    border-left: 3px solid var(--cat-color);
    border-radius: 0.625rem;
    padding: 1rem;
    transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
    text-align: left;
    width: 100%;
    color: inherit;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 15%, transparent);
  }

  .category-card.expandable {
    cursor: pointer;
  }

  .category-card.expandable:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-color: color-mix(in srgb, var(--cat-color) 70%, transparent);
  }

  .category-card:disabled {
    cursor: default;
  }

  .category-card:disabled:hover {
    transform: none;
    box-shadow: none;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.875rem;
  }

  .card-icon-wrapper {
    width: 2.25rem;
    height: 2.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 70%, black));
    border-radius: 0.5rem;
    color: white;
    font-size: 1rem;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 40%, transparent);
    flex-shrink: 0;
  }

  .card-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .expand-icon {
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: transform 0.2s ease;
    flex-shrink: 0;
  }

  .expand-icon.rotated {
    transform: rotate(180deg);
  }

  .card-stats {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .stat-row.secondary {
    margin-top: 0.25rem;
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .stat-value {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .stat-row.secondary .stat-label,
  .stat-row.secondary .stat-value {
    font-size: 0.6875rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .progress-bar {
    height: 6px;
    background-color: color-mix(in srgb, var(--cat-color) 20%, transparent);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 80%, white));
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  /* Expanded state */
  .category-card.expanded {
    grid-column: 1 / -1;
  }

  /* Subcategories */
  .subcategories {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid color-mix(in srgb, var(--cat-color) 20%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .subcategory-loading {
    display: flex;
    justify-content: center;
    padding: 0.5rem;
  }

  .spinner.small {
    width: 1rem;
    height: 1rem;
    border: 2px solid var(--border-default);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.375rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    flex: 1;
  }

  .subcategory-icon {
    font-size: 0.75rem;
    color: var(--cat-color);
    flex-shrink: 0;
  }

  .subcategory-name {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .warning-badge {
    font-size: 0.625rem;
    width: 1rem;
    height: 1rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background-color: var(--accent-warning);
    color: var(--bg-base);
    border-radius: 50%;
    font-weight: 700;
    flex-shrink: 0;
  }

  .subcategory-stats {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .subcategory-count {
    font-weight: 500;
  }

  .subcategory-divider {
    color: var(--text-faint);
  }

  .subcategory-weight {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-weight: 600;
  }

  .subcategory-weight.weight-high {
    background-color: rgba(34, 197, 94, 0.2);
    color: var(--accent-success);
  }

  .subcategory-weight.weight-medium {
    background-color: rgba(251, 191, 36, 0.2);
    color: var(--accent-warning);
  }

  .subcategory-weight.weight-low {
    background-color: var(--bg-overlay);
    color: var(--text-muted);
  }

  .subcategory-empty {
    text-align: center;
    font-size: 0.75rem;
    color: var(--text-muted);
    padding: 0.5rem;
  }

  /* Empty state */
  .empty-cards {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    color: var(--text-muted);
    text-align: center;
  }

  .empty-icon {
    font-size: 2rem;
    margin-bottom: 0.5rem;
    opacity: 0.5;
  }

  .empty-cards p {
    margin: 0;
    font-size: 0.875rem;
  }
</style>
