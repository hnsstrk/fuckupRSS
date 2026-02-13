<script lang="ts">
  import { _ } from "svelte-i18n";
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { Keyword, KeywordType } from "../types";
  import KeywordContextTooltip from "./KeywordContextTooltip.svelte";
  import { formatChangedDate } from "$lib/utils/articleFormat";

  // Props
  interface Props {
    onKeywordSelect: (id: number) => void;
    onShowKeywordArticles?: (id: number, name: string) => void;
  }

  let { onKeywordSelect, onShowKeywordArticles }: Props = $props();

  // Type icons and colors (using theme CSS variables)
  const typeConfig: Record<KeywordType, { icon: string; color: string }> = {
    concept: { icon: "fa-solid fa-lightbulb", color: "var(--text-muted)" },
    person: { icon: "fa-solid fa-user", color: "var(--accent-info)" },
    organization: { icon: "fa-solid fa-building", color: "var(--accent-primary)" },
    location: { icon: "fa-solid fa-location-dot", color: "var(--accent-success)" },
    acronym: { icon: "fa-solid fa-a", color: "var(--accent-warning)" },
  };

  // State
  let keywords = $state<Keyword[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Untyped keywords detection
  let untypedCount = $state(0);
  let detectingTypes = $state(false);

  // Sorting
  type SortColumn = "name" | "keyword_type" | "article_count" | "first_seen" | "last_used";
  type SortDirection = "asc" | "desc";
  let sortColumn = $state<SortColumn>("article_count");
  let sortDirection = $state<SortDirection>("desc");

  // Filtering
  let minArticleCount = $state(0);
  let searchQuery = $state("");
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  // Pagination
  let offset = $state(0);
  let hasMore = $state(true);
  const limit = 50;

  // Derived: filtered and sorted keywords
  let displayedKeywords = $derived.by(() => {
    let result = [...keywords];

    // Apply filters
    if (minArticleCount > 0) {
      result = result.filter((k) => k.article_count >= minArticleCount);
    }

    // Sort
    result.sort((a, b) => {
      let comparison = 0;
      switch (sortColumn) {
        case "name":
          comparison = a.name.localeCompare(b.name);
          break;
        case "keyword_type":
          comparison = (a.keyword_type || "concept").localeCompare(b.keyword_type || "concept");
          break;
        case "article_count":
          comparison = a.article_count - b.article_count;
          break;
        case "first_seen":
          if (!a.first_seen && !b.first_seen) comparison = 0;
          else if (!a.first_seen) comparison = 1;
          else if (!b.first_seen) comparison = -1;
          else comparison = new Date(a.first_seen).getTime() - new Date(b.first_seen).getTime();
          break;
        case "last_used":
          if (!a.last_used && !b.last_used) comparison = 0;
          else if (!a.last_used) comparison = 1;
          else if (!b.last_used) comparison = -1;
          else comparison = new Date(a.last_used).getTime() - new Date(b.last_used).getTime();
          break;
      }
      return sortDirection === "asc" ? comparison : -comparison;
    });

    return result;
  });

  async function loadKeywords(reset = false) {
    if (loading) return;
    loading = true;
    error = null;

    if (reset) {
      offset = 0;
      keywords = [];
      hasMore = true;
    }

    try {
      let newKeywords: Keyword[];

      if (searchQuery.trim()) {
        // Use search API when there's a query
        newKeywords = await invoke<Keyword[]>("search_keywords", {
          query: searchQuery,
          limit: limit,
        });
        // Search doesn't support pagination, so no more results
        hasMore = false;
      } else {
        // Use regular paginated API
        newKeywords = await invoke<Keyword[]>("get_keywords", {
          limit,
          offset,
        });
        if (newKeywords.length < limit) {
          hasMore = false;
        }
        offset += newKeywords.length;
      }

      keywords = reset ? newKeywords : [...keywords, ...newKeywords];
    } catch (e) {
      error = String(e);
      console.error("Failed to load keywords:", e);
    } finally {
      loading = false;
    }
  }

  function handleSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);

    searchTimeout = setTimeout(() => {
      loadKeywords(true);
    }, 300);
  }

  function clearSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchQuery = "";
    loadKeywords(true);
  }

  function handleSort(column: SortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = column;
      sortDirection = column === "name" ? "asc" : "desc";
    }
  }

  function getSortIcon(column: SortColumn): string {
    if (sortColumn !== column) return "fa-solid fa-sort";
    return sortDirection === "asc" ? "fa-solid fa-sort-up" : "fa-solid fa-sort-down";
  }

  function handleRowClick(keyword: Keyword) {
    onKeywordSelect(keyword.id);
  }

  function handleKeydown(event: KeyboardEvent, keyword: Keyword) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onKeywordSelect(keyword.id);
    }
  }

  function handleShowArticles(event: MouseEvent, keyword: Keyword) {
    event.stopPropagation();
    if (onShowKeywordArticles) {
      onShowKeywordArticles(keyword.id, keyword.name);
    }
  }

  function getTypeLabel(type: KeywordType): string {
    const key = `network.keywordType.${type}`;
    return $_?.(key) || type;
  }

  function getTypeConfig(type: KeywordType | undefined) {
    return typeConfig[type || "concept"] || typeConfig.concept;
  }

  function handleMinArticleChange(e: Event) {
    const target = e.target as HTMLInputElement;
    minArticleCount = parseInt(target.value) || 0;
  }

  async function loadUntypedCount() {
    try {
      untypedCount = await invoke<number>("count_untyped_keywords");
    } catch (e) {
      console.error("Failed to load untyped count:", e);
      untypedCount = 0;
    }
  }

  async function detectUntypedKeywords() {
    if (detectingTypes || untypedCount === 0) return;
    detectingTypes = true;

    try {
      await invoke("update_untyped_keywords");
      await loadUntypedCount();
      await loadKeywords(true);
    } catch (e) {
      console.error("Failed to detect keyword types:", e);
    } finally {
      detectingTypes = false;
    }
  }

  async function handleBatchComplete() {
    await loadKeywords(true);
    await loadUntypedCount();
  }

  async function handleKeywordsChanged() {
    await loadKeywords(true);
    await loadUntypedCount();
  }

  onMount(() => {
    window.addEventListener("batch-complete", handleBatchComplete);
    window.addEventListener("keywords-changed", handleKeywordsChanged);
    loadKeywords(true);
    loadUntypedCount();
  });

  onDestroy(() => {
    window.removeEventListener("batch-complete", handleBatchComplete);
    window.removeEventListener("keywords-changed", handleKeywordsChanged);
  });
