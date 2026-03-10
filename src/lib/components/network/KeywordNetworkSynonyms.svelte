<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { SvelteSet } from "svelte/reactivity";
  import Tooltip from "../Tooltip.svelte";
  import ActionButton from "$lib/components/ui/ActionButton.svelte";
  import { formatError } from "$lib/utils/formatError";
  import type { Keyword, TrueSynonymCandidate, SynonymVerificationResult } from "../../types";

  // Type for synonym candidate
  interface SynonymCandidate {
    keyword_a_id: number;
    keyword_a_name: string;
    keyword_b_id: number;
    keyword_b_name: string;
    similarity: number;
  }

  // Props
  interface Props {
    synonymCandidates: SynonymCandidate[];
    synonymsLoading: boolean;
    synonymsError: string | null;
    synonymSuccess: string | null;
    // Manual merge state
    keepSearchInput: string;
    keepSearchResults: Keyword[];
    selectedKeepKeyword: Keyword | null;
    removeSearchInput: string;
    removeSearchResults: Keyword[];
    selectedRemoveKeyword: Keyword | null;
    // Create keyword state
    newKeywordInput: string;
    createKeywordLoading: boolean;
    createKeywordSuccess: string | null;
    createKeywordError: string | null;
    // Callbacks
    onFindSynonyms: () => void;
    onMergeKeywords: (
      keepId: number,
      removeId: number,
      keepName: string,
      removeName: string,
    ) => void;
    onDismissSynonymPair: (keywordAId: number, keywordBId: number) => void;
    onKeepSearchInput: (value: string) => void;
    onSelectKeepKeyword: (keyword: Keyword) => void;
    onClearKeepSearch: () => void;
    onRemoveSearchInput: (value: string) => void;
    onSelectRemoveKeyword: (keyword: Keyword) => void;
    onClearRemoveSearch: () => void;
    onExecuteManualMerge: () => void;
    onNewKeywordInput: (value: string) => void;
    onCreateNewKeyword: () => void;
  }

  let {
    synonymCandidates,
    synonymsLoading,
    synonymsError,
    synonymSuccess,
    keepSearchInput,
    keepSearchResults,
    selectedKeepKeyword,
    removeSearchInput,
    removeSearchResults,
    selectedRemoveKeyword,
    newKeywordInput,
    createKeywordLoading,
    createKeywordSuccess,
    createKeywordError,
    onFindSynonyms,
    onMergeKeywords,
    onDismissSynonymPair,
    onKeepSearchInput,
    onSelectKeepKeyword,
    onClearKeepSearch,
    onRemoveSearchInput,
    onSelectRemoveKeyword,
    onClearRemoveSearch,
    onExecuteManualMerge,
    onNewKeywordInput,
    onCreateNewKeyword,
  }: Props = $props();

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

  function handleNewKeywordKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      onCreateNewKeyword();
    }
  }
</script>

