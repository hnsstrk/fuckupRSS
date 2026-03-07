import { invoke } from "@tauri-apps/api/core";
import type {
  Keyword,
  KeywordNeighbor,
  KeywordCategory,
  TrendingKeyword,
  NetworkStats,
  NetworkGraph,
} from "../types";

// Type for keyword articles
export interface KeywordArticle {
  id: number;
  title: string;
  pentacle_title: string | null;
  published_at: string | null;
  status: string;
}

// Type for co-occurring keywords
export interface CooccurringKeyword {
  id: number;
  name: string;
  cooccurrence_count: number;
}

// Type for synonym candidate
export interface SynonymCandidate {
  keyword_a_id: number;
  keyword_a_name: string;
  keyword_b_id: number;
  keyword_b_name: string;
  similarity: number;
}

// Type for merge result
export interface MergeSynonymsResult {
  merged_pairs: number;
  affected_articles: number;
}

// Type for create keyword result
export interface CreateKeywordResult {
  id: number;
  name: string;
  created: boolean;
}

// Similar keyword type
export interface SimilarKeywordEntry {
  id: number;
  name: string;
  similarity: number;
  cooccurrence: number;
}

class ImmanentizeNetworkStore {
  // Core keyword state
  keywords = $state<Keyword[]>([]);
  selectedKeyword = $state<Keyword | null>(null);
  neighbors = $state<KeywordNeighbor[]>([]);
  keywordCategories = $state<KeywordCategory[]>([]);
  trendingKeywords = $state<TrendingKeyword[]>([]);
  trendingPeriod = $state(7);
  trendingSortBy = $state<"score" | "growth" | "count" | "new">("score");
  networkStats = $state<NetworkStats | null>(null);
  searchResults = $state<Keyword[]>([]);
  searchQuery = $state("");
  loading = $state(false);
  error = $state<string | null>(null);
  graphData = $state<NetworkGraph | null>(null);
  graphLoading = $state(false);
  offset = $state(0);
  limit = $state(50);
  hasMore = $state(true);

  // Detail panel state
  keywordArticles = $state<KeywordArticle[]>([]);
  cooccurringKeywords = $state<CooccurringKeyword[]>([]);
  similarKeywords = $state<SimilarKeywordEntry[]>([]);
  similarKeywordsLoading = $state(false);
  articlesLoading = $state(false);
  hasMoreArticles = $state(true);
  articlesOffset = $state(0);
  articlesLimit = 10;
  trendDays = $state(30);

  // Rename state
  isRenaming = $state(false);
  renameInput = $state("");
  renameLoading = $state(false);
  renameError = $state<string | null>(null);

  // Synonyms tab state
  synonymCandidates = $state<SynonymCandidate[]>([]);
  synonymsLoading = $state(false);
  synonymsError = $state<string | null>(null);
  synonymSuccess = $state<string | null>(null);

  // Manual merge state
  keepSearchInput = $state("");
  keepSearchResults = $state<Keyword[]>([]);
  selectedKeepKeyword = $state<Keyword | null>(null);
  removeSearchInput = $state("");
  removeSearchResults = $state<Keyword[]>([]);
  selectedRemoveKeyword = $state<Keyword | null>(null);

  // Create keyword state
  newKeywordInput = $state("");
  createKeywordLoading = $state(false);
  createKeywordSuccess = $state<string | null>(null);
  createKeywordError = $state<string | null>(null);

  // Event listener tracking
  private _listenersActive = false;
  private _boundRefreshAll: (() => void) | null = null;

  async loadKeywords(reset = false): Promise<void> {
    if (this.loading) return;

    try {
      this.loading = true;
      this.error = null;

      if (reset) {
        this.offset = 0;
        this.keywords = [];
        this.hasMore = true;
      }

      const newKeywords = await invoke<Keyword[]>("get_keywords", {
        limit: this.limit,
        offset: this.offset,
      });

      if (newKeywords.length < this.limit) {
        this.hasMore = false;
      }

      this.keywords = reset ? newKeywords : [...this.keywords, ...newKeywords];
      this.offset += newKeywords.length;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load keywords:", e);
    } finally {
      this.loading = false;
    }
  }

  async loadMoreKeywords(): Promise<void> {
    if (!this.hasMore || this.loading) return;
    await this.loadKeywords(false);
  }

