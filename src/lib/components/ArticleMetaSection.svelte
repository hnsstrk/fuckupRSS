<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import type { ArticleCategory, Tag } from "../stores/state.svelte";
  import type { ArticleKeyword, ArticleCategoryDetailed } from "$lib/types";
  import { ArticleKeywords, ArticleCategories } from "./article";
  import { getCategoryColorVar } from "$lib/utils/articleFormat";
  import EntityBadge from "./EntityBadge.svelte";

  interface EntityInfo {
    id: number;
    name: string;
    entity_type: string;
    normalized_name: string;
    article_count: number;
    mention_count: number | null;
    confidence: number | null;
  }

  let {
    fnordId,
    categories,
    tags,
    articleKeywords,
    articleCategoriesDetailed,
    editingKeywords,
    editingCategories,
    onKeywordsUpdate,
    onCategoriesUpdate,
    onToggleEditingKeywords,
    onToggleEditingCategories,
    onNavigateToKeyword,
  }: {
    fnordId: number;
    categories: ArticleCategory[];
    tags: Tag[];
    articleKeywords: ArticleKeyword[];
    articleCategoriesDetailed: ArticleCategoryDetailed[];
    editingKeywords: boolean;
    editingCategories: boolean;
    onKeywordsUpdate: (keywords: ArticleKeyword[]) => void;
    onCategoriesUpdate: (categories: ArticleCategoryDetailed[]) => void;
    onToggleEditingKeywords: () => void;
    onToggleEditingCategories: () => void;
    onNavigateToKeyword: (tagId: number) => void;
  } = $props();

  // Entity state
  let entities = $state<EntityInfo[]>([]);
  let extractingEntities = $state(false);
  let lastEntityFnordId = $state<number | null>(null);

  // Group entities by type for display
  let entityGroups = $derived(
    entities.reduce(
      (groups, entity) => {
        const type = entity.entity_type;
        if (!groups[type]) groups[type] = [];
        groups[type].push(entity);
        return groups;
      },
      {} as Record<string, EntityInfo[]>,
    ),
  );

  const typeOrder = ["person", "organization", "location", "event"];

  // Load entities when fnordId changes
  $effect(() => {
    const currentId = fnordId;
    if (currentId && currentId !== lastEntityFnordId) {
      lastEntityFnordId = currentId;
      loadEntities(currentId);
    }
  });

  async function loadEntities(id: number) {
    try {
      entities = await invoke<EntityInfo[]>("get_article_entities", { fnordId: id });
    } catch {
      entities = [];
    }
  }

  async function extractEntities() {
    extractingEntities = true;
    try {
      await invoke("extract_entities", { fnordId });
      await loadEntities(fnordId);
    } catch (e) {
      console.error("Failed to extract entities:", e);
    } finally {
      extractingEntities = false;
    }
  }
</script>

