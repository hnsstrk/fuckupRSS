<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { toasts } from "../../stores/state.svelte";

  // Stopword state
  interface UserStopword {
    word: string;
    added_at: string | null;
  }

  interface StopwordSearchResult {
    word: string;
    is_builtin: boolean;
  }

  interface StopwordStats {
    builtin_count: number;
    user_count: number;
    total_count: number;
  }

  let stopwordStats = $state<StopwordStats | null>(null);
  let userStopwords = $state<UserStopword[]>([]);
  let stopwordSearchQuery = $state("");
  let stopwordSearchResults = $state<StopwordSearchResult[]>([]);
  let newStopword = $state("");
  let confirmClearStopwords = $state(false);
  let stopwordLoading = $state(false);

  export async function init() {
    await Promise.all([loadStopwordStats(), loadUserStopwords()]);
  }

  async function loadStopwordStats() {
    try {
      stopwordStats = await invoke<StopwordStats>("get_stopwords_stats");
    } catch (e) {
      console.error("Failed to load stopword stats:", e);
    }
  }

  async function loadUserStopwords() {
    try {
      userStopwords = await invoke<UserStopword[]>("get_user_stopwords");
    } catch (e) {
      console.error("Failed to load user stopwords:", e);
    }
  }

  async function searchStopwords(query: string) {
    if (query.length < 2) {
      stopwordSearchResults = [];
      return;
    }
    try {
      stopwordSearchResults = await invoke<StopwordSearchResult[]>(
        "search_stopwords",
        { query, limit: 50 }
      );
    } catch (e) {
      console.error("Failed to search stopwords:", e);
    }
  }

  async function addStopword() {
    const word = newStopword.trim().toLowerCase();
    if (word.length < 2) {
      toasts.add($_("settings.stopwords.minLength"), "error");
      return;
    }

    stopwordLoading = true;
    try {
      const added = await invoke<boolean>("add_stopword", { word });
      if (added) {
        toasts.add($_("settings.stopwords.added"), "success");
        newStopword = "";
        await Promise.all([loadStopwordStats(), loadUserStopwords()]);
      } else {
        toasts.add($_("settings.stopwords.alreadyExists"), "warning");
      }
    } catch (e) {
      toasts.add(`Error: ${e}`, "error");
    } finally {
      stopwordLoading = false;
    }
  }

  async function removeStopword(word: string) {
    stopwordLoading = true;
    try {
      await invoke<boolean>("remove_stopword", { word });
      toasts.add($_("settings.stopwords.removed"), "success");
      await Promise.all([loadStopwordStats(), loadUserStopwords()]);
    } catch (e) {
      toasts.add(`Error: ${e}`, "error");
    } finally {
      stopwordLoading = false;
    }
  }

  async function clearAllUserStopwords() {
    stopwordLoading = true;
    try {
      const deleted = await invoke<number>("clear_user_stopwords");
      toasts.add(`${$_("settings.stopwords.cleared")} (${deleted})`, "success");
      confirmClearStopwords = false;
      await Promise.all([loadStopwordStats(), loadUserStopwords()]);
    } catch (e) {
      toasts.add(`Error: ${e}`, "error");
    } finally {
      stopwordLoading = false;
    }
  }
</script>

<!-- Stopwords Editor -->
<p class="settings-description">{$_("settings.stopwords.description")}</p>

