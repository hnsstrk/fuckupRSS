<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import KeywordContextTooltip from "./KeywordContextTooltip.svelte";
  import { formatError } from "$lib/utils/formatError";

  // Type for compound preview item
  interface CompoundItem {
    id: number;
    original: string;
    components: string[];
    articles_affected: number;
    is_preserved: boolean;
    keyword_type?: string;
  }

  // Keyword types
  type KeywordType = "concept" | "person" | "organization" | "location" | "acronym";

  const keywordTypes: KeywordType[] = ["concept", "person", "organization", "location", "acronym"];

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

  // Decision tracking - items that have been acted upon
  interface DecisionItem extends CompoundItem {
    decision: "preserved" | "split";
    decided_at: string;
  }

  // Props
  interface Props {
    loadKeywords?: () => Promise<void>;
  }

  let { loadKeywords }: Props = $props();

  // State
  let compoundList = $state<CompoundItem[]>([]);
  let decisionHistory = $state<DecisionItem[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let successMessage = $state<string | null>(null);
  let splitting = $state(false);
  let preserving = $state(false);

  // Selection
  let selectedIds = new SvelteSet<number>();
  let selectAll = $state(false);

  // Filter/View tabs
  type FilterTab = "pending" | "preserved" | "split" | "all";
  let activeFilter = $state<FilterTab>("pending");

  // Search
  let searchQuery = $state("");

  // Sorting
  type SortColumn = "original" | "components" | "articles" | "type";
  type SortDirection = "asc" | "desc";
  let sortColumn = $state<SortColumn>("articles");
  let sortDirection = $state<SortDirection>("desc");

  // Pagination
  let currentPage = $state(1);
  const pageSize = 25;

  // Derived: count of selected items
  let selectedCount = $derived(selectedIds.size);

  // Derived: filtered and sorted list
  let filteredList = $derived.by(() => {
    let result: CompoundItem[] = [];

    if (activeFilter === "pending") {
      result = compoundList.filter((c) => !c.is_preserved);
    } else if (activeFilter === "preserved") {
      result = compoundList.filter((c) => c.is_preserved);
    } else if (activeFilter === "split") {
      // Items that were split are in decisionHistory
      result = decisionHistory.filter((c) => c.decision === "split");
    } else {
      // 'all' - show everything including history
      result = [...compoundList, ...decisionHistory.filter((c) => c.decision === "split")];
    }

    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (c) =>
          c.original.toLowerCase().includes(query) ||
          c.components.some((comp) => comp.toLowerCase().includes(query)),
      );
    }

    // Sort
    result.sort((a, b) => {
      let comparison = 0;
      switch (sortColumn) {
        case "original":
          comparison = a.original.localeCompare(b.original);
          break;
        case "components":
          comparison = a.components.join(" ").localeCompare(b.components.join(" "));
          break;
        case "articles":
          comparison = a.articles_affected - b.articles_affected;
          break;
        case "type":
          comparison = (a.keyword_type || "concept").localeCompare(b.keyword_type || "concept");
          break;
      }
      return sortDirection === "asc" ? comparison : -comparison;
    });

    return result;
  });

  // Derived: paginated list
  let paginatedList = $derived.by(() => {
    const start = (currentPage - 1) * pageSize;
    return filteredList.slice(start, start + pageSize);
  });

  // Derived: total pages
  let totalPages = $derived(Math.max(1, Math.ceil(filteredList.length / pageSize)));

  // Derived: counts for tabs
  let pendingCount = $derived(compoundList.filter((c) => !c.is_preserved).length);
  let preservedCount = $derived(compoundList.filter((c) => c.is_preserved).length);
  let splitCount = $derived(decisionHistory.filter((c) => c.decision === "split").length);
  let allCount = $derived(
    compoundList.length + decisionHistory.filter((c) => c.decision === "split").length,
  );

  // Load compound keywords
  async function loadCompounds() {
    loading = true;
    error = null;
    successMessage = null;

    try {
      const result = await invoke<CompoundItem[]>("preview_compound_splits");
      compoundList = result;
      selectedIds.clear();
      selectAll = false;
      currentPage = 1;
    } catch (e) {
      error = formatError(e);
      console.error("Failed to load compound keywords:", e);
    } finally {
      loading = false;
    }
  }

  // Sort handler
  function handleSort(column: SortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = column;
      sortDirection = column === "original" ? "asc" : "desc";
    }
    currentPage = 1;
  }

  function getSortIcon(column: SortColumn): string {
    if (sortColumn !== column) return "fa-solid fa-sort";
    return sortDirection === "asc" ? "fa-solid fa-sort-up" : "fa-solid fa-sort-down";
  }

  // Selection handlers
  function toggleSelection(id: number) {
    if (selectedIds.has(id)) {
      selectedIds.delete(id);
    } else {
      selectedIds.add(id);
    }
    updateSelectAllState();
  }

  function updateSelectAllState() {
    const selectableItems = paginatedList.filter((c) => !c.is_preserved && !("decision" in c));
    selectAll = selectableItems.length > 0 && selectableItems.every((c) => selectedIds.has(c.id));
  }

  function toggleSelectAll() {
    const selectableItems = paginatedList.filter((c) => !c.is_preserved && !("decision" in c));
    if (selectAll) {
      // Deselect all visible
      for (const item of selectableItems) {
        selectedIds.delete(item.id);
      }
      selectAll = false;
    } else {
      // Select all visible non-preserved
      for (const item of selectableItems) {
        selectedIds.add(item.id);
      }
      selectAll = true;
    }
  }

  // Action: Preserve keyword (Shield button)
  async function preserveKeyword(item: CompoundItem) {
    error = null;
    successMessage = null;

    try {
      await invoke("preserve_compound_keyword", { keywordId: item.id });

      // Update in list
      compoundList = compoundList.map((c) => (c.id === item.id ? { ...c, is_preserved: true } : c));

      // Remove from selection
      selectedIds.delete(item.id);
      updateSelectAllState();

      successMessage =
        $_("compound.preserved", { values: { name: item.original } }) ||
        `"${item.original}" preserved`;
    } catch (e) {
      error = formatError(e);
      console.error("Failed to preserve keyword:", e);
    }
  }

  // Action: Unpreserve keyword
  async function unpreserveKeyword(item: CompoundItem) {
    error = null;
    successMessage = null;

    try {
      await invoke("unpreserve_compound_keyword", { keywordId: item.id });

      // Update in list
      compoundList = compoundList.map((c) =>
        c.id === item.id ? { ...c, is_preserved: false } : c,
      );

      successMessage =
        $_("compound.unpreserved", { values: { name: item.original } }) ||
        `"${item.original}" protection removed`;
    } catch (e) {
      error = formatError(e);
      console.error("Failed to unpreserve keyword:", e);
    }
  }

  // Action: Split single keyword (Bomb button)
  async function splitKeyword(item: CompoundItem) {
    error = null;
    successMessage = null;
    splitting = true;

    try {
      await invoke("split_single_compound", { keywordId: item.id });

      // Add to decision history
      const decisionItem: DecisionItem = {
        ...item,
        decision: "split",
        decided_at: new Date().toISOString(),
      };
      decisionHistory = [...decisionHistory, decisionItem];

      // Remove from active list
      compoundList = compoundList.filter((c) => c.id !== item.id);
      selectedIds.delete(item.id);
      updateSelectAllState();

      successMessage =
        $_("compound.split", {
          values: { name: item.original, components: item.components.join(" + ") },
        }) || `"${item.original}" split into "${item.components.join('" + "')}"`;

      if (loadKeywords) await loadKeywords();
    } catch (e) {
      error = formatError(e);
      console.error("Failed to split keyword:", e);
    } finally {
      splitting = false;
    }
  }

  // Action: Split selected keywords
  async function splitSelected() {
    if (selectedIds.size === 0) return;

    splitting = true;
    error = null;
    successMessage = null;

    const idsToSplit = Array.from(selectedIds);
    let splitSuccessCount = 0;
    let splitErrorCount = 0;

    for (const id of idsToSplit) {
      const item = compoundList.find((c) => c.id === id);
      if (!item) continue;

      try {
        await invoke("split_single_compound", { keywordId: id });

        // Add to decision history
        const decisionItem: DecisionItem = {
          ...item,
          decision: "split",
          decided_at: new Date().toISOString(),
        };
        decisionHistory = [...decisionHistory, decisionItem];

        // Remove from active list
        compoundList = compoundList.filter((c) => c.id !== id);
        selectedIds.delete(id);
        splitSuccessCount++;
      } catch (e) {
        console.error(`Failed to split ${id}:`, e);
        splitErrorCount++;
      }
    }

    selectAll = false;
    splitting = false;

    if (splitErrorCount > 0) {
      successMessage =
        $_("compound.splitBatchPartial", {
          values: { success: splitSuccessCount, failed: splitErrorCount },
        }) || `${splitSuccessCount} split, ${splitErrorCount} failed`;
    } else {
      successMessage =
        $_("compound.splitBatch", {
          values: { count: splitSuccessCount },
        }) || `${splitSuccessCount} keywords split`;
    }

    if (loadKeywords) await loadKeywords();
  }

  // Action: Preserve selected keywords (batch)
  async function preserveSelected() {
    if (selectedIds.size === 0) return;

    preserving = true;
    error = null;
    successMessage = null;

    const idsToPreserve = Array.from(selectedIds);
    let preserveSuccessCount = 0;
    let preserveErrorCount = 0;

    try {
      // Use batch command if available
      await invoke("batch_set_compound_decisions", {
        keywordIds: idsToPreserve,
        decision: "preserve",
      });

      // Update in list
      for (const id of idsToPreserve) {
        compoundList = compoundList.map((c) => (c.id === id ? { ...c, is_preserved: true } : c));
        selectedIds.delete(id);
        preserveSuccessCount++;
      }
    } catch (e) {
      // Fallback to individual calls if batch not available
      for (const id of idsToPreserve) {
        try {
          await invoke("preserve_compound_keyword", { keywordId: id });
          compoundList = compoundList.map((c) => (c.id === id ? { ...c, is_preserved: true } : c));
          selectedIds.delete(id);
          preserveSuccessCount++;
        } catch (innerError) {
          console.error(`Failed to preserve ${id}:`, innerError);
          preserveErrorCount++;
        }
      }
    }

    selectAll = false;
    preserving = false;

    if (preserveErrorCount > 0) {
      successMessage =
        $_("compound.preserveBatchPartial", {
          values: { success: preserveSuccessCount, failed: preserveErrorCount },
        }) || `${preserveSuccessCount} preserved, ${preserveErrorCount} failed`;
    } else {
      successMessage =
        $_("compound.preserveBatch", {
          values: { count: preserveSuccessCount },
        }) || `${preserveSuccessCount} keywords preserved`;
    }
  }

  // Tab change handler
  function setActiveFilter(filter: FilterTab) {
    activeFilter = filter;
    currentPage = 1;
    selectedIds.clear();
    selectAll = false;
  }

  // Action: Update keyword type
  async function updateKeywordType(item: CompoundItem, newType: KeywordType) {
    error = null;

    try {
      await invoke("update_keyword_type", { keywordId: item.id, keywordType: newType });

      // Update in list
      compoundList = compoundList.map((c) =>
        c.id === item.id ? { ...c, keyword_type: newType } : c,
      );

      // Also update in decision history if present
      decisionHistory = decisionHistory.map((c) =>
        c.id === item.id ? { ...c, keyword_type: newType } : c,
      );
    } catch (e) {
      error = formatError(e);
      console.error("Failed to update keyword type:", e);
    }
  }

  function handleTypeChange(event: Event, item: CompoundItem) {
    const select = event.target as HTMLSelectElement;
    const newType = select.value as KeywordType;
    updateKeywordType(item, newType);
  }

  // Pagination handlers
  function goToPage(page: number) {
    if (page >= 1 && page <= totalPages) {
      currentPage = page;
      // Clear selection when changing page
      selectedIds.clear();
      selectAll = false;
    }
  }

  // Navigate to KeywordNetwork for this keyword
  function navigateToKeyword(keywordId: number) {
    window.dispatchEvent(
      new CustomEvent("navigate-to-network", {
        detail: { keywordId },
      }),
    );
  }

  // Clear messages after timeout
  $effect(() => {
    if (successMessage) {
      const timeout = setTimeout(() => {
        successMessage = null;
      }, 5000);
      return () => clearTimeout(timeout);
    }
  });

  async function handleBatchComplete() {
    await loadCompounds();
  }

  async function handleKeywordsChanged() {
    await loadCompounds();
  }

  // Load on mount
  onMount(() => {
    window.addEventListener("batch-complete", handleBatchComplete);
    window.addEventListener("keywords-changed", handleKeywordsChanged);
    loadCompounds();
  });

  onDestroy(() => {
    window.removeEventListener("batch-complete", handleBatchComplete);
    window.removeEventListener("keywords-changed", handleKeywordsChanged);
  });
