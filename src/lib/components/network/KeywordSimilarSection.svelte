<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { SvelteSet } from "svelte/reactivity";
  import { tick } from "svelte";
  import Tooltip from "../Tooltip.svelte";
  import KeywordContextTooltip from "../KeywordContextTooltip.svelte";

  interface SimilarKeyword {
    id: number;
    name: string;
    similarity: number;
    cooccurrence: number;
    is_true_synonym?: boolean;
  }

  interface Props {
    keywordId: number;
    similarKeywords: SimilarKeyword[];
    similarKeywordsLoading: boolean;
    onKeywordSelect: (id: number) => void;
    onSynonymAssigned?: () => void;
  }

  let {
    keywordId,
    similarKeywords,
    similarKeywordsLoading,
    onKeywordSelect,
    onSynonymAssigned,
  }: Props = $props();

  let selectedSynonymIds = new SvelteSet<number>();
  let assigningSynonyms = $state(false);
  let synonymError = $state<string | null>(null);
  let synonymSuccess = $state<string | null>(null);
  let synonymSectionOpen = $state(false);

  // Track keyword ID to reset selection when keyword changes
  let lastKeywordId = $state<number | null>(null);

  $effect(() => {
    if (keywordId !== lastKeywordId) {
      lastKeywordId = keywordId;
      selectedSynonymIds.clear();
      synonymError = null;
      synonymSuccess = null;
    }
  });

  // Clear success message after timeout
  $effect(() => {
    if (synonymSuccess) {
      const timeout = setTimeout(() => {
        synonymSuccess = null;
      }, 3000);
      return () => clearTimeout(timeout);
    }
  });

  function toggleSynonymSelection(id: number) {
    if (selectedSynonymIds.has(id)) {
      selectedSynonymIds.delete(id);
    } else {
      selectedSynonymIds.add(id);
    }
  }

  async function assignSelectedAsSynonyms() {
    if (selectedSynonymIds.size === 0) return;

    assigningSynonyms = true;
    synonymError = null;
    synonymSuccess = null;

    // Force UI update to show spinner before starting async operations
    await tick();

    const keepId = keywordId;
    const idsToMerge = Array.from(selectedSynonymIds);
    let successCount = 0;
    let errorCount = 0;
    let totalAffectedArticles = 0;

    for (const removeId of idsToMerge) {
      try {
        // Use merge_keyword_pair which actually deletes the keyword
        // and transfers all relationships (articles, categories, etc.)
        // Tauri auto-converts camelCase to snake_case for Rust parameter names
        const result = await invoke<{ merged_pairs: number; affected_articles: number }>(
          "merge_keyword_pair",
          {
            keepId,
            removeId,
          },
        );
        successCount++;
        totalAffectedArticles += result.affected_articles;
        selectedSynonymIds.delete(removeId);
      } catch (e) {
        console.error(`Failed to merge keyword ${removeId}:`, e);
        errorCount++;
      }
    }

    assigningSynonyms = false;

    if (errorCount > 0) {
      synonymError =
        $_("network.synonymAssignedPartial", {
          values: { success: successCount, failed: errorCount },
        }) || `${successCount} merged, ${errorCount} failed`;
    } else {
      synonymSuccess =
        $_("network.synonymsMerged", {
          values: { count: successCount, articles: totalAffectedArticles },
        }) || `${successCount} keyword(s) merged (${totalAffectedArticles} articles transferred)`;
    }

    // Clear selection after successful merge
    if (successCount > 0) {
      selectedSynonymIds.clear();
    }

    // Notify parent to reload data
    if (onSynonymAssigned) {
      onSynonymAssigned();
    }
  }
</script>

