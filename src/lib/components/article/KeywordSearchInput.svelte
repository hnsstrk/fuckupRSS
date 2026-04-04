<script lang="ts">
  import { _ } from "svelte-i18n";

  interface SearchResult {
    id: number;
    name: string;
    count: number;
  }

  interface Props {
    searchInput: string;
    searchResults: SearchResult[];
    showDropdown: boolean;
    loading: boolean;
    loadingSuggestions: boolean;
    onSearch: () => void;
    onAdd: (name: string) => void;
  }

  let {
    searchInput = $bindable(),
    searchResults,
    showDropdown,
    loading,
    loadingSuggestions,
    onSearch,
    onAdd,
  }: Props = $props();
</script>

<div class="keyword-search-container">
  <div class="search-input-wrapper">
    <i class="search-icon fa-solid fa-search"></i>
    <input
      type="text"
      class="search-input"
      bind:value={searchInput}
      oninput={onSearch}
      placeholder={$_("articleKeywords.searchPlaceholder") ||
        "Keyword suchen oder hinzufügen..."}
      disabled={loading}
    />
    {#if loading || loadingSuggestions}
      <i class="loading-icon fa-solid fa-spinner fa-spin"></i>
    {/if}
  </div>

  {#if showDropdown}
    <div class="search-dropdown">
      {#if searchResults.length > 0}
        {#each searchResults as result (result.id)}
          <button
            type="button"
            class="search-result-item"
            onclick={() => onAdd(result.name)}
          >
            <span class="result-name">{result.name}</span>
            <span class="result-count"
              >{result.count} {$_("articleKeywords.articles") || "Artikel"}</span
            >
          </button>
        {/each}
      {/if}
      <!-- Option to create new keyword -->
      {#if searchInput.trim().length >= 2 && !searchResults.some((r) => r.name.toLowerCase() === searchInput.toLowerCase())}
        <button
          type="button"
          class="search-result-item create-new"
          onclick={() => onAdd(searchInput.trim())}
        >
          <span class="result-name">
            <i class="fa-solid fa-plus"></i>
            "{searchInput.trim()}" erstellen
          </span>
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .keyword-search-container {
    position: relative;
  }

  .search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 0.75rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 0.5rem 2rem 0.5rem 2rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.875rem;
    transition: border-color 0.2s;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .search-input:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .loading-icon {
    position: absolute;
    right: 0.75rem;
    font-size: 0.75rem;
    color: var(--accent-primary);
  }

  .search-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 100;
    margin-top: 0.25rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    max-height: 200px;
    overflow-y: auto;
  }

  .search-result-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-size: 0.875rem;
    transition: background-color 0.15s;
  }

  .search-result-item:hover {
    background-color: var(--bg-overlay);
  }

  .search-result-item:not(:last-child) {
    border-bottom: 1px solid var(--border-default);
  }

  .search-result-item.create-new {
    color: var(--accent-primary);
  }

  .search-result-item.create-new i {
    margin-right: 0.25rem;
  }

  .result-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-count {
    font-size: 0.75rem;
    color: var(--text-muted);
    flex-shrink: 0;
    margin-left: 0.5rem;
  }
</style>
