<script lang="ts">
  import { SvelteMap } from "svelte/reactivity";
  import { _ } from "svelte-i18n";

  interface SuggestedKeyword {
    term: string;
    score: number;
  }

  interface SimilarKeyword {
    id: number;
    name: string;
    similarity: number;
  }

  interface Props {
    suggestedKeywords: SuggestedKeyword[];
    similarKeywords: SimilarKeyword[];
    semanticScores: SvelteMap<string, number>;
    loadingSuggestions: boolean;
    loadingSimilar: boolean;
    loadingSemanticScores: boolean;
    loading: boolean;
    onAddSuggestion: (term: string) => void;
    onAddSimilar: (id: number, name: string) => void;
  }

  let {
    suggestedKeywords,
    similarKeywords,
    semanticScores,
    loadingSuggestions,
    loadingSimilar,
    loadingSemanticScores,
    loading,
    onAddSuggestion,
    onAddSimilar,
  }: Props = $props();
</script>

<!-- Suggested Keywords -->
{#if suggestedKeywords.length > 0}
  <div class="suggestions-section">
    <span class="suggestions-label">
      <i class="fa-solid fa-lightbulb"></i>
      {$_("articleKeywords.suggestions") || "Vorschläge"}:
      {#if loadingSemanticScores}
        <i class="fa-solid fa-spinner fa-spin semantic-loading"></i>
      {/if}
    </span>
    <div class="suggestions-list">
      {#each suggestedKeywords as suggestion (suggestion.term)}
        {@const semanticScore = semanticScores.get(suggestion.term.toLowerCase())}
        <button
          type="button"
          class="suggestion-chip"
          class:has-semantic={semanticScore !== undefined && semanticScore > 0}
          onclick={() => onAddSuggestion(suggestion.term)}
          disabled={loading}
          title={semanticScore !== undefined
            ? `${$_("keywordTooltips.tfidfLabel") || "TF-IDF"}: ${Math.round(suggestion.score * 100)}% (${$_("keywordTooltips.tfidfNote") || "Statistischer Relevanz-Score."}) | ${$_("keywordTooltips.semanticLabel") || $_("articleKeywords.semanticScore") || "Semantik"}: ${Math.round(semanticScore * 100)}% (${$_("keywordTooltips.semanticNote") || "Embedding-Score zum Artikel."})`
            : `${$_("keywordTooltips.tfidfLabel") || "TF-IDF"}: ${Math.round(suggestion.score * 100)}% (${$_("keywordTooltips.tfidfNote") || "Statistischer Relevanz-Score."})`}
        >
          <i class="fa-solid fa-plus"></i>
          {suggestion.term}
          {#if semanticScore !== undefined && semanticScore > 0}
            <span
              class="semantic-badge"
              title={`${$_("keywordTooltips.semanticLabel") || $_("articleKeywords.semanticScore") || "Semantische Ähnlichkeit"}: ${Math.round(semanticScore * 100)}% | ${$_("keywordTooltips.semanticNote") || "Embedding-Score zum Artikel."}`}
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

<!-- Similar Keywords from Network -->
{#if similarKeywords.length > 0}
  <div class="similar-section">
    <span class="similar-label">
      <i class="fa-solid fa-diagram-project"></i>
      {$_("articleKeywords.fromNetwork") || "Aus Netzwerk"}:
    </span>
    <div class="similar-list">
      {#each similarKeywords as similar (similar.id)}
        <button
          type="button"
          class="similar-chip"
          onclick={() => onAddSimilar(similar.id, similar.name)}
          disabled={loading}
          title={`${$_("keywordTooltips.similarityLabel") || $_("articleKeywords.similarity") || "Ähnlichkeit"}: ${Math.round(similar.similarity * 100)}% (${$_("keywordTooltips.similarityNote") || "Semantische Nähe (Embedding)."})`}
        >
          <i class="fa-solid fa-plus"></i>
          {similar.name}
          <span class="similarity-badge">{Math.round(similar.similarity * 100)}%</span>
        </button>
      {/each}
    </div>
  </div>
{/if}

{#if loadingSimilar}
  <div class="loading-similar">
    <i class="fa-solid fa-spinner fa-spin"></i>
    {$_("articleKeywords.loadingNetwork") || "Lade Netzwerk-Vorschläge..."}
  </div>
{/if}

<!-- Loading state for initial suggestions fetch -->
{#if loadingSuggestions && suggestedKeywords.length === 0}
  <div class="loading-similar">
    <i class="fa-solid fa-spinner fa-spin"></i>
    {$_("articleKeywords.loadingSuggestions") || "Lade Vorschläge..."}
  </div>
{/if}

<style>
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
</style>
