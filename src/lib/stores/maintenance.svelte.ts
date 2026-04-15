import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { appState } from "./state.svelte";
import { settings } from "./settings.svelte";
import { createLogger } from "../logger";
import type { BatchProgress, BatchResult } from "../types";

const log = createLogger("maintenanceStore");

export interface ShortContentStats {
  total_fetched: number;
  content_null_or_empty: number;
  content_under_200: number;
  content_200_to_500: number;
  content_over_500: number;
  by_feed: {
    pentacle_id: number;
    pentacle_title: string;
    short_articles: number;
  }[];
}

export interface RefetchProgress {
  current: number;
  total: number;
  fnord_id: number;
  title: string;
  success: boolean;
  error: string | null;
}

export interface RefetchResponse {
  total_found: number;
  processed: number;
  improved: number;
  unchanged: number;
  failed: number;
}

export interface QualityScoreProgress {
  current: number;
  total: number;
  keyword_name: string;
  score: number | null;
}

export class MaintenanceStore {
  maintenanceRunning = $state<string | null>(null);
  resultMessage = $state<string | null>(null);

  reanalyzeRunning = $state(false);
  reanalyzeProgress = $state<BatchProgress | null>(null);
  private reanalyzeUnlisten: UnlistenFn | null = null;

  statisticalRunning = $state(false);
  statisticalProgress = $state<BatchProgress | null>(null);
  private statisticalUnlisten: UnlistenFn | null = null;

  qualityRunning = $state(false);
  qualityProgress = $state<QualityScoreProgress | null>(null);
  private qualityUnlisten: UnlistenFn | null = null;

  generatingPrototypes = $state(false);

  shortContentAnalyzing = $state(false);
  shortContentStats = $state<ShortContentStats | null>(null);
  shortContentError = $state<string | null>(null);
  shortContentRefetching = $state(false);
  shortContentRefetchResult = $state<RefetchResponse | null>(null);
  shortContentProgress = $state<RefetchProgress | null>(null);
  refetchingFeed = $state<number | null>(null);
  shortContentFeedListExpanded = $state(false);
  private shortContentUnlisten: UnlistenFn | null = null;

  async startReanalyze(resetCount: number, model: string): Promise<void> {
    if (
      this.reanalyzeRunning ||
      (this.maintenanceRunning !== null && this.maintenanceRunning !== "reset")
    ) {
      return;
    }

    this.resultMessage = null;
    this.reanalyzeRunning = true;
    this.maintenanceRunning = "reanalyze";
    this.reanalyzeProgress = {
      current: 0,
      total: resetCount,
      fnord_id: 0,
      title: get(_)("batch.starting"),
      success: true,
      error: null,
    };
    appState.batchProcessing = true;

    try {
      this.reanalyzeUnlisten = await listen<BatchProgress>("batch-progress", (event) => {
        this.reanalyzeProgress = { ...event.payload };
      });

      const batchResult = await invoke<BatchResult>("process_batch", {
        model,
        limit: null,
      });

      this.resultMessage = get(_)("settings.maintenance.reanalyzeComplete", {
        values: {
          succeeded: batchResult.succeeded,
          failed: batchResult.failed,
        },
      });

      await appState.loadFnords();
      await appState.loadPentacles();
      await appState.loadUnprocessedCount();

      window.dispatchEvent(new CustomEvent("batch-complete"));
    } catch (e) {
      this.resultMessage = `Error: ${e}`;
    } finally {
      this.reanalyzeRunning = false;
      this.maintenanceRunning = null;
      appState.batchProcessing = false;
      this.reanalyzeProgress = null;
      if (this.reanalyzeUnlisten) {
        this.reanalyzeUnlisten();
        this.reanalyzeUnlisten = null;
      }
    }
  }

  async cancelReanalyze(): Promise<void> {
    try {
      await invoke("cancel_batch");
      this.resultMessage = get(_)("settings.maintenance.reanalyzeCancelled");
    } catch (e) {
      log.error("Failed to cancel reanalyze:", e);
    }
  }

