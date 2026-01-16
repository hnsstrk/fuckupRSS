<script lang="ts">
  /**
   * CategoryBadge - Reusable category badge component
   *
   * Uses theme-aware CSS variables for the 6 main categories (Sephiroth).
   * Subcategories inherit their parent's color.
   *
   * Category IDs:
   * 1: Wissen & Technologie (cyan)
   * 2: Politik & Gesellschaft (purple)
   * 3: Wirtschaft (gold)
   * 4: Umwelt & Gesundheit (green)
   * 5: Sicherheit (orange)
   * 6: Kultur & Leben (pink)
   */

  interface Props {
    /** Category ID (1-6 for main categories, or subcategory ID) */
    categoryId: number;
    /** Category name to display */
    name: string;
    /** Optional icon class (e.g., 'fa-solid fa-laptop') */
    icon?: string | null;
    /** Parent category ID for subcategories (determines color) */
    parentId?: number | null;
    /** Size variant */
    size?: 'xs' | 'sm' | 'md';
    /** Whether to show the icon */
    showIcon?: boolean;
    /** Click handler */
    onclick?: () => void;
    /** Whether badge is clickable */
    clickable?: boolean;
  }

  let {
    categoryId,
    name,
    icon = null,
    parentId = null,
    size = 'sm',
    showIcon = true,
    onclick,
    clickable = false
  }: Props = $props();

  // Determine which main category color to use
  // Subcategories inherit their parent's color
  const mainCategoryId = $derived(parentId || categoryId);

  // Clamp to valid range (1-6)
  const colorId = $derived(Math.max(1, Math.min(6, mainCategoryId)));

  function handleClick() {
    if (clickable && onclick) {
      onclick();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.key === 'Enter' || e.key === ' ') && clickable && onclick) {
      e.preventDefault();
      onclick();
    }
  }
</script>

{#if clickable}
  <button
    type="button"
    class="category-badge size-{size} clickable"
    class:subcategory={parentId !== null}
    style="
      --cat-color: var(--category-{colorId});
      --cat-bg: var(--category-{colorId}-bg);
      --cat-border: var(--category-{colorId}-border);
    "
    onclick={handleClick}
  >
    {#if showIcon && icon}
      <i class="badge-icon {icon}"></i>
    {/if}
    <span class="badge-name">{name}</span>
  </button>
{:else}
  <span
    class="category-badge size-{size}"
    class:subcategory={parentId !== null}
    style="
      --cat-color: var(--category-{colorId});
      --cat-bg: var(--category-{colorId}-bg);
      --cat-border: var(--category-{colorId}-border);
    "
  >
    {#if showIcon && icon}
      <i class="badge-icon {icon}"></i>
    {/if}
    <span class="badge-name">{name}</span>
  </span>
{/if}

<style>
  .category-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    background-color: var(--cat-bg);
    border: 1px solid var(--cat-border);
    border-radius: 0.375rem;
    font-weight: 500;
    transition: all 0.15s ease;
    white-space: nowrap;
    overflow: hidden;
  }

  /* Size variants */
  .size-xs {
    padding: 0.125rem 0.375rem;
    font-size: 0.6875rem;
    gap: 0.25rem;
  }

  .size-sm {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
  }

  .size-md {
    padding: 0.375rem 0.625rem;
    font-size: 0.8125rem;
  }

  /* Clickable state */
  .clickable {
    cursor: pointer;
  }

  .clickable:hover {
    background-color: var(--cat-border);
    border-color: var(--cat-color);
  }

  .clickable:focus-visible {
    outline: 2px solid var(--cat-color);
    outline-offset: 1px;
  }

  /* Subcategory indicator - slightly muted */
  .subcategory {
    opacity: 0.9;
  }

  /* Icon */
  .badge-icon {
    font-size: 0.75em;
    color: var(--cat-color);
    flex-shrink: 0;
  }

  .size-xs .badge-icon {
    font-size: 0.625rem;
  }

  /* Name */
  .badge-name {
    color: var(--text-primary);
    text-overflow: ellipsis;
    overflow: hidden;
  }
</style>