<!-- Confirmation Dialog for Clear -->
{#if confirmClearStopwords}
  <div class="confirm-overlay">
    <div class="confirm-dialog">
      <p class="confirm-message">{$_("settings.stopwords.confirmClear")}</p>
      <div class="confirm-actions">
        <button
          type="button"
          class="btn-secondary"
          onclick={() => (confirmClearStopwords = false)}
        >
          {$_("confirm.no")}
        </button>
        <button
          type="button"
          class="btn-danger-solid"
          onclick={clearAllUserStopwords}
        >
          {$_("confirm.yes")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Stopword Statistics -->
{#if stopwordStats}
  <div class="keyword-stats">
    <h3>{$_("settings.maintenance.stats")}</h3>
    <div class="stats-grid">
      <div class="stat-item">
        <span class="stat-value">{stopwordStats.builtin_count}</span>
        <span class="stat-label">{$_("settings.stopwords.builtinCount")}</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{stopwordStats.user_count}</span>
        <span class="stat-label">{$_("settings.stopwords.userCount")}</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{stopwordStats.total_count}</span>
        <span class="stat-label">{$_("settings.stopwords.totalCount")}</span>
      </div>
    </div>
  </div>
{/if}

<!-- Add Stopword -->
<div class="stopword-add-section">
  <h3>{$_("settings.stopwords.add")}</h3>
  <form
    class="stopword-add-form"
    onsubmit={(e) => {
      e.preventDefault();
      addStopword();
    }}
  >
    <input
      type="text"
      bind:value={newStopword}
      placeholder={$_("settings.stopwords.addStopword")}
      class="stopword-input"
      disabled={stopwordLoading}
    />
    <button type="submit" class="btn-action" disabled={stopwordLoading || newStopword.trim().length < 2}>
      {$_("settings.stopwords.add")}
    </button>
  </form>
</div>

<!-- User Stopwords List -->
<div class="stopword-list-section">
  <div class="stopword-list-header">
    <h3>{$_("settings.stopwords.userCount")} ({userStopwords.length})</h3>
    {#if userStopwords.length > 0}
      <button
        type="button"
        class="btn-danger"
        onclick={() => (confirmClearStopwords = true)}
        disabled={stopwordLoading}
      >
        {$_("settings.stopwords.clear")}
      </button>
    {/if}
  </div>

  {#if userStopwords.length === 0}
    <p class="stopword-empty">{$_("settings.stopwords.noResults")}</p>
  {:else}
    <div class="stopword-chips">
      {#each userStopwords as sw}
        <span class="stopword-chip user">
          <span class="stopword-word">{sw.word}</span>
          <button
            type="button"
            class="stopword-remove"
            onclick={() => removeStopword(sw.word)}
            disabled={stopwordLoading}
            title={$_("settings.stopwords.remove")}
          >
            <i class="fa-solid fa-xmark"></i>
          </button>
        </span>
      {/each}
    </div>
  {/if}
</div>

<!-- Search Stopwords -->
<div class="stopword-search-section">
  <h3>{$_("settings.stopwords.search")}</h3>
  <input
    type="text"
    bind:value={stopwordSearchQuery}
    oninput={() => searchStopwords(stopwordSearchQuery)}
    placeholder={$_("settings.stopwords.search")}
    class="stopword-input"
  />

  {#if stopwordSearchResults.length > 0}
    <div class="stopword-search-results">
      {#each stopwordSearchResults as result}
        <span class="stopword-chip {result.is_builtin ? 'builtin' : 'user'}">
          <span class="stopword-word">{result.word}</span>
          <span class="stopword-type">
            {result.is_builtin
              ? $_("settings.stopwords.isBuiltin")
              : $_("settings.stopwords.isUser")}
          </span>
        </span>
      {/each}
    </div>
  {:else if stopwordSearchQuery.length >= 2}
    <p class="stopword-empty">{$_("settings.stopwords.noResults")}</p>
  {/if}
</div>

<style>
  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .settings-description {
    margin: 0 0 1rem 0;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  .keyword-stats {
    margin-bottom: 1.5rem;
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
  }

  .keyword-stats h3 {
    margin: 0 0 0.75rem 0;
    font-size: 0.875rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1rem;
  }

  .stat-item {
    text-align: center;
  }

  .stat-value {
    display: block;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--accent-primary);
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Stopword Editor */
  .stopword-add-section,
  .stopword-list-section,
  .stopword-search-section {
    margin-top: 1.5rem;
  }

  .stopword-add-form {
    display: flex;
    gap: 0.5rem;
  }

  .stopword-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .stopword-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .stopword-list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .stopword-list-header h3 {
    margin: 0;
  }

  .stopword-chips,
  .stopword-search-results {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    max-height: 300px;
    overflow-y: auto;
    padding: 0.5rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .stopword-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border-radius: 1rem;
    font-size: 0.75rem;
  }

  .stopword-chip.user {
    background-color: var(--accent-primary-alpha);
    border: 1px solid var(--accent-primary);
    color: var(--accent-primary);
  }

  .stopword-chip.builtin {
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    color: var(--text-muted);
  }

  .stopword-word {
    font-family: monospace;
  }

  .stopword-type {
    font-size: 0.625rem;
    opacity: 0.7;
    text-transform: uppercase;
  }

  .stopword-remove {
    background: none;
    border: none;
    padding: 0 0.125rem;
    cursor: pointer;
    color: inherit;
    opacity: 0.7;
    transition: opacity 0.2s;
  }

  .stopword-remove:hover {
    opacity: 1;
  }

  .stopword-remove:disabled {
    cursor: not-allowed;
    opacity: 0.3;
  }

  .stopword-empty {
    color: var(--text-muted);
    font-style: italic;
    padding: 0.5rem;
  }

  .btn-action {
    padding: 0.5rem 1rem;
    border: 1px solid var(--accent-primary);
    border-radius: 0.375rem;
    background: none;
    color: var(--accent-primary);
    font-size: 0.875rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .btn-action:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-action:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-danger {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--status-error);
    border-radius: 0.25rem;
    background: none;
    color: var(--status-error);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-danger:hover {
    background-color: var(--status-error);
    color: var(--text-on-accent);
  }

  .btn-danger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Confirmation Dialog */
  .confirm-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .confirm-dialog {
    background: var(--bg-surface);
    padding: 1.5rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
    max-width: 400px;
    text-align: center;
  }

  .confirm-message {
    margin: 0 0 1.5rem 0;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .confirm-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: center;
  }

  .btn-secondary {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary:hover {
    background-color: var(--bg-muted);
  }

  .btn-danger-solid {
    padding: 0.5rem 1.5rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--status-error);
    color: var(--text-on-accent);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-danger-solid:hover {
    filter: brightness(1.1);
  }
</style>
