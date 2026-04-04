<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { SvelteSet } from "svelte/reactivity";
  import Tooltip from "../Tooltip.svelte";
  import ActionButton from "$lib/components/ui/ActionButton.svelte";
  import { formatError } from "$lib/utils/formatError";
  import type { TrueSynonymCandidate, SynonymVerificationResult } from "../../types";

  interface Props {
    synonymsLoading: boolean;
    onMergeKeywords: (
      keepId: number,
      removeId: number,
      keepName: string,
      removeName: string,
    ) => void;
    onDismissSynonymPair: (keywordAId: number, keywordBId: number) => void;
  }

  let { synonymsLoading, onMergeKeywords, onDismissSynonymPair }: Props = $props();

  // True synonyms state (hybrid string + embedding similarity)
  let trueSynonymCandidates = $state<TrueSynonymCandidate[]>([]);
  let trueSynonymsLoading = $state(false);
  let trueSynonymsError = $state<string | null>(null);

  // Verification state - map from "keywordA|keywordB" to verification result
  let verificationResults = $state<Map<string, SynonymVerificationResult>>(new Map());
  let verifyingPairs = new SvelteSet<string>();

  // Invalidate cached data when keywords change (e.g. after batch processing or merges)
  function handleKeywordsChanged() {
    trueSynonymCandidates = [];
    verificationResults = new Map();
  }

  onMount(() => {
    window.addEventListener("keywords-changed", handleKeywordsChanged);
  });

  onDestroy(() => {
    window.removeEventListener("keywords-changed", handleKeywordsChanged);
  });

  // Get verification key for a pair
  function getVerificationKey(nameA: string, nameB: string): string {
    return [nameA, nameB].sort().join("|");
  }

  // Load true synonym candidates
  async function loadTrueSynonyms() {
    trueSynonymsLoading = true;
    trueSynonymsError = null;
    try {
      trueSynonymCandidates = await invoke<TrueSynonymCandidate[]>("find_true_synonyms", {
        stringThreshold: 0.6,
        embeddingThreshold: 0.7,
        limit: 50,
      });
    } catch (e) {
      trueSynonymsError = formatError(e);
      console.error("Failed to load true synonyms:", e);
    } finally {
      trueSynonymsLoading = false;
    }
  }

  // Verify a single synonym pair with AI
  async function verifySynonymPair(keywordA: string, keywordB: string) {
    const key = getVerificationKey(keywordA, keywordB);
    verifyingPairs.add(key);

    try {
      const result = await invoke<SynonymVerificationResult>("verify_synonym_pair", {
        keywordA,
        keywordB,
      });
      verificationResults = new Map([...verificationResults, [key, result]]);
    } catch (e) {
      console.error("Failed to verify synonym pair:", e);
      // Store failed result
      verificationResults = new Map([
        ...verificationResults,
        [
          key,
          {
            keyword_a: keywordA,
            keyword_b: keywordB,
            is_synonym: false,
            confidence: 0,
            explanation: $_("network.verificationFailed") || "Verification failed",
          },
        ],
      ]);
    } finally {
      verifyingPairs.delete(key);
    }
  }

  // Check if a pair is being verified
  function isVerifying(nameA: string, nameB: string): boolean {
    return verifyingPairs.has(getVerificationKey(nameA, nameB));
  }

  // Get verification result for a pair
  function getVerificationResult(
    nameA: string,
    nameB: string,
  ): SynonymVerificationResult | undefined {
    return verificationResults.get(getVerificationKey(nameA, nameB));
  }
</script>

