<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { Keyword, KeywordNeighbor, KeywordCategory } from "../../types";
  import KeywordDetailHeader from "./KeywordDetailHeader.svelte";
  import KeywordDetailMeta from "./KeywordDetailMeta.svelte";
  import KeywordSimilarSection from "./KeywordSimilarSection.svelte";
  import KeywordCategorySection from "./KeywordCategorySection.svelte";
  import KeywordTrendSection from "./KeywordTrendSection.svelte";
  import KeywordArticleList from "./KeywordArticleList.svelte";

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
</script>

<div class="detail-panel">
  {#if selectedKeyword}
    <div class="keyword-detail">
      <KeywordDetailHeader
        name={selectedKeyword.name}
        {isRenaming}
        {renameInput}
        {renameLoading}
        {renameError}
        {onStartRename}
        {onCancelRename}
        {onHandleRename}
        {onRenameInputChange}
      />

      <KeywordDetailMeta
        articleCount={selectedKeyword.article_count}
        firstSeen={selectedKeyword.first_seen}
        lastUsed={selectedKeyword.last_used}
      />

      <KeywordSimilarSection
        keywordId={selectedKeyword.id}
        {similarKeywords}
        {similarKeywordsLoading}
        {onKeywordSelect}
        {onSynonymAssigned}
      />

      <KeywordCategorySection {keywordCategories} />

      <KeywordTrendSection
        keywordId={selectedKeyword.id}
        keywordName={selectedKeyword.name}
        {cooccurringKeywords}
        {onKeywordSelect}
        {onDaysChange}
      />

      <KeywordArticleList
        {keywordArticles}
        {hasMoreArticles}
        {articlesLoading}
        {loading}
        {onOpenArticle}
        {onLoadMoreArticles}
      />
    </div>
  {:else}
    <div class="no-selection">
      <i class="no-selection-icon fa-solid fa-link"></i>
      <p>{$_("network.selectKeyword")}</p>
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
</style>