{#if similarKeywords.length > 0}
  <details class="synonyms-section" bind:open={synonymSectionOpen}>
    <summary class="synonyms-summary">
      <i class="fa-solid fa-link summary-icon"></i>
      <span>{$_("network.synonyms") || "Ähnliche Keywords"}</span>
      <span class="synonym-count">({similarKeywords.length})</span>
      <Tooltip
        content={$_("network.similarKeywordsHelp") ||
          "Keywords mit ähnlicher semantischer Bedeutung basierend auf Embeddings"}
      >
        <i class="fa-solid fa-circle-info help-icon"></i>
      </Tooltip>
      <i class="fa-solid fa-chevron-down chevron-icon" class:open={synonymSectionOpen}></i>
    </summary>
    <div class="synonyms-content">
      <p class="synonyms-help">
        {$_("network.synonymsHelp") ||
          "Wähle semantisch ähnliche Keywords aus, um sie zusammenzuführen. Die Ähnlichkeit basiert auf Embedding-Vektoren, nicht auf lexikalischer Übereinstimmung."}
      </p>

      {#if synonymError}
        <div class="synonym-message error">
          <i class="fa-solid fa-triangle-exclamation"></i>
          {synonymError}
        </div>
      {/if}

      {#if synonymSuccess}
        <div class="synonym-message success">
          <i class="fa-solid fa-check-circle"></i>
          {synonymSuccess}
        </div>
      {/if}

      {#if similarKeywordsLoading}
        <div class="loading-similar">
          <i class="fa-solid fa-spinner fa-spin"></i>
          {$_("network.loading") || "Laden..."}
        </div>
      {:else}
        <div class="synonyms-list">
          {#each similarKeywords as simKw (simKw.id)}
            {@const similarityPercent = Math.round(simKw.similarity * 100)}
            {@const isSelected = selectedSynonymIds.has(simKw.id)}
            {@const similarityClass = simKw.is_true_synonym
              ? "synonym"
              : similarityPercent >= 80
                ? "high"
                : similarityPercent >= 60
                  ? "medium"
                  : "low"}
            <label
              class="synonym-item"
              class:selected={isSelected}
              class:true-synonym={simKw.is_true_synonym}
              title={simKw.is_true_synonym
                ? $_("network.trueSynonymHint") || "Bereits zusammengeführtes Synonym"
                : simKw.cooccurrence > 0
                  ? simKw.cooccurrence + " gemeinsame Artikel"
                  : "Semantisch ähnlich"}
            >
              {#if !simKw.is_true_synonym}
                <input
                  type="checkbox"
                  checked={isSelected}
                  onchange={() => toggleSynonymSelection(simKw.id)}
                  disabled={assigningSynonyms}
                />
              {:else}
                <span class="true-synonym-badge">
                  <i class="fa-solid fa-check"></i>
                </span>
              {/if}
              <KeywordContextTooltip keywordId={simKw.id} keywordName={simKw.name}>
                <span class="synonym-name">{simKw.name}</span>
              </KeywordContextTooltip>
              <div class="synonym-bar-wrap">
                <div
                  class="similarity-bar {similarityClass}"
                  style="width: {similarityPercent}%"
                ></div>
              </div>
              <span class="synonym-stats">
                {#if simKw.is_true_synonym}
                  <span class="similarity-pct synonym">{$_("network.merged") || "Synonym"}</span>
                {:else}
                  <span class="similarity-pct {similarityClass}">{similarityPercent}%</span>
                  {#if simKw.cooccurrence > 0}
                    <span class="cooccur-count">({simKw.cooccurrence})</span>
                  {/if}
                {/if}
              </span>
              <button
                class="synonym-view-btn"
                onclick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  onKeywordSelect(simKw.id);
                }}
                title={$_("network.showInNetwork") || "Im Netzwerk anzeigen"}
              >
                <i class="fa-solid fa-arrow-up-right-from-square"></i>
              </button>
            </label>
          {/each}
        </div>
      {/if}

      <div class="synonyms-actions">
        <button
          class="btn-assign-synonyms"
          onclick={assignSelectedAsSynonyms}
          disabled={selectedSynonymIds.size === 0 || assigningSynonyms}
        >
          {#if assigningSynonyms}
            <i class="fa-solid fa-spinner fa-spin"></i>
          {:else}
            <i class="fa-solid fa-link"></i>
          {/if}
          {$_("network.assignAsSynonyms") || "Als Synonyme zuordnen"}
          {#if selectedSynonymIds.size > 0}
            ({selectedSynonymIds.size})
          {/if}
        </button>
      </div>
    </div>
  </details>
{/if}

<style>
  .synonyms-section {
    margin-bottom: 1.5rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    overflow: hidden;
  }

  .synonyms-summary {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    cursor: pointer;
    user-select: none;
    transition: background-color 0.15s;
    list-style: none;
  }

  .synonyms-summary::-webkit-details-marker {
    display: none;
  }

  .synonyms-summary:hover {
    background-color: var(--bg-overlay);
  }

  .summary-icon {
    color: var(--accent-primary);
    font-size: 0.875rem;
  }

  .synonyms-summary span {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .synonym-count {
    font-weight: 400;
    color: var(--text-muted);
  }

  .help-icon {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .chevron-icon {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: transform 0.2s;
  }

  .chevron-icon.open {
    transform: rotate(180deg);
  }

  .synonyms-content {
    padding: 1rem;
    border-top: 1px solid var(--border-muted);
  }

  .synonyms-help {
    font-size: 0.8125rem;
    color: var(--text-muted);
    margin: 0 0 0.75rem 0;
    line-height: 1.4;
  }

  .synonym-message {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.8125rem;
    margin-bottom: 0.75rem;
  }

  .synonym-message.error {
    background-color: rgba(243, 139, 168, 0.15);
    border: 1px solid var(--status-error);
    color: var(--status-error);
  }

  .synonym-message.success {
    background-color: rgba(166, 227, 161, 0.15);
    border: 1px solid var(--status-success);
    color: var(--status-success);
  }

  .loading-similar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .synonyms-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-bottom: 0.75rem;
  }

  .synonym-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-muted);
    border-radius: 0.375rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .synonym-item:hover {
    border-color: var(--accent-primary);
  }

  .synonym-item.selected {
    background-color: rgba(137, 180, 250, 0.1);
    border-color: var(--accent-primary);
  }

  .synonym-item input[type="checkbox"] {
    width: 1rem;
    height: 1rem;
    cursor: pointer;
    accent-color: var(--accent-primary);
  }

  .synonym-name {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 120px;
    flex-shrink: 0;
  }

  .synonym-bar-wrap {
    flex: 1;
    height: 6px;
    background-color: var(--bg-muted);
    border-radius: 3px;
    overflow: hidden;
    min-width: 60px;
  }

  .similarity-bar {
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .similarity-bar.high {
    background: var(--accent-success);
  }

  .similarity-bar.medium {
    background: var(--accent-warning);
  }

  .similarity-bar.low {
    background: var(--text-muted);
  }

  .similarity-bar.synonym {
    background: var(--accent-primary);
  }

  .synonym-stats {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
    min-width: 55px;
    justify-content: flex-end;
    flex-shrink: 0;
  }

  .similarity-pct {
    font-size: 0.75rem;
    font-weight: 600;
  }

  .similarity-pct.high {
    color: var(--accent-success);
  }

  .similarity-pct.medium {
    color: var(--accent-warning);
  }

  .similarity-pct.low {
    color: var(--text-muted);
  }

  .similarity-pct.synonym {
    color: var(--accent-primary);
    font-size: 0.6875rem;
  }

  .synonym-item.true-synonym {
    background-color: rgba(137, 180, 250, 0.08);
    border-color: var(--accent-primary);
  }

  .true-synonym-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    background-color: var(--accent-primary);
    border-radius: 50%;
    color: var(--text-on-accent);
    font-size: 0.625rem;
    flex-shrink: 0;
  }

  .cooccur-count {
    font-size: 0.625rem;
    color: var(--text-muted);
  }

  .synonyms-actions {
    display: flex;
    justify-content: flex-end;
  }

  .btn-assign-synonyms {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-assign-synonyms:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-assign-synonyms:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