<div class="synonyms-section full-width true-synonyms-section">
  <h3 class="section-heading">
    <i class="fa-solid fa-equals"></i>
    {$_("network.trueSynonyms") || "Echte Synonyme"}
  </h3>
  <p class="section-hint">
    {$_("network.trueSynonymsHint") ||
      "Diese Paare sind wahrscheinlich echte Synonyme (Abkürzungen, alternative Namen) basierend auf String-Ähnlichkeit."}
  </p>

  <ActionButton
    variant="primary"
    onclick={loadTrueSynonyms}
    disabled={trueSynonymsLoading}
    loading={trueSynonymsLoading}
    icon={trueSynonymsLoading ? undefined : "fa-solid fa-magnifying-glass"}
  >
    {#if trueSynonymsLoading}
      {$_("network.loading") || "Lade..."}
    {:else}
      {$_("network.findTrueSynonyms") || "Echte Synonyme finden"}
    {/if}
  </ActionButton>

  {#if trueSynonymsError}
    <div class="feedback-message error">{trueSynonymsError}</div>
  {/if}

  {#if trueSynonymCandidates.length > 0}
    <div class="true-synonym-list">
      {#each trueSynonymCandidates as candidate (candidate.keyword_a_id + "-" + candidate.keyword_b_id)}
        {@const verificationResult = getVerificationResult(
          candidate.keyword_a_name,
          candidate.keyword_b_name,
        )}
        {@const isVerifyingPair = isVerifying(candidate.keyword_a_name, candidate.keyword_b_name)}
        <div
          class="true-synonym-item"
          class:verified-synonym={verificationResult?.is_synonym}
          class:verified-not-synonym={verificationResult && !verificationResult.is_synonym}
        >
          <div class="true-synonym-pair">
            <span class="true-synonym-keyword">{candidate.keyword_a_name}</span>
            <i class="fa-solid fa-arrows-left-right pair-arrow"></i>
            <span class="true-synonym-keyword">{candidate.keyword_b_name}</span>
          </div>

          <div class="true-synonym-badges">
            {#if candidate.is_abbreviation}
              <span
                class="badge badge-abbreviation"
                title={$_("network.abbreviation") || "Abkürzung"}
              >
                <i class="fa-solid fa-text-width"></i>
                {$_("network.abbreviation") || "Abkürzung"}
              </span>
            {/if}
            {#if candidate.is_name_variant}
              <span
                class="badge badge-name-variant"
                title={$_("network.nameVariant") || "Namensform"}
              >
                <i class="fa-solid fa-user-tag"></i>
                {$_("network.nameVariant") || "Namensform"}
              </span>
            {/if}
            <span class="badge badge-string" title={$_("network.stringSimLabel") || "String"}>
              {$_("network.stringSimLabel") || "Str"}: {(
                candidate.string_similarity * 100
              ).toFixed(0)}%
            </span>
            <span
              class="badge badge-embedding"
              title={$_("network.embeddingSimLabel") || "Embedding"}
            >
              {$_("network.embeddingSimLabel") || "Emb"}: {(
                candidate.embedding_similarity * 100
              ).toFixed(0)}%
            </span>
            <span class="badge badge-combined">
              {(candidate.combined_score * 100).toFixed(0)}%
            </span>
          </div>

          <div class="true-synonym-actions">
            <!-- AI Verification Button -->
            {#if verificationResult}
              <Tooltip
                content={verificationResult.explanation ||
                  (verificationResult.is_synonym
                    ? $_("network.verifiedAsSynonym")
                    : $_("network.verifiedAsNotSynonym"))}
              >
                <span
                  class="verification-result"
                  class:is-synonym={verificationResult.is_synonym}
                  class:not-synonym={!verificationResult.is_synonym}
                >
                  {#if verificationResult.is_synonym}
                    <i class="fa-solid fa-check-circle"></i>
                  {:else}
                    <i class="fa-solid fa-times-circle"></i>
                  {/if}
                </span>
              </Tooltip>
            {:else}
              <button
                class="verify-btn"
                onclick={() =>
                  verifySynonymPair(candidate.keyword_a_name, candidate.keyword_b_name)}
                disabled={isVerifyingPair || synonymsLoading}
                title={$_("network.verifyWithAI") || "Mit KI verifizieren"}
              >
                {#if isVerifyingPair}
                  <i class="fa-solid fa-spinner fa-spin"></i>
                {:else}
                  <i class="fa-solid fa-wand-magic-sparkles"></i>
                {/if}
              </button>
            {/if}

            <!-- Merge buttons -->
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
              onclick={() => {
                onDismissSynonymPair(candidate.keyword_a_id, candidate.keyword_b_id);
                trueSynonymCandidates = trueSynonymCandidates.filter(
                  (c) =>
                    !(
                      c.keyword_a_id === candidate.keyword_a_id &&
                      c.keyword_b_id === candidate.keyword_b_id
                    ),
                );
              }}
              title={$_("network.dismissSynonym") || "Ignorieren"}
            >
              <i class="fa-solid fa-xmark"></i>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {:else if !trueSynonymsLoading && trueSynonymCandidates.length === 0}
    <div class="empty-hint">
      {$_("network.clickFindSynonyms") ||
        'Klicke auf "Echte Synonyme finden" um Vorschläge zu laden'}
    </div>
  {/if}
</div>

<style>
  .synonyms-section {
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--border-default);
  }

  .synonyms-section.full-width {
    width: 100%;
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

  /* True Synonyms Section */
  .true-synonyms-section {
    margin-top: 1rem;
  }

  .true-synonyms-section .section-heading {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .true-synonym-list {
    margin-top: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 400px;
    overflow-y: auto;
  }

  .true-synonym-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.625rem 0.875rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-muted);
    transition: all 0.2s;
  }

  .true-synonym-item:hover {
    border-color: var(--border-default);
  }

  .true-synonym-item.verified-synonym {
    border-color: var(--accent-success);
    background-color: rgba(34, 197, 94, 0.08);
  }

  .true-synonym-item.verified-not-synonym {
    border-color: var(--accent-error);
    background-color: rgba(239, 68, 68, 0.08);
    opacity: 0.7;
  }

  .true-synonym-pair {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
  }

  .true-synonym-keyword {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pair-arrow {
    color: var(--text-muted);
    font-size: 0.625rem;
    flex-shrink: 0;
  }

  .true-synonym-badges {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-shrink: 0;
  }

  .badge {
    font-size: 0.625rem;
    font-weight: 600;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    white-space: nowrap;
  }

  .badge-abbreviation {
    background-color: rgba(251, 191, 36, 0.2);
    color: var(--accent-warning);
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .badge-abbreviation i {
    font-size: 0.5rem;
  }

  .badge-name-variant {
    background-color: rgba(166, 227, 161, 0.2);
    color: var(--accent-success, #a6e3a1);
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .badge-name-variant i {
    font-size: 0.5rem;
  }

  .badge-string {
    background-color: rgba(137, 180, 250, 0.15);
    color: var(--accent-primary);
  }

  .badge-embedding {
    background-color: rgba(203, 166, 247, 0.15);
    color: var(--accent-secondary, #cba6f7);
  }

  .badge-combined {
    background-color: var(--bg-surface);
    color: var(--text-primary);
    font-weight: 700;
  }

  .true-synonym-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .verify-btn {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0.25rem;
    border: 1px solid var(--accent-secondary, #cba6f7);
    background-color: rgba(203, 166, 247, 0.1);
    color: var(--accent-secondary, #cba6f7);
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.75rem;
  }

  .verify-btn:hover:not(:disabled) {
    background-color: var(--accent-secondary, #cba6f7);
    color: var(--bg-base);
  }

  .verify-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .verification-result {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
  }

  .verification-result.is-synonym {
    color: var(--accent-success);
  }

  .verification-result.not-synonym {
    color: var(--accent-error);
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
