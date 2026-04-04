<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { ArticleKeyword, KeywordType, ExtractionMethod } from "$lib/types";

  interface NeighborEntry {
    id: number;
    name: string;
    similarity: number;
    cooccurrence: number;
  }

  interface Props {
    keyword: ArticleKeyword;
    editing: boolean;
    loading: boolean;
    expandedKeywordId: number | null;
    expandedNeighbors: NeighborEntry[];
    loadingNeighbors: boolean;
    onNavigate: (keywordId: number) => void;
    onRemove: (keyword: ArticleKeyword) => void;
    onToggleExpand: (keywordId: number) => void;
    onAddNeighbor: (neighborId: number, neighborName: string) => void;
  }

  let {
    keyword,
    editing,
    loading,
    expandedKeywordId,
    expandedNeighbors,
    loadingNeighbors,
    onNavigate,
    onRemove,
    onToggleExpand,
    onAddNeighbor,
  }: Props = $props();

  function getSourceIcon(source: ArticleKeyword["source"]): string {
    switch (source) {
      case "ai":
        return "fa-solid fa-robot";
      case "statistical":
        return "fa-solid fa-chart-line";
      case "manual":
        return "fa-solid fa-user";
      default:
        return "fa-solid fa-tag";
    }
  }

  function getSourceLabel(source: ArticleKeyword["source"]): string {
    switch (source) {
      case "ai":
        return $_("keywordTooltips.sourceAi") || $_("articleView.sourceAi") || "KI-generiert";
      case "statistical":
        return (
          $_("keywordTooltips.sourceStatistical") ||
          $_("articleView.sourceStatistical") ||
          "Statistisch"
        );
      case "manual":
        return $_("keywordTooltips.sourceManual") || $_("articleView.sourceManual") || "Manuell";
      default:
        return source;
    }
  }

  function getTypeIcon(type: KeywordType | undefined): string {
    switch (type) {
      case "person":
        return "fa-solid fa-user-tie";
      case "organization":
        return "fa-solid fa-building";
      case "location":
        return "fa-solid fa-location-dot";
      case "acronym":
        return "fa-solid fa-font";
      case "concept":
      default:
        return "fa-solid fa-lightbulb";
    }
  }

  function getTypeLabel(type: KeywordType | undefined): string {
    switch (type) {
      case "person":
        return $_("keywordTooltips.typePerson") || $_("articleKeywords.typePerson") || "Person";
      case "organization":
        return (
          $_("keywordTooltips.typeOrganization") ||
          $_("articleKeywords.typeOrganization") ||
          "Organisation"
        );
      case "location":
        return $_("keywordTooltips.typeLocation") || $_("articleKeywords.typeLocation") || "Ort";
      case "acronym":
        return $_("keywordTooltips.typeAcronym") || $_("articleKeywords.typeAcronym") || "Akronym";
      case "concept":
      default:
        return $_("keywordTooltips.typeConcept") || $_("articleKeywords.typeConcept") || "Konzept";
    }
  }

  function getMethodLabels(methods: ExtractionMethod[] | undefined): string {
    if (!methods || methods.length === 0) return "";
    const labels: Record<string, string> = {
      yake: "YAKE",
      rake: "RAKE",
      ngram: "N-Gram",
      textrank: "TextRank",
      entity: "NER",
      enhanced_ner: "NER+",
      tfidf: "TF-IDF",
      ai: "LLM",
      manual: "Manuell",
    };
    return methods.map((m) => labels[m] || m).join(", ");
  }

  function isMultiConfirmed(kw: ArticleKeyword): boolean {
    return (kw.extraction_methods?.length ?? 0) > 1;
  }

  function getQualityColor(score: number | undefined): string {
    if (score === undefined || score === null) return "var(--text-muted)";
    if (score >= 0.7) return "var(--status-success)";
    if (score >= 0.4) return "var(--status-warning)";
    return "var(--status-error)";
  }

  function getConfidenceColor(confidence: number): string {
    if (confidence >= 0.7) return "var(--status-success)";
    if (confidence >= 0.4) return "var(--status-warning)";
    return "var(--status-error)";
  }

  const isExpanded = $derived(expandedKeywordId === keyword.id);
  const methodLabels = $derived(getMethodLabels(keyword.extraction_methods));
  const multiConfirmed = $derived(isMultiConfirmed(keyword));
</script>

