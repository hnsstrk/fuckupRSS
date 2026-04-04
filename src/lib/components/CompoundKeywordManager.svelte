<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { formatError } from "$lib/utils/formatError";
  import { navigationStore } from "$lib/stores/navigation.svelte";
  import CompoundKeywordToolbar from "./keywords/CompoundKeywordToolbar.svelte";
  import CompoundKeywordTable from "./keywords/CompoundKeywordTable.svelte";
  import type {
    CompoundItem,
    DecisionItem,
    KeywordType,
    SortColumn,
    SortDirection,
    FilterTab,
  } from "./keywords/CompoundKeywordTable.svelte";

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
  let activeFilter = $state<FilterTab>("pending");

  // Search
  let searchQuery = $state("");

  // Sorting
  let sortColumn = $state<SortColumn>("articles");
  let sortDirection = $state<SortDirection>("desc");

  // Pagination
  let currentPage = $state(1);
  const pageSize = 25;

  // Derived: count of selected items
  let selectedCount = $derived(selectedIds.size);

  // Derived: filtered and sorted list
  let filteredList = $derived.by(() => {
    let result: (CompoundItem | DecisionItem)[] = [];

    if (activeFilter === "pending") {
      result = compoundList.filter((c) => !c.is_preserved);
    } else if (activeFilter === "preserved") {
      result = compoundList.filter((c) => c.is_preserved);
    } else if (activeFilter === "split") {
      result = decisionHistory.filter((c) => c.decision === "split");
    } else {
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
      for (const item of selectableItems) {
        selectedIds.delete(item.id);
      }
      selectAll = false;
    } else {
      for (const item of selectableItems) {
        selectedIds.add(item.id);
      }
      selectAll = true;
    }
  }

  // Action: Preserve keyword
  async function preserveKeyword(item: CompoundItem) {
    error = null;
    successMessage = null;

    try {
      await invoke("preserve_compound_keyword", { keywordId: item.id });
      compoundList = compoundList.map((c) => (c.id === item.id ? { ...c, is_preserved: true } : c));
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

  // Action: Split single keyword
  async function splitKeyword(item: CompoundItem) {
    error = null;
    successMessage = null;
    splitting = true;

    try {
      await invoke("split_single_compound", { keywordId: item.id });
      const decisionItem: DecisionItem = {
        ...item,
        decision: "split",
        decided_at: new Date().toISOString(),
      };
      decisionHistory = [...decisionHistory, decisionItem];
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
        const decisionItem: DecisionItem = {
          ...item,
          decision: "split",
          decided_at: new Date().toISOString(),
        };
        decisionHistory = [...decisionHistory, decisionItem];
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
        $_("compound.splitBatch", { values: { count: splitSuccessCount } }) ||
        `${splitSuccessCount} keywords split`;
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
      await invoke("batch_set_compound_decisions", {
        keywordIds: idsToPreserve,
        decision: "preserve",
      });
      for (const id of idsToPreserve) {
        compoundList = compoundList.map((c) => (c.id === id ? { ...c, is_preserved: true } : c));
        selectedIds.delete(id);
        preserveSuccessCount++;
      }
    } catch {
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
        $_("compound.preserveBatch", { values: { count: preserveSuccessCount } }) ||
        `${preserveSuccessCount} keywords preserved`;
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
      compoundList = compoundList.map((c) =>
        c.id === item.id ? { ...c, keyword_type: newType } : c,
      );
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

  // Pagination handler
  function goToPage(page: number) {
    if (page >= 1 && page <= totalPages) {
      currentPage = page;
      selectedIds.clear();
      selectAll = false;
    }
  }

  // Navigate to KeywordNetwork for this keyword
  function navigateToKeyword(keywordId: number) {
    navigationStore.navigateToNetwork(keywordId);
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
  <CompoundKeywordToolbar
    {searchQuery}
    {activeFilter}
    {pendingCount}
    {selectedCount}
    {splitting}
    {preserving}
    onsearchchange={(value) => {
      searchQuery = value;
    }}
    onpreserveselected={preserveSelected}
    onsplitselected={splitSelected}
  />

  <!-- Table + Pagination + Summary -->
  <CompoundKeywordTable
    {paginatedList}
    {filteredList}
    {activeFilter}
    {loading}
    {splitting}
    {selectAll}
    {selectedIds}
    {sortColumn}
    {sortDirection}
    {currentPage}
    {totalPages}
    {searchQuery}
    onsort={handleSort}
    ontoggleall={toggleSelectAll}
    ontoggleselection={toggleSelection}
    onpreserve={preserveKeyword}
    onunpreserve={unpreserveKeyword}
    onsplit={splitKeyword}
    ontypechange={handleTypeChange}
    onnavigatetonetwork={navigateToKeyword}
    onpaginate={goToPage}
  />
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
</style>
