import { invoke } from "@tauri-apps/api/core";

// ============================================================
// TOAST NOTIFICATIONS
// ============================================================

export interface Toast {
  id: number;
  type: 'success' | 'error' | 'info';
  message: string;
}

let toastId = 0;

class ToastStore {
  items = $state<Toast[]>([]);

  add(type: Toast['type'], message: string, duration = 4000): void {
    const id = ++toastId;
    this.items = [...this.items, { id, type, message }];

    // Auto-remove after duration
    if (duration > 0) {
      setTimeout(() => {
        this.remove(id);
      }, duration);
    }
  }

  remove(id: number): void {
    this.items = this.items.filter(t => t.id !== id);
  }

  success(message: string, duration = 4000): void {
    this.add('success', message, duration);
  }

  error(message: string, duration = 6000): void {
    this.add('error', message, duration);
  }

  info(message: string, duration = 4000): void {
    this.add('info', message, duration);
  }
}

export const toasts = new ToastStore();

export function removeToast(id: number): void {
  toasts.remove(id);
}

// ============================================================
// Types matching Rust structs
// ============================================================
export interface Pentacle {
  id: number;
  url: string;
  title: string | null;
  description: string | null;
  site_url: string | null;
  icon_url: string | null;
  default_quality: number;
  article_count: number;
  unread_count: number;
}

export interface Fnord {
  id: number;
  pentacle_id: number;
  pentacle_title: string | null;
  guid: string;
  url: string;
  title: string;
  author: string | null;
  content_raw: string | null;
  content_full: string | null;
  summary: string | null;
  image_url: string | null;
  published_at: string | null;
  processed_at: string | null;
  status: "concealed" | "illuminated" | "golden_apple";
  political_bias: number | null;
  sachlichkeit: number | null;
  quality_score: number | null;
  article_type: string | null;
  has_changes: boolean;
  changed_at: string | null;
  revision_count: number;
}

export interface FnordRevision {
  id: number;
  fnord_id: number;
  title: string;
  author: string | null;
  content_raw: string | null;
  summary: string | null;
  content_hash: string;
  revision_at: string;
}

export interface FnordFilter {
  pentacle_id?: number;
  status?: string;
  limit?: number;
}

export interface SyncResponse {
  success: boolean;
  results: SyncResultResponse[];
  total_new: number;
  total_updated: number;
}

export interface SyncResultResponse {
  pentacle_id: number;
  pentacle_title: string | null;
  new_articles: number;
  updated_articles: number;
  error: string | null;
}

export interface RetrievalResponse {
  fnord_id: number;
  success: boolean;
  content: string | null;
  error: string | null;
}

export interface OllamaStatus {
  available: boolean;
  models: string[];
  recommended_main: string;
  recommended_embedding: string;
  has_recommended_main: boolean;
  has_recommended_embedding: boolean;
}

export interface SummaryResponse {
  fnord_id: number;
  success: boolean;
  summary: string | null;
  error: string | null;
}

export interface BiasAnalysis {
  political_bias: number;
  sachlichkeit: number;
  article_type: string;
}

export interface AnalysisResponse {
  fnord_id: number;
  success: boolean;
  analysis: BiasAnalysis | null;
  error: string | null;
}

export interface UnprocessedCount {
  total: number;
  with_content: number;
}

export interface BatchProgress {
  current: number;
  total: number;
  fnord_id: number;
  title: string;
  success: boolean;
  error: string | null;
}

export interface BatchResult {
  processed: number;
  succeeded: number;
  failed: number;
}

// ============================================================
// SEPHIROTH (Categories) & IMMANENTIZE (Tags)
// ============================================================

export interface Sephiroth {
  id: number;
  name: string;
  description: string | null;
  color: string | null;
  icon: string | null;
  article_count: number;
}

export interface ArticleCategory {
  sephiroth_id: number;
  name: string;
  icon: string | null;
  color: string | null;
  confidence: number;
  source: 'ai' | 'manual';       // Quelle der Zuweisung
  assigned_at: string | null;    // Zeitpunkt der Zuweisung
}

export interface Tag {
  id: number;
  name: string;
  count: number;
  last_used: string | null;
}

export interface DiscordianAnalysis {
  summary: string;
  categories: string[];
  keywords: string[];
  political_bias: number;
  sachlichkeit: number;
  article_type: string;
}