</script>

<div class="compound-manager">
  <!-- Header -->
  <div class="manager-header">
    <div class="header-title">
      <h2>
        <i class="fa-solid fa-scissors"></i>
        {$_("compound.title") || "Compound Keywords"}
      </h2>
      <p class="header-description">
        {$_("compound.description") ||
          "Manage compound keywords - split them into components or preserve them as-is."}
      </p>
    </div>

    <div class="header-actions">
      <button type="button" class="btn-refresh" onclick={loadCompounds} disabled={loading}>
        {#if loading}
          <i class="fa-solid fa-spinner fa-spin"></i>
        {:else}
          <i class="fa-solid fa-rotate"></i>
        {/if}
        {$_("compound.refresh") || "Refresh"}
      </button>
    </div>
  </div>

  <!-- Messages -->
  {#if error}
    <div class="message error">
      <i class="fa-solid fa-triangle-exclamation"></i>
      {error}
      <button class="dismiss-btn" onclick={() => (error = null)} aria-label="Dismiss"
        ><i class="fa-solid fa-xmark"></i></button
      >
    </div>
  {/if}

  {#if successMessage}
    <div class="message success">
      <i class="fa-solid fa-check-circle"></i>
      {successMessage}
      <button class="dismiss-btn" onclick={() => (successMessage = null)} aria-label="Dismiss"
        ><i class="fa-solid fa-xmark"></i></button
      >
    </div>
  {/if}

  <!-- Filter Tabs -->
  <div class="filter-tabs">
    <button
      class="tab-btn {activeFilter === 'pending' ? 'active' : ''}"
      onclick={() => setActiveFilter("pending")}
    >
      <i class="fa-solid fa-clock"></i>
      {$_("compound.tabPending") || "Pending"}
      <span class="tab-count">{pendingCount}</span>
    </button>
    <button
      class="tab-btn {activeFilter === 'preserved' ? 'active' : ''}"
      onclick={() => setActiveFilter("preserved")}
    >
      <i class="fa-solid fa-shield"></i>
      {$_("compound.tabPreserved") || "Preserved"}
      <span class="tab-count">{preservedCount}</span>
    </button>
    <button
      class="tab-btn {activeFilter === 'split' ? 'active' : ''}"
      onclick={() => setActiveFilter("split")}
    >
      <i class="fa-solid fa-scissors"></i>
      {$_("compound.tabSplit") || "Split"}
      <span class="tab-count">{splitCount}</span>
    </button>
    <button
      class="tab-btn {activeFilter === 'all' ? 'active' : ''}"
      onclick={() => setActiveFilter("all")}
    >
      <i class="fa-solid fa-list"></i>
      {$_("compound.tabAll") || "All"}
      <span class="tab-count">{allCount}</span>
    </button>
  </div>

  <!-- Toolbar: Search and Batch Actions -->
  <div class="toolbar">
    <div class="search-box">
      <i class="fa-solid fa-search search-icon"></i>
      <input
        type="text"
        bind:value={searchQuery}
        placeholder={$_("compound.searchPlaceholder") || "Search keywords..."}
        class="search-input"
      />
      {#if searchQuery}
        <button class="clear-btn" onclick={() => (searchQuery = "")} aria-label="Clear search">
          <i class="fa-solid fa-xmark"></i>
        </button>
      {/if}
    </div>

    {#if activeFilter === "pending" && pendingCount > 0}
      <div class="batch-actions">
        <span class="selection-info">
          {selectedCount}
          {$_("compound.selected") || "selected"}
        </span>
        <button
          type="button"
          class="btn-batch-preserve"
          onclick={preserveSelected}
          disabled={selectedCount === 0 || preserving || splitting}
          title={$_("compound.preserveSelectedTitle") || "Preserve selected keywords (keep as-is)"}
        >
          {#if preserving}
            <i class="fa-solid fa-spinner fa-spin"></i>
          {:else}
            <i class="fa-solid fa-shield"></i>
          {/if}
          {$_("compound.preserveSelected") || "Preserve Selected"}
        </button>
        <button
          type="button"
          class="btn-batch-split"
          onclick={splitSelected}
          disabled={selectedCount === 0 || splitting || preserving}
          title={$_("compound.splitSelectedTitle") || "Split selected keywords into components"}
        >
          {#if splitting}
            <i class="fa-solid fa-spinner fa-spin"></i>
          {:else}
            <i class="fa-solid fa-scissors"></i>
          {/if}
          {$_("compound.splitSelected") || "Split Selected"}
        </button>
      </div>
    {/if}
  </div>

  <!-- Table -->
  <div class="table-container">
    {#if loading && compoundList.length === 0}
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
                  onchange={toggleSelectAll}
                  disabled={splitting}
                  title={$_("compound.selectAll") || "Select all"}
                />
              </th>
            {/if}
            <th class="sortable" onclick={() => handleSort("original")}>
              <span>{$_("compound.colOriginal") || "Original"}</span>
              <i class={getSortIcon("original")}></i>
            </th>
            <th class="sortable" onclick={() => handleSort("components")}>
              <span>{$_("compound.colComponents") || "Components"}</span>
              <i class={getSortIcon("components")}></i>
            </th>
            <th class="sortable numeric" onclick={() => handleSort("articles")}>
              <span>{$_("compound.colArticles") || "Articles"}</span>
              <i class={getSortIcon("articles")}></i>
            </th>
            <th class="sortable" onclick={() => handleSort("type")}>
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
                      onchange={() => toggleSelection(item.id)}
                      disabled={splitting}
                    />
                  {/if}
                </td>
              {/if}
              <td class="original-col">
                <KeywordContextTooltip
                  keywordId={item.id}
                  keywordName={item.original}
                  onclick={() => navigateToKeyword(item.id)}
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
                {#if !("decision" in item)}
                  <div class="type-select-wrapper">
                    <select
                      class="type-select"
                      value={item.keyword_type || "concept"}
                      onchange={(e) => handleTypeChange(e, item)}
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
                        onclick={() => unpreserveKeyword(item)}
                        title={$_("compound.unpreserve") || "Remove protection"}
                        disabled={splitting}
                      >
                        <i class="fa-solid fa-shield-xmark"></i>
                      </button>
                    {:else}
                      <button
                        class="action-btn preserve"
                        onclick={() => preserveKeyword(item)}
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
                        onclick={() => splitKeyword(item)}
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
        onclick={() => goToPage(1)}
        disabled={currentPage === 1}
        aria-label="First page"
      >
        <i class="fa-solid fa-angles-left"></i>
      </button>
      <button
        class="page-btn"
        onclick={() => goToPage(currentPage - 1)}
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
        onclick={() => goToPage(currentPage + 1)}
        disabled={currentPage === totalPages}
        aria-label="Next page"
      >
        <i class="fa-solid fa-angle-right"></i>
      </button>
      <button
        class="page-btn"
        onclick={() => goToPage(totalPages)}
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
</div>

<style>
  .compound-manager {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-base);
    overflow: hidden;
  }

  /* Header */
  .manager-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 1rem 1.5rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .header-title h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-title h2 i {
    color: var(--accent-primary);
  }

  .header-description {
    margin: 0.25rem 0 0 0;
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .btn-refresh {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-refresh:hover:not(:disabled) {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .btn-refresh:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Messages */
  .message {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    margin: 0.5rem 1rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
  }

  .message.error {
    background-color: rgba(243, 139, 168, 0.15);
    border: 1px solid var(--status-error);
    color: var(--status-error);
  }

  .message.success {
    background-color: rgba(166, 227, 161, 0.15);
    border: 1px solid var(--status-success);
    color: var(--status-success);
  }

  .message .dismiss-btn {
    margin-left: auto;
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    padding: 0.25rem;
    opacity: 0.7;
  }

  .message .dismiss-btn:hover {
    opacity: 1;
  }

  /* Filter Tabs */
  .filter-tabs {
    display: flex;
    gap: 0.25rem;
    padding: 0.75rem 1rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid transparent;
    border-radius: 0.375rem;
    background: none;
    color: var(--text-muted);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .tab-btn:hover {
    background-color: var(--bg-overlay);
    color: var(--text-primary);
  }

  .tab-btn.active {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    border-color: var(--accent-primary);
  }

  .tab-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.5rem;
    height: 1.25rem;
    padding: 0 0.375rem;
    background-color: var(--bg-overlay);
    border-radius: 0.75rem;
    font-size: 0.6875rem;
    font-weight: 600;
  }

  .tab-btn.active .tab-count {
    background-color: rgba(255, 255, 255, 0.2);
  }

  /* Toolbar */
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
    gap: 1rem;
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    max-width: 300px;
  }

  .search-icon {
    position: absolute;
    left: 0.75rem;
    color: var(--text-muted);
    font-size: 0.875rem;
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 0.5rem 2rem 0.5rem 2.25rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
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
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }

  .batch-actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .selection-info {
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .btn-batch-preserve {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--status-success);
    color: white;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-batch-preserve:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-batch-preserve:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-batch-split {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--status-error);
    color: white;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-batch-split:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-batch-split:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

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
