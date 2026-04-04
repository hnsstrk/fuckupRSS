<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { Keyword } from "../../types";
  import SynonymManualMerge from "./SynonymManualMerge.svelte";
  import SynonymSemanticCandidates from "./SynonymSemanticCandidates.svelte";
  import SynonymCreateKeyword from "./SynonymCreateKeyword.svelte";
  import SynonymTrueSynonyms from "./SynonymTrueSynonyms.svelte";

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
</script>

<div class="synonyms-view">
  <!-- Manual Merge - Full Width at Top -->
  <SynonymManualMerge
    {keepSearchInput}
    {keepSearchResults}
    {selectedKeepKeyword}
    {removeSearchInput}
    {removeSearchResults}
    {selectedRemoveKeyword}
    {synonymsLoading}
    {onKeepSearchInput}
    {onSelectKeepKeyword}
    {onClearKeepSearch}
    {onRemoveSearchInput}
    {onSelectRemoveKeyword}
    {onClearRemoveSearch}
    {onExecuteManualMerge}
  />

  <!-- Two columns below -->
  <div class="synonyms-columns">
    <!-- Semantically Similar Keywords (Embedding-based) -->
    <SynonymSemanticCandidates
      {synonymCandidates}
      {synonymsLoading}
      {synonymsError}
      {synonymSuccess}
      {onFindSynonyms}
      {onMergeKeywords}
      {onDismissSynonymPair}
    />

    <!-- Create New Keyword -->
    <SynonymCreateKeyword
      {newKeywordInput}
      {createKeywordLoading}
      {createKeywordSuccess}
      {createKeywordError}
      {onNewKeywordInput}
      {onCreateNewKeyword}
    />
  </div>

  <!-- True Synonyms Section (Hybrid String + Embedding) -->
  <SynonymTrueSynonyms
    {synonymsLoading}
    {onMergeKeywords}
    {onDismissSynonymPair}
  />
</div>

<style>
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
</style>