export interface DiscordianResponse {
  fnord_id: number;
  success: boolean;
  analysis: DiscordianAnalysis | null;
  categories_saved: string[];
  tags_saved: string[];
  error: string | null;
}

// ============================================================
// IMMANENTIZE NETWORK (Semantic Keyword Network)
// ============================================================

export interface Keyword {
  id: number;
  name: string;
  count: number;
  article_count: number;
  cluster_id: number | null;
  is_canonical: boolean;
  canonical_id: number | null;
  first_seen: string | null;
  last_used: string | null;
}

export interface KeywordNeighbor {
  id: number;
  name: string;
  cooccurrence: number;
  embedding_similarity: number | null;
  combined_weight: number;
}

export interface KeywordCategory {
  sephiroth_id: number;
  name: string;
  icon: string | null;
  color: string | null;
  weight: number;
  article_count: number;
}

export interface TrendingKeyword {
  id: number;
  name: string;
  total_count: number;
  recent_count: number;
  growth_rate: number;
}

export interface NetworkStats {
  total_keywords: number;
  total_connections: number;
  total_clusters: number;
  avg_neighbors_per_keyword: number;
}

// Svelte 5 runes-based state
class AppState {
  pentacles = $state<Pentacle[]>([]);
  fnords = $state<Fnord[]>([]);
  selectedPentacleId = $state<number | null>(null);
  selectedFnordId = $state<number | null>(null);
  loading = $state(false);
  syncing = $state(false);
  retrieving = $state(false);
  analyzing = $state(false);
  error = $state<string | null>(null);
  lastSyncResult = $state<SyncResponse | null>(null);
  ollamaStatus = $state<OllamaStatus>({
    available: false,
    models: [],
    recommended_main: '',
    recommended_embedding: '',
    has_recommended_main: false,
    has_recommended_embedding: false
  });
  selectedModel = $state<string | null>(null);
  changedFnords = $state<Fnord[]>([]);
  selectedView = $state<"all" | "changed" | "pentacle">("all");

  // Batch processing state
  batchProcessing = $state(false);
  batchProgress = $state<BatchProgress | null>(null);
  unprocessedCount = $state<UnprocessedCount>({ total: 0, with_content: 0 });

  get selectedPentacle(): Pentacle | undefined {
    return this.pentacles.find((p) => p.id === this.selectedPentacleId);
  }

  get selectedFnord(): Fnord | undefined {
    return this.fnords.find((f) => f.id === this.selectedFnordId);
  }

  get totalUnread(): number {
    return this.pentacles.reduce((sum, p) => sum + p.unread_count, 0);
  }

  get changedCount(): number {
    return this.changedFnords.length;
  }