<div class="meta-section">
  <div class="section-content">
    <!-- Categories - always show to allow adding when none exist -->
    <div class="meta-row">
      <div class="meta-label">
        {$_("articleView.categories")}
      </div>
      <div class="meta-content">
        {#if articleCategoriesDetailed.length > 0}
          <ArticleCategories
            {fnordId}
            categories={articleCategoriesDetailed}
            editing={editingCategories}
            onUpdate={onCategoriesUpdate}
          />
        {:else if categories.length > 0}
          <!-- Fallback to old display for articles not yet loaded with detailed info -->
          <div class="category-badges">
            {#each categories as cat (cat.sephiroth_id)}
              <span
                class="category-badge"
                style="background-color: {getCategoryColorVar(
                  cat.sephiroth_id,
                  'var(--bg-overlay)',
                )}; color: white"
              >
                {#if cat.icon}<i class="{cat.icon} badge-icon"></i>{/if}
                {cat.name}
              </span>
            {/each}
          </div>
        {:else}
          <!-- No categories - show add option in edit mode -->
          <ArticleCategories
            {fnordId}
            categories={[]}
            editing={editingCategories}
            onUpdate={onCategoriesUpdate}
          />
        {/if}
        <button
          class="edit-toggle"
          onclick={onToggleEditingCategories}
          title="Edit categories"
          aria-label={editingCategories ? "Done editing categories" : "Edit categories"}
        >
          <i class="fa-solid {editingCategories ? 'fa-check' : 'fa-pen'}"></i>
        </button>
      </div>
    </div>

    {#if articleKeywords.length > 0 || tags.length > 0}
      <div class="meta-row">
        <div class="meta-label">
          {$_("articleView.keywords")}
        </div>
        <div class="meta-content">
          {#if articleKeywords.length > 0}
            <ArticleKeywords
              {fnordId}
              keywords={articleKeywords}
              editing={editingKeywords}
              onUpdate={onKeywordsUpdate}
            />
          {:else}
            <!-- Fallback to old display -->
            <div class="tag-list">
              {#each tags as tag (tag.id)}
                <button
                  class="tag-badge clickable"
                  onclick={() => onNavigateToKeyword(tag.id)}
                  title={$_("network.title")}
                >
                  {tag.name}
                </button>
              {/each}
            </div>
          {/if}
          <button
            class="edit-toggle"
            onclick={onToggleEditingKeywords}
            title="Edit keywords"
            aria-label={editingKeywords ? "Done editing keywords" : "Edit keywords"}
          >
            <i class="fa-solid {editingKeywords ? 'fa-check' : 'fa-pen'}"></i>
          </button>
        </div>
      </div>
    {/if}

    <!-- Entities Section -->
    <div class="meta-row">
      <div class="meta-label">
        {$_("entities.title")}
      </div>
      <div class="meta-content">
        {#if entities.length > 0}
          <div class="entity-badges">
            {#each typeOrder as type}
              {#if entityGroups[type]}
                {#each entityGroups[type] as entity (entity.id)}
                  <EntityBadge
                    name={entity.name}
                    entityType={entity.entity_type}
                    mentionCount={entity.mention_count ?? undefined}
                    articleCount={entity.article_count}
                  />
                {/each}
              {/if}
            {/each}
          </div>
        {:else}
          <button
            class="extract-btn"
            onclick={extractEntities}
            disabled={extractingEntities}
          >
            {#if extractingEntities}
              <i class="fa-solid fa-spinner fa-spin"></i>
              {$_("entities.extracting")}
            {:else}
              <i class="fa-solid fa-wand-magic-sparkles"></i>
              {$_("entities.extract")}
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .meta-section {
    padding: 1rem 1.5rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .section-content {
    max-width: 48rem;
    margin: 0 auto;
  }

  .meta-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .meta-row:last-child {
    margin-bottom: 0;
  }

  .meta-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    min-width: 5rem;
    padding-top: 0.25rem;
  }

  .meta-content {
    flex: 1;
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .edit-toggle {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0.25rem;
    font-size: 0.75rem;
    opacity: 0.5;
    transition: opacity 0.2s;
    flex-shrink: 0;
  }

  .edit-toggle:hover {
    opacity: 1;
    color: var(--accent-primary);
  }

  .category-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .category-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.625rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .badge-icon {
    font-size: 0.875rem;
  }

  .tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .tag-badge {
    display: inline-block;
    padding: 0.125rem 0.5rem;
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
    border-radius: 0.25rem;
    font-size: 0.75rem;
    border: none;
  }

  .tag-badge.clickable {
    cursor: pointer;
    transition: all 0.2s;
  }

  .tag-badge.clickable:hover {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .entity-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .extract-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.625rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .extract-btn:hover:not(:disabled) {
    background: var(--bg-overlay);
    color: var(--accent-primary);
    border-color: var(--accent-primary);
  }

  .extract-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  @media print {
    .meta-section,
    .edit-toggle {
      display: none !important;
    }
  }
</style>
