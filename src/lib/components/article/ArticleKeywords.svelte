<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import type { ArticleKeyword, CorrectionInput } from "$lib/types";
  import { navigationStore } from "$lib/stores/navigation.svelte";
  import KeywordChip from "./KeywordChip.svelte";
  import KeywordSuggestions from "./KeywordSuggestions.svelte";
  import KeywordSearchInput from "./KeywordSearchInput.svelte";
  import { createLogger } from "$lib/logger";

  const log = createLogger("ArticleKeywords");
  interface Props {
    fnordId: number;
    keywords: ArticleKeyword[];
    editing: boolean;
    onUpdate: (keywords: ArticleKeyword[]) => void;
  }

  let { fnordId, keywords, editing, onUpdate }: Props = $props();

  // Search state
  let searchInput = $state("");
  let searchResults = $state<{ id: number; name: string; count: number }[]>([]);
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  let loading = $state(false);
  let showDropdown = $state(false);

  // Suggested keywords from statistical analysis
  let suggestedKeywords = $state<{ term: string; score: number }[]>([]);
  let loadingSuggestions = $state(false);

  // Similar keywords from network
  let similarKeywords = $state<{ id: number; name: string; similarity: number }[]>([]);
  let loadingSimilar = $state(false);

  // Expanded keyword neighbors
  let expandedKeywordId = $state<number | null>(null);
  let expandedNeighbors = $state<
    { id: number; name: string; similarity: number; cooccurrence: number }[]
  >([]);
  let loadingNeighbors = $state(false);

  // Semantic scores for suggestions
  let semanticScores: SvelteMap<string, number> = new SvelteMap();
  let loadingSemanticScores = $state(false);

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
        const results = await invoke<{ id: number; name: string; count: number }[]>(
          "search_keywords",
          {
            query: searchInput,
            limit: 10,
          },
        );
        const existingIds = new Set(keywords.map((k) => k.id));
        searchResults = results.filter((r) => !existingIds.has(r.id));
        showDropdown = searchResults.length > 0 || searchInput.length >= 2;
      } catch (e) {
        log.error("Failed to search keywords:", e);
        searchResults = [];
        showDropdown = false;
      }
    }, 300);
  }

  // Load suggested keywords from statistical analysis
  async function loadSuggestions() {
    if (suggestedKeywords.length > 0 || loadingSuggestions) return;
    loadingSuggestions = true;
    try {
      const analysis = await invoke<{ keyword_candidates: { term: string; score: number }[] }>(
        "analyze_article_statistical",
        { fnordId },
      );
      const existingNames = new Set(keywords.map((k) => k.name.toLowerCase()));
      suggestedKeywords = analysis.keyword_candidates
        .filter((k) => !existingNames.has(k.term.toLowerCase()))
        .slice(0, 5);
    } catch (e) {
      log.error("Failed to load suggestions:", e);
    } finally {
      loadingSuggestions = false;
    }
  }

  // Load similar keywords from Immanentize network
  async function loadSimilarFromNetwork() {
    if (similarKeywords.length > 0 || loadingSimilar || keywords.length === 0) return;
    loadingSimilar = true;
    try {
      const similar = await invoke<{ id: number; name: string; similarity: number }[]>(
        "get_keyword_suggestions_from_network",
        { fnordId, limit: 5 },
      );
      const existingIds = new Set(keywords.map((k) => k.id));
      similarKeywords = similar.filter((k) => !existingIds.has(k.id));
    } catch (e) {
      log.error("Failed to load similar keywords:", e);
    } finally {
      loadingSimilar = false;
    }
  }

  // Toggle expanded state for a keyword to show its neighbors
  async function toggleKeywordExpand(keywordId: number) {
    if (expandedKeywordId === keywordId) {
      expandedKeywordId = null;
      expandedNeighbors = [];
      return;
    }

    expandedKeywordId = keywordId;
    loadingNeighbors = true;
    try {
      const neighbors = await invoke<
        { id: number; name: string; similarity: number; cooccurrence: number }[]
      >("get_similar_keywords", { keywordId, limit: 5 });
      const existingIds = new Set(keywords.map((k) => k.id));
      expandedNeighbors = neighbors.filter((n) => !existingIds.has(n.id));
    } catch (e) {
      log.error("Failed to load keyword neighbors:", e);
      expandedNeighbors = [];
    } finally {
      loadingNeighbors = false;
    }
  }

  // Load semantic scores for suggestions
  async function loadSemanticScores() {
    if (suggestedKeywords.length === 0 || loadingSemanticScores) return;
    loadingSemanticScores = true;
    try {
      const keywordTerms = suggestedKeywords.map((s) => s.term);
      const scores = await invoke<
        { keyword: string; semantic_score: number; combined_score: number }[]
      >("score_keywords_semantically", { fnordId, keywords: keywordTerms, semanticWeight: 0.4 });
      const newScores = new SvelteMap<string, number>();
      for (const score of scores) {
        newScores.set(score.keyword.toLowerCase(), score.semantic_score);
      }
      semanticScores = newScores;

      suggestedKeywords = [...suggestedKeywords].sort((a, b) => {
        const aSemanticScore = semanticScores.get(a.term.toLowerCase()) ?? 0;
        const bSemanticScore = semanticScores.get(b.term.toLowerCase()) ?? 0;
        const aCombined = a.score * 0.6 + aSemanticScore * 0.4;
        const bCombined = b.score * 0.6 + bSemanticScore * 0.4;
        return bCombined - aCombined;
      });
    } catch (e) {
      log.error("Failed to load semantic scores:", e);
    } finally {
      loadingSemanticScores = false;
    }
  }

  // Add a keyword to the article
  async function addKeyword(keywordName: string) {
    loading = true;
    try {
      const newKeyword = await invoke<ArticleKeyword>("add_article_keyword", {
        fnordId,
        keyword: keywordName,
      });

      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: "keyword_added",
        new_value: keywordName,
      };
      await invoke("record_correction", { correction });

      onUpdate([...keywords, newKeyword]);

      suggestedKeywords = suggestedKeywords.filter(
        (s) => s.term.toLowerCase() !== keywordName.toLowerCase(),
      );

      searchInput = "";
      searchResults = [];
      showDropdown = false;
    } catch (e) {
      log.error("Failed to add keyword:", e);
    } finally {
      loading = false;
    }
  }

  // Remove a keyword from the article
  async function removeKeyword(keyword: ArticleKeyword) {
    loading = true;
    try {
      await invoke("remove_article_keyword", {
        fnordId,
        keywordId: keyword.id,
      });

      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: "keyword_removed",
        old_value: keyword.name,
      };
      await invoke("record_correction", { correction });

      onUpdate(keywords.filter((k) => k.id !== keyword.id));
    } catch (e) {
      log.error("Failed to remove keyword:", e);
    } finally {
      loading = false;
    }
  }

  // Add an existing keyword by ID (for similar keywords from network)
  async function addKeywordById(keywordId: number, keywordName: string) {
    loading = true;
    try {
      const newKeyword = await invoke<ArticleKeyword>("add_article_keyword", {
        fnordId,
        keyword: keywordName,
      });

      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: "keyword_added",
        new_value: keywordName,
      };
      await invoke("record_correction", { correction });

      onUpdate([...keywords, newKeyword]);

      similarKeywords = similarKeywords.filter((s) => s.id !== keywordId);
    } catch (e) {
      log.error("Failed to add keyword by ID:", e);
    } finally {
      loading = false;
    }
  }

  // Handle click outside to close dropdown
  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest(".keyword-search-container")) {
      showDropdown = false;
    }
  }

  // Navigate to keyword in network view
  function navigateToKeyword(keywordId: number) {
    navigationStore.navigateToNetwork(keywordId);
  }

  // Load suggestions when editing mode is enabled
  $effect(() => {
    if (editing) {
      loadSuggestions();
      loadSimilarFromNetwork();
    } else {
      expandedKeywordId = null;
      expandedNeighbors = [];
    }
  });

  // Load semantic scores after suggestions are loaded
  $effect(() => {
    if (suggestedKeywords.length > 0 && semanticScores.size === 0) {
      loadSemanticScores();
    }
  });

  // Refresh suggestions when keyword network changes
  function handleKeywordsChanged() {
    if (!editing) return;
    suggestedKeywords = [];
    similarKeywords = [];
    semanticScores = new SvelteMap();
    loadSuggestions();
    loadSimilarFromNetwork();
  }

  onMount(() => {
    window.addEventListener("keywords-changed", handleKeywordsChanged);
  });

  onDestroy(() => {
    window.removeEventListener("keywords-changed", handleKeywordsChanged);
  });
