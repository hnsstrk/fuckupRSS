<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import Tooltip from '../Tooltip.svelte';
  import KeywordContextTooltip from '../KeywordContextTooltip.svelte';
  import KeywordTrendChart from '../KeywordTrendChart.svelte';
  import type { Keyword, KeywordNeighbor, KeywordCategory } from '../../types';
  import { SvelteSet } from 'svelte/reactivity';

  // Get the main category ID (1-6) from a category or subcategory ID
  function getMainCategoryId(id: number | undefined): number {
    if (!id) return 0;
    if (id <= 6) return id;
    return Math.floor(id / 100); // Subcategory IDs are 101, 102, 201, etc.
  }

  // Get CSS variable name for category color
  function getCategoryColorVar(id: number | undefined): string {
    const mainId = getMainCategoryId(id);
    if (mainId >= 1 && mainId <= 6) {
      return `var(--category-${mainId})`;
    }
    return 'var(--accent-primary)';
  }

  // Type for keyword articles
  interface KeywordArticle {
    id: number;
    title: string;
    pentacle_title: string | null;
    published_at: string | null;
    status: string;
  }

  // Type for co-occurring keywords
  interface CooccurringKeyword {
    id: number;
    name: string;
    cooccurrence_count: number;
  }

  // Type for similar keywords
  interface SimilarKeyword {
    id: number;
    name: string;
    similarity: number;
    cooccurrence: number;
    is_true_synonym?: boolean;
  }

  // Props
  interface Props {
    selectedKeyword: Keyword | null;
    neighbors: KeywordNeighbor[];
    keywordCategories: KeywordCategory[];
    keywordArticles: KeywordArticle[];
    cooccurringKeywords: CooccurringKeyword[];
    similarKeywords: SimilarKeyword[];
    similarKeywordsLoading: boolean;
    hasMoreArticles: boolean;
    articlesLoading: boolean;
    loading: boolean;
    trendDays: number;
    // Rename state
    isRenaming: boolean;
    renameInput: string;
    renameLoading: boolean;
    renameError: string | null;
    // Callbacks
    onKeywordSelect: (id: number) => void;
    onOpenArticle: (id: number) => void;
    onLoadMoreArticles: () => void;
    onDaysChange: (days: number) => void;
    onStartRename: () => void;
    onCancelRename: () => void;
    onHandleRename: () => void;
    onRenameInputChange: (value: string) => void;
    onSynonymAssigned?: () => void;
  }

  let {
    selectedKeyword,
    neighbors: _neighbors,
    keywordCategories,
    keywordArticles,
    cooccurringKeywords,
    similarKeywords,
    similarKeywordsLoading,
    hasMoreArticles,
    articlesLoading,
    loading,
    trendDays: _trendDays,
    isRenaming,
    renameInput,
    renameLoading,
    renameError,
    onKeywordSelect,
    onOpenArticle,
    onLoadMoreArticles,
    onDaysChange,
    onStartRename,
    onCancelRename,
    onHandleRename,
    onRenameInputChange,
    onSynonymAssigned,
  }: Props = $props();

  // Synonym selection state
  let selectedSynonymIds = new SvelteSet<number>();
  let assigningSynonyms = $state(false);
  let synonymError = $state<string | null>(null);
  let synonymSuccess = $state<string | null>(null);
  let synonymSectionOpen = $state(false);

  // Reset selection when keyword changes
  $effect(() => {
    if (selectedKeyword) {
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
    if (!selectedKeyword || selectedSynonymIds.size === 0) return;

    assigningSynonyms = true;
    synonymError = null;
    synonymSuccess = null;

    const keepId = selectedKeyword.id;
    const idsToMerge = Array.from(selectedSynonymIds);
    let successCount = 0;
    let errorCount = 0;
    let totalAffectedArticles = 0;

    for (const idToRemove of idsToMerge) {
      try {
        // Use merge_keyword_pair which actually deletes the keyword
        // and transfers all relationships (articles, categories, etc.)
        // IMPORTANT: Tauri does NOT auto-convert camelCase to snake_case!
        const result = await invoke<{ merged_pairs: number; affected_articles: number }>(
          'merge_keyword_pair',
          {
            keep_id: keepId,       // Must use snake_case to match Rust parameter names
            remove_id: idToRemove  // Must use snake_case to match Rust parameter names
          }
        );
        successCount++;
        totalAffectedArticles += result.affected_articles;
        selectedSynonymIds.delete(idToRemove);
      } catch (e) {
        console.error(`Failed to merge keyword ${idToRemove}:`, e);
        errorCount++;
      }
    }

    assigningSynonyms = false;

    if (errorCount > 0) {
      synonymError = $_('network.synonymAssignedPartial', {
        values: { success: successCount, failed: errorCount }
      }) || `${successCount} merged, ${errorCount} failed`;
    } else {
      synonymSuccess = $_('network.synonymsMerged', {
        values: { count: successCount, articles: totalAffectedArticles }
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

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleDateString();
  }

  function formatArticleDate(dateStr: string | null): string {
    if (!dateStr) return '';
    const date = new Date(dateStr);
    return date.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });
  }

  function getWeightClass(weight: number): string {
    if (weight >= 0.7) return 'weight-high';
    if (weight >= 0.4) return 'weight-medium';
    return 'weight-low';
  }

  function getStatusIconClass(status: string): string {
    switch (status) {
      case 'concealed': return 'fa-solid fa-eye-slash';
      case 'illuminated': return 'fa-solid fa-check';
      case 'golden_apple': return 'fa-solid fa-apple-whole';
      default: return '';
    }
  }

  function handleRenameKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      onHandleRename();
    } else if (e.key === 'Escape') {
      onCancelRename();
    }
  }

  // Group categories by main category
  let groupedCategories = $derived.by(() => {
    if (keywordCategories.length === 0) return [];

    // Separate main categories (parent_id is null) from subcategories
    const mainCats = keywordCategories.filter(c => c.parent_id === null);
    const subCats = keywordCategories.filter(c => c.parent_id !== null);

    // Group subcategories by their parent
    const subsByParent = subCats.reduce((acc, cat) => {
      const key = cat.parent_name || 'Sonstige';
      if (!acc[key]) acc[key] = [];
      acc[key].push(cat);
      return acc;
    }, {} as Record<string, typeof keywordCategories>);

    // Build result: main categories with their subcategories
    const result: Array<{
      id: number;
      name: string;
      icon: string | null;
      color: string | null;
      weight: number;
      subcategories: typeof keywordCategories;
    }> = [];

    // Add main categories that are directly assigned
    for (const main of mainCats) {
      const subs = subsByParent[main.name] || [];
      delete subsByParent[main.name];
      result.push({
        id: main.sephiroth_id,
        name: main.name,
        icon: main.icon,
        color: main.color,
        weight: main.weight + subs.reduce((sum, s) => sum + s.weight, 0),
        subcategories: subs
      });
    }

    // Add subcategories whose main category is not directly assigned
    for (const [parentName, subs] of Object.entries(subsByParent)) {
      const firstSub = subs[0];
      result.push({
        id: firstSub.parent_id || 0,
        name: parentName,
        icon: firstSub.parent_icon,
        color: firstSub.color,
        weight: subs.reduce((sum, s) => sum + s.weight, 0),
        subcategories: subs
      });
    }

    return result.sort((a, b) => b.weight - a.weight);
  });

  let maxWeight = $derived(Math.max(...groupedCategories.map(c => c.weight), 0.01));
</script>

<div class="detail-panel">
  {#if selectedKeyword}
    <div class="keyword-detail">
      <div class="detail-title-row">
        {#if isRenaming}
          <div class="rename-form">
            <!-- svelte-ignore a11y_autofocus -->
            <input
              type="text"
              class="rename-input"
              value={renameInput}
              oninput={(e) => onRenameInputChange(e.currentTarget.value)}
              onkeydown={handleRenameKeydown}
              disabled={renameLoading}
              autofocus
            />
            <button
              class="rename-btn save"
              onclick={onHandleRename}
              disabled={renameLoading || !renameInput.trim()}
              title={$_('common.save') || 'Speichern'}
              aria-label={$_('common.save') || 'Speichern'}
            >
              {#if renameLoading}
                <i class="fa-solid fa-spinner fa-spin"></i>
              {:else}
                <i class="fa-solid fa-check"></i>
              {/if}
            </button>
            <button
              class="rename-btn cancel"
              onclick={onCancelRename}
              disabled={renameLoading}
              title={$_('common.cancel') || 'Abbrechen'}
              aria-label={$_('common.cancel') || 'Abbrechen'}
            >
              <i class="fa-solid fa-times"></i>
            </button>
          </div>
          {#if renameError}
            <div class="rename-error">{renameError}</div>
          {/if}
        {:else}
          <h3 class="detail-title">{selectedKeyword.name}</h3>
          <button
            class="edit-btn"
            onclick={onStartRename}
            title={$_('network.renameKeyword') || 'Umbenennen'}
            aria-label={$_('network.renameKeyword') || 'Umbenennen'}
          >
            <i class="fa-solid fa-pen"></i>
          </button>
        {/if}
      </div>

      <div class="detail-meta">
        <span class="meta-item">
          <span class="meta-label">{$_('network.articleCount')}:</span>
          <span class="meta-value">{selectedKeyword.article_count}</span>
        </span>
        <span class="meta-item">
          <span class="meta-label">{$_('network.firstSeen')}:</span>
          <span class="meta-value">{formatDate(selectedKeyword.first_seen)}</span>
        </span>
        <span class="meta-item">
          <span class="meta-label">{$_('network.lastUsed')}:</span>
          <span class="meta-value">{formatDate(selectedKeyword.last_used)}</span>
        </span>
      </div>

      <!-- Similar Keywords Section (collapsible) - based on semantic embedding similarity -->
      {#if similarKeywords.length > 0}
        <details class="synonyms-section" bind:open={synonymSectionOpen}>
          <summary class="synonyms-summary">
            <i class="fa-solid fa-link summary-icon"></i>
            <span>{$_('network.synonyms') || 'Ähnliche Keywords'}</span>
            <span class="synonym-count">({similarKeywords.length})</span>
            <Tooltip content={$_('network.similarKeywordsHelp') || 'Keywords mit ähnlicher semantischer Bedeutung basierend auf Embeddings'}>
              <i class="fa-solid fa-circle-info help-icon"></i>
            </Tooltip>
            <i class="fa-solid fa-chevron-down chevron-icon" class:open={synonymSectionOpen}></i>
          </summary>
          <div class="synonyms-content">
            <p class="synonyms-help">
              {$_('network.synonymsHelp') || 'Wähle semantisch ähnliche Keywords aus, um sie zusammenzuführen. Die Ähnlichkeit basiert auf Embedding-Vektoren, nicht auf lexikalischer Übereinstimmung.'}
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

            <div class="synonyms-list">
              {#each similarKeywords as simKw (simKw.id)}
                {@const similarityPercent = Math.round(simKw.similarity * 100)}
                {@const isSelected = selectedSynonymIds.has(simKw.id)}
                <label class="synonym-item" class:selected={isSelected}>
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onchange={() => toggleSynonymSelection(simKw.id)}
                    disabled={assigningSynonyms}
                  />
                  <KeywordContextTooltip keywordId={simKw.id} keywordName={simKw.name}>
                    <span class="synonym-name">{simKw.name}</span>
                  </KeywordContextTooltip>
                  <span class="synonym-similarity">{similarityPercent}%</span>
                  <button
                    class="synonym-view-btn"
                    onclick={(e) => { e.preventDefault(); e.stopPropagation(); onKeywordSelect(simKw.id); }}
                    title={$_('network.showInNetwork') || 'Im Netzwerk anzeigen'}
                  >
                    <i class="fa-solid fa-arrow-up-right-from-square"></i>
                  </button>
                </label>
              {/each}
            </div>

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
                {$_('network.assignAsSynonyms') || 'Als Synonyme zuordnen'}
                {#if selectedSynonymIds.size > 0}
                  ({selectedSynonymIds.size})
                {/if}
              </button>
            </div>
          </div>
        </details>
      {/if}

      <!-- Categories grouped by main category -->
      {#if groupedCategories.length > 0}
        <div class="detail-section">
          <h4 class="section-title">
            {$_('network.categories')}
            <Tooltip content={$_('network.categoriesHelp')}>
              <i class="fa-solid fa-circle-info help-icon"></i>
            </Tooltip>
          </h4>
          <div class="category-cards">
            {#each groupedCategories as group (group.id)}
              {@const barWidth = (group.weight / maxWeight) * 100}
              <div class="category-card" style="--cat-color: {getCategoryColorVar(group.id)}">
                <div class="card-header">
                  <div class="card-icon-wrapper">
                    <i class="{group.icon || 'fa-solid fa-folder'}"></i>
                  </div>
                  <span class="card-title">{group.name}</span>
                </div>
                <div class="card-stats">
                  <div class="stat-row">
                    <span class="stat-label">{$_('network.weight') || 'Gewicht'}</span>
                    <span class="stat-value">{(group.weight * 100).toFixed(0)}%</span>
                  </div>
                  <div class="progress-bar">
                    <div class="progress-fill" style="width: {barWidth}%"></div>
                  </div>
                </div>
                {#if group.subcategories.length > 0}
                  <div class="subcategories">
                    {#each group.subcategories as cat (cat.sephiroth_id)}
                      <div class="subcategory-item">
                        <div class="subcategory-info">
                          <i class="{cat.icon || 'fa-solid fa-folder'} subcategory-icon"></i>
                          <span class="subcategory-name">{cat.name}</span>
                        </div>
                        <span class="subcategory-weight {getWeightClass(cat.weight)}">{(cat.weight * 100).toFixed(0)}%</span>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Trend Chart with Co-occurring Keywords -->
      <div class="detail-section">
        <h4 class="section-title">{$_('network.trendComparison')}</h4>
        <KeywordTrendChart
          keywordId={selectedKeyword.id}
          keywordName={selectedKeyword.name}
          neighborIds={cooccurringKeywords.slice(0, 4).map(k => k.id)}
          ondayschange={onDaysChange}
        />
        {#if cooccurringKeywords.length > 0}
          <div class="neighbor-legend">
            <span class="legend-label">{$_('network.comparedWith')}:</span>
            <!-- Top 4 with colors matching the chart -->
            <div class="colored-neighbors">
              {#each cooccurringKeywords.slice(0, 4) as coKw, idx (coKw.id)}
                <button
                  class="neighbor-tag neighbor-tag-colored neighbor-color-{idx + 1}"
                  onclick={() => onKeywordSelect(coKw.id)}
                  title="{coKw.cooccurrence_count} {$_('network.articleCount')}"
                >
                  {coKw.name}
                </button>
              {/each}
            </div>
            <!-- Remaining keywords in neutral style -->
            {#if cooccurringKeywords.length > 4}
              <div class="neutral-neighbors">
                {#each cooccurringKeywords.slice(4) as coKw (coKw.id)}
                  <button
                    class="neighbor-tag neighbor-tag-neutral"
                    onclick={() => onKeywordSelect(coKw.id)}
                    title="{coKw.cooccurrence_count} {$_('network.articleCount')}"
                  >
                    {coKw.name}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Similar Keywords (Embedding-based) -->
      <div class="detail-section">
        <h4 class="section-title">
          <i class="fa-solid fa-diagram-project section-icon"></i>
          {$_('network.similarKeywords') || 'Aehnliche Keywords'}
          <Tooltip content={$_('network.similarKeywordsHelp') || 'Keywords mit aehnlicher semantischer Bedeutung basierend auf Embeddings'}>
            <i class="fa-solid fa-circle-info help-icon"></i>
          </Tooltip>
        </h4>
        {#if similarKeywordsLoading}
          <div class="loading-similar">
            <i class="fa-solid fa-spinner fa-spin"></i>
            {$_('network.loading') || 'Laden...'}
          </div>
        {:else if similarKeywords.length > 0}
          <div class="similar-keywords-list">
            {#each similarKeywords as simKw (simKw.id)}
              {@const similarityPercent = Math.round(simKw.similarity * 100)}
              {@const similarityClass = simKw.is_true_synonym ? 'synonym' : similarityPercent >= 80 ? 'high' : similarityPercent >= 60 ? 'medium' : 'low'}
              <button
                class="similar-keyword-row"
                class:true-synonym={simKw.is_true_synonym}
                onclick={() => onKeywordSelect(simKw.id)}
                title="{simKw.is_true_synonym ? $_('network.trueSynonymHint') || 'Bereits zusammengeführtes Synonym' : simKw.cooccurrence > 0 ? simKw.cooccurrence + ' gemeinsame Artikel' : 'Semantisch ähnlich'}"
              >
                {#if simKw.is_true_synonym}
                  <span class="true-synonym-badge">
                    <i class="fa-solid fa-check"></i>
                  </span>
                {/if}
                <KeywordContextTooltip keywordId={simKw.id} keywordName={simKw.name}>
                  <span class="similar-name">{simKw.name}</span>
                </KeywordContextTooltip>
                <div class="similar-bar-wrap">
                  <div class="similarity-bar {similarityClass}" style="width: {similarityPercent}%"></div>
                </div>
                <span class="similar-stats">
                  {#if simKw.is_true_synonym}
                    <span class="similarity-pct synonym">{$_('network.merged') || 'Synonym'}</span>
                  {:else}
                    <span class="similarity-pct {similarityClass}">{similarityPercent}%</span>
                    {#if simKw.cooccurrence > 0}
                      <span class="cooccur-count">({simKw.cooccurrence})</span>
                    {/if}
                  {/if}
                </span>
              </button>
            {/each}
          </div>
        {:else}
          <div class="no-similar">{$_('network.noSimilarKeywords') || 'Keine aehnlichen Keywords gefunden'}</div>
        {/if}
      </div>

      <!-- Linked Articles -->
      <div class="detail-section">
        <h4 class="section-title">{$_('network.linkedArticles') || 'Verlinkte Artikel'}</h4>
        {#if keywordArticles.length > 0}
          <div class="articles-list">
            {#each keywordArticles as article (article.id)}
              <button class="article-item" onclick={() => onOpenArticle(article.id)}>
                <i class="article-status {getStatusIconClass(article.status)}" title={article.status}></i>
                <div class="article-info">
                  <span class="article-title">{article.title}</span>
                  <span class="article-meta">
                    {#if article.pentacle_title}
                      <span class="article-source">{article.pentacle_title}</span>
                    {/if}
                    {#if article.published_at}
                      <span class="article-date">{formatArticleDate(article.published_at)}</span>
                    {/if}
                  </span>
                </div>
              </button>
            {/each}
            {#if hasMoreArticles}
              <button
                class="load-more-articles"
                onclick={onLoadMoreArticles}
                disabled={articlesLoading}
              >
                {#if articlesLoading}
                  {$_('network.loading') || 'Laden...'}
                {:else}
                  {$_('network.loadMore') || 'Mehr laden'}
                {/if}
              </button>
            {/if}
          </div>
        {:else if !loading}
          <div class="no-articles">{$_('network.noArticles') || 'Keine Artikel gefunden'}</div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="no-selection">
      <i class="no-selection-icon fa-solid fa-link"></i>
      <p>{$_('network.selectKeyword')}</p>
    </div>
  {/if}
</div>

<style>
  /* Detail Panel */
  .detail-panel {
    flex: 1;
    overflow-y: auto;
    background-color: var(--bg-base);
  }

  .keyword-detail {
    padding: 1.5rem;
  }

  .detail-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .detail-title {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .edit-btn {
    padding: 0.375rem 0.5rem;
    background: none;
    border: 1px solid transparent;
    border-radius: 0.25rem;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .edit-btn:hover {
    color: var(--accent-primary);
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .rename-form {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
  }

  .rename-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    font-size: 1.25rem;
    font-weight: 600;
    border: 2px solid var(--accent-primary);
    border-radius: 0.375rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    outline: none;
  }

  .rename-input:focus {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-primary) 25%, transparent);
  }

  .rename-btn {
    padding: 0.5rem 0.625rem;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.875rem;
  }

  .rename-btn.save {
    background-color: var(--accent-success);
    color: var(--text-on-accent);
  }

  .rename-btn.save:hover:not(:disabled) {
    background-color: color-mix(in srgb, var(--accent-success) 80%, black);
  }

  .rename-btn.cancel {
    background-color: var(--bg-overlay);
    color: var(--text-muted);
  }

  .rename-btn.cancel:hover:not(:disabled) {
    background-color: var(--bg-muted);
    color: var(--text-primary);
  }

  .rename-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .rename-error {
    margin-top: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--accent-error) 15%, transparent);
    border-radius: 0.375rem;
    color: var(--accent-error);
    font-size: 0.8125rem;
  }

  .detail-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 1.5rem;
    padding: 0.75rem;
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
  }

  .meta-item {
    font-size: 0.875rem;
  }

  .meta-label {
    color: var(--text-muted);
  }

  .meta-value {
    color: var(--text-primary);
    font-weight: 500;
  }

  .detail-section {
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  /* Category Cards (matching FnordView) */
  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 0.75rem;
  }

  .category-card {
    background: linear-gradient(135deg, color-mix(in srgb, var(--cat-color) 15%, var(--bg-base)) 0%, var(--bg-base) 100%);
    border: 1px solid color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-radius: 0.5rem;
    padding: 0.75rem;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.625rem;
  }

  .card-icon-wrapper {
    width: 1.75rem;
    height: 1.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 70%, black));
    border-radius: 0.375rem;
    color: var(--text-on-accent);
    font-size: 0.8125rem;
    box-shadow: 0 2px 6px color-mix(in srgb, var(--cat-color) 40%, transparent);
  }

  .card-title {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-stats {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .stat-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .stat-value {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .progress-bar {
    height: 4px;
    background-color: color-mix(in srgb, var(--cat-color) 20%, transparent);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 80%, white));
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  /* Subcategories in cards */
  .subcategories {
    margin-top: 0.625rem;
    padding-top: 0.625rem;
    border-top: 1px solid color-mix(in srgb, var(--cat-color) 20%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.25rem 0.5rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.25rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    min-width: 0;
    flex: 1;
  }

  .subcategory-icon {
    font-size: 0.625rem;
    color: var(--cat-color);
    flex-shrink: 0;
  }

  .subcategory-name {
    font-size: 0.6875rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .subcategory-weight {
    font-size: 0.5625rem;
    padding: 0.125rem 0.25rem;
    border-radius: 0.1875rem;
    font-weight: 600;
    flex-shrink: 0;
  }

  .subcategory-weight.weight-high {
    background-color: rgba(34, 197, 94, 0.2);
    color: var(--accent-success);
  }

  .subcategory-weight.weight-medium {
    background-color: rgba(251, 191, 36, 0.2);
    color: var(--accent-warning);
  }

  .subcategory-weight.weight-low {
    background-color: var(--bg-overlay);
    color: var(--text-muted);
  }

  /* No Selection State */
  .no-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    padding: 2rem;
    text-align: center;
  }

  .no-selection-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .no-selection p {
    font-size: 0.875rem;
    margin: 0;
  }

  /* Neighbor Legend */
  .neighbor-legend {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding: 0.5rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }

  .legend-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .colored-neighbors {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .neutral-neighbors {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    padding-top: 0.375rem;
    border-top: 1px solid var(--border-muted);
  }

  .neighbor-tag {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    border-radius: 0.25rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  /* Colored neighbor tags using theme category colors */
  .neighbor-tag-colored {
    font-weight: 500;
  }

  .neighbor-color-1 {
    background-color: var(--category-1-bg);
    border: 1px solid var(--category-1-border);
    color: var(--category-1);
  }

  .neighbor-color-1:hover {
    background-color: var(--category-1);
    color: var(--text-on-accent);
  }

  .neighbor-color-2 {
    background-color: var(--category-2-bg);
    border: 1px solid var(--category-2-border);
    color: var(--category-2);
  }

  .neighbor-color-2:hover {
    background-color: var(--category-2);
    color: var(--text-on-accent);
  }

  .neighbor-color-3 {
    background-color: var(--category-3-bg);
    border: 1px solid var(--category-3-border);
    color: var(--category-3);
  }

  .neighbor-color-3:hover {
    background-color: var(--category-3);
    color: var(--text-on-accent);
  }

  .neighbor-color-4 {
    background-color: var(--category-4-bg);
    border: 1px solid var(--category-4-border);
    color: var(--category-4);
  }

  .neighbor-color-4:hover {
    background-color: var(--category-4);
    color: var(--text-on-accent);
  }

  /* Neutral neighbor tags */
  .neighbor-tag-neutral {
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    color: var(--text-secondary);
    font-size: 0.6875rem;
  }

  .neighbor-tag-neutral:hover {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  /* Articles List */
  .articles-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .article-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-surface);
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    text-align: left;
    transition: background-color 0.15s;
    width: 100%;
  }

  .article-item:hover {
    background-color: var(--bg-overlay);
  }

  .article-status {
    font-size: 0.875rem;
    flex-shrink: 0;
    width: 1.25rem;
  }

  .article-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    min-width: 0;
    flex: 1;
  }

  .article-title {
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .article-meta {
    display: flex;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .article-source {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 150px;
  }

  .article-date {
    flex-shrink: 0;
  }

  .load-more-articles {
    padding: 0.5rem;
    background: none;
    border: 1px dashed var(--border-default);
    border-radius: 0.375rem;
    color: var(--accent-primary);
    cursor: pointer;
    font-size: 0.75rem;
    transition: all 0.2s;
  }

  .load-more-articles:hover:not(:disabled) {
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .load-more-articles:disabled {
    color: var(--text-muted);
    cursor: not-allowed;
  }

  .no-articles {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }

  /* Similar Keywords Section */
  .section-icon {
    font-size: 0.875rem;
    color: var(--accent-primary);
  }

  .similar-keywords-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .similar-keyword-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.5rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
  }

  .similar-keyword-row:hover {
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .similar-name {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 120px;
    flex-shrink: 0;
  }

  .similar-bar-wrap {
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

  .similar-stats {
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

  /* True Synonym styling */
  .similar-keyword-row.true-synonym {
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

  .loading-similar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .no-similar {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }

  /* Synonyms Section */
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
    flex: 1;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .synonym-similarity {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--accent-success);
    padding: 0.125rem 0.375rem;
    background-color: rgba(166, 227, 161, 0.15);
    border-radius: 0.25rem;
  }

  .synonym-view-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border: none;
    border-radius: 0.25rem;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s;
  }

  .synonym-view-btn:hover {
    color: var(--accent-primary);
    background-color: var(--bg-surface);
  }

  .synonym-view-btn i {
    font-size: 0.6875rem;
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
