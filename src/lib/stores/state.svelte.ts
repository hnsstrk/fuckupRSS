import { invoke } from "@tauri-apps/api/core";
import { createLogger } from "../logger";
import type {
  Pentacle,
  Fnord,
  FnordRevision,
  FnordStats,
  FnordFilter,
  SyncResponse,
  RetrievalResponse,
  OllamaStatus,
  SummaryResponse,
  AnalysisResponse,
  UnprocessedCount,
  BatchProgress,
  BatchResult,
  EmbeddingProgress,
  EmbeddingQueueStatus,
  Sephiroth,
  ArticleCategory,
  Tag,
  DiscordianResponse,
  Keyword,
  KeywordNeighbor,
  KeywordCategory,
  TrendingKeyword,
  NetworkStats,
  NetworkGraph,
  MainView,
} from "../types";

export { toasts, removeToast } from "./toast.svelte";

export type {
  Pentacle,
  Fnord,
  FnordRevision,
  FnordStats,
  FnordFilter,
  SyncResponse,
  RetrievalResponse,
  OllamaStatus,
  SummaryResponse,
  AnalysisResponse,
  UnprocessedCount,
  BatchProgress,
  BatchResult,
  EmbeddingProgress,
  EmbeddingQueueStatus,
  Sephiroth,
  ArticleCategory,
  Tag,
  DiscordianResponse,
  Keyword,
  KeywordNeighbor,
  KeywordCategory,
  TrendingKeyword,
  NetworkStats,
  NetworkGraph,
  MainView,
};

const log = createLogger("state");

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

  // Embedding processing state
  embeddingProgress = $state<EmbeddingProgress | null>(null);

  // Lazy loading state
  totalFnordsCount = $state(0);
  loadingMore = $state(false);
  private currentFilter = $state<FnordFilter | undefined>(undefined);

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

  get hasMoreFnords(): boolean {
    return this.fnords.length < this.totalFnordsCount;
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
      this.currentFilter = filter;

      // Load count and first batch in parallel
      const [fnords, count] = await Promise.all([
        invoke<Fnord[]>("get_fnords", { filter: { ...filter, limit: 50, offset: 0 } }),
        invoke<number>("get_fnords_count", { filter })
      ]);

      this.fnords = fnords;
      this.totalFnordsCount = count;
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load fnords:", e);
    } finally {
      this.loading = false;
    }
  }

  async loadMoreFnords(): Promise<void> {
    if (this.loadingMore || !this.hasMoreFnords) return;

    try {
      this.loadingMore = true;
      this.error = null;

      const moreFnords = await invoke<Fnord[]>("get_fnords", {
        filter: {
          ...this.currentFilter,
          limit: 50,
          offset: this.fnords.length
        }
      });

      this.fnords = [...this.fnords, ...moreFnords];
    } catch (e) {
      this.error = String(e);
      console.error("Failed to load more fnords:", e);
    } finally {
      this.loadingMore = false;
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
      log.info("Starting sync of all feeds...");
      const result = await invoke<SyncResponse>("sync_all_feeds");
      this.lastSyncResult = result;
      log.info(`Sync complete: ${result.total_new} new, ${result.total_updated} updated`);

      // Reload data after sync
      await this.loadPentacles();
      await this.loadFnords(
        this.selectedPentacleId ? { pentacle_id: this.selectedPentacleId } : undefined
      );

      return result;
    } catch (e) {
      this.error = String(e);
      log.error("Failed to sync feeds:", e);
      return null;
    } finally {
      this.syncing = false;
    }
  }

  async syncFeed(pentacleId: number): Promise<void> {
    try {
      this.syncing = true;
      this.error = null;
      log.debug(`Syncing feed ${pentacleId}...`);
      await invoke("sync_feed", { pentacleId });
      log.debug(`Feed ${pentacleId} synced`);

      // Reload data after sync
      await this.loadPentacles();
      if (this.selectedPentacleId === pentacleId || this.selectedPentacleId === null) {
        await this.loadFnords(
          this.selectedPentacleId ? { pentacle_id: this.selectedPentacleId } : undefined
        );
      }
    } catch (e) {
      this.error = String(e);
      log.error("Failed to sync feed:", e);
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

  async getFnordStats(): Promise<FnordStats | null> {
    try {
      return await invoke<FnordStats>("get_fnord_stats");
    } catch (e) {
      this.error = String(e);
      console.error("Failed to get fnord stats:", e);
      return null;
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
        limit: limit ?? null,  // null = process all
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

  updateEmbeddingProgress(progress: EmbeddingProgress): void {
    log.debug("Embedding progress update:", progress);
    if (progress.is_processing || progress.queue_size > 0) {
      this.embeddingProgress = { ...progress };
    } else {
      // Clear progress when done
      this.embeddingProgress = null;
    }
  }

  async cancelBatch(): Promise<void> {
    try {
      await invoke("cancel_batch");
    } catch (e) {
      console.error("Failed to cancel batch:", e);
    }
  }

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

export { networkStore } from "./network.svelte";
export { navigationStore } from "./navigation.svelte";
