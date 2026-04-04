<script lang="ts">
  import { _ } from "svelte-i18n";
  import { SvelteSet } from "svelte/reactivity";
  import KeywordContextTooltip from "../KeywordContextTooltip.svelte";

  // Type definitions (re-exported for shared use)
  export interface CompoundItem {
    id: number;
    original: string;
    components: string[];
    articles_affected: number;
    is_preserved: boolean;
    keyword_type?: string;
  }

  export interface DecisionItem extends CompoundItem {
    decision: "preserved" | "split";
    decided_at: string;
  }

  export type KeywordType = "concept" | "person" | "organization" | "location" | "acronym";
  export type SortColumn = "original" | "components" | "articles" | "type";
  export type SortDirection = "asc" | "desc";
  export type FilterTab = "pending" | "preserved" | "split" | "all";

  const typeIcons: Record<KeywordType, string> = {
    concept: "fa-lightbulb",
    person: "fa-user-tie",
    organization: "fa-building",
    location: "fa-location-dot",
    acronym: "fa-font",
  };

  const typeColors: Record<KeywordType, string> = {
    concept: "var(--text-muted)",
    person: "var(--accent-info)",
    organization: "var(--accent-primary)",
    location: "var(--accent-success)",
    acronym: "var(--accent-warning)",
  };

  export const keywordTypes: KeywordType[] = [
    "concept",
    "person",
    "organization",
    "location",
    "acronym",
  ];

  interface Props {
    paginatedList: (CompoundItem | DecisionItem)[];
    filteredList: (CompoundItem | DecisionItem)[];
    activeFilter: FilterTab;
    loading: boolean;
    splitting: boolean;
    selectAll: boolean;
    selectedIds: SvelteSet<number>;
    sortColumn: SortColumn;
    sortDirection: SortDirection;
    currentPage: number;
    totalPages: number;
    searchQuery: string;
    onsort: (column: SortColumn) => void;
    ontoggleall: () => void;
    ontoggleselection: (id: number) => void;
    onpreserve: (item: CompoundItem) => void;
    onunpreserve: (item: CompoundItem) => void;
    onsplit: (item: CompoundItem) => void;
    ontypechange: (event: Event, item: CompoundItem) => void;
    onnavigatetonetwork: (id: number) => void;
    onpaginate: (page: number) => void;
  }

  let {
    paginatedList,
    filteredList,
    activeFilter,
    loading,
    splitting,
    selectAll,
    selectedIds,
    sortColumn,
    sortDirection,
    currentPage,
    totalPages,
    searchQuery,
    onsort,
    ontoggleall,
    ontoggleselection,
    onpreserve,
    onunpreserve,
    onsplit,
    ontypechange,
    onnavigatetonetwork,
    onpaginate,
  }: Props = $props();

  function getSortIcon(column: SortColumn): string {
    if (sortColumn !== column) return "fa-solid fa-sort";
    return sortDirection === "asc" ? "fa-solid fa-sort-up" : "fa-solid fa-sort-down";
  }
</script>