<div class="keyword-wrapper" class:expanded={isExpanded}>
  <div
    class="keyword-chip"
    class:editable={editing}
    class:multi-confirmed={multiConfirmed}
    class:is-expanded={isExpanded}
  >
    <!-- Keyword Type Icon -->
    <i class="type-icon {getTypeIcon(keyword.keyword_type)}"></i>

    <!-- Keyword Name -->
    <div class="keyword-name-wrapper">
      <button
        type="button"
        class="keyword-name"
        onclick={() => onNavigate(keyword.id)}
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
        {#if methodLabels || multiConfirmed}
          <hr class="tooltip-divider" />
          {#if methodLabels}
            <div class="tooltip-row">
              <i class="fa-solid fa-layer-group"></i>
              <span
                >{$_("keywordTooltips.extractionMethodsLabel") || "Extraktionsmethoden"}: {methodLabels}</span
              >
              <span class="tooltip-note"
                >{$_("keywordTooltips.extractionMethodsNote") ||
                  "Mehrere Methoden erhöhen das Vertrauen."}</span
              >
            </div>
          {/if}
          {#if multiConfirmed}
            <div class="tooltip-row">
              <i class="fa-solid fa-check-double"></i>
              <span
                >{$_("keywordTooltips.multiConfirmed") ||
                  $_("articleKeywords.multiConfirmed") ||
                  "Mehrfach bestätigt"}</span
              >
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
                  {$_("keywordTooltips.confidenceLabel") ||
                    $_("articleKeywords.confidence") ||
                    "Konfidenz"}:
                  {Math.round(keyword.confidence * 100)}%
                </span>
                <span class="tooltip-note"
                  >{$_("keywordTooltips.confidenceNote") ||
                    "Schätzung der Relevanz für den Artikel."}</span
                >
              </div>
              <div class="tooltip-confidence-bar">
                <div
                  class="tooltip-confidence-fill"
                  style="width: {keyword.confidence *
                    100}%; background-color: {getConfidenceColor(keyword.confidence)}"
                ></div>
              </div>
            </div>
          {/if}
          {#if keyword.quality_score !== undefined && keyword.quality_score !== null}
            <div class="tooltip-item">
              <div class="tooltip-row">
                <i class="fa-solid fa-chart-line"></i>
                <span>
                  {$_("keywordTooltips.qualityLabel") || "Qualität"}: {Math.round(
                    keyword.quality_score * 100,
                  )}%
                </span>
                <span class="tooltip-note"
                  >{$_("keywordTooltips.qualityNote") ||
                    "Bewertung aus Nutzung und Vernetzung."}</span
                >
              </div>
              <div class="tooltip-quality-bar">
                <div
                  class="tooltip-quality-fill"
                  style="width: {keyword.quality_score *
                    100}%; background-color: {getQualityColor(keyword.quality_score)}"
                ></div>
              </div>
            </div>
          {/if}
        {/if}

        <!-- Aktion -->
        <hr class="tooltip-divider" />
        <div class="tooltip-row tooltip-action">
          <i class="fa-solid fa-diagram-project"></i>
          <span
            >{$_("keywordTooltips.openNetwork") ||
              $_("network.title") ||
              "Im Netzwerk anzeigen"}</span
          >
        </div>
      </div>
    </div>

    <!-- Source Icon -->
    <i class="source-icon {getSourceIcon(keyword.source)}"></i>

    <!-- Multi-Source Badge -->
    {#if multiConfirmed}
      <span class="multi-badge">
        <i class="fa-solid fa-check-double"></i>
      </span>
    {/if}

    <!-- Expand Button (Edit Mode) -->
    {#if editing}
      <button
        type="button"
        class="expand-btn"
        class:active={isExpanded}
        onclick={() => onToggleExpand(keyword.id)}
        disabled={loadingNeighbors && isExpanded}
        title={$_("keywordTooltips.showNeighbors") ||
          $_("articleKeywords.showNeighbors") ||
          "Ähnliche Keywords anzeigen"}
        aria-label={$_("articleKeywords.showNeighbors") || "Ähnliche Keywords anzeigen"}
      >
        {#if loadingNeighbors && isExpanded}
          <i class="fa-solid fa-spinner fa-spin"></i>
        {:else}
          <i class="fa-solid fa-{isExpanded ? 'chevron-up' : 'chevron-down'}"></i>
        {/if}
      </button>
    {/if}

    <!-- Remove Button (Edit Mode) -->
    {#if editing}
      <button
        type="button"
        class="remove-btn"
        onclick={() => onRemove(keyword)}
        disabled={loading}
        title={$_("keywordTooltips.remove") || $_("articleKeywords.remove") || "Entfernen"}
        aria-label={$_("articleKeywords.remove") || "Entfernen"}
      >
        <i class="fa-solid fa-xmark"></i>
      </button>
    {/if}
  </div>

  <!-- Expanded Neighbors Panel -->
  {#if isExpanded && editing}
    <div class="neighbors-panel">
      {#if loadingNeighbors}
        <span class="neighbors-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
        </span>
      {:else if expandedNeighbors.length > 0}
        <div class="neighbors-list">
          {#each expandedNeighbors as neighbor (neighbor.id)}
            <button
              type="button"
              class="neighbor-chip"
              onclick={() => onAddNeighbor(neighbor.id, neighbor.name)}
              disabled={loading}
              title={`${$_("keywordTooltips.similarityLabel") || $_("articleKeywords.similarity") || "Ähnlichkeit"}: ${Math.round(neighbor.similarity * 100)}% (${$_("keywordTooltips.similarityNote") || "Semantische Nähe (Embedding)."}) | ${$_("keywordTooltips.cooccurrenceLabel") || $_("articleKeywords.cooccurrence") || "Kookkurrenz"}: ${neighbor.cooccurrence} (${$_("keywordTooltips.cooccurrenceNote") || "Gemeinsame Artikelanzahl."})`}
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
        <span class="no-neighbors"
          >{$_("articleKeywords.noNeighbors") || "Keine ähnlichen Keywords"}</span
        >
      {/if}
    </div>
  {/if}
</div>

<style>
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

  .keyword-chip.multi-confirmed {
    border-color: var(--accent-success);
    background-color: color-mix(in srgb, var(--accent-success) 10%, var(--bg-overlay));
  }

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
    transition:
      opacity 0.15s ease,
      transform 0.15s ease,
      visibility 0.15s ease;
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
</style>
