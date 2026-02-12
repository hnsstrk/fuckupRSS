<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import type { ArticleCategoryDetailed, Sephiroth, CorrectionInput } from '$lib/types';
  import { getCategoryColorVar } from '$lib/utils/articleFormat';

  interface Props {
    fnordId: number;
    categories: ArticleCategoryDetailed[];
    editing: boolean;
    onUpdate: (categories: ArticleCategoryDetailed[]) => void;
  }

  let { fnordId, categories, editing, onUpdate }: Props = $props();

  // Available categories for dropdown
  let allCategories = $state<Sephiroth[]>([]);
  let loading = $state(false);
  let showDropdown = $state(false);
  let loadError = $state<string | null>(null);

  // Load all categories when editing starts
  $effect(() => {
    if (editing && allCategories.length === 0) {
      loadCategories();
    }
  });

  // Group categories by parent for dropdown display
  let groupedCategories = $derived.by(() => {
    const mainCats = allCategories.filter((c) => c.parent_id === null);
    const subCats = allCategories.filter((c) => c.parent_id !== null);

    // Filter out already assigned categories
    const assignedIds = new Set(categories.map((c) => c.sephiroth_id));

    return mainCats
      .map((main) => ({
        main,
        subs: subCats.filter((sub) => sub.parent_id === main.id && !assignedIds.has(sub.id)),
      }))
      .filter((group) => group.subs.length > 0 || !assignedIds.has(group.main.id));
  });

  // Check if there are any available categories to add
  let hasAvailableCategories = $derived.by(() => {
    const assignedIds = new Set(categories.map((c) => c.sephiroth_id));
    return allCategories.some((c) => !assignedIds.has(c.id));
  });

  async function loadCategories() {
    loading = true;
    loadError = null;
    try {
      allCategories = await invoke<Sephiroth[]>('get_all_categories');
    } catch (e) {
      console.error('Failed to load categories:', e);
      loadError = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  // Get source icon class
  function getSourceIcon(source: ArticleCategoryDetailed['source']): string {
    switch (source) {
      case 'ai':
        return 'fa-solid fa-robot';
      case 'manual':
        return 'fa-solid fa-user';
      default:
        return 'fa-solid fa-folder';
    }
  }

  // Get source tooltip
  function getSourceLabel(source: ArticleCategoryDetailed['source']): string {
    switch (source) {
      case 'ai':
        return $_('articleView.sourceAi') || 'KI-generiert';
      case 'manual':
        return $_('articleView.sourceManual') || 'Manuell';
      default:
        return source;
    }
  }

  // Add a category to the article
  async function addCategory(category: Sephiroth) {
    loading = true;
    try {
      await invoke('add_article_category', {
        fnordId,
        sephirothId: category.id,
      });

      // Record correction for bias learning
      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: 'category_added',
        new_value: category.name,
        category_id: category.id,
        matching_terms: [], // Manual additions don't have statistical matching terms
      };
      await invoke('record_correction', { correction });

      // Find parent info if this is a subcategory
      const parent = category.parent_id
        ? allCategories.find((c) => c.id === category.parent_id)
        : null;

      // Update local state
      const newCategory: ArticleCategoryDetailed = {
        sephiroth_id: category.id,
        name: category.name,
        icon: category.icon,
        color: category.color,
        source: 'manual',
        confidence: 1.0,
        parent_id: category.parent_id,
        parent_name: parent?.name || null,
        parent_color: parent?.color || null,
      };
      onUpdate([...categories, newCategory]);

      // Close dropdown
      showDropdown = false;
    } catch (e) {
      console.error('Failed to add category:', e);
    } finally {
      loading = false;
    }
  }

  // Remove a category from the article
  async function removeCategory(category: ArticleCategoryDetailed) {
    loading = true;
    try {
      await invoke('remove_article_category', {
        fnordId,
        sephirothId: category.sephiroth_id,
      });

      // Record correction for bias learning
      // Note: matching_terms are not stored with category assignments yet,
      // so we pass an empty array. The backend will still update the general
      // category boost weight. For term-level learning, matching_terms would
      // need to be tracked from the initial statistical analysis.
      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: 'category_removed',
        old_value: category.name,
        category_id: category.sephiroth_id,
        matching_terms: [], // Could be populated if we store this with assignments
      };
      await invoke('record_correction', { correction });

      // Update local state
      onUpdate(categories.filter((c) => c.sephiroth_id !== category.sephiroth_id));
    } catch (e) {
      console.error('Failed to remove category:', e);
    } finally {
      loading = false;
    }
  }

  // Handle click outside to close dropdown
  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.category-dropdown-container')) {
      showDropdown = false;
    }
  }

  // Toggle dropdown
  function toggleDropdown() {
    if (!showDropdown && allCategories.length === 0) {
      loadCategories();
    }
    showDropdown = !showDropdown;
  }

  // Refresh categories from backend when batch processing completes
  async function handleBatchComplete() {
    // Don't refresh while user is editing to avoid overwriting their changes
    if (editing) return;

    try {
      const freshCategories = await invoke<ArticleCategoryDetailed[]>(
        'get_article_categories_detailed',
        { fnordId }
      );
      onUpdate(freshCategories);
    } catch {
      // Ignore errors during background refresh
    }
  }

  onMount(() => {
    window.addEventListener('batch-complete', handleBatchComplete);
  });

  onDestroy(() => {
    window.removeEventListener('batch-complete', handleBatchComplete);
  });
