<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import type { ArticleKeyword, KeywordType, ExtractionMethod, CorrectionInput } from '$lib/types';

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

  // Suggested keywords from statistical analysis
  let suggestedKeywords = $state<{ term: string; score: number }[]>([]);
  let loadingSuggestions = $state(false);

  // Similar keywords from network
  let similarKeywords = $state<{ id: number; name: string; similarity: number }[]>([]);
  let loadingSimilar = $state(false);

  // Expanded keyword neighbors (for get_similar_keywords)
  let expandedKeywordId = $state<number | null>(null);
  let expandedNeighbors = $state<{ id: number; name: string; similarity: number; cooccurrence: number }[]>([]);
  let loadingNeighbors = $state(false);

  // Semantic scores for suggestions
  let semanticScores = $state<Map<string, number>>(new Map());
  let loadingSemanticScores = $state(false);

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
        return $_('keywordTooltips.sourceAi') || $_('articleView.sourceAi') || 'KI-generiert';
      case 'statistical':
        return $_('keywordTooltips.sourceStatistical') || $_('articleView.sourceStatistical') || 'Statistisch';
      case 'manual':
        return $_('keywordTooltips.sourceManual') || $_('articleView.sourceManual') || 'Manuell';
      default:
        return source;
    }
  }

  // Get keyword type icon (Font Awesome)
  function getTypeIcon(type: KeywordType | undefined): string {
    switch (type) {
      case 'person':
        return 'fa-solid fa-user-tie';
      case 'organization':
        return 'fa-solid fa-building';
      case 'location':
        return 'fa-solid fa-location-dot';
      case 'acronym':
        return 'fa-solid fa-font';
      case 'concept':
      default:
        return 'fa-solid fa-lightbulb';
    }
  }

  // Get keyword type label
  function getTypeLabel(type: KeywordType | undefined): string {
    switch (type) {
      case 'person':
        return $_('keywordTooltips.typePerson') || $_('articleKeywords.typePerson') || 'Person';
      case 'organization':
        return $_('keywordTooltips.typeOrganization') || $_('articleKeywords.typeOrganization') || 'Organisation';
      case 'location':
        return $_('keywordTooltips.typeLocation') || $_('articleKeywords.typeLocation') || 'Ort';
      case 'acronym':
        return $_('keywordTooltips.typeAcronym') || $_('articleKeywords.typeAcronym') || 'Akronym';
      case 'concept':
      default:
        return $_('keywordTooltips.typeConcept') || $_('articleKeywords.typeConcept') || 'Konzept';
    }
  }

  // Get extraction method labels
  function getMethodLabels(methods: ExtractionMethod[] | undefined): string {
    if (!methods || methods.length === 0) return '';
    const labels: Record<string, string> = {
      'yake': 'YAKE',
      'rake': 'RAKE',
      'ngram': 'N-Gram',
      'textrank': 'TextRank',
      'entity': 'NER',
      'enhanced_ner': 'NER+',
      'tfidf': 'TF-IDF',
      'ai': 'LLM',
      'manual': 'Manuell'
    };
    return methods.map(m => labels[m] || m).join(', ');
  }

  // Check if keyword has multiple extraction methods (confirmed by multiple sources)
  function isMultiConfirmed(keyword: ArticleKeyword): boolean {
    return (keyword.extraction_methods?.length ?? 0) > 1;
  }

  // Get quality score color
  function getQualityColor(score: number | undefined): string {
    if (score === undefined || score === null) return 'var(--text-muted)';
    if (score >= 0.7) return 'var(--status-success)';
    if (score >= 0.4) return 'var(--status-warning)';
    return 'var(--status-error)';
  }

  // Get confidence color (same thresholds as quality)
  function getConfidenceColor(confidence: number): string {
    if (confidence >= 0.7) return 'var(--status-success)';
    if (confidence >= 0.4) return 'var(--status-warning)';
    return 'var(--status-error)';
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
        showDropdown = searchResults.length > 0 || searchInput.length >= 2;
      } catch (e) {
        console.error('Failed to search keywords:', e);
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
        'analyze_article_statistical',
        { fnordId }
      );
      // Filter out already assigned keywords
      const existingNames = new Set(keywords.map(k => k.name.toLowerCase()));
      suggestedKeywords = analysis.keyword_candidates
        .filter(k => !existingNames.has(k.term.toLowerCase()))
        .slice(0, 5);
    } catch (e) {
      console.error('Failed to load suggestions:', e);
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
        'get_keyword_suggestions_from_network',
        { fnordId, limit: 5 }
      );
      // Filter out already assigned keywords
      const existingIds = new Set(keywords.map(k => k.id));
      similarKeywords = similar.filter(k => !existingIds.has(k.id));
    } catch (e) {
      console.error('Failed to load similar keywords:', e);
    } finally {
      loadingSimilar = false;
    }
  }

  // Toggle expanded state for a keyword to show its neighbors
  async function toggleKeywordExpand(keywordId: number) {
    if (expandedKeywordId === keywordId) {
      // Collapse
      expandedKeywordId = null;
      expandedNeighbors = [];
      return;
    }

    // Expand and load neighbors
    expandedKeywordId = keywordId;
    loadingNeighbors = true;
    try {
      const neighbors = await invoke<{ id: number; name: string; similarity: number; cooccurrence: number }[]>(
        'get_similar_keywords',
        { keywordId, limit: 5 }
      );
      // Filter out already assigned keywords
      const existingIds = new Set(keywords.map(k => k.id));
      expandedNeighbors = neighbors.filter(n => !existingIds.has(n.id));
    } catch (e) {
      console.error('Failed to load keyword neighbors:', e);
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
      const keywordTerms = suggestedKeywords.map(s => s.term);
      const scores = await invoke<{ keyword: string; semantic_score: number; combined_score: number }[]>(
        'score_keywords_semantically',
        { fnordId, keywords: keywordTerms, semanticWeight: 0.4 }
      );
      // Update the map
      const newScores = new Map<string, number>();
      for (const score of scores) {
        newScores.set(score.keyword.toLowerCase(), score.semantic_score);
      }
      semanticScores = newScores;

      // Re-sort suggestions by combined relevance (base score + semantic)
      suggestedKeywords = [...suggestedKeywords].sort((a, b) => {
        const aSemanticScore = semanticScores.get(a.term.toLowerCase()) ?? 0;
        const bSemanticScore = semanticScores.get(b.term.toLowerCase()) ?? 0;
        const aCombined = a.score * 0.6 + aSemanticScore * 0.4;
        const bCombined = b.score * 0.6 + bSemanticScore * 0.4;
        return bCombined - aCombined;
      });
    } catch (e) {
      console.error('Failed to load semantic scores:', e);
    } finally {
      loadingSemanticScores = false;
    }
  }

  // Add a keyword to the article
  async function addKeyword(keywordName: string) {
    loading = true;
    try {
      // The Rust command expects the keyword as a string and returns the created/found keyword
      const newKeyword = await invoke<ArticleKeyword>('add_article_keyword', {
        fnordId,
        keyword: keywordName,
      });

      // Record correction for bias learning
      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: 'keyword_added',
        new_value: keywordName,
      };
      await invoke('record_correction', { correction });

      // Update local state
      onUpdate([...keywords, newKeyword]);

      // Remove from suggestions if present
      suggestedKeywords = suggestedKeywords.filter(s => s.term.toLowerCase() !== keywordName.toLowerCase());

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

  // Load suggestions when editing mode is enabled
  $effect(() => {
    if (editing) {
      loadSuggestions();
      loadSimilarFromNetwork();
    } else {
      // Reset expanded state when leaving edit mode
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

  // Refresh suggestions when keyword network changes (e.g. batch processing, manual edits)
  function handleKeywordsChanged() {
    if (!editing) return;
    // Clear cached data so load functions will re-fetch
    suggestedKeywords = [];
    similarKeywords = [];
    semanticScores = new Map();
    loadSuggestions();
    loadSimilarFromNetwork();
  }

  onMount(() => {
    window.addEventListener('keywords-changed', handleKeywordsChanged);
  });

  onDestroy(() => {
    window.removeEventListener('keywords-changed', handleKeywordsChanged);
  });

  // Add an existing keyword by ID (for similar keywords from network)
  async function addKeywordById(keywordId: number, keywordName: string) {
    loading = true;
    try {
      const newKeyword = await invoke<ArticleKeyword>('add_article_keyword', {
        fnordId,
        keyword: keywordName,
      });

      // Record correction for bias learning
      const correction: CorrectionInput = {
        fnord_id: fnordId,
        correction_type: 'keyword_added',
        new_value: keywordName,
      };
      await invoke('record_correction', { correction });

      // Update local state
      onUpdate([...keywords, newKeyword]);

      // Remove from similar keywords
      similarKeywords = similarKeywords.filter(s => s.id !== keywordId);
    } catch (e) {
      console.error('Failed to add keyword by ID:', e);
    } finally {
      loading = false;
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="article-keywords">
  <!-- Keywords List -->
  <div class="keywords-list">
    {#each keywords as keyword (keyword.id)}
      {@const methodLabels = getMethodLabels(keyword.extraction_methods)}
      <div class="keyword-wrapper" class:expanded={expandedKeywordId === keyword.id}>
        <div
          class="keyword-chip"
          class:editable={editing}
          class:multi-confirmed={isMultiConfirmed(keyword)}
          class:is-expanded={expandedKeywordId === keyword.id}
        >
          <!-- Keyword Type Icon -->
          <i class="type-icon {getTypeIcon(keyword.keyword_type)}"></i>

          <!-- Keyword Name -->
          <div class="keyword-name-wrapper">
            <button
              type="button"
              class="keyword-name"
              onclick={() => navigateToKeyword(keyword.id)}
              disabled={editing}
            >
              {keyword.name}
            </button>
            <div class="keyword-tooltip" role="tooltip">
              <!-- Basis-Info: Typ & Quelle -->
              <div class="tooltip-row">
                <i class="type-icon {getTypeIcon(keyword.keyword_type)}"></i>
                <span>{getTypeLabel(keyword.keyword_type)}</span>
              </div>
              <div class="tooltip-row">
                <i class="source-icon {getSourceIcon(keyword.source)}"></i>
                <span>{getSourceLabel(keyword.source)}</span>
              </div>

              <!-- Extraktions-Info -->
              {#if methodLabels || isMultiConfirmed(keyword)}
                <hr class="tooltip-divider" />
                {#if methodLabels}
                  <div class="tooltip-row">
                    <i class="fa-solid fa-layer-group"></i>
                    <span>{$_('keywordTooltips.extractionMethodsLabel') || 'Extraktionsmethoden'}: {methodLabels}</span>
                    <span class="tooltip-note">{$_('keywordTooltips.extractionMethodsNote') || 'Mehrere Methoden erhöhen das Vertrauen.'}</span>
                  </div>
                {/if}
                {#if isMultiConfirmed(keyword)}
                  <div class="tooltip-row">
                    <i class="fa-solid fa-check-double"></i>
                    <span>{$_('keywordTooltips.multiConfirmed') || $_('articleKeywords.multiConfirmed') || 'Mehrfach bestätigt'}</span>
                  </div>
                {/if}
              {/if}

              <!-- Scores: Konfidenz & Qualität -->
              {#if keyword.confidence < 1.0 || (keyword.quality_score !== undefined && keyword.quality_score !== null)}
                <hr class="tooltip-divider" />
                {#if keyword.confidence < 1.0}
                  <div class="tooltip-item">
                    <div class="tooltip-row">
                      <i class="fa-solid fa-percent"></i>
                      <span>
                        {$_('keywordTooltips.confidenceLabel') || $_('articleKeywords.confidence') || 'Konfidenz'}:
                        {Math.round(keyword.confidence * 100)}%
                      </span>
                      <span class="tooltip-note">{$_('keywordTooltips.confidenceNote') || 'Schätzung der Relevanz für den Artikel.'}</span>
                    </div>
                    <div class="tooltip-confidence-bar">
                      <div
                        class="tooltip-confidence-fill"
                        style="width: {keyword.confidence * 100}%; background-color: {getConfidenceColor(keyword.confidence)}"
                      ></div>
                    </div>
                  </div>
                {/if}
                {#if keyword.quality_score !== undefined && keyword.quality_score !== null}
                  <div class="tooltip-item">
                    <div class="tooltip-row">
                      <i class="fa-solid fa-chart-line"></i>
                      <span>
                        {$_('keywordTooltips.qualityLabel') || 'Qualität'}: {Math.round(keyword.quality_score * 100)}%
                      </span>
                      <span class="tooltip-note">{$_('keywordTooltips.qualityNote') || 'Bewertung aus Nutzung und Vernetzung.'}</span>
                    </div>
                    <div class="tooltip-quality-bar">
                      <div
                        class="tooltip-quality-fill"
                        style="width: {keyword.quality_score * 100}%; background-color: {getQualityColor(keyword.quality_score)}"
                      ></div>
                    </div>
                  </div>
                {/if}
              {/if}

              <!-- Aktion -->
              <hr class="tooltip-divider" />
              <div class="tooltip-row tooltip-action">
                <i class="fa-solid fa-diagram-project"></i>
                <span>{$_('keywordTooltips.openNetwork') || $_('network.title') || 'Im Netzwerk anzeigen'}</span>
              </div>
            </div>
          </div>

          <!-- Source Icon -->
          <i class="source-icon {getSourceIcon(keyword.source)}"></i>

          <!-- Multi-Source Badge -->
          {#if isMultiConfirmed(keyword)}
            <span class="multi-badge">
              <i class="fa-solid fa-check-double"></i>
            </span>
          {/if}

          <!-- Expand Button (Edit Mode) - Show similar keywords -->
          {#if editing}
            <button
              type="button"
              class="expand-btn"
              class:active={expandedKeywordId === keyword.id}
              onclick={() => toggleKeywordExpand(keyword.id)}
              disabled={loadingNeighbors && expandedKeywordId === keyword.id}
              title={$_('keywordTooltips.showNeighbors') || $_('articleKeywords.showNeighbors') || 'Ähnliche Keywords anzeigen'}
              aria-label={$_('articleKeywords.showNeighbors') || 'Ähnliche Keywords anzeigen'}
            >
              {#if loadingNeighbors && expandedKeywordId === keyword.id}
                <i class="fa-solid fa-spinner fa-spin"></i>
              {:else}
                <i class="fa-solid fa-{expandedKeywordId === keyword.id ? 'chevron-up' : 'chevron-down'}"></i>
              {/if}
            </button>
          {/if}

          <!-- Remove Button (Edit Mode) -->
          {#if editing}
            <button
              type="button"
              class="remove-btn"
              onclick={() => removeKeyword(keyword)}
              disabled={loading}
              title={$_('keywordTooltips.remove') || $_('articleKeywords.remove') || 'Entfernen'}
              aria-label={$_('articleKeywords.remove') || 'Entfernen'}
            >
              <i class="fa-solid fa-xmark"></i>
            </button>
          {/if}
        </div>

        <!-- Expanded Neighbors Panel -->
        {#if expandedKeywordId === keyword.id && editing}
          <div class="neighbors-panel">
            {#if loadingNeighbors}
              <span class="neighbors-loading">
                <i class="fa-solid fa-spinner fa-spin"></i>
              </span>
            {:else if expandedNeighbors.length > 0}
              <div class="neighbors-list">
                {#each expandedNeighbors as neighbor}
                  <button
                    type="button"
                    class="neighbor-chip"
                    onclick={() => addKeywordById(neighbor.id, neighbor.name)}
                    disabled={loading}
                    title={`${$_('keywordTooltips.similarityLabel') || $_('articleKeywords.similarity') || 'Ähnlichkeit'}: ${Math.round(neighbor.similarity * 100)}% (${$_('keywordTooltips.similarityNote') || 'Semantische Nähe (Embedding).'}) | ${$_('keywordTooltips.cooccurrenceLabel') || $_('articleKeywords.cooccurrence') || 'Kookkurrenz'}: ${neighbor.cooccurrence} (${$_('keywordTooltips.cooccurrenceNote') || 'Gemeinsame Artikelanzahl.'})`}
                  >
                    <i class="fa-solid fa-plus"></i>
                    {neighbor.name}
                    {#if neighbor.similarity > 0}
                      <span class="neighbor-score">{Math.round(neighbor.similarity * 100)}%</span>
                    {/if}
                  </button>
                {/each}
              </div>
            {:else}
              <span class="no-neighbors">{$_('articleKeywords.noNeighbors') || 'Keine ähnlichen Keywords'}</span>
            {/if}
          </div>
        {/if}
      </div>
    {/each}

    {#if keywords.length === 0 && !editing}
      <span class="no-keywords">{$_('articleKeywords.none') || 'Keine Keywords'}</span>
    {/if}
  </div>

  <!-- Suggested Keywords (Edit Mode) -->
  {#if editing && suggestedKeywords.length > 0}
    <div class="suggestions-section">
      <span class="suggestions-label">
        <i class="fa-solid fa-lightbulb"></i>
        {$_('articleKeywords.suggestions') || 'Vorschläge'}:
        {#if loadingSemanticScores}
          <i class="fa-solid fa-spinner fa-spin semantic-loading"></i>
        {/if}
      </span>
      <div class="suggestions-list">
        {#each suggestedKeywords as suggestion}
          {@const semanticScore = semanticScores.get(suggestion.term.toLowerCase())}
          <button
            type="button"
            class="suggestion-chip"
            class:has-semantic={semanticScore !== undefined && semanticScore > 0}
            onclick={() => addKeyword(suggestion.term)}
            disabled={loading}
            title={semanticScore !== undefined
              ? `${$_('keywordTooltips.tfidfLabel') || 'TF-IDF'}: ${Math.round(suggestion.score * 100)}% (${$_('keywordTooltips.tfidfNote') || 'Statistischer Relevanz-Score.'}) | ${$_('keywordTooltips.semanticLabel') || $_('articleKeywords.semanticScore') || 'Semantik'}: ${Math.round(semanticScore * 100)}% (${$_('keywordTooltips.semanticNote') || 'Embedding-Score zum Artikel.'})`
              : `${$_('keywordTooltips.tfidfLabel') || 'TF-IDF'}: ${Math.round(suggestion.score * 100)}% (${$_('keywordTooltips.tfidfNote') || 'Statistischer Relevanz-Score.'})`}
          >
            <i class="fa-solid fa-plus"></i>
            {suggestion.term}
            {#if semanticScore !== undefined && semanticScore > 0}
              <span
                class="semantic-badge"
                title={`${$_('keywordTooltips.semanticLabel') || $_('articleKeywords.semanticScore') || 'Semantische Ähnlichkeit'}: ${Math.round(semanticScore * 100)}% | ${$_('keywordTooltips.semanticNote') || 'Embedding-Score zum Artikel.'}`}
              >
                <i class="fa-solid fa-brain"></i>
                {Math.round(semanticScore * 100)}%
              </span>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Similar Keywords from Network (Edit Mode) -->
  {#if editing && similarKeywords.length > 0}
    <div class="similar-section">
      <span class="similar-label">
        <i class="fa-solid fa-diagram-project"></i>
        {$_('articleKeywords.fromNetwork') || 'Aus Netzwerk'}:
      </span>
      <div class="similar-list">
        {#each similarKeywords as similar}
          <button
            type="button"
            class="similar-chip"
            onclick={() => addKeywordById(similar.id, similar.name)}
            disabled={loading}
            title={`${$_('keywordTooltips.similarityLabel') || $_('articleKeywords.similarity') || 'Ähnlichkeit'}: ${Math.round(similar.similarity * 100)}% (${$_('keywordTooltips.similarityNote') || 'Semantische Nähe (Embedding).'})`}
          >
            <i class="fa-solid fa-plus"></i>
            {similar.name}
            <span class="similarity-badge">{Math.round(similar.similarity * 100)}%</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if editing && loadingSimilar}
    <div class="loading-similar">
      <i class="fa-solid fa-spinner fa-spin"></i>
      {$_('articleKeywords.loadingNetwork') || 'Lade Netzwerk-Vorschläge...'}
    </div>
  {/if}

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
          placeholder={$_('articleKeywords.searchPlaceholder') || 'Keyword suchen oder hinzufügen...'}
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
                onclick={() => addKeyword(result.name)}
              >
                <span class="result-name">{result.name}</span>
                <span class="result-count">{result.count} {$_('articleKeywords.articles') || 'Artikel'}</span>
              </button>
            {/each}
          {/if}
          <!-- Option to create new keyword -->
          {#if searchInput.trim().length >= 2 && !searchResults.some(r => r.name.toLowerCase() === searchInput.toLowerCase())}
            <button
              type="button"
              class="search-result-item create-new"
              onclick={() => addKeyword(searchInput.trim())}
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

  .keyword-wrapper {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .keyword-wrapper.expanded {
    flex-basis: 100%;
  }

  .keyword-chip {
    position: relative;
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

  /* Multi-confirmed keywords get a special highlight */
  .keyword-chip.multi-confirmed {
    border-color: var(--accent-success);
    background-color: color-mix(in srgb, var(--accent-success) 10%, var(--bg-overlay));
  }

  /* Type Icon */
  .type-icon {
    font-size: 0.625rem;
    color: var(--text-secondary);
    opacity: 0.7;
  }

  .type-icon:global(.fa-user-tie) {
    color: var(--accent-info);
  }

  .type-icon:global(.fa-building) {
    color: var(--accent-warning);
  }

  .type-icon:global(.fa-location-dot) {
    color: var(--status-success);
  }

  .type-icon:global(.fa-font) {
    color: var(--accent-primary);
  }

  /* Source Icon */
  .source-icon {
    font-size: 0.5rem;
    color: var(--text-muted);
  }

  .source-icon:global(.fa-robot) {
    color: var(--accent-primary);
  }

  .source-icon:global(.fa-chart-line) {
    color: var(--accent-info);
  }

  .source-icon:global(.fa-user) {
    color: var(--accent-success);
  }

  /* Multi-Source Badge */
  .multi-badge {
    font-size: 0.5rem;
    color: var(--accent-success);
  }

  .keyword-name-wrapper {
    position: relative;
    display: flex;
    align-items: center;
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

  .keyword-tooltip {
    position: absolute;
    top: calc(100% + 0.5rem);
    left: 0;
    min-width: 14rem;
    max-width: 22rem;
    padding: 0.5rem 0.625rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    color: var(--text-primary);
    font-size: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.25);
    opacity: 0;
    visibility: hidden;
    transform: translateY(-4px);
    transition: opacity 0.15s ease, transform 0.15s ease, visibility 0.15s ease;
    z-index: 20;
    pointer-events: none;
  }

  .keyword-name-wrapper:hover .keyword-tooltip,
  .keyword-name-wrapper:focus-within .keyword-tooltip {
    opacity: 1;
    visibility: visible;
    transform: translateY(0);
  }

  .tooltip-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .tooltip-divider {
    border: none;
    border-top: 1px solid var(--border-muted);
    margin: 0.25rem 0;
  }

  .tooltip-action {
    color: var(--accent-primary);
  }

  .tooltip-action i {
    color: var(--accent-primary);
  }

  .tooltip-row {
    display: flex;
    align-items: flex-start;
    gap: 0.375rem;
  }

  .tooltip-row i {
    margin-top: 0.1rem;
    font-size: 0.625rem;
    color: var(--text-muted);
  }

  .tooltip-note {
    margin-left: 0.25rem;
    color: var(--text-muted);
  }

  .tooltip-quality-bar {
    width: 100%;
    height: 0.25rem;
    background-color: var(--bg-overlay);
    border-radius: 0.125rem;
    overflow: hidden;
  }

  .tooltip-quality-fill {
    height: 100%;
    border-radius: 0.125rem;
  }

  .tooltip-confidence-bar {
    width: 100%;
    height: 0.25rem;
    background-color: var(--bg-overlay);
    border-radius: 0.125rem;
    overflow: hidden;
  }

  .tooltip-confidence-fill {
    height: 100%;
    border-radius: 0.125rem;
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.125rem;
    height: 1.125rem;
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
    font-size: 0.5625rem;
  }

  /* Expand Button */
  .expand-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.125rem;
    height: 1.125rem;
    background: none;
    border: none;
    border-radius: 0.25rem;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .expand-btn:hover:not(:disabled) {
    color: var(--accent-primary);
    background-color: rgba(var(--accent-primary-rgb), 0.1);
  }

  .expand-btn.active {
    color: var(--accent-primary);
  }

  .expand-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .expand-btn i {
    font-size: 0.5rem;
  }

  .keyword-chip.is-expanded {
    border-color: var(--accent-primary);
    border-bottom-left-radius: 0;
    border-bottom-right-radius: 0;
  }

  /* Neighbors Panel */
  .neighbors-panel {
    padding: 0.375rem 0.5rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--accent-primary);
    border-top: none;
    border-radius: 0 0 0.375rem 0.375rem;
  }

  .neighbors-loading {
    font-size: 0.75rem;
    color: var(--accent-primary);
  }

  .neighbors-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .neighbor-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.375rem;
    background-color: var(--bg-overlay);
    border: 1px dashed var(--accent-secondary);
    border-radius: 0.75rem;
    font-size: 0.6875rem;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .neighbor-chip:hover:not(:disabled) {
    background-color: var(--accent-secondary);
    color: var(--text-primary);
    border-style: solid;
  }

  .neighbor-chip:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .neighbor-chip i {
    font-size: 0.5rem;
  }

  .neighbor-score {
    font-size: 0.5625rem;
    padding: 0.0625rem 0.1875rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    color: var(--text-muted);
  }

  .no-neighbors {
    font-size: 0.6875rem;
    color: var(--text-muted);
    font-style: italic;
  }

  .no-keywords {
    font-size: 0.8125rem;
    color: var(--text-muted);
    font-style: italic;
  }

  /* Suggestions Section */
  .suggestions-section {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border: 1px dashed var(--border-default);
  }

  .suggestions-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .suggestions-label i {
    color: var(--accent-warning);
  }

  .suggestions-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .suggestion-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.1875rem 0.5rem;
    background-color: var(--bg-overlay);
    border: 1px dashed var(--accent-info);
    border-radius: 1rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .suggestion-chip:hover:not(:disabled) {
    background-color: var(--accent-info);
    color: var(--text-primary);
    border-style: solid;
  }

  .suggestion-chip:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .suggestion-chip i {
    font-size: 0.625rem;
  }

  .suggestion-chip.has-semantic {
    border-color: var(--accent-success);
  }

  .semantic-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.125rem;
    font-size: 0.5625rem;
    padding: 0.0625rem 0.25rem;
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
    color: var(--accent-success);
  }

  .semantic-badge i {
    font-size: 0.5rem;
  }

  .semantic-loading {
    font-size: 0.625rem;
    margin-left: 0.25rem;
    color: var(--accent-info);
  }

  /* Similar Keywords from Network Section */
  .similar-section {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border: 1px dashed var(--accent-primary);
  }

  .similar-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .similar-label i {
    color: var(--accent-primary);
  }

  .similar-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .similar-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.1875rem 0.5rem;
    background-color: var(--bg-overlay);
    border: 1px dashed var(--accent-primary);
    border-radius: 1rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .similar-chip:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    border-style: solid;
  }

  .similar-chip:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .similar-chip i {
    font-size: 0.625rem;
  }

  .similarity-badge {
    font-size: 0.625rem;
    padding: 0.0625rem 0.25rem;
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
    color: var(--text-muted);
  }

  .loading-similar {
    font-size: 0.75rem;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .loading-similar i {
    color: var(--accent-primary);
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