  async selectKeyword(id: number | null): Promise<void> {
    if (id === null) {
      this.selectedKeyword = null;
      this.neighbors = [];
      this.keywordCategories = [];
      this.keywordArticles = [];
      this.cooccurringKeywords = [];
      this.similarKeywords = [];
      this.articlesOffset = 0;
      this.hasMoreArticles = true;
      return;
    }

    this.loading = true;
    this.error = null;
    // Reset detail state for new selection
    this.keywordArticles = [];
    this.cooccurringKeywords = [];
    this.similarKeywords = [];
    this.articlesOffset = 0;
    this.hasMoreArticles = true;

    try {
      const [keyword, neighbors, categories, articles, cooccurring] = await Promise.all([
        invoke<Keyword | null>("get_keyword", { id }),
        invoke<KeywordNeighbor[]>("get_keyword_neighbors", { id, limit: 10 }),
        invoke<KeywordCategory[]>("get_keyword_categories", { id }),
        invoke<KeywordArticle[]>("get_keyword_articles", {
          id,
          limit: this.articlesLimit,
          offset: 0,
        }),
        invoke<CooccurringKeyword[]>("get_cooccurring_keywords", {
          keywordId: id,
          days: this.trendDays,
          limit: 20,
        }),
      ]);

      this.selectedKeyword = keyword;
      this.neighbors = neighbors;
      this.keywordCategories = categories;
      this.keywordArticles = articles;
      this.cooccurringKeywords = cooccurring;
      this.articlesOffset = articles.length;
      this.hasMoreArticles = articles.length >= this.articlesLimit;
      // Load similar keywords after main data is loaded
      this.loadSimilarKeywords(id);
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load keyword details:", e);
    } finally {
      this.loading = false;
    }
  }

  async loadSimilarKeywords(keywordId: number): Promise<void> {
    this.similarKeywordsLoading = true;
    try {
      const similar = await invoke<SimilarKeywordEntry[]>("get_similar_keywords", {
        keywordId,
        limit: 8,
      });
      this.similarKeywords = similar;
    } catch (e) {
      console.error("Failed to load similar keywords:", e);
      this.similarKeywords = [];
    } finally {
      this.similarKeywordsLoading = false;
    }
  }

  async loadMoreArticles(): Promise<void> {
    if (!this.selectedKeyword || this.articlesLoading || !this.hasMoreArticles) return;
    this.articlesLoading = true;
    try {
      const articles = await invoke<KeywordArticle[]>("get_keyword_articles", {
        id: this.selectedKeyword.id,
        limit: this.articlesLimit,
        offset: this.articlesOffset,
      });
      this.keywordArticles = [...this.keywordArticles, ...articles];
      this.articlesOffset += articles.length;
      this.hasMoreArticles = articles.length >= this.articlesLimit;
    } catch (e) {
      console.error("Failed to load more articles:", e);
    } finally {
      this.articlesLoading = false;
    }
  }

  async loadCooccurringKeywords(keywordId: number, days: number): Promise<void> {
    try {
      this.cooccurringKeywords = await invoke<CooccurringKeyword[]>("get_cooccurring_keywords", {
        keywordId,
        days,
        limit: 20,
      });
    } catch (e) {
      console.error("Failed to load co-occurring keywords:", e);
      this.cooccurringKeywords = [];
    }
  }

  handleDaysChange(days: number): void {
    this.trendDays = days;
    if (this.selectedKeyword) {
      this.loadCooccurringKeywords(this.selectedKeyword.id, days);
    }
  }

  async searchKeywords(query: string): Promise<void> {
    this.searchQuery = query;

    if (!query.trim()) {
      this.searchResults = [];
      return;
    }

    try {
      this.loading = true;
      this.error = null;
      this.searchResults = await invoke<Keyword[]>("search_keywords", {
        query,
        limit: 20,
      });
    } catch (e) {
      this.error = String(e);
      console.error("Failed to search keywords:", e);
    } finally {
      this.loading = false;
    }
  }

  async loadTrendingKeywords(days?: number): Promise<void> {
    const d = days ?? this.trendingPeriod;
    try {
      this.trendingKeywords = await invoke<TrendingKeyword[]>("get_trending_keywords", {
        days: d,
        limit: 50,
        sortBy: this.trendingSortBy,
      });
    } catch (e) {
      console.error("Failed to load trending keywords:", e);
    }
  }

  async setTrendingPeriod(days: number): Promise<void> {
    this.trendingPeriod = days;
    await this.loadTrendingKeywords(days);
  }

  async setTrendingSort(sort: "score" | "growth" | "count" | "new"): Promise<void> {
    this.trendingSortBy = sort;
    await this.loadTrendingKeywords();
  }

  async loadNetworkStats(): Promise<void> {
    try {
      this.networkStats = await invoke<NetworkStats>("get_network_stats");
    } catch (e) {
      console.error("Failed to load network stats:", e);
    }
  }