  async loadPentacles(): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      this.pentacles = await invoke<Pentacle[]>("get_pentacles");
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load pentacles:", e);
    } finally {
      this.loading = false;
    }
  }

  async loadFnords(filter?: FnordFilter): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      this.fnords = await invoke<Fnord[]>("get_fnords", { filter });
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load fnords:", e);
    } finally {
      this.loading = false;
    }
  }

  async addPentacle(url: string, title?: string): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      const pentacle = await invoke<Pentacle>("add_pentacle", { url, title });
      this.pentacles = [...this.pentacles, pentacle];
    } catch (e) {
      this.error = String(e);
      console.error("Failed to add pentacle:", e);
    } finally {
      this.loading = false;
    }
  }

  async deletePentacle(id: number): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      await invoke("delete_pentacle", { id });
      this.pentacles = this.pentacles.filter((p) => p.id !== id);
      if (this.selectedPentacleId === id) {
        this.selectedPentacleId = null;
      }
    } catch (e) {
      this.error = String(e);
      console.error("Failed to delete pentacle:", e);
    } finally {
      this.loading = false;
    }
  }

  async updateFnordStatus(
    id: number,
    status: "concealed" | "illuminated" | "golden_apple"
  ): Promise<void> {
    try {
      await invoke("update_fnord_status", { id, status });
      // Update local state
      const fnord = this.fnords.find((f) => f.id === id);
      if (fnord) {
        fnord.status = status;
      }
      // Reload pentacles to update counts
      await this.loadPentacles();
    } catch (e) {
      this.error = String(e);
      console.error("Failed to update fnord status:", e);
    }
  }

  selectPentacle(id: number | null): void {
    this.selectedPentacleId = id;
    this.selectedFnordId = null;
    if (id !== null) {
      this.loadFnords({ pentacle_id: id });
    } else {
      this.loadFnords();
    }
  }

  selectFnord(id: number | null): void {
    this.selectedFnordId = id;
    // Auto-mark as read when selecting
    if (id !== null) {
      const fnord = this.fnords.find((f) => f.id === id);
      if (fnord && fnord.status === "concealed") {
        this.updateFnordStatus(id, "illuminated");
      }
    }
  }

  selectNextFnord(): void {
    if (this.fnords.length === 0) return;

    const currentIndex = this.fnords.findIndex(
      (f) => f.id === this.selectedFnordId
    );
    const nextIndex =
      currentIndex < this.fnords.length - 1 ? currentIndex + 1 : 0;
    this.selectFnord(this.fnords[nextIndex].id);
  }

  selectPrevFnord(): void {
    if (this.fnords.length === 0) return;

    const currentIndex = this.fnords.findIndex(
      (f) => f.id === this.selectedFnordId
    );
    const prevIndex =
      currentIndex > 0 ? currentIndex - 1 : this.fnords.length - 1;
    this.selectFnord(this.fnords[prevIndex].id);
  }

  toggleGoldenApple(id: number): void {
    const fnord = this.fnords.find((f) => f.id === id);
    if (!fnord) return;

    const newStatus =
      fnord.status === "golden_apple" ? "illuminated" : "golden_apple";
    this.updateFnordStatus(id, newStatus);
  }

  async syncAllFeeds(): Promise<SyncResponse | null> {
    if (this.syncing) return null;

    try {
      this.syncing = true;
      this.error = null;
      const result = await invoke<SyncResponse>("sync_all_feeds");
      this.lastSyncResult = result;

      // Reload data after sync
      await this.loadPentacles();
      await this.loadFnords(
        this.selectedPentacleId ? { pentacle_id: this.selectedPentacleId } : undefined
      );

      return result;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to sync feeds:", e);
      return null;
    } finally {
      this.syncing = false;
    }
  }

  async syncFeed(pentacleId: number): Promise<void> {
    try {
      this.syncing = true;
      this.error = null;
      await invoke("sync_feed", { pentacleId });

      // Reload data after sync
      await this.loadPentacles();
      if (this.selectedPentacleId === pentacleId || this.selectedPentacleId === null) {
        await this.loadFnords(
          this.selectedPentacleId ? { pentacle_id: this.selectedPentacleId } : undefined
        );
      }
    } catch (e) {
      this.error = String(e);
      console.error("Failed to sync feed:", e);
    } finally {
      this.syncing = false;
    }
  }

  // Hagbard's Retrieval - Full-text fetching
  async fetchFullContent(fnordId: number): Promise<RetrievalResponse | null> {
    if (this.retrieving) return null;

    try {
      this.retrieving = true;
      this.error = null;
      const result = await invoke<RetrievalResponse>("fetch_full_content", { fnordId });

      // Update local state if successful
      if (result.success && result.content) {
        const fnord = this.fnords.find((f) => f.id === fnordId);
        if (fnord) {
          fnord.content_full = result.content;
        }
      }

      return result;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to fetch full content:", e);
      return null;
    } finally {
      this.retrieving = false;
    }
  }

  async fetchTruncatedArticles(pentacleId?: number): Promise<RetrievalResponse[]> {
    if (this.retrieving) return [];

    try {
      this.retrieving = true;
      this.error = null;
      const results = await invoke<RetrievalResponse[]>("fetch_truncated_articles", {
        pentacleId: pentacleId ?? null,
      });

      // Update local state for successful fetches
      for (const result of results) {
        if (result.success && result.content) {
          const fnord = this.fnords.find((f) => f.id === result.fnord_id);
          if (fnord) {
            fnord.content_full = result.content;
          }
        }
      }

      return results;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to fetch truncated articles:", e);
      return [];
    } finally {
      this.retrieving = false;
    }
  }

  // Ollama AI Integration
  async checkOllama(): Promise<OllamaStatus> {
    try {
      const status = await invoke<OllamaStatus>("check_ollama");
      this.ollamaStatus = status;

      if (status.available && status.models.length > 0 && !this.selectedModel) {
        // Try to load saved model preference
        try {
          const savedModel = await invoke<string | null>("get_setting", { key: "main_model" });
          if (savedModel && status.models.includes(savedModel)) {
            this.selectedModel = savedModel;
          } else {
            // Fall back to recommended model if available, otherwise first model
            this.selectedModel = status.has_recommended_main
              ? status.recommended_main
              : status.models[0];
          }
        } catch {
          // Fall back to recommended model if available, otherwise first model
          this.selectedModel = status.has_recommended_main
            ? status.recommended_main
            : status.models[0];
        }
      }
      return status;
    } catch (e) {
      console.error("Failed to check Ollama:", e);
      this.ollamaStatus = {
        available: false,
        models: [],
        recommended_main: '',
        recommended_embedding: '',
        has_recommended_main: false,
        has_recommended_embedding: false
      };
      return this.ollamaStatus;
    }
  }

  async generateSummary(fnordId: number): Promise<SummaryResponse | null> {
    if (this.analyzing || !this.selectedModel) return null;

    try {
      this.analyzing = true;
      this.error = null;
      const result = await invoke<SummaryResponse>("generate_summary", {
        fnordId,
        model: this.selectedModel,
      });

      // Update local state if successful
      if (result.success && result.summary) {
        const fnord = this.fnords.find((f) => f.id === fnordId);
        if (fnord) {
          fnord.summary = result.summary;
          fnord.processed_at = new Date().toISOString();
        }
      }

      return result;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to generate summary:", e);
      return null;
    } finally {
      this.analyzing = false;
    }
  }

  async analyzeArticle(fnordId: number): Promise<AnalysisResponse | null> {
    if (this.analyzing || !this.selectedModel) return null;

    try {
      this.analyzing = true;
      this.error = null;
      const result = await invoke<AnalysisResponse>("analyze_article", {
        fnordId,
        model: this.selectedModel,
      });

      // Update local state if successful
      if (result.success && result.analysis) {
        const fnord = this.fnords.find((f) => f.id === fnordId);
        if (fnord) {
          fnord.political_bias = result.analysis.political_bias;
          fnord.sachlichkeit = result.analysis.sachlichkeit;
          fnord.article_type = result.analysis.article_type;
        }
      }

      return result;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to analyze article:", e);
      return null;
    } finally {
      this.analyzing = false;
    }
  }

  async processArticle(fnordId: number): Promise<void> {
    await this.generateSummary(fnordId);
    await this.analyzeArticle(fnordId);
  }

  // Changed articles (Fnord view)
  async loadChangedFnords(): Promise<void> {
    try {
      this.loading = true;
      this.error = null;
      this.changedFnords = await invoke<Fnord[]>("get_changed_fnords");
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load changed fnords:", e);
    } finally {
      this.loading = false;
    }
  }

  async acknowledgeChanges(id: number): Promise<void> {
    try {
      await invoke("acknowledge_changes", { id });
      // Update local state
      const fnord = this.fnords.find((f) => f.id === id);
      if (fnord) {
        fnord.has_changes = false;
      }
      // Remove from changedFnords list
      this.changedFnords = this.changedFnords.filter((f) => f.id !== id);
    } catch (e) {
      this.error = String(e);
      console.error("Failed to acknowledge changes:", e);
    }
  }

  async resetAllChanges(): Promise<number> {
    try {
      const count = await invoke<number>("reset_all_changes");
      // Reset local state
      this.changedFnords = [];
      this.fnords.forEach((f) => {
        f.has_changes = false;
      });
      return count;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to reset changes:", e);
      return 0;
    }
  }

  async getRevisions(fnordId: number): Promise<FnordRevision[]> {
    try {
      return await invoke<FnordRevision[]>("get_fnord_revisions", { fnordId });
    } catch (e) {
      this.error = String(e);
      console.error("Failed to get revisions:", e);
      return [];
    }
  }

  selectView(view: "all" | "changed" | "pentacle"): void {
    this.selectedView = view;
    this.selectedFnordId = null;

    if (view === "changed") {
      this.selectedPentacleId = null;
      this.loadChangedFnords();
      // Use changedFnords for display
      this.fnords = this.changedFnords;
    } else if (view === "all") {
      this.selectedPentacleId = null;
      this.loadFnords();
    }
    // "pentacle" view is handled by selectPentacle
  }

  // ============================================================
  // BATCH PROCESSING (Fnord Processing)
  // ============================================================

  async loadUnprocessedCount(): Promise<void> {
    try {
      this.unprocessedCount = await invoke<UnprocessedCount>("get_unprocessed_count");
    } catch (e) {
      console.error("Failed to get unprocessed count:", e);
    }
  }

  async startBatchProcessing(limit?: number): Promise<BatchResult | null> {
    if (this.batchProcessing || !this.ollamaStatus.available) return null;

    const model = this.selectedModel || this.ollamaStatus.models[0];
    if (!model) return null;

    this.batchProcessing = true;
    // Set initial progress immediately so UI shows something
    this.batchProgress = {
      current: 0,
      total: this.unprocessedCount.with_content,
      fnord_id: 0,
      title: "Starting...",
      success: true,
      error: null
    };
    this.error = null;

    console.log("Starting batch processing, initial progress:", this.batchProgress);

    try {
      const result = await invoke<BatchResult>("process_batch", {
        model,
        limit: limit ?? 50,
      });

      // Refresh data after batch processing
      await this.loadFnords();
      await this.loadPentacles();
      await this.loadUnprocessedCount();

      return result;
    } catch (e) {
      this.error = String(e);
      console.error("Batch processing failed:", e);
      return null;
    } finally {
      this.batchProcessing = false;
      this.batchProgress = null;
    }
  }

  updateBatchProgress(progress: BatchProgress): void {
    console.log("updateBatchProgress called with:", progress);
    this.batchProgress = { ...progress };  // Create new object to ensure reactivity
    console.log("batchProgress is now:", this.batchProgress);
  }

  async cancelBatch(): Promise<void> {
    try {
      await invoke("cancel_batch");
    } catch (e) {
      console.error("Failed to cancel batch:", e);
    }
  }

  // ============================================================
  // SEPHIROTH (Categories) & IMMANENTIZE (Tags)
  // ============================================================

  async getArticleCategories(fnordId: number): Promise<ArticleCategory[]> {
    try {
      return await invoke<ArticleCategory[]>("get_article_categories", { fnordId });
    } catch (e) {
      console.error("Failed to get article categories:", e);
      return [];
    }
  }

  async getArticleTags(fnordId: number): Promise<Tag[]> {
    try {
      return await invoke<Tag[]>("get_article_tags", { fnordId });
    } catch (e) {
      console.error("Failed to get article tags:", e);
      return [];
    }
  }

  async getAllCategories(): Promise<Sephiroth[]> {
    try {
      return await invoke<Sephiroth[]>("get_all_categories");
    } catch (e) {
      console.error("Failed to get all categories:", e);
      return [];
    }
  }

  async getAllTags(limit?: number): Promise<Tag[]> {
    try {
      return await invoke<Tag[]>("get_all_tags", { limit: limit ?? 100 });
    } catch (e) {
      console.error("Failed to get all tags:", e);
      return [];
    }
  }

  // Full Discordian Analysis - Summary + Bias + Categories + Keywords
  async processArticleDiscordian(fnordId: number): Promise<DiscordianResponse | null> {
    if (this.analyzing || !this.selectedModel) return null;

    try {
      this.analyzing = true;
      this.error = null;
      const result = await invoke<DiscordianResponse>("process_article_discordian", {
        fnordId,
        model: this.selectedModel,
      });

      // Update local fnord state if successful
      if (result.success && result.analysis) {
        const fnord = this.fnords.find((f) => f.id === fnordId);
        if (fnord) {
          fnord.summary = result.analysis.summary;
          fnord.political_bias = result.analysis.political_bias;
          fnord.sachlichkeit = result.analysis.sachlichkeit;
          fnord.article_type = result.analysis.article_type;
          fnord.processed_at = new Date().toISOString();
        }
      }

      return result;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to process article (Discordian):", e);
      return null;
    } finally {
      this.analyzing = false;
    }
  }
}

export const appState = new AppState();

// Export selected state for components
export const selectedPentacle = {
  get current() {
    return appState.selectedPentacle;
  },
};

export const selectedFnord = {
  get current() {
    return appState.selectedFnord;
  },
};

// ============================================================
// IMMANENTIZE NETWORK STORE
// ============================================================

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

  // Pagination
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

      // Load keyword details, neighbors, and categories in parallel
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

  // Navigate to a neighbor keyword
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
    this.offset = 0;
    this.hasMore = true;
    this.error = null;
  }
}

export const networkStore = new ImmanentizeNetworkStore();