</script>

<svelte:window onclick={handleClickOutside} />

<div class="article-categories">
  <!-- Categories List -->
  <div class="categories-list">
    {#each categories as category (category.sephiroth_id)}
      <div
        class="category-chip"
        class:editable={editing}
        style="--cat-color: {getCategoryColorVar(category.sephiroth_id)}"
      >
        {#if category.icon}
          <i class="category-icon {category.icon}"></i>
        {/if}
        <span class="category-name">{category.name}</span>
        <i class="source-icon {getSourceIcon(category.source)}" title={getSourceLabel(category.source)}></i>
        {#if category.confidence < 1.0}
          <span class="category-confidence" title={$_('articleCategories.confidence') || 'Konfidenz'}>
            {Math.round(category.confidence * 100)}%
          </span>
        {/if}
        {#if editing}
          <button
            type="button"
            class="remove-btn"
            onclick={() => removeCategory(category)}
            disabled={loading}
            title={$_('articleCategories.remove') || 'Entfernen'}
            aria-label={$_('articleCategories.remove') || 'Entfernen'}
          >
            <i class="fa-solid fa-xmark"></i>
          </button>
        {/if}
      </div>
    {/each}

    {#if categories.length === 0 && !editing}
      <span class="no-categories">{$_('articleCategories.none') || 'Keine Kategorien'}</span>
    {/if}
  </div>

  <!-- Add Category Dropdown (Edit Mode Only) -->
  {#if editing}
    <div class="category-dropdown-container">
      <button
        type="button"
        class="add-category-btn"
        onclick={toggleDropdown}
        disabled={loading || !hasAvailableCategories}
      >
        {#if loading}
          <i class="fa-solid fa-spinner fa-spin"></i>
        {:else}
          <i class="fa-solid fa-plus"></i>
        {/if}
        <span>{$_('articleCategories.add') || 'Kategorie hinzufuegen'}</span>
        {#if !loading}
          <i class="fa-solid fa-chevron-down dropdown-chevron" class:open={showDropdown}></i>
        {/if}
      </button>

      {#if showDropdown}
        <div class="category-dropdown">
          {#if loadError}
            <div class="dropdown-error">
              <i class="fa-solid fa-exclamation-triangle"></i>
              <span>{loadError}</span>
            </div>
          {:else if groupedCategories.length === 0}
            <div class="dropdown-empty">
              {$_('articleCategories.allAssigned') || 'Alle Kategorien zugewiesen'}
            </div>
          {:else}
            {#each groupedCategories as group (group.main.id)}
              <div class="category-group">
                <div class="group-header" style="--cat-color: {getCategoryColorVar(group.main.id)}">
                  {#if group.main.icon}
                    <i class="group-icon {group.main.icon}"></i>
                  {/if}
                  <span class="group-name">{group.main.name}</span>
                </div>
                <div class="group-items">
                  {#each group.subs as sub (sub.id)}
                    <button
                      type="button"
                      class="dropdown-item"
                      onclick={() => addCategory(sub)}
                      disabled={loading}
                    >
                      {#if sub.icon}
                        <i class="item-icon {sub.icon}"></i>
                      {/if}
                      <span class="item-name">{sub.name}</span>
                    </button>
                  {/each}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .article-categories {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .categories-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .category-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    background-color: color-mix(in srgb, var(--cat-color) 10%, var(--bg-overlay));
    border: 1px solid var(--cat-color);
    border-radius: 0.375rem;
    font-size: 0.8125rem;
    transition: all 0.2s;
  }

  .category-chip.editable {
    padding-right: 0.25rem;
  }

  .category-chip:hover {
    background-color: color-mix(in srgb, var(--cat-color) 20%, var(--bg-overlay));
  }

  .category-icon {
    font-size: 0.6875rem;
    color: var(--cat-color);
  }

  .category-name {
    color: var(--text-primary);
  }

  .source-icon {
    font-size: 0.5625rem;
    color: var(--text-muted);
    margin-left: 0.125rem;
  }

  .source-icon.fa-robot {
    color: var(--accent-primary);
  }

  .source-icon.fa-user {
    color: var(--accent-success);
  }

  .category-confidence {
    font-size: 0.625rem;
    color: var(--text-muted);
    padding: 0.0625rem 0.25rem;
    background-color: var(--bg-surface);
    border-radius: 0.1875rem;
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    background: none;
    border: none;
    border-radius: 0.25rem;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
    margin-left: 0.125rem;
  }

  .remove-btn:hover:not(:disabled) {
    color: var(--status-error);
    background-color: rgba(239, 68, 68, 0.1);
  }

  .remove-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .remove-btn i {
    font-size: 0.625rem;
  }

  .no-categories {
    font-size: 0.8125rem;
    color: var(--text-muted);
    font-style: italic;
  }

  /* Add Category Dropdown */
  .category-dropdown-container {
    position: relative;
  }

  .add-category-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.625rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    color: var(--text-secondary);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .add-category-btn:hover:not(:disabled) {
    border-color: var(--accent-primary);
    color: var(--text-primary);
  }

  .add-category-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dropdown-chevron {
    font-size: 0.625rem;
    transition: transform 0.2s;
    margin-left: 0.25rem;
  }

  .dropdown-chevron.open {
    transform: rotate(180deg);
  }

  .category-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 100;
    min-width: 220px;
    max-width: 280px;
    margin-top: 0.25rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    max-height: 300px;
    overflow-y: auto;
  }

  .dropdown-error,
  .dropdown-empty {
    padding: 0.75rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
    text-align: center;
  }

  .dropdown-error {
    color: var(--status-error);
  }

  .dropdown-error i {
    margin-right: 0.375rem;
  }

  .category-group {
    border-bottom: 1px solid var(--border-muted);
  }

  .category-group:last-child {
    border-bottom: none;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 0.625rem;
    background-color: var(--bg-overlay);
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--cat-color);
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .group-icon {
    font-size: 0.6875rem;
  }

  .group-items {
    display: flex;
    flex-direction: column;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    width: 100%;
    padding: 0.5rem 0.75rem;
    padding-left: 1.25rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-size: 0.8125rem;
    transition: background-color 0.15s;
  }

  .dropdown-item:hover:not(:disabled) {
    background-color: var(--bg-overlay);
  }

  .dropdown-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .item-icon {
    font-size: 0.6875rem;
    color: var(--text-muted);
    width: 1rem;
    text-align: center;
  }

  .item-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