  async loadNetworkGraph(limit = 100, minWeight = 0.1, minArticleCount = 3): Promise<void> {
    if (this.graphLoading) return;

    try {
      this.graphLoading = true;
      this.error = null;
      this.graphData = await invoke<NetworkGraph>("get_network_graph", {
        limit,
        minWeight,
        minArticleCount,
      });
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load network graph:", e);
    } finally {
      this.graphLoading = false;
    }
  }

  async loadCategoryKeywords(sephirothId: number): Promise<Keyword[]> {
    try {
      return await invoke<Keyword[]>("get_category_keywords", {
        sephirothId,
        limit: 50,
      });
    } catch (e) {
      console.error("Failed to load category keywords:", e);
      return [];
    }
  }

  async navigateToNeighbor(neighborId: number): Promise<void> {
    await this.selectKeyword(neighborId);
  }

  clearSearch(): void {
    this.searchQuery = "";
    this.searchResults = [];
  }

  // === Rename ===

  startRename(): void {
    if (this.selectedKeyword) {
      this.renameInput = this.selectedKeyword.name;
      this.isRenaming = true;
      this.renameError = null;
    }
  }

  cancelRename(): void {
    this.isRenaming = false;
    this.renameInput = "";
    this.renameError = null;
  }

  async handleRename(): Promise<void> {
    if (!this.selectedKeyword || !this.renameInput.trim()) return;
    if (this.renameInput.trim() === this.selectedKeyword.name) {
      this.cancelRename();
      return;
    }

    this.renameLoading = true;
    this.renameError = null;

    try {
      const newName = await invoke<string>("rename_keyword", {
        id: this.selectedKeyword.id,
        newName: this.renameInput.trim(),
      });

      // Update local state
      this.selectedKeyword = { ...this.selectedKeyword, name: newName };
      this.keywords = this.keywords.map((k) =>
        k.id === this.selectedKeyword!.id ? { ...k, name: newName } : k,
      );
      this.trendingKeywords = this.trendingKeywords.map((k) =>
        k.id === this.selectedKeyword!.id ? { ...k, name: newName } : k,
      );

      this.isRenaming = false;
      this.renameInput = "";
      window.dispatchEvent(new CustomEvent("keywords-changed"));
    } catch (e) {
      this.renameError = String(e);
      console.error("Failed to rename keyword:", e);
    } finally {
      this.renameLoading = false;
    }
  }

  // === Synonyms ===

  async findSynonymCandidates(): Promise<void> {
    this.synonymsLoading = true;
    this.synonymsError = null;
    this.synonymSuccess = null;

    try {
      this.synonymCandidates = await invoke<SynonymCandidate[]>("find_synonym_candidates", {
        threshold: 0.85,
        limit: 20,
      });
    } catch (e) {
      this.synonymsError = String(e);
      console.error("Failed to find synonym candidates:", e);
    } finally {
      this.synonymsLoading = false;
    }
  }

  async mergeKeywords(
    keepId: number,
    removeId: number,
    keepName: string,
    removeName: string,
  ): Promise<void> {
    this.synonymsLoading = true;
    this.synonymsError = null;
    this.synonymSuccess = null;

    try {
      const result = await invoke<MergeSynonymsResult>("merge_keyword_pair", {
        keepId,
        removeId,
      });
      this.synonymSuccess = `"${removeName}" -> "${keepName}" (${result.affected_articles} Artikel)`;
      this.synonymCandidates = this.synonymCandidates.filter(
        (c) =>
          !(
            (c.keyword_a_id === keepId && c.keyword_b_id === removeId) ||
            (c.keyword_a_id === removeId && c.keyword_b_id === keepId)
          ),
      );
      await this.loadKeywords(true);
      await this.loadNetworkStats();
      window.dispatchEvent(new CustomEvent("keywords-changed"));
    } catch (e) {
      this.synonymsError = String(e);
      console.error("Failed to merge keywords:", e);
    } finally {
      this.synonymsLoading = false;
    }
  }

  async dismissSynonymPair(keywordAId: number, keywordBId: number): Promise<void> {
    this.synonymsError = null;
    this.synonymSuccess = null;

    try {
      await invoke("dismiss_synonym_pair", { keywordAId, keywordBId });
      this.synonymCandidates = this.synonymCandidates.filter(
        (c) =>
          !(
            (c.keyword_a_id === keywordAId && c.keyword_b_id === keywordBId) ||
            (c.keyword_a_id === keywordBId && c.keyword_b_id === keywordAId)
          ),
      );
      this.synonymSuccess = "Synonym-Vorschlag ignoriert";
      window.dispatchEvent(new CustomEvent("keywords-changed"));
    } catch (e) {
      this.synonymsError = String(e);
      console.error("Failed to dismiss synonym pair:", e);
    }
  }