<div class="synonyms-view">
  <!-- Manual Merge - Full Width at Top -->
  <div class="synonyms-section full-width">
    <h3 class="section-heading">{$_("network.manualMerge") || "Manuelles Zusammenfuehren"}</h3>
    <p class="section-description">
      {$_("network.manualMergeDescription") ||
        "Waehle zwei Keywords aus: Das erste bleibt erhalten, das zweite wird geloescht und alle Verknuepfungen werden uebertragen."}
    </p>

    <div class="merge-form">
      <!-- Keep Keyword (Target) -->
      <div class="merge-field">
        <label class="merge-label" for="keep-keyword-search">
          <i class="fa-solid fa-check merge-label-icon keep"></i>
          {$_("network.keepKeyword") || "Behalten"}
        </label>
        <div class="merge-search-box">
          <input
            type="text"
            id="keep-keyword-search"
            value={keepSearchInput}
            oninput={(e) => onKeepSearchInput(e.currentTarget.value)}
            placeholder={$_("network.searchKeywordPlaceholder") || "Keyword suchen..."}
            class="merge-search-input"
          />
          {#if keepSearchInput}
            <button
              onclick={onClearKeepSearch}
              class="clear-btn"
              aria-label={$_("search.clearSearch")}
              ><i class="fa-solid fa-xmark" aria-hidden="true"></i></button
            >
          {/if}
          {#if keepSearchResults.length > 0 && !selectedKeepKeyword}
            <div class="merge-search-results">
              {#each keepSearchResults as keyword (keyword.id)}
                <button class="merge-search-item" onclick={() => onSelectKeepKeyword(keyword)}>
                  <span class="item-name">{keyword.name}</span>
                  <span class="item-count">{keyword.article_count}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
        {#if selectedKeepKeyword}
          <div class="selected-chip keep">
            <i class="fa-solid fa-check"></i>
            <span>{selectedKeepKeyword.name}</span>
            <span class="chip-count">({selectedKeepKeyword.article_count} Artikel)</span>
          </div>
        {/if}
      </div>

      <!-- Visual Arrow -->
      <div class="merge-arrow">
        <i class="fa-solid fa-arrow-right"></i>
        <span class="arrow-label">{$_("network.replacesLabel") || "ersetzt"}</span>
      </div>

      <!-- Remove Keyword (Source) -->
      <div class="merge-field">
        <label class="merge-label" for="remove-keyword-search">
          <i class="fa-solid fa-trash merge-label-icon remove"></i>
          {$_("network.removeKeyword") || "Loeschen"}
        </label>
        <div class="merge-search-box">
          <input
            type="text"
            id="remove-keyword-search"
            value={removeSearchInput}
            oninput={(e) => onRemoveSearchInput(e.currentTarget.value)}
            placeholder={$_("network.searchKeywordPlaceholder") || "Keyword suchen..."}
            class="merge-search-input"
          />
          {#if removeSearchInput}
            <button
              onclick={onClearRemoveSearch}
              class="clear-btn"
              aria-label={$_("search.clearSearch")}
              ><i class="fa-solid fa-xmark" aria-hidden="true"></i></button
            >
          {/if}
          {#if removeSearchResults.length > 0 && !selectedRemoveKeyword}
            <div class="merge-search-results">
              {#each removeSearchResults as keyword (keyword.id)}
                {#if keyword.id !== selectedKeepKeyword?.id}
                  <button class="merge-search-item" onclick={() => onSelectRemoveKeyword(keyword)}>
                    <span class="item-name">{keyword.name}</span>
                    <span class="item-count">{keyword.article_count}</span>
                  </button>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
        {#if selectedRemoveKeyword}
          <div class="selected-chip remove">
            <i class="fa-solid fa-trash"></i>
            <span>{selectedRemoveKeyword.name}</span>
            <span class="chip-count">({selectedRemoveKeyword.article_count} Artikel)</span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Merge Preview & Action -->
    {#if selectedKeepKeyword && selectedRemoveKeyword}
      <div class="merge-preview">
        <div class="preview-text">
          <i class="fa-solid fa-circle-info"></i>
          <span>
            <strong>"{selectedRemoveKeyword.name}"</strong> wird geloescht. Alle {selectedRemoveKeyword.article_count}
            Artikel werden zu <strong>"{selectedKeepKeyword.name}"</strong> uebertragen.
          </span>
        </div>
        <button
          class="action-btn danger"
          onclick={onExecuteManualMerge}
          disabled={synonymsLoading || selectedKeepKeyword.id === selectedRemoveKeyword.id}
        >
          {#if synonymsLoading}
            <i class="fa-solid fa-rotate fa-spin"></i>
          {:else}
            <i class="fa-solid fa-code-merge"></i>
          {/if}
          {$_("network.executeMerge") || "Zusammenfuehren"}
        </button>
      </div>
    {:else}
      <div class="merge-hint">
        <i class="fa-solid fa-hand-pointer"></i>
        {$_("network.selectBothKeywords") || "Waehle beide Keywords aus, um sie zusammenzufuehren."}
      </div>
    {/if}

    {#if selectedKeepKeyword && selectedRemoveKeyword && selectedKeepKeyword.id === selectedRemoveKeyword.id}
      <div class="merge-error">
        <i class="fa-solid fa-triangle-exclamation"></i>
        {$_("network.sameKeywordError") || "Die beiden Keywords muessen unterschiedlich sein."}
      </div>
    {/if}
  </div>

  <!-- Two columns below -->
  <div class="synonyms-columns">
    <!-- Semantically Similar Keywords (Embedding-based) -->
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

    <!-- Create New Keyword -->
    <div class="synonyms-section">
      <h3 class="section-heading">{$_("network.createKeyword") || "Neues Keyword erstellen"}</h3>
      <div class="create-keyword-form">
        <input
          type="text"
          value={newKeywordInput}
          oninput={(e) => onNewKeywordInput(e.currentTarget.value)}
          placeholder={$_("network.newKeywordPlaceholder") || "Keyword eingeben..."}
          class="create-keyword-input"
          onkeydown={handleNewKeywordKeydown}
        />
        <ActionButton
          variant="primary"
          onclick={onCreateNewKeyword}
          disabled={createKeywordLoading || !newKeywordInput.trim()}
          loading={createKeywordLoading}
        >
          {#if createKeywordLoading}
            {$_("network.loading") || "Lade..."}
          {:else}
            {$_("network.create") || "Erstellen"}
          {/if}
        </ActionButton>
      </div>
      {#if createKeywordError}
        <div class="feedback-message error">{createKeywordError}</div>
      {/if}
      {#if createKeywordSuccess}
        <div class="feedback-message success">{createKeywordSuccess}</div>
      {/if}
    </div>
  </div>

  <!-- True Synonyms Section (Hybrid String + Embedding) -->
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
</div>

<style>
  /* Synonyms View */
  .synonyms-view {
    flex: 1;
    padding: 1rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .synonyms-columns {
    display: flex;
    gap: 1rem;
  }

  .synonyms-columns .synonyms-section {
    flex: 1;
  }

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

  .section-description {
    font-size: 0.8125rem;
    color: var(--text-muted);
    margin-bottom: 1rem;
    line-height: 1.5;
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

  .action-btn {
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid var(--border-default);
    background-color: var(--bg-overlay);
    color: var(--text-primary);
  }

  .action-btn:hover:not(:disabled) {
    background-color: var(--bg-tertiary);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-btn.primary {
    background-color: var(--accent-primary);
    border-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .action-btn.primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .action-btn.danger {
    background-color: var(--accent-error);
    border-color: var(--accent-error);
    color: white;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .action-btn.danger:hover:not(:disabled) {
    opacity: 0.9;
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

  /* Manual Merge Form */
  .merge-form {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .merge-field {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .merge-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .merge-label-icon {
    font-size: 0.875rem;
  }

  .merge-label-icon.keep {
    color: var(--accent-success);
  }

  .merge-label-icon.remove {
    color: var(--accent-error);
  }

  .merge-search-box {
    position: relative;
  }

  .merge-search-input {
    width: 100%;
    padding: 0.625rem 2rem 0.625rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .merge-search-input::placeholder {
    color: var(--text-faint);
  }

  .merge-search-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .clear-btn {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.875rem;
    line-height: 1;
    padding: 0.25rem;
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }

  .merge-search-results {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 10;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-top: none;
    border-radius: 0 0 0.375rem 0.375rem;
    max-height: 200px;
    overflow-y: auto;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .merge-search-item {
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
    transition: background-color 0.15s;
  }

  .merge-search-item:hover {
    background-color: var(--bg-overlay);
  }

  .merge-search-item .item-name {
    font-size: 0.875rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .merge-search-item .item-count {
    font-size: 0.75rem;
    color: var(--text-muted);
    background-color: var(--bg-overlay);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    flex-shrink: 0;
  }

  .selected-chip {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .selected-chip.keep {
    background-color: rgba(34, 197, 94, 0.15);
    border: 1px solid var(--accent-success);
    color: var(--accent-success);
  }

  .selected-chip.remove {
    background-color: rgba(239, 68, 68, 0.15);
    border: 1px solid var(--accent-error);
    color: var(--accent-error);
  }

  .selected-chip i {
    font-size: 0.75rem;
  }

  .chip-count {
    font-size: 0.75rem;
    opacity: 0.8;
    font-weight: 400;
  }

  .merge-arrow {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    padding-top: 1.5rem;
    color: var(--text-muted);
  }

  .merge-arrow i {
    font-size: 1.25rem;
  }

  .arrow-label {
    font-size: 0.625rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .merge-preview {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border-left: 3px solid var(--accent-warning);
  }

  .preview-text {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .preview-text i {
    color: var(--accent-warning);
    margin-top: 0.125rem;
    flex-shrink: 0;
  }

  .preview-text strong {
    color: var(--text-primary);
  }

  .merge-hint {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--text-muted);
    font-size: 0.8125rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px dashed var(--border-default);
  }

  .merge-error {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    margin-top: 0.5rem;
    background-color: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--accent-error);
    border-radius: 0.375rem;
    color: var(--accent-error);
    font-size: 0.8125rem;
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

  /* Create Keyword Form */
  .create-keyword-form {
    display: flex;
    gap: 0.5rem;
  }

  .create-keyword-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .create-keyword-input::placeholder {
    color: var(--text-faint);
  }

  .create-keyword-input:focus {
    outline: none;
    border-color: var(--accent-primary);
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
</style>
