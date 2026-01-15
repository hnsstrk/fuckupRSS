<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import type { ArticleKeyword, CorrectionInput } from '$lib/types';

  interface Props {
    fnordId: number;
    keywords: ArticleKeyword[];
    editing: boolean;
    onUpdate: (keywords: ArticleKeyword[]) => void;
  }

  let { fnordId, keywords, editing, onUpdate }: Props = $props();

  // Search state
  let searchInput = $state('');
  let searchResults = $state<{ id: number; name: string; count: number }[]>([]);
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  let loading = $state(false);
  let showDropdown = $state(false);

  // Get source icon class
  function getSourceIcon(source: ArticleKeyword['source']): string {
    switch (source) {
      case 'ai':
        return 'fa-solid fa-robot';
      case 'statistical':
        return 'fa-solid fa-chart-line';
      case 'manual':
        return 'fa-solid fa-user';
      default:
        return 'fa-solid fa-tag';
    }
  }

  // Get source tooltip
  function getSourceLabel(source: ArticleKeyword['source']): string {
    switch (source) {
      case 'ai':
        return $_('articleKeywords.sourceAI') || 'KI-generiert';
      case 'statistical':
        return $_('articleKeywords.sourceStatistical') || 'Statistisch';
      case 'manual':
        return $_('articleKeywords.sourceManual') || 'Manuell';
      default:
        return source;
    }
  }

  // Search for keywords
  async function handleSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);

    if (!searchInput.trim()) {
      searchResults = [];
      showDropdown = false;
      return;
    }

    searchTimeout = setTimeout(async () => {
      try {
        const results = await invoke<{ id: number; name: string; count: number }[]>('search_keywords', {
          query: searchInput,
          limit: 10,
        });
        // Filter out keywords already assigned
        const existingIds = new Set(keywords.map((k) => k.id));
        searchResults = results.filter((r) => !existingIds.has(r.id));
        showDropdown = searchResults.length > 0;
      } catch (e) {
        console.error('Failed to search keywords:', e);
        searchResults = [];
        showDropdown = false;
      }
    }, 300);
  }

  // Add a keyword to the article
  async function addKeyword(keywordId: number, keywordName: string) {
    loading = true;
    try {
      await invoke('add_article_keyword', {
        fnordId,
        keywordId,
        source: 'manual',
        confidence: 1.0,
      });

      // Record correction for bias learning
      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: 'keyword_added',
        new_value: keywordName,
      };
      await invoke('record_correction', { correction });

      // Update local state
      const newKeyword: ArticleKeyword = {
        id: keywordId,
        name: keywordName,
        source: 'manual',
        confidence: 1.0,
      };
      onUpdate([...keywords, newKeyword]);

      // Clear search
      searchInput = '';
      searchResults = [];
      showDropdown = false;
    } catch (e) {
      console.error('Failed to add keyword:', e);
    } finally {
      loading = false;
    }
  }

  // Remove a keyword from the article
  async function removeKeyword(keyword: ArticleKeyword) {
    loading = true;
    try {
      await invoke('remove_article_keyword', {
        fnordId,
        keywordId: keyword.id,
      });

      // Record correction for bias learning
      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: 'keyword_removed',
        old_value: keyword.name,
      };
      await invoke('record_correction', { correction });

      // Update local state
      onUpdate(keywords.filter((k) => k.id !== keyword.id));
    } catch (e) {
      console.error('Failed to remove keyword:', e);
    } finally {
      loading = false;
    }
  }

  // Handle click outside to close dropdown
  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.keyword-search-container')) {
      showDropdown = false;
    }
  }

  // Navigate to keyword in network view
  function navigateToKeyword(keywordId: number) {
    window.dispatchEvent(new CustomEvent('navigate-to-network', { detail: { keywordId } }));
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="article-keywords">
  <!-- Keywords List -->
  <div class="keywords-list">
    {#each keywords as keyword (keyword.id)}
      <div class="keyword-chip" class:editable={editing}>
        <i class="source-icon {getSourceIcon(keyword.source)}" title={getSourceLabel(keyword.source)}></i>
        <button
          type="button"
          class="keyword-name"
          onclick={() => navigateToKeyword(keyword.id)}
          title={$_('network.title') || 'Im Netzwerk anzeigen'}
          disabled={editing}
        >
          {keyword.name}
        </button>
        {#if keyword.confidence < 1.0}
          <span class="keyword-confidence" title={$_('articleKeywords.confidence') || 'Konfidenz'}>
            {Math.round(keyword.confidence * 100)}%
          </span>
        {/if}
        {#if editing}
          <button
            type="button"
            class="remove-btn"
            onclick={() => removeKeyword(keyword)}
            disabled={loading}
            title={$_('articleKeywords.remove') || 'Entfernen'}
            aria-label={$_('articleKeywords.remove') || 'Entfernen'}
          >
            <i class="fa-solid fa-xmark"></i>
          </button>
        {/if}
      </div>
    {/each}

    {#if keywords.length === 0 && !editing}
      <span class="no-keywords">{$_('articleKeywords.none') || 'Keine Keywords'}</span>
    {/if}
  </div>

  <!-- Add Keyword Input (Edit Mode Only) -->
  {#if editing}
    <div class="keyword-search-container">
      <div class="search-input-wrapper">
        <i class="search-icon fa-solid fa-search"></i>
        <input
          type="text"
          class="search-input"
          bind:value={searchInput}
          oninput={handleSearch}
          placeholder={$_('articleKeywords.searchPlaceholder') || 'Keyword suchen oder hinzufuegen...'}
          disabled={loading}
        />
        {#if loading}
          <i class="loading-icon fa-solid fa-spinner fa-spin"></i>
        {/if}
      </div>

      {#if showDropdown && searchResults.length > 0}
        <div class="search-dropdown">
          {#each searchResults as result (result.id)}
            <button
              type="button"
              class="search-result-item"
              onclick={() => addKeyword(result.id, result.name)}
            >
              <span class="result-name">{result.name}</span>
              <span class="result-count">{result.count} {$_('articleKeywords.articles') || 'Artikel'}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .article-keywords {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .keywords-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .keyword-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    font-size: 0.8125rem;
    transition: all 0.2s;
  }

  .keyword-chip.editable {
    padding-right: 0.25rem;
  }

  .keyword-chip:hover {
    border-color: var(--accent-primary);
  }

  .source-icon {
    font-size: 0.625rem;
    color: var(--text-muted);
  }

  .source-icon.fa-robot {
    color: var(--accent-primary);
  }

  .source-icon.fa-chart-line {
    color: var(--accent-info);
  }

  .source-icon.fa-user {
    color: var(--accent-success);
  }

  .keyword-name {
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    color: var(--text-primary);
    font-size: inherit;
    cursor: pointer;
    transition: color 0.2s;
  }

  .keyword-name:hover:not(:disabled) {
    color: var(--accent-primary);
  }

  .keyword-name:disabled {
    cursor: default;
  }

  .keyword-confidence {
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

  .no-keywords {
    font-size: 0.8125rem;
    color: var(--text-muted);
    font-style: italic;
  }

  /* Search Input */
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

  /* Search Dropdown */
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