  async calculateQualityScores(): Promise<void> {
    if (this.qualityRunning || this.maintenanceRunning !== null) {
      return;
    }

    this.resultMessage = null;
    this.maintenanceRunning = "scores";
    this.qualityProgress = null;
    this.qualityRunning = true;

    try {
      this.qualityUnlisten = await listen<QualityScoreProgress>(
        "quality-score-progress",
        (event) => {
          this.qualityProgress = { ...event.payload };
        },
      );

      const result = await invoke<{
        updated_count: number;
        avg_score: number;
        low_quality_count: number;
      }>("calculate_keyword_quality_scores", {});

      if (result.updated_count === 0) {
        this.resultMessage = get(_)("settings.maintenance.noKeywordsToUpdate");
      } else {
        this.resultMessage =
          `${result.updated_count} ${get(_)("settings.maintenance.updated")} ` +
          `(Ø ${result.avg_score.toFixed(2)})`;
      }
    } catch (e) {
      this.resultMessage = `Error: ${e}`;
    } finally {
      this.maintenanceRunning = null;
      this.qualityRunning = false;
      this.qualityProgress = null;
      if (this.qualityUnlisten) {
        this.qualityUnlisten();
        this.qualityUnlisten = null;
      }
    }
  }

  async queueEmbeddings(): Promise<void> {
    if (this.maintenanceRunning !== null) {
      return;
    }

    this.resultMessage = null;
    this.maintenanceRunning = "embeddings";

    try {
      const queuedCount = await invoke<number>("queue_missing_embeddings");
      this.resultMessage = `${queuedCount} ${get(_)("settings.maintenance.queued")}`;
    } catch (e) {
      this.resultMessage = `Error: ${e}`;
    } finally {
      this.maintenanceRunning = null;
    }
  }

  async processStatisticalAnalysis(): Promise<void> {
    if (this.statisticalRunning || this.maintenanceRunning !== null) {
      return;
    }

    this.resultMessage = null;
    this.maintenanceRunning = "statistical";
    this.statisticalProgress = null;

    try {
      const count = await invoke<number>("get_unprocessed_statistical_count");
      if (count === 0) {
        this.resultMessage = get(_)("settings.maintenance.noUnprocessedArticles");
        this.maintenanceRunning = null;
        return;
      }

      this.statisticalRunning = true;
      this.statisticalProgress = {
        current: 0,
        total: count,
        fnord_id: 0,
        title: get(_)("batch.starting"),
        success: true,
        error: null,
      };

      this.statisticalUnlisten = await listen<BatchProgress>("statistical-progress", (event) => {
        this.statisticalProgress = { ...event.payload };
      });

      const result = await invoke<{
        processed: number;
        total: number;
        errors: string[];
      }>("process_statistical_batch", { limit: 10000 });

      this.resultMessage = `${result.processed} ${get(_)("settings.maintenance.articlesAnalyzed")}`;
    } catch (e) {
      this.resultMessage = `Error: ${e}`;
    } finally {
      this.maintenanceRunning = null;
      this.statisticalRunning = false;
      this.statisticalProgress = null;
      if (this.statisticalUnlisten) {
        this.statisticalUnlisten();
        this.statisticalUnlisten = null;
      }
    }
  }

  async generatePrototypes(): Promise<void> {
    if (this.generatingPrototypes || this.maintenanceRunning !== null) {
      return;
    }

    this.resultMessage = null;
    this.generatingPrototypes = true;
    this.maintenanceRunning = "prototypes";

    try {
      const result = await invoke<{
        total: number;
        generated: number;
        errors: number;
      }>("generate_keyword_type_prototypes");

      if (result.errors > 0) {
        this.resultMessage = get(_)("settings.maintenance.prototypesGeneratedWithErrors", {
          values: {
            count: result.generated,
            errors: result.errors,
          },
        });
      } else {
        this.resultMessage = get(_)("settings.maintenance.prototypesGenerated", {
          values: { count: result.generated },
        });
      }
    } catch (e) {
      this.resultMessage = `Error: ${e}`;
    } finally {
      this.generatingPrototypes = false;
      this.maintenanceRunning = null;
    }
  }