  // Manual merge search
  async searchKeepKeywords(query: string): Promise<void> {
    this.keepSearchInput = query;
    if (!query.trim()) {
      this.keepSearchResults = [];
      return;
    }
    try {
      this.keepSearchResults = await invoke<Keyword[]>("search_keywords", {
        query,
        limit: 10,
      });
    } catch (e) {
      console.error("Failed to search keywords:", e);
    }
  }

  selectKeepKeyword(keyword: Keyword): void {
    this.selectedKeepKeyword = keyword;
    this.keepSearchInput = keyword.name;
    this.keepSearchResults = [];
  }

  clearKeepSearch(): void {
    this.keepSearchInput = "";
    this.keepSearchResults = [];
    this.selectedKeepKeyword = null;
  }

  async searchRemoveKeywords(query: string): Promise<void> {
    this.removeSearchInput = query;
    if (!query.trim()) {
      this.removeSearchResults = [];
      return;
    }
    try {
      this.removeSearchResults = await invoke<Keyword[]>("search_keywords", {
        query,
        limit: 10,
      });
    } catch (e) {
      console.error("Failed to search keywords:", e);
    }
  }

  selectRemoveKeyword(keyword: Keyword): void {
    this.selectedRemoveKeyword = keyword;
    this.removeSearchInput = keyword.name;
    this.removeSearchResults = [];
  }

  clearRemoveSearch(): void {
    this.removeSearchInput = "";
    this.removeSearchResults = [];
    this.selectedRemoveKeyword = null;
  }

  async executeManualMerge(): Promise<void> {
    if (!this.selectedKeepKeyword || !this.selectedRemoveKeyword) return;
    if (this.selectedKeepKeyword.id === this.selectedRemoveKeyword.id) return;

    await this.mergeKeywords(
      this.selectedKeepKeyword.id,
      this.selectedRemoveKeyword.id,
      this.selectedKeepKeyword.name,
      this.selectedRemoveKeyword.name,
    );

    this.clearKeepSearch();
    this.clearRemoveSearch();
  }

  // Create keyword
  async createNewKeyword(): Promise<void> {
    if (!this.newKeywordInput.trim()) return;

    this.createKeywordLoading = true;
    this.createKeywordError = null;
    this.createKeywordSuccess = null;

    try {
      const result = await invoke<CreateKeywordResult>("create_keyword", {
        name: this.newKeywordInput.trim(),
      });

      if (result.created) {
        this.createKeywordSuccess = `"${result.name}" erstellt`;
      } else {
        this.createKeywordSuccess = `"${result.name}" existiert bereits`;
      }
      this.newKeywordInput = "";
      await this.loadKeywords(true);
      await this.loadNetworkStats();
      window.dispatchEvent(new CustomEvent("keywords-changed"));
    } catch (e) {
      this.createKeywordError = String(e);
      console.error("Failed to create keyword:", e);
    } finally {
      this.createKeywordLoading = false;
    }
  }

  // === Refresh & Events ===

  async refreshAll(): Promise<void> {
    await Promise.all([
      this.loadKeywords(true),
      this.loadTrendingKeywords(),
      this.loadNetworkStats(),
    ]);
  }

  setupEventListeners(): void {
    if (this._listenersActive) return;
    this._boundRefreshAll = () => {
      this.refreshAll();
    };
    window.addEventListener("batch-complete", this._boundRefreshAll);
    window.addEventListener("keywords-changed", this._boundRefreshAll);
    this._listenersActive = true;
  }

  teardownEventListeners(): void {
    if (!this._listenersActive || !this._boundRefreshAll) return;
    window.removeEventListener("batch-complete", this._boundRefreshAll);
    window.removeEventListener("keywords-changed", this._boundRefreshAll);
    this._boundRefreshAll = null;
    this._listenersActive = false;
  }

  reset(): void {
    this.keywords = [];
    this.selectedKeyword = null;
    this.neighbors = [];
    this.keywordCategories = [];
    this.trendingKeywords = [];
    this.searchResults = [];
    this.searchQuery = "";
    this.graphData = null;
    this.graphLoading = false;
    this.offset = 0;
    this.hasMore = true;
    this.error = null;
    this.keywordArticles = [];
    this.cooccurringKeywords = [];
    this.similarKeywords = [];
    this.articlesOffset = 0;
    this.hasMoreArticles = true;
    this.synonymCandidates = [];
    this.synonymsError = null;
    this.synonymSuccess = null;
  }
}

export const networkStore = new ImmanentizeNetworkStore();
