<script lang="ts">
  import { _ } from "svelte-i18n";
  import Tooltip from "../Tooltip.svelte";
  import { getCategoryColorVar } from "$lib/utils/articleFormat";
  import type { KeywordCategory } from "../../types";

  interface Props {
    keywordCategories: KeywordCategory[];
  }

  let { keywordCategories }: Props = $props();

  function getWeightClass(weight: number): string {
    if (weight >= 0.7) return "weight-high";
    if (weight >= 0.4) return "weight-medium";
    return "weight-low";
  }

  // Group categories by main category
  let groupedCategories = $derived.by(() => {
    if (keywordCategories.length === 0) return [];

    // Separate main categories (parent_id is null) from subcategories
    const mainCats = keywordCategories.filter((c) => c.parent_id === null);
    const subCats = keywordCategories.filter((c) => c.parent_id !== null);

    // Group subcategories by their parent
    const subsByParent = subCats.reduce(
      (acc, cat) => {
        const key = cat.parent_name || "Sonstige";
        if (!acc[key]) acc[key] = [];
        acc[key].push(cat);
        return acc;
      },
      {} as Record<string, typeof keywordCategories>,
    );

    // Build result: main categories with their subcategories
    const result: Array<{
      id: number;
      name: string;
      icon: string | null;
      color: string | null;
      weight: number;
      subcategories: typeof keywordCategories;
    }> = [];

    // Add main categories that are directly assigned
    for (const main of mainCats) {
      const subs = subsByParent[main.name] || [];
      delete subsByParent[main.name];
      result.push({
        id: main.sephiroth_id,
        name: main.name,
        icon: main.icon,
        color: main.color,
        weight: main.weight + subs.reduce((sum, s) => sum + s.weight, 0),
        subcategories: subs,
      });
    }

    // Add subcategories whose main category is not directly assigned
    for (const [parentName, subs] of Object.entries(subsByParent)) {
      const firstSub = subs[0];
      result.push({
        id: firstSub.parent_id || 0,
        name: parentName,
        icon: firstSub.parent_icon,
        color: firstSub.color,
        weight: subs.reduce((sum, s) => sum + s.weight, 0),
        subcategories: subs,
      });
    }

    return result.sort((a, b) => b.weight - a.weight);
  });

  let maxWeight = $derived(Math.max(...groupedCategories.map((c) => c.weight), 0.01));
</script>

{#if groupedCategories.length > 0}
  <div class="detail-section">
    <h4 class="section-title">
      {$_("network.categories")}
      <Tooltip content={$_("network.categoriesHelp")}>
        <i class="fa-solid fa-circle-info help-icon"></i>
      </Tooltip>
    </h4>
    <div class="category-cards">
      {#each groupedCategories as group (group.id)}
        {@const barWidth = (group.weight / maxWeight) * 100}
        <div class="category-card" style="--cat-color: {getCategoryColorVar(group.id)}">
          <div class="card-header">
            <div class="card-icon-wrapper">
              <i class={group.icon || "fa-solid fa-folder"}></i>
            </div>
            <span class="card-title">{group.name}</span>
          </div>
          <div class="card-stats">
            <div class="stat-row">
              <span class="stat-label">{$_("network.weight") || "Gewicht"}</span>
              <span class="stat-value">{(group.weight * 100).toFixed(0)}%</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill" style="width: {barWidth}%"></div>
            </div>
          </div>
          {#if group.subcategories.length > 0}
            <div class="subcategories">
              {#each group.subcategories as cat (cat.sephiroth_id)}
                <div class="subcategory-item">
                  <div class="subcategory-info">
                    <i class="{cat.icon || 'fa-solid fa-folder'} subcategory-icon"></i>
                    <span class="subcategory-name">{cat.name}</span>
                  </div>
                  <span class="subcategory-weight {getWeightClass(cat.weight)}"
                    >{(cat.weight * 100).toFixed(0)}%</span
                  >
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}

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

  .help-icon {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Category Cards (matching FnordView) */
  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 0.75rem;
  }

  .category-card {
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--cat-color) 15%, var(--bg-base)) 0%,
      var(--bg-base) 100%
    );
    border: 1px solid color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-radius: 0.5rem;
    padding: 0.75rem;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.625rem;
  }

  .card-icon-wrapper {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(
      135deg,
      var(--cat-color),
      color-mix(in srgb, var(--cat-color) 70%, black)
    );
    border-radius: 0.375rem;
    color: var(--text-on-accent);
    font-size: 0.8125rem;
    box-shadow: 0 2px 6px color-mix(in srgb, var(--cat-color) 40%, transparent);
  }

  .card-title {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-stats {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .stat-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .stat-value {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .progress-bar {
    height: 4px;
    background-color: color-mix(in srgb, var(--cat-color) 20%, transparent);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(
      90deg,
      var(--cat-color),
      color-mix(in srgb, var(--cat-color) 80%, white)
    );
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  /* Subcategories in cards */
  .subcategories {
    margin-top: 0.625rem;
    padding-top: 0.625rem;
    border-top: 1px solid color-mix(in srgb, var(--cat-color) 20%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.25rem 0.5rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.25rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    min-width: 0;
    flex: 1;
  }

  .subcategory-icon {
    font-size: 0.625rem;
    color: var(--cat-color);
    flex-shrink: 0;
  }

  .subcategory-name {
    font-size: 0.6875rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .subcategory-weight {
    font-size: 0.5625rem;
    padding: 0.125rem 0.25rem;
    border-radius: 0.1875rem;
    font-weight: 600;
    flex-shrink: 0;
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
</style>