</script>

<svelte:window onclick={handleClickOutside} />

<div class="article-keywords">
  <!-- Keywords List -->
  <div class="keywords-list">
    {#each keywords as keyword (keyword.id)}
      <KeywordChip
        {keyword}
        {editing}
        {loading}
        {expandedKeywordId}
        {expandedNeighbors}
        {loadingNeighbors}
        onNavigate={navigateToKeyword}
        onRemove={removeKeyword}
        onToggleExpand={toggleKeywordExpand}
        onAddNeighbor={addKeywordById}
      />
    {/each}

    {#if keywords.length === 0 && !editing}
      <span class="no-keywords">{$_("articleKeywords.none") || "Keine Keywords"}</span>
    {/if}
  </div>

  <!-- Suggestions + Similar (Edit Mode) -->
  {#if editing}
    <KeywordSuggestions
      {suggestedKeywords}
      {similarKeywords}
      {semanticScores}
      {loadingSuggestions}
      {loadingSimilar}
      {loadingSemanticScores}
      {loading}
      onAddSuggestion={addKeyword}
      onAddSimilar={addKeywordById}
    />
  {/if}

  <!-- Search Input (Edit Mode) -->
  {#if editing}
    <KeywordSearchInput
      bind:searchInput
      {searchResults}
      {showDropdown}
      {loading}
      {loadingSuggestions}
      onSearch={handleSearch}
      onAdd={addKeyword}
    />
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
    align-items: flex-start;
  }

  .no-keywords {
    font-size: 0.8125rem;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
