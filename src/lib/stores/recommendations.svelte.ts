import { invoke } from "@tauri-apps/api/core";
import { createLogger } from "../logger";
import { formatError } from "../utils/formatError";
import type {
  Recommendation,
  RecommendationLoadState,
  RecommendationPhase,
  RecommendationStats,
} from "../types";

const log = createLogger("recommendationsStore");

const TIMEOUT_MS = 30000;
const PHASE_INTERVAL_MS = 2000;

export const recommendationPhases: RecommendationPhase[] = [
  "init",
  "loading_profile",
  "generating_candidates",
  "scoring",
  "finalizing",
];

export function parseRecommendationError(message: string): { code: string; retryable: boolean } {
  if (message.includes("database is locked")) {
    return { code: "DB_LOCKED", retryable: true };
  }
  if (message.includes("no such table")) {
    return { code: "SCHEMA_ERROR", retryable: false };
  }
  if (message.includes("connection")) {
    return { code: "CONNECTION_ERROR", retryable: true };
  }
  return { code: "UNKNOWN", retryable: true };
}

export function getRecommendationEmptyReason(stats: RecommendationStats | null): string {
  if (!stats) return "no_stats";
  if (stats.articles_read < 5) return "not_enough_articles";
  if (stats.articles_with_embedding === 0) return "no_embeddings";
  if (stats.candidate_pool_size === 0) return "no_candidates";
  return "no_matches";
}

function generateRequestId(): string {
  return `rec-${Date.now()}-${Math.random().toString(36).substring(2, 8)}`;
}

export class RecommendationStore {
  loadState = $state<RecommendationLoadState>({ status: "idle" });
  recommendations = $state<Recommendation[]>([]);
  stats = $state<RecommendationStats | null>(null);
  requestId = $state<string | null>(null);
  loadTimingMs = $state<number | null>(null);

  private timeoutHandle: ReturnType<typeof setTimeout> | null = null;
  private phaseHandle: ReturnType<typeof setInterval> | null = null;
  private loadPromise: Promise<void> | null = null;

  ensureLoaded(): Promise<void> {
    if (
      this.loadState.status === "loading" ||
      this.loadState.status === "success" ||
      this.loadState.status === "empty"
    ) {
      return this.loadPromise ?? Promise.resolve();
    }

    return this.loadRecommendations();
  }

  async loadRecommendations(force = false): Promise<void> {
    if (this.loadPromise && !force) {
      return this.loadPromise;
    }

    this.cleanupTimers();

    const reqId = generateRequestId();
    this.requestId = reqId;

    const startTime = Date.now();
    let currentPhaseIndex = 0;

    this.loadState = {
      status: "loading",
      phase: recommendationPhases[0],
      startedAt: startTime,
    };

    this.phaseHandle = setInterval(() => {
      if (
        this.loadState.status === "loading" &&
        currentPhaseIndex < recommendationPhases.length - 1
      ) {
        currentPhaseIndex++;
        this.loadState = {
          status: "loading",
          phase: recommendationPhases[currentPhaseIndex],
          startedAt: startTime,
        };
      }
    }, PHASE_INTERVAL_MS);

    const timeoutPromise = new Promise<never>((_, reject) => {
      this.timeoutHandle = setTimeout(() => {
        reject(new Error("TIMEOUT"));
      }, TIMEOUT_MS);
    });

    this.loadPromise = (async () => {
      try {
        const result = await Promise.race([
          invoke<Recommendation[]>("get_recommendations", { limit: 10 }),
          timeoutPromise,
        ]);

        this.cleanupTimers();
        this.loadTimingMs = Date.now() - startTime;

        if (result.length === 0) {
          await this.loadStats();
          this.loadState = {
            status: "empty",
            stats: this.stats,
            reason: getRecommendationEmptyReason(this.stats),
          };
        } else {
          this.recommendations = result;
          this.loadState = {
            status: "success",
            recommendations: result,
            loadedAt: Date.now(),
          };
          void this.loadStats();
        }

        log.warn(`[${reqId}] Recommendations loaded in ${this.loadTimingMs}ms:`, {
          count: result.length,
          phase: "complete",
        });
      } catch (e) {
        this.cleanupTimers();
        this.loadTimingMs = Date.now() - startTime;

        const errorMessage = formatError(e);

        if (errorMessage === "TIMEOUT") {
          this.loadState = {
            status: "timeout",
            elapsedMs: TIMEOUT_MS,
          };
          log.error(`[${reqId}] Recommendation request timed out after ${TIMEOUT_MS}ms`);
        } else {
          const { code, retryable } = parseRecommendationError(errorMessage);
          this.loadState = {
            status: "error",
            code,
            message: errorMessage,
            retryable,
          };
          log.error(`[${reqId}] Failed to load recommendations:`, e);
        }
      } finally {
        this.loadPromise = null;
      }
    })();

    return this.loadPromise;
  }

  async loadStats(): Promise<void> {
    try {
      this.stats = await invoke<RecommendationStats>("get_recommendation_stats");
    } catch (e) {
      log.error("Failed to load recommendation stats:", e);
    }
  }

  cancel(): void {
    this.cleanupTimers();
    this.loadState = { status: "cancelled" };
    log.warn(`[${this.requestId}] Request cancelled by user`);
  }

  async save(fnordId: number): Promise<void> {
    try {
      await invoke("save_article", { fnordId });
      this.recommendations = this.recommendations.map((recommendation) =>
        recommendation.fnord_id === fnordId
          ? { ...recommendation, is_saved: true }
          : recommendation,
      );
      void this.loadStats();
    } catch (e) {
      log.error("Failed to save article:", e);
    }
  }

  async unsave(fnordId: number): Promise<void> {
    try {
      await invoke("unsave_article", { fnordId });
      this.recommendations = this.recommendations.map((recommendation) =>
        recommendation.fnord_id === fnordId
          ? { ...recommendation, is_saved: false }
          : recommendation,
      );
      void this.loadStats();
    } catch (e) {
      log.error("Failed to unsave article:", e);
    }
  }

  private cleanupTimers(): void {
    if (this.timeoutHandle) {
      clearTimeout(this.timeoutHandle);
      this.timeoutHandle = null;
    }

    if (this.phaseHandle) {
      clearInterval(this.phaseHandle);
      this.phaseHandle = null;
    }
  }
}

export const recommendationStore = new RecommendationStore();
