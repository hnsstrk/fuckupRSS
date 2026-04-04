<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { type CategoryRevisionStats } from "../stores/state.svelte";
  import { getCategoryColorVar } from "$lib/utils/articleFormat";
  import Tooltip from "./Tooltip.svelte";
  import { createLogger } from "$lib/logger";

  const log = createLogger("FnordCategoryCards");
  let {
    byCategory,
  }: {
    byCategory: {
      sephiroth_id: number;
      name: string;
      icon: string | null;
      revision_count: number;
      article_count: number;
    }[];
  } = $props();

  // Expanded category state
  let expandedCategoryId = $state<number | null>(null);
  let subcategories = $state<CategoryRevisionStats[]>([]);
  let loadingSubcategories = $state(false);

  async function toggleCategory(categoryId: number) {
    if (expandedCategoryId === categoryId) {
      expandedCategoryId = null;
      subcategories = [];
    } else {
      expandedCategoryId = categoryId;
      loadingSubcategories = true;
      try {
        subcategories = await invoke<CategoryRevisionStats[]>("get_subcategory_stats", {
          mainCategoryId: categoryId,
        });
      } catch (e) {
        log.error("Failed to load subcategories:", e);
        subcategories = [];
      } finally {
        loadingSubcategories = false;
      }
    }
  }
</script>

{#if byCategory.length > 0}
  {@const maxRevisions = Math.max(...byCategory.map((c) => c.revision_count), 1)}
  <div class="stats-section">
    <h3 class="section-title">
      {$_("fnordView.byCategory") || "Nach Kategorie"}
      <Tooltip
        content={$_("fnordView.byCategory.help") ||
          "Verteilung der Revisionen nach Themengebiet. Klicke auf eine Kategorie um Unterkategorien zu sehen."}
      >
        <i class="fa-solid fa-circle-info help-icon"></i>
      </Tooltip>
    </h3>
    <div class="category-cards">
      {#each byCategory as cat (cat.sephiroth_id)}
        {@const barWidth = (cat.revision_count / maxRevisions) * 100}
        {@const isExpanded = expandedCategoryId === cat.sephiroth_id}
        <button
          class="category-card {isExpanded ? 'expanded' : ''}"
          style="--cat-color: {getCategoryColorVar(cat.sephiroth_id)}"
          data-category-id={cat.sephiroth_id}
          onclick={() => toggleCategory(cat.sephiroth_id)}
        >
          <div class="card-header">
            <div class="card-icon-wrapper">
              {#if cat.icon}
                <i class={cat.icon}></i>
              {:else}
                <i class="fa-solid fa-folder"></i>
              {/if}
            </div>
            <span class="card-title-text">{cat.name}</span>
            <i class="fa-solid fa-chevron-down expand-icon {isExpanded ? 'rotated' : ''}"></i>
          </div>
          <div class="card-stats">
            <div class="stat-row">
              <span class="stat-label">{$_("fnordView.revisions") || "Revisionen"}</span>
              <span class="stat-value">{cat.revision_count}</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill" style="width: {barWidth}%"></div>
            </div>
            <div class="stat-row secondary">
              <span class="stat-label">{$_("fnordView.articles") || "Artikel"}</span>
              <span class="stat-value">{cat.article_count}</span>
            </div>
          </div>

          {#if isExpanded}
            <div class="subcategories">
              {#if loadingSubcategories}
                <div class="subcategory-loading">
                  <div class="spinner small"></div>
                </div>
              {:else if subcategories.length > 0}
                {#each subcategories as sub (sub.sephiroth_id)}
                  <div class="subcategory-item">
                    <div class="subcategory-info">
                      {#if sub.icon}
                        <i class="{sub.icon} subcategory-icon"></i>
                      {/if}
                      <span class="subcategory-name">{sub.name}</span>
                    </div>
                    <div class="subcategory-stats">
                      <span class="subcategory-count">{sub.revision_count}</span>
                      <span class="subcategory-divider">/</span>
                      <span class="subcategory-count">{sub.article_count}</span>
                    </div>
                  </div>
                {/each}
              {:else}
                <div class="subcategory-empty">
                  {$_("fnordView.noSubcategories") || "Keine Unterkategorien"}
                </div>
              {/if}
            </div>
          {/if}
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  /* Category Cards */
  .stats-section {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    padding: 1.25rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 1rem 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 1rem;
  }

  .category-card {
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--cat-color) 25%, var(--bg-base)) 0%,
      color-mix(in srgb, var(--cat-color) 8%, var(--bg-base)) 100%
    );
    border: 1px solid color-mix(in srgb, var(--cat-color) 50%, transparent);
    border-left: 3px solid var(--cat-color);
    border-radius: 0.625rem;
    padding: 1rem;
    transition:
      transform 0.15s ease,
      box-shadow 0.15s ease,
      border-color 0.15s ease;
    cursor: pointer;
    text-align: left;
    width: 100%;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 15%, transparent);
  }

  .category-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-color: color-mix(in srgb, var(--cat-color) 70%, transparent);
  }

  .category-card.expanded {
    grid-column: 1 / -1;
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
    background: linear-gradient(
      135deg,
      var(--cat-color),
      color-mix(in srgb, var(--cat-color) 70%, black)
    );
    border-radius: 0.5rem;
    color: white;
    font-size: 1rem;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 40%, transparent);
  }

  .card-title-text {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
    line-height: 1.2;
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
    background: linear-gradient(
      90deg,
      var(--cat-color),
      color-mix(in srgb, var(--cat-color) 80%, white)
    );
    border-radius: 3px;
    transition: width 0.3s ease;
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
    to {
      transform: rotate(360deg);
    }
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.375rem;
    font-size: 0.8125rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .subcategory-icon {
    font-size: 0.75rem;
    color: var(--cat-color);
    opacity: 0.8;
  }

  .subcategory-name {
    color: var(--text-primary);
  }

  .subcategory-stats {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .subcategory-count {
    font-weight: 500;
  }

  .subcategory-divider {
    opacity: 0.5;
  }

  .subcategory-empty {
    text-align: center;
    color: var(--text-muted);
    font-size: 0.75rem;
    padding: 0.5rem;
  }
</style>