</script>

<div class="keyword-table-container">
  <!-- Filters -->
  <div class="table-filters">
    <div class="filter-group">
      <div class="search-box">
        <i class="fa-solid fa-search search-icon"></i>
        <input
          type="text"
          bind:value={searchQuery}
          oninput={handleSearch}
          placeholder={$_("network.searchPlaceholder") || "Stichwort suchen..."}
          class="search-input"
        />
        {#if searchQuery}
          <button onclick={clearSearch} class="clear-btn" aria-label="Clear search">
            <i class="fa-solid fa-xmark"></i>
          </button>
        {/if}
      </div>
    </div>

    <div class="filter-group">
      <label class="filter-label">
        <span>{$_("network.minArticles") || "Min. Artikel"}:</span>
        <input
          type="number"
          min="0"
          value={minArticleCount}
          onchange={handleMinArticleChange}
          class="min-articles-input"
        />
      </label>
      {#if minArticleCount > 0}
        <button onclick={() => (minArticleCount = 0)} class="clear-filter-btn">
          <i class="fa-solid fa-xmark"></i>
          {$_("network.clearFilter") || "Filter loeschen"}
        </button>
      {/if}
    </div>

    <!-- Detect untyped keywords -->
    {#if untypedCount > 0}
      <div class="filter-group filter-group-right">
        <button
          onclick={detectUntypedKeywords}
          disabled={detectingTypes}
          class="detect-types-btn"
          title={$_("network.detectTypesTitle") ||
            "Erkennt Keyword-Typen fuer noch nicht klassifizierte Keywords"}
        >
          {#if detectingTypes}
            <i class="fa-solid fa-spinner fa-spin"></i>
          {:else}
            <i class="fa-solid fa-wand-magic-sparkles"></i>
          {/if}
          <span>{$_("network.detectTypes") || "Typen erkennen"}</span>
          <span class="untyped-badge">{untypedCount}</span>
        </button>
      </div>
    {/if}
  </div>

  <!-- Table -->
  <div class="table-wrapper">
    {#if error}
      <div class="error-message">
        <i class="fa-solid fa-triangle-exclamation"></i>
        {error}
      </div>
    {:else}
      <table class="keyword-table">
        <thead>
          <tr>
            <th class="sortable" onclick={() => handleSort("name")}>
              <span>{$_("network.name") || "Name"}</span>
              <i class={getSortIcon("name")}></i>
            </th>
            <th class="sortable type-col" onclick={() => handleSort("keyword_type")}>
              <span>{$_("network.type") || "Typ"}</span>
              <i class={getSortIcon("keyword_type")}></i>
            </th>
            <th class="sortable numeric" onclick={() => handleSort("article_count")}>
              <span>{$_("network.articles") || "Artikel"}</span>
              <i class={getSortIcon("article_count")}></i>
            </th>
            <th class="sortable" onclick={() => handleSort("first_seen")}>
              <span>{$_("network.firstSeen") || "Erstmals"}</span>
              <i class={getSortIcon("first_seen")}></i>
            </th>
            <th class="sortable" onclick={() => handleSort("last_used")}>
              <span>{$_("network.lastUsed") || "Zuletzt"}</span>
              <i class={getSortIcon("last_used")}></i>
            </th>
            <th class="actions-col">
              <span>{$_("network.actions") || "Aktionen"}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {#if loading && keywords.length === 0}
            <tr class="loading-row">
              <td colspan="6">
                <div class="loading-indicator">
                  <i class="fa-solid fa-spinner fa-spin"></i>
                  <span>{$_("network.loading") || "Lade..."}</span>
                </div>
              </td>
            </tr>
          {:else if displayedKeywords.length === 0}
            <tr class="empty-row">
              <td colspan="6">
                <div class="empty-message">
                  <i class="fa-solid fa-inbox"></i>
                  <span>{$_("network.noResults") || "Keine Ergebnisse gefunden"}</span>
                </div>
              </td>
            </tr>
          {:else}
            {#each displayedKeywords as keyword (keyword.id)}
              <tr
                class="keyword-row"
                onclick={() => handleRowClick(keyword)}
                onkeydown={(e) => handleKeydown(e, keyword)}
                tabindex="0"
                role="button"
                aria-label={`${$_("network.selectKeyword") || "Keyword auswaehlen"}: ${keyword.name}`}
              >
                <td class="name-cell">
                  <KeywordContextTooltip keywordId={keyword.id} keywordName={keyword.name}>
                    <span class="keyword-name">{keyword.name}</span>
                  </KeywordContextTooltip>
                  {#if keyword.is_canonical === false}
                    <span class="alias-badge" title="Alias">
                      <i class="fa-solid fa-link"></i>
                    </span>
                  {/if}
                </td>
                <td class="type-cell">
                  <span
                    class="type-badge"
                    style="color: {getTypeConfig(keyword.keyword_type).color}"
                    title={getTypeLabel(keyword.keyword_type || "concept")}
                  >
                    <i class={getTypeConfig(keyword.keyword_type).icon}></i>
                  </span>
                </td>
                <td class="numeric">
                  <span class="article-count">{keyword.article_count}</span>
                </td>
                <td class="date-cell">{formatChangedDate(keyword.first_seen)}</td>
                <td class="date-cell">{formatChangedDate(keyword.last_used)}</td>
                <td class="actions-cell">
                  {#if onShowKeywordArticles}
                    <button
                      class="action-btn"
                      onclick={(e) => handleShowArticles(e, keyword)}
                      title={$_("network.showArticles") || "Artikel anzeigen"}
                    >
                      <i class="fa-solid fa-newspaper"></i>
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    {/if}
  </div>

  <!-- Load More / Pagination -->
  {#if hasMore && !loading && !searchQuery && keywords.length > 0}
    <div class="load-more-container">
      <button onclick={() => loadKeywords(false)} class="load-more-btn">
        <i class="fa-solid fa-plus"></i>
        {$_("network.loadMore") || "Weitere laden"}
      </button>
    </div>
  {/if}

  {#if loading && keywords.length > 0}
    <div class="loading-more">
      <i class="fa-solid fa-spinner fa-spin"></i>
      <span>{$_("network.loading") || "Lade..."}</span>
    </div>
  {/if}

  <!-- Summary -->
  <div class="table-summary">
    <span class="summary-text">
      {displayedKeywords.length}
      {#if minArticleCount > 0 || searchQuery}
        {$_("network.searchResults") || "gefiltert"} /
      {/if}
      {keywords.length}
      {$_("network.keywords") || "Stichworte"}
    </span>
  </div>
</div>

<style>
  .keyword-table-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  /* Filters */
  .table-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 0.75rem;
    color: var(--text-muted);
    font-size: 0.875rem;
    pointer-events: none;
  }

  .search-input {
    padding: 0.5rem 2rem 0.5rem 2.25rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
    min-width: 200px;
  }

  .search-input::placeholder {
    color: var(--text-faint);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .clear-btn {
    position: absolute;
    right: 0.5rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0.25rem;
    font-size: 0.875rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }

  .filter-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }

  .min-articles-input {
    width: 60px;
    padding: 0.375rem 0.5rem;
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
    text-align: center;
  }

  .min-articles-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .clear-filter-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.375rem 0.625rem;
    background: none;
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .clear-filter-btn:hover {
    color: var(--accent-error);
    border-color: var(--accent-error);
  }

  /* Detect types button */
  .filter-group-right {
    margin-left: auto;
  }

  .detect-types-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--accent-primary);
    border-radius: 0.375rem;
    color: var(--accent-primary);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .detect-types-btn:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--bg-surface);
  }

  .detect-types-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .untyped-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.5rem;
    height: 1.25rem;
    padding: 0 0.375rem;
    background-color: var(--accent-warning);
    color: var(--bg-surface);
    font-size: 0.6875rem;
    font-weight: 600;
    border-radius: 0.75rem;
  }

  /* Table Wrapper */
  .table-wrapper {
    flex: 1;
    overflow: auto;
  }

  /* Table */
  .keyword-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }

  .keyword-table thead {
    position: sticky;
    top: 0;
    z-index: 10;
    background-color: var(--bg-surface);
  }

  .keyword-table th {
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

  .keyword-table th.sortable {
    cursor: pointer;
    transition: color 0.2s;
  }

  .keyword-table th.sortable:hover {
    color: var(--text-primary);
  }

  .keyword-table th span {
    margin-right: 0.5rem;
  }

  .keyword-table th i {
    font-size: 0.625rem;
    opacity: 0.5;
  }

  .keyword-table th.sortable:hover i {
    opacity: 1;
  }

  .keyword-table th.numeric {
    text-align: right;
  }

  /* Column widths */
  .keyword-table th:nth-child(1) {
    width: 30%;
  } /* Name */
  .keyword-table th:nth-child(2) {
    width: 8%;
  } /* Type */
  .keyword-table th:nth-child(3) {
    width: 12%;
  } /* Articles */
  .keyword-table th:nth-child(4) {
    width: 18%;
  } /* First seen */
  .keyword-table th:nth-child(5) {
    width: 18%;
  } /* Last used */
  .keyword-table th:nth-child(6) {
    width: 14%;
  } /* Actions */

  .type-col {
    text-align: center;
  }

  .actions-col {
    text-align: center;
  }

  .keyword-table td {
    padding: 0.625rem 1rem;
    font-size: 0.875rem;
    color: var(--text-primary);
    border-bottom: 1px solid var(--border-muted);
  }

  .keyword-table td.numeric {
    text-align: right;
  }

  /* Rows */
  .keyword-row {
    cursor: pointer;
    transition: background-color 0.15s;
  }

  .keyword-row:hover {
    background-color: var(--bg-overlay);
  }

  .keyword-row:focus {
    outline: none;
    background-color: var(--bg-overlay);
    box-shadow: inset 2px 0 0 var(--accent-primary);
  }

  .keyword-row:focus-visible {
    box-shadow: inset 2px 0 0 var(--accent-primary);
  }

  /* Name Cell */
  .name-cell {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .keyword-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .alias-badge {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    font-size: 0.625rem;
    color: var(--text-muted);
    background-color: var(--bg-overlay);
    border-radius: 0.25rem;
  }

  /* Article Count */
  .article-count {
    display: inline-block;
    min-width: 2.5rem;
    padding: 0.125rem 0.5rem;
    background-color: var(--bg-overlay);
    border-radius: 0.25rem;
    text-align: center;
    font-weight: 500;
    font-size: 0.8125rem;
  }

  /* Date Cell */
  .date-cell {
    color: var(--text-muted);
    font-size: 0.8125rem;
  }

  /* Empty and Loading States */
  .loading-row td,
  .empty-row td {
    padding: 3rem 1rem;
    text-align: center;
  }

  .loading-indicator,
  .empty-message {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .loading-indicator i,
  .empty-message i {
    font-size: 1.25rem;
  }

  /* Error Message */
  .error-message {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 2rem;
    color: var(--accent-error);
    font-size: 0.875rem;
    background-color: rgba(239, 68, 68, 0.1);
    border-radius: 0.375rem;
    margin: 1rem;
  }

  /* Load More */
  .load-more-container {
    padding: 0.75rem 1rem;
    display: flex;
    justify-content: center;
    border-top: 1px solid var(--border-muted);
  }

  .load-more-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1.25rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    color: var(--text-secondary);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .load-more-btn:hover {
    background-color: var(--bg-muted);
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .loading-more {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem;
    color: var(--text-muted);
    font-size: 0.8125rem;
  }

  /* Summary */
  .table-summary {
    padding: 0.5rem 1rem;
    border-top: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .summary-text {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Type Cell */
  .type-cell {
    text-align: center;
  }

  .type-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    font-size: 0.875rem;
    transition: transform 0.15s;
  }

  .type-badge:hover {
    transform: scale(1.1);
  }

  /* Actions Cell */
  .actions-cell {
    text-align: center;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    padding: 0;
    background: none;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s;
  }

  .action-btn:hover {
    background-color: var(--bg-overlay);
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .action-btn i {
    font-size: 0.75rem;
  }
</style>