  async analyzeShortContent(): Promise<void> {
    if (this.shortContentAnalyzing || this.maintenanceRunning !== null) {
      return;
    }

    this.resultMessage = null;
    this.shortContentAnalyzing = true;
    this.shortContentError = null;
    this.shortContentStats = null;
    this.shortContentRefetchResult = null;
    this.shortContentFeedListExpanded = false;
    this.maintenanceRunning = "shortContentAnalyze";

    try {
      this.shortContentStats = await invoke<ShortContentStats>("get_short_content_stats");
    } catch (e) {
      this.shortContentError = String(e);
    } finally {
      this.shortContentAnalyzing = false;
      this.maintenanceRunning = null;
    }
  }

  async refetchShortContent(): Promise<void> {
    if (
      !settings.enableHeadlessBrowser ||
      this.shortContentRefetching ||
      this.maintenanceRunning !== null
    ) {
      return;
    }

    this.resultMessage = null;
    this.shortContentRefetching = true;
    this.shortContentError = null;
    this.shortContentRefetchResult = null;
    this.shortContentProgress = null;
    this.maintenanceRunning = "shortContentRefetch";

    try {
      this.shortContentUnlisten = await listen<RefetchProgress>("refetch-progress", (event) => {
        this.shortContentProgress = { ...event.payload };
      });

      const result = await invoke<RefetchResponse>("refetch_short_articles", {
        min_content_length: 500,
        limit: 100,
      });
      this.shortContentRefetchResult = result;
      await this.loadShortContentStats();
    } catch (e) {
      this.shortContentError = String(e);
    } finally {
      this.shortContentRefetching = false;
      this.shortContentProgress = null;
      this.maintenanceRunning = null;
      if (this.shortContentUnlisten) {
        this.shortContentUnlisten();
        this.shortContentUnlisten = null;
      }
    }
  }

  async refetchFeedShortContent(pentacleId: number): Promise<void> {
    if (
      !settings.enableHeadlessBrowser ||
      this.shortContentRefetching ||
      this.refetchingFeed !== null ||
      this.maintenanceRunning !== null
    ) {
      return;
    }

    this.resultMessage = null;
    this.refetchingFeed = pentacleId;
    this.shortContentError = null;
    this.shortContentProgress = null;
    this.maintenanceRunning = "shortContentFeedRefetch";

    try {
      this.shortContentUnlisten = await listen<RefetchProgress>("refetch-progress", (event) => {
        this.shortContentProgress = { ...event.payload };
      });

      const result = await invoke<RefetchResponse>("refetch_feed_short_articles", {
        pentacle_id: pentacleId,
        min_content_length: 500,
        limit: 50,
      });
      this.shortContentRefetchResult = result;
      await this.loadShortContentStats();
    } catch (e) {
      this.shortContentError = String(e);
    } finally {
      this.refetchingFeed = null;
      this.shortContentProgress = null;
      this.maintenanceRunning = null;
      if (this.shortContentUnlisten) {
        this.shortContentUnlisten();
        this.shortContentUnlisten = null;
      }
    }
  }

  async deleteNullArticles(): Promise<void> {
    this.shortContentError = null;
    this.resultMessage = null;

    try {
      const result = await invoke<{ deleted_count: number }>("delete_null_content_articles");
      this.resultMessage = get(_)("settings.maintenance.shortContent.deleted", {
        values: { count: result.deleted_count },
      });
      await this.loadShortContentStats();
      await appState.loadFnords();
      await appState.loadPentacles();
    } catch (e) {
      this.shortContentError = String(e);
    }
  }

  async excludeShortFromAi(): Promise<void> {
    this.shortContentError = null;
    this.resultMessage = null;

    try {
      const excluded = await invoke<number>("exclude_short_from_ai", {
        max_length: 200,
      });
      this.resultMessage = get(_)("settings.maintenance.shortContent.excluded", {
        values: { count: excluded },
      });
      await appState.loadUnprocessedCount();
    } catch (e) {
      this.shortContentError = String(e);
    }
  }

  clearResult(): void {
    this.resultMessage = null;
  }

  toggleShortContentFeedList(): void {
    this.shortContentFeedListExpanded = !this.shortContentFeedListExpanded;
  }

  private async loadShortContentStats(): Promise<void> {
    const stats = await invoke<ShortContentStats>("get_short_content_stats");
    this.shortContentStats = stats;
  }
}

export const maintenanceStore = new MaintenanceStore();
