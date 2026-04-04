<script lang="ts">
  import { _ } from "svelte-i18n";
  import ActionButton from "$lib/components/ui/ActionButton.svelte";

  // Type for synonym candidate
  interface SynonymCandidate {
    keyword_a_id: number;
    keyword_a_name: string;
    keyword_b_id: number;
    keyword_b_name: string;
    similarity: number;
  }

  interface Props {
    synonymCandidates: SynonymCandidate[];
    synonymsLoading: boolean;
    synonymsError: string | null;
    synonymSuccess: string | null;
    onFindSynonyms: () => void;
    onMergeKeywords: (
      keepId: number,
      removeId: number,
      keepName: string,
      removeName: string,
    ) => void;
    onDismissSynonymPair: (keywordAId: number, keywordBId: number) => void;
  }

  let {
    synonymCandidates,
    synonymsLoading,
    synonymsError,
    synonymSuccess,
    onFindSynonyms,
    onMergeKeywords,
    onDismissSynonymPair,
  }: Props = $props();
</script>

<div class="synonyms-section">
  <h3 class="section-heading">
    {$_("network.synonymCandidates") || "Semantisch ähnliche Keywords"}
  </h3>
  <p class="section-hint">
    {$_("network.synonymCandidatesHint") ||
      "Diese Vorschläge basieren auf Embedding-Ähnlichkeit (semantische Nähe), nicht auf lexikalischer Übereinstimmung."}
  </p>
  <ActionButton
    variant="primary"
    onclick={onFindSynonyms}
    disabled={synonymsLoading}
    loading={synonymsLoading}
  >
    {#if synonymsLoading}
      {$_("network.loading") || "Lade..."}
    {:else}
      {$_("network.findSynonyms") || "Synonyme finden"}
    {/if}
  </ActionButton>

  {#if synonymsError}
    <div class="feedback-message error">{synonymsError}</div>
  {/if}
  {#if synonymSuccess}
    <div class="feedback-message success">{synonymSuccess}</div>
  {/if}

  {#if synonymCandidates.length > 0}
    <div class="synonym-list">
      {#each synonymCandidates as candidate (candidate.keyword_a_id + "-" + candidate.keyword_b_id)}
        <div class="synonym-item">
          <div class="synonym-pair">
            <span class="synonym-keyword">{candidate.keyword_a_name}</span>
            <span class="synonym-similarity">{(candidate.similarity * 100).toFixed(0)}%</span>
            <span class="synonym-keyword">{candidate.keyword_b_name}</span>
          </div>
          <div class="synonym-actions">
            <button
              class="merge-btn left"
              onclick={() =>
                onMergeKeywords(
                  candidate.keyword_a_id,
                  candidate.keyword_b_id,
                  candidate.keyword_a_name,
                  candidate.keyword_b_name,
                )}
              title="{candidate.keyword_b_name} -> {candidate.keyword_a_name}"
              disabled={synonymsLoading}
            >
              <i class="fa-solid fa-arrow-left"></i>
            </button>
            <button
              class="merge-btn right"
              onclick={() =>
                onMergeKeywords(
                  candidate.keyword_b_id,
                  candidate.keyword_a_id,
                  candidate.keyword_b_name,
                  candidate.keyword_a_name,
                )}
              title="{candidate.keyword_a_name} -> {candidate.keyword_b_name}"
              disabled={synonymsLoading}
            >
              <i class="fa-solid fa-arrow-right"></i>
            </button>
            <button
              class="dismiss-btn"
              onclick={() =>
                onDismissSynonymPair(candidate.keyword_a_id, candidate.keyword_b_id)}
              title={$_("network.dismissSynonym") || "Ignorieren"}
            >
              <i class="fa-solid fa-xmark"></i>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {:else if !synonymsLoading && synonymCandidates.length === 0}
    <div class="empty-hint">
      {$_("network.clickFindSynonyms") ||
        'Klicke auf "Synonyme finden" um KI-Vorschlaege zu laden'}
    </div>
  {/if}
</div>

<style>
  .synonyms-section {
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--border-default);
    flex: 1;
  }

  .section-heading {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .section-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0 0 0.75rem 0;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.25rem;
    border-left: 3px solid var(--accent-primary);
    line-height: 1.4;
  }

  .feedback-message {
    margin-top: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.75rem;
  }

  .feedback-message.error {
    background-color: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--accent-error);
    color: var(--accent-error);
  }

  .feedback-message.success {
    background-color: rgba(34, 197, 94, 0.1);
    border: 1px solid var(--accent-success);
    color: var(--accent-success);
  }

  /* Synonym List */
  .synonym-list {
    margin-top: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 400px;
    overflow-y: auto;
  }

  .synonym-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    gap: 0.5rem;
  }

  .synonym-pair {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
  }

  .synonym-keyword {
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .synonym-similarity {
    font-size: 0.625rem;
    font-weight: 600;
    padding: 0.125rem 0.375rem;
    background-color: var(--bg-surface);
    border-radius: 0.25rem;
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  .synonym-actions {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .merge-btn {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0.25rem;
    border: 1px solid var(--border-default);
    background-color: var(--bg-surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.875rem;
  }

  .merge-btn:hover:not(:disabled) {
    background-color: var(--accent-success);
    border-color: var(--accent-success);
    color: var(--text-on-accent);
  }

  .merge-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dismiss-btn {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0.25rem;
    border: 1px solid var(--border-default);
    background-color: var(--bg-surface);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.75rem;
  }

  .dismiss-btn:hover {
    background-color: var(--accent-error);
    border-color: var(--accent-error);
    color: var(--text-on-accent);
  }

  .empty-hint {
    margin-top: 0.75rem;
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
  }
</style>