<div class="table-container">
  {#if loading && paginatedList.length === 0}
    <div class="loading-state">
      <i class="fa-solid fa-spinner fa-spin fa-2x"></i>
      <span>{$_("compound.loading") || "Loading compound keywords..."}</span>
    </div>
  {:else if filteredList.length === 0}
    <div class="empty-state">
      <i class="fa-solid fa-inbox fa-3x"></i>
      <span>
        {#if activeFilter === "pending"}
          {$_("compound.noPending") || "No pending compound keywords"}
        {:else if activeFilter === "preserved"}
          {$_("compound.noPreserved") || "No preserved keywords"}
        {:else if activeFilter === "split"}
          {$_("compound.noSplit") || "No split keywords yet"}
        {:else}
          {$_("compound.noCompounds") || "No compound keywords found"}
        {/if}
      </span>
    </div>
  {:else}
    <table class="compound-table">
      <thead>
        <tr>
          {#if activeFilter === "pending"}
            <th class="checkbox-col">
              <input
                type="checkbox"
                checked={selectAll}
                onchange={ontoggleall}
                disabled={splitting}
                title={$_("compound.selectAll") || "Select all"}
              />
            </th>
          {/if}
          <th class="sortable" onclick={() => onsort("original")}>
            <span>{$_("compound.colOriginal") || "Original"}</span>
            <i class={getSortIcon("original")}></i>
          </th>
          <th class="sortable" onclick={() => onsort("components")}>
            <span>{$_("compound.colComponents") || "Components"}</span>
            <i class={getSortIcon("components")}></i>
          </th>
          <th class="sortable numeric" onclick={() => onsort("articles")}>
            <span>{$_("compound.colArticles") || "Articles"}</span>
            <i class={getSortIcon("articles")}></i>
          </th>
          <th class="sortable" onclick={() => onsort("type")}>
            <span>{$_("compound.colType") || "Type"}</span>
            <i class={getSortIcon("type")}></i>
          </th>
          <th class="actions-col">
            <span>{$_("compound.colActions") || "Actions"}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {#each paginatedList as item (item.id)}
          {@const isDecision = "decision" in item}
          {@const decision = isDecision ? (item as DecisionItem).decision : null}
          <tr
            class:preserved={item.is_preserved}
            class:split={decision === "split"}
            class:selected={selectedIds.has(item.id)}
          >
            {#if activeFilter === "pending"}
              <td class="checkbox-col">
                {#if !item.is_preserved && !isDecision}
                  <input
                    type="checkbox"
                    checked={selectedIds.has(item.id)}
                    onchange={() => ontoggleselection(item.id)}
                    disabled={splitting}
                  />
                {/if}
              </td>
            {/if}
            <td class="original-col">
              <KeywordContextTooltip
                keywordId={item.id}
                keywordName={item.original}
                onclick={() => onnavigatetonetwork(item.id)}
              >
                <span class="original-name clickable">{item.original}</span>
              </KeywordContextTooltip>
              {#if item.is_preserved}
                <span
                  class="status-badge preserved"
                  title={$_("compound.statusPreserved") || "Preserved"}
                >
                  <i class="fa-solid fa-shield-check"></i>
                </span>
              {/if}
              {#if decision === "split"}
                <span class="status-badge split" title={$_("compound.statusSplit") || "Split"}>
                  <i class="fa-solid fa-check"></i>
                </span>
              {/if}
            </td>
            <td class="components-col">
              <span class="components-list">
                {#each item.components as comp, i (comp)}
                  <span class="component-tag">{comp}</span>
                  {#if i < item.components.length - 1}
                    <span class="component-separator">+</span>
                  {/if}
                {/each}
              </span>
            </td>
            <td class="articles-col numeric">
              <span class="article-count">{item.articles_affected}</span>
            </td>
            <td class="type-col">
              {#if !isDecision}
                <div class="type-select-wrapper">
                  <select
                    class="type-select"
                    value={item.keyword_type || "concept"}
                    onchange={(e) => ontypechange(e, item as CompoundItem)}
                    style="color: {typeColors[(item.keyword_type || 'concept') as KeywordType]}"
                  >
                    {#each keywordTypes as type (type)}
                      <option value={type}>
                        {$_(`network.keywordType.${type}`) || type}
                      </option>
                    {/each}
                  </select>
                  <i
                    class="fa-solid {typeIcons[
                      (item.keyword_type || 'concept') as KeywordType
                    ]} type-icon"
                    style="color: {typeColors[(item.keyword_type || 'concept') as KeywordType]}"
                  ></i>
                </div>
              {:else}
                <span class="type-badge {item.keyword_type || 'concept'}">
                  <i class="fa-solid {typeIcons[(item.keyword_type || 'concept') as KeywordType]}"
                  ></i>
                  {$_(`network.keywordType.${item.keyword_type || "concept"}`) ||
                    item.keyword_type ||
                    "concept"}
                </span>
              {/if}
            </td>
            <td class="actions-col">
              {#if !isDecision}
                <div class="action-buttons">
                  <!-- Shield button: Preserve -->
                  {#if item.is_preserved}
                    <button
                      class="action-btn unpreserve"
                      onclick={() => onunpreserve(item as CompoundItem)}
                      title={$_("compound.unpreserve") || "Remove protection"}
                      disabled={splitting}
                    >
                      <i class="fa-solid fa-shield-xmark"></i>
                    </button>
                  {:else}
                    <button
                      class="action-btn preserve"
                      onclick={() => onpreserve(item as CompoundItem)}
                      title={$_("compound.preserve") || "Preserve (keep as-is)"}
                      disabled={splitting}
                    >
                      <i class="fa-solid fa-shield"></i>
                    </button>
                  {/if}

                  <!-- Scissors button: Split -->
                  {#if !item.is_preserved}
                    <button
                      class="action-btn split"
                      onclick={() => onsplit(item as CompoundItem)}
                      title={$_("compound.split") || "Split into components"}
                      disabled={splitting}
                    >
                      <i class="fa-solid fa-scissors"></i>
                    </button>
                  {/if}
                </div>
              {:else}
                <span class="decision-info">
                  {$_("compound.alreadySplit") || "Already split"}
                </span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<!-- Pagination -->
{#if totalPages > 1}
  <div class="pagination">
    <button
      class="page-btn"
      onclick={() => onpaginate(1)}
      disabled={currentPage === 1}
      aria-label="First page"
    >
      <i class="fa-solid fa-angles-left"></i>
    </button>
    <button
      class="page-btn"
      onclick={() => onpaginate(currentPage - 1)}
      disabled={currentPage === 1}
      aria-label="Previous page"
    >
      <i class="fa-solid fa-angle-left"></i>
    </button>

    <span class="page-info">
      {$_("compound.pageInfo", { values: { current: currentPage, total: totalPages } }) ||
        `Page ${currentPage} of ${totalPages}`}
    </span>

    <button
      class="page-btn"
      onclick={() => onpaginate(currentPage + 1)}
      disabled={currentPage === totalPages}
      aria-label="Next page"
    >
      <i class="fa-solid fa-angle-right"></i>
    </button>
    <button
      class="page-btn"
      onclick={() => onpaginate(totalPages)}
      disabled={currentPage === totalPages}
      aria-label="Last page"
    >
      <i class="fa-solid fa-angles-right"></i>
    </button>
  </div>
{/if}

<!-- Summary -->
<div class="summary">
  <span class="summary-text">
    {filteredList.length}
    {$_("compound.items") || "items"}
    {#if searchQuery}
      ({$_("compound.filtered") || "filtered"})
    {/if}
  </span>
</div>

<style>
  /* Table */
  .table-container {
    flex: 1;
    overflow: auto;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 1rem;
    color: var(--text-muted);
  }

  .empty-state i {
    opacity: 0.3;
  }

  .compound-table {
    width: 100%;
    border-collapse: collapse;
  }

  .compound-table thead {
    position: sticky;
    top: 0;
    z-index: 10;
    background-color: var(--bg-surface);
  }

  .compound-table th {
    padding: 0.75rem 1rem;
    text-align: left;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    color: var(--text-muted);
    border-bottom: 2px solid var(--border-default);
    user-select: none;
  }

  .compound-table th.sortable {
    cursor: pointer;
    transition: color 0.2s;
  }

  .compound-table th.sortable:hover {
    color: var(--text-primary);
  }

  .compound-table th span {
    margin-right: 0.5rem;
  }

  .compound-table th i {
    font-size: 0.625rem;
    opacity: 0.5;
  }

  .compound-table th.sortable:hover i {
    opacity: 1;
  }

  .compound-table th.numeric {
    text-align: right;
  }

  .checkbox-col {
    width: 40px;
    text-align: center;
  }

  .actions-col {
    width: 120px;
    text-align: center;
  }

  .compound-table td {
    padding: 0.625rem 1rem;
    font-size: 0.875rem;
    color: var(--text-primary);
    border-bottom: 1px solid var(--border-muted);
    vertical-align: middle;
  }

  .compound-table tr:hover {
    background-color: var(--bg-overlay);
  }

  .compound-table tr.selected {
    background-color: rgba(137, 180, 250, 0.1);
  }

  .compound-table tr.preserved {
    opacity: 0.7;
    background-color: rgba(166, 227, 161, 0.05);
  }

  .compound-table tr.split {
    opacity: 0.6;
    background-color: rgba(249, 226, 175, 0.05);
  }

  /* Columns */
  .original-col {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .original-name {
    font-weight: 500;
  }

  .original-name.clickable {
    cursor: pointer;
    transition: color 0.15s;
  }

  .original-name.clickable:hover {
    color: var(--accent-primary);
    text-decoration: underline;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 0.25rem;
    font-size: 0.625rem;
  }

  .status-badge.preserved {
    background-color: rgba(166, 227, 161, 0.2);
    color: var(--status-success);
  }

  .status-badge.split {
    background-color: rgba(249, 226, 175, 0.2);
    color: var(--status-warning);
  }

  .components-list {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-wrap: wrap;
  }

  .component-tag {
    display: inline-block;
    padding: 0.125rem 0.5rem;
    background-color: var(--bg-overlay);
    border-radius: 0.25rem;
    font-size: 0.8125rem;
    color: var(--accent-success);
  }

  .component-separator {
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .articles-col.numeric {
    text-align: right;
  }

  .article-count {
    display: inline-block;
    min-width: 2rem;
    padding: 0.125rem 0.5rem;
    background-color: var(--bg-overlay);
    border-radius: 0.25rem;
    text-align: center;
    font-weight: 500;
    font-size: 0.8125rem;
  }

  .type-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    text-transform: capitalize;
  }

  .type-badge i {
    font-size: 0.625rem;
  }

  .type-badge.concept {
    background-color: rgba(166, 173, 200, 0.2);
    color: var(--text-muted);
  }

  .type-badge.person {
    background-color: rgba(137, 180, 250, 0.2);
    color: var(--accent-info);
  }

  .type-badge.organization {
    background-color: rgba(203, 166, 247, 0.2);
    color: var(--accent-primary);
  }

  .type-badge.location {
    background-color: rgba(166, 227, 161, 0.2);
    color: var(--accent-success);
  }

  .type-badge.acronym {
    background-color: rgba(250, 179, 135, 0.2);
    color: var(--accent-warning);
  }

  /* Type Select Dropdown */
  .type-col {
    width: 130px;
  }

  .type-select-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .type-icon {
    position: absolute;
    left: 0.5rem;
    font-size: 0.75rem;
    pointer-events: none;
  }

  .type-select {
    appearance: none;
    padding: 0.25rem 1.5rem 0.25rem 1.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    background-color: var(--bg-overlay);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    text-transform: capitalize;
    min-width: 100px;
  }

  .type-select:hover {
    border-color: var(--accent-primary);
    background-color: var(--bg-surface);
  }

  .type-select:focus {
    outline: none;
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px rgba(137, 180, 250, 0.2);
  }

  .type-select-wrapper::after {
    content: "\f078";
    font-family: "Font Awesome 6 Pro";
    font-size: 0.5rem;
    font-weight: 900;
    position: absolute;
    right: 0.5rem;
    color: var(--text-muted);
    pointer-events: none;
  }

  /* Action Buttons */
  .action-buttons {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-surface);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-btn:hover:not(:disabled) {
    transform: scale(1.05);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-btn.preserve:hover:not(:disabled) {
    background-color: var(--status-success);
    border-color: var(--status-success);
    color: white;
  }

  .action-btn.unpreserve {
    color: var(--status-success);
  }

  .action-btn.unpreserve:hover:not(:disabled) {
    background-color: var(--bg-overlay);
    border-color: var(--text-muted);
    color: var(--text-muted);
  }

  .action-btn.split:hover:not(:disabled) {
    background-color: var(--status-error);
    border-color: var(--status-error);
    color: white;
  }

  .decision-info {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-style: italic;
  }

  /* Pagination */
  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-top: 1px solid var(--border-default);
    background-color: var(--bg-surface);
  }

  .page-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .page-btn:hover:not(:disabled) {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .page-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .page-info {
    padding: 0 1rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  /* Summary */
  .summary {
    padding: 0.5rem 1rem;
    border-top: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .summary-text {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Checkbox styling */
  .compound-table input[type="checkbox"] {
    width: 1rem;
    height: 1rem;
    cursor: pointer;
    accent-color: var(--accent-primary);
  }

  .compound-table input[type="checkbox"]:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
</style>
