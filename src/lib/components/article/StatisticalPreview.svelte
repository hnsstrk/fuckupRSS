<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import type { StatisticalAnalysis } from '$lib/types';

  interface Props {
    fnordId: number;
    hasContent: boolean;
    isProcessed: boolean;
  }

  let { fnordId, hasContent, isProcessed }: Props = $props();

  let analysis = $state<StatisticalAnalysis | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let collapsed = $state(false);

  // Load analysis when component mounts and article has content but is not processed
  $effect(() => {
    if (fnordId && hasContent && !isProcessed) {
      loadAnalysis();
    } else {
      analysis = null;
    }
  });

  async function loadAnalysis() {
    loading = true;
    error = null;
    try {
      analysis = await invoke<StatisticalAnalysis>('analyze_article_statistical', { fnordId });
    } catch (e) {
      console.error('Failed to load statistical analysis:', e);
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  // Get confidence color
  function getConfidenceColor(confidence: number): string {
    if (confidence >= 0.7) return 'var(--accent-success)';
    if (confidence >= 0.4) return 'var(--accent-warning)';
    return 'var(--text-muted)';
  }

  // Format score as percentage
  function formatScore(score: number): string {
    return `${Math.round(score * 100)}%`;
  }
</script>

{#if hasContent && !isProcessed}
  <div class="statistical-preview">
    <button
      type="button"
      class="preview-header"
      onclick={() => collapsed = !collapsed}
    >
      <div class="header-left">
        <i class="fa-solid fa-chart-simple"></i>
        <span class="header-title">
          {$_('articleView.statisticalPreview') || 'Statistische Voranalyse'}
        </span>
        <span class="preview-badge">
          {$_('articleView.preview') || 'Vorschau'}
        </span>
      </div>
      <i class="fa-solid fa-chevron-down chevron" class:collapsed></i>
    </button>

    {#if !collapsed}
      <div class="preview-content">
        {#if loading}
          <div class="loading-state">
            <i class="fa-solid fa-spinner fa-spin"></i>
            <span>{$_('articleView.analyzing') || 'Analysiere...'}</span>
          </div>
        {:else if error}
          <div class="error-state">
            <i class="fa-solid fa-exclamation-triangle"></i>
            <span>{error}</span>
          </div>
        {:else if analysis}
          <!-- Keyword Candidates -->
          {#if analysis.keyword_candidates.length > 0}
            <div class="preview-section">
              <h4 class="section-label">
                <i class="fa-solid fa-tags"></i>
                {$_('articleView.keywordCandidates') || 'Keyword-Kandidaten'}
                <span class="count">({analysis.keyword_candidates.length})</span>
              </h4>
              <div class="keyword-chips">
                {#each analysis.keyword_candidates.slice(0, 10) as kw}
                  <span
                    class="keyword-chip"
                    title={`Score: ${formatScore(kw.score)}, Häufigkeit: ${kw.frequency}`}
                  >
                    <span class="keyword-name">{kw.term}</span>
                    <span class="keyword-score" style="color: {getConfidenceColor(kw.score)}">
                      {formatScore(kw.score)}
                    </span>
                  </span>
                {/each}
                {#if analysis.keyword_candidates.length > 10}
                  <span class="more-indicator">
                    +{analysis.keyword_candidates.length - 10}
                  </span>
                {/if}
              </div>
            </div>
          {/if}

          <!-- Category Scores -->
          {#if analysis.category_scores.length > 0}
            <div class="preview-section">
              <h4 class="section-label">
                <i class="fa-solid fa-folder-tree"></i>
                {$_('articleView.categoryScores') || 'Kategorie-Scores'}
                <span class="count">({analysis.category_scores.length})</span>
              </h4>
              <div class="category-list">
                {#each analysis.category_scores as cat}
                  <div class="category-item">
                    <div class="category-header">
                      <span class="category-name">{cat.name}</span>
                      <span
                        class="category-confidence"
                        style="color: {getConfidenceColor(cat.confidence)}"
                      >
                        {formatScore(cat.confidence)}
                      </span>
                    </div>
                    {#if cat.matching_terms.length > 0}
                      <div class="matching-terms">
                        <span class="terms-label">
                          {$_('articleView.matchingTerms') || 'Passende Begriffe'}:
                        </span>
                        {#each cat.matching_terms.slice(0, 5) as term}
                          <span class="term-tag">{term}</span>
                        {/each}
                        {#if cat.matching_terms.length > 5}
                          <span class="more-terms">+{cat.matching_terms.length - 5}</span>
                        {/if}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if analysis.keyword_candidates.length === 0 && analysis.category_scores.length === 0}
            <div class="empty-state">
              <i class="fa-solid fa-circle-info"></i>
              <span>{$_('articleView.noStatisticalData') || 'Keine statistischen Daten gefunden'}</span>
            </div>
          {/if}

          <div class="preview-note">
            <i class="fa-solid fa-circle-info"></i>
            <span>
              {$_('articleView.statisticalNote') || 'Diese Vorschau zeigt die statistische Analyse vor der LLM-Validierung.'}
            </span>
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .statistical-preview {
    background: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    margin-bottom: 1rem;
    overflow: hidden;
  }

  .preview-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.75rem 1rem;
    background: var(--bg-overlay);
    border: none;
    cursor: pointer;
    color: var(--text-primary);
    transition: background-color 0.2s;
  }

  .preview-header:hover {
    background: var(--bg-muted);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-left i {
    color: var(--accent-primary);
    font-size: 0.875rem;
  }

  .header-title {
    font-weight: 600;
    font-size: 0.875rem;
  }

  .preview-badge {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    background: var(--accent-primary);
    color: var(--text-on-accent);
    border-radius: 0.25rem;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.025em;
  }

  .chevron {
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: transform 0.2s;
  }

  .chevron.collapsed {
    transform: rotate(-90deg);
  }

  .preview-content {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .loading-state,
  .error-state,
  .empty-state {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
  }

  .loading-state {
    color: var(--text-muted);
  }

  .error-state {
    color: var(--status-error);
    background: rgba(239, 68, 68, 0.1);
  }

  .empty-state {
    color: var(--text-muted);
    background: var(--bg-overlay);
  }

  .preview-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .section-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.025em;
    margin: 0;
  }

  .section-label i {
    font-size: 0.6875rem;
  }

  .section-label .count {
    font-weight: normal;
    color: var(--text-muted);
  }

  /* Keyword Chips */
  .keyword-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .keyword-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg-overlay);
    border: 1px solid var(--border-muted);
    border-radius: 0.375rem;
    font-size: 0.8125rem;
  }

  .keyword-name {
    color: var(--text-primary);
  }

  .keyword-score {
    font-size: 0.6875rem;
    font-weight: 600;
  }

  .more-indicator {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    font-style: italic;
  }

  /* Category List */
  .category-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .category-item {
    padding: 0.5rem 0.75rem;
    background: var(--bg-overlay);
    border-radius: 0.375rem;
  }

  .category-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.25rem;
  }

  .category-name {
    font-weight: 500;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .category-confidence {
    font-size: 0.75rem;
    font-weight: 600;
  }

  .matching-terms {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
  }

  .terms-label {
    color: var(--text-muted);
    margin-right: 0.25rem;
  }

  .term-tag {
    padding: 0.0625rem 0.375rem;
    background: var(--bg-surface);
    border-radius: 0.25rem;
    color: var(--text-secondary);
  }

  .more-terms {
    color: var(--text-muted);
    font-style: italic;
  }

  /* Preview Note */
  .preview-note {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: var(--bg-overlay);
    border-radius: 0.375rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    border-left: 3px solid var(--accent-primary);
  }

  .preview-note i {
    margin-top: 0.125rem;
    flex-shrink: 0;
  }
</style>
