import { invoke } from "@tauri-apps/api/core";
import type {
  Keyword,
  KeywordNeighbor,
  KeywordCategory,
  TrendingKeyword,
  NetworkStats,
  NetworkGraph,
} from "../types";

class ImmanentizeNetworkStore {
  keywords = $state<Keyword[]>([]);
  selectedKeyword = $state<Keyword | null>(null);
  neighbors = $state<KeywordNeighbor[]>([]);
  keywordCategories = $state<KeywordCategory[]>([]);
  trendingKeywords = $state<TrendingKeyword[]>([]);
  networkStats = $state<NetworkStats | null>(null);
  searchResults = $state<Keyword[]>([]);
  searchQuery = $state('');
  loading = $state(false);
  error = $state<string | null>(null);
  graphData = $state<NetworkGraph | null>(null);
  graphLoading = $state(false);
  offset = $state(0);
  limit = $state(50);
  hasMore = $state(true);

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
      return;
    }

    try {
      this.loading = true;
      this.error = null;

      const [keyword, neighbors, categories] = await Promise.all([
        invoke<Keyword | null>("get_keyword", { id }),
        invoke<KeywordNeighbor[]>("get_keyword_neighbors", { id, limit: 20 }),
        invoke<KeywordCategory[]>("get_keyword_categories", { id }),
      ]);

      this.selectedKeyword = keyword;
      this.neighbors = neighbors;
      this.keywordCategories = categories;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load keyword details:", e);
    } finally {
      this.loading = false;
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

  async loadTrendingKeywords(days = 7): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      this.trendingKeywords = await invoke<TrendingKeyword[]>("get_trending_keywords", {
        days,
        limit: 20,
      });
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load trending keywords:", e);
    } finally {
      this.loading = false;
    }
  }

  async loadNetworkStats(): Promise<void> {
    try {
      this.networkStats = await invoke<NetworkStats>("get_network_stats");
    } catch (e) {
      console.error("Failed to load network stats:", e);
    }
  }

  async loadNetworkGraph(limit = 100, minWeight = 0.1): Promise<void> {
    if (this.graphLoading) return;

    try {
      this.graphLoading = true;
      this.error = null;
      this.graphData = await invoke<NetworkGraph>("get_network_graph", {
        limit,
        minWeight,
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
    this.searchQuery = '';
    this.searchResults = [];
  }

  reset(): void {
    this.keywords = [];
    this.selectedKeyword = null;
    this.neighbors = [];
    this.keywordCategories = [];
    this.trendingKeywords = [];
    this.searchResults = [];
    this.searchQuery = '';
    this.graphData = null;
    this.graphLoading = false;
    this.offset = 0;
    this.hasMore = true;
    this.error = null;
  }
}

export const networkStore = new ImmanentizeNetworkStore();
