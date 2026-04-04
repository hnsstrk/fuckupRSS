<script lang="ts">
  import { _ } from "svelte-i18n";

  interface Props {
    searchQuery: string;
    activeFilter: "pending" | "preserved" | "split" | "all";
    pendingCount: number;
    selectedCount: number;
    splitting: boolean;
    preserving: boolean;
    onsearchchange: (value: string) => void;
    onpreserveselected: () => void;
    onsplitselected: () => void;
  }

  let {
    searchQuery,
    activeFilter,
    pendingCount,
    selectedCount,
    splitting,
    preserving,
    onsearchchange,
    onpreserveselected,
    onsplitselected,
  }: Props = $props();
</script>

<div class="toolbar">
  <div class="search-box">
    <i class="fa-solid fa-search search-icon"></i>
    <input
      type="text"
      value={searchQuery}
      oninput={(e) => onsearchchange((e.target as HTMLInputElement).value)}
      placeholder={$_("compound.searchPlaceholder") || "Search keywords..."}
      class="search-input"
    />
    {#if searchQuery}
      <button class="clear-btn" onclick={() => onsearchchange("")} aria-label="Clear search">
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
        onclick={onpreserveselected}
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
        onclick={onsplitselected}
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

<style>
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
</style>
