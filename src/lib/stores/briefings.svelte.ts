import { invoke } from "@tauri-apps/api/core";
import { createLogger } from "../logger";
import { formatError } from "../utils/formatError";

const log = createLogger("briefingsStore");

export interface ArticleRef {
  index: number;
  fnord_id: number;
  title: string;
  source: string;
}

export interface BriefingTldr {
  overview: string;
  trends: string;
  conclusion: string;
}

export interface BriefingTopic {
  title: string;
  body: string;
  article_indices: number[];
  keywords: string[];
}

export interface StructuredBriefing {
  tldr: BriefingTldr;
  topics: BriefingTopic[];
}

export interface Briefing {
  id: number;
  period_type: string;
  period_start: string;
  period_end: string;
  content: string;
  top_keywords: string | null;
  article_count: number;
  model_used: string | null;
  created_at: string;
  article_refs: string | null;
}

export class BriefingStore {
  briefings = $state<Briefing[]>([]);
  loading = $state(false);
  generating = $state(false);
  error = $state<string | null>(null);

  private initialized = false;
  private loadPromise: Promise<void> | null = null;
  private generatePromise: Promise<Briefing | null> | null = null;

  async ensureLoaded(): Promise<void> {
    if (this.initialized || this.loading) {
      return this.loadPromise ?? Promise.resolve();
    }

    return this.loadBriefings();
  }

  async loadBriefings(): Promise<void> {
    if (this.loadPromise) {
      return this.loadPromise;
    }

    this.loading = true;
    this.error = null;

    this.loadPromise = (async () => {
      try {
        const fetchedBriefings = await invoke<Briefing[]>("get_briefings", { limit: 20 });
        const fetchedIds = fetchedBriefings.map((briefing) => briefing.id);
        const localOnlyBriefings = this.briefings.filter(
          (briefing) => !fetchedIds.includes(briefing.id),
        );

        // Preserve local entries created after a stale fetch started, e.g. during briefing generation.
        this.briefings = [...localOnlyBriefings, ...fetchedBriefings];
        this.initialized = true;
      } catch (e) {
        log.error("Error loading briefings:", e);
        this.error = formatError(e);
      } finally {
        this.loading = false;
        this.loadPromise = null;
      }
    })();

    return this.loadPromise;
  }

  async generateBriefing(periodType: string): Promise<Briefing | null> {
    if (this.generatePromise) {
      return this.generatePromise;
    }

    this.generating = true;
    this.error = null;

    this.generatePromise = (async () => {
      try {
        const newBriefing = await invoke<Briefing>("generate_briefing", {
          periodType,
        });

        this.briefings = [
          newBriefing,
          ...this.briefings.filter((briefing) => briefing.id !== newBriefing.id),
        ];
        this.initialized = true;
        return newBriefing;
      } catch (e) {
        log.error("Error generating briefing:", e);
        this.error = formatError(e);
        return null;
      } finally {
        this.generating = false;
        this.generatePromise = null;
      }
    })();

    return this.generatePromise;
  }

  async deleteBriefing(id: number): Promise<void> {
    try {
      await invoke("delete_briefing", { id });
      this.briefings = this.briefings.filter((briefing) => briefing.id !== id);
    } catch (e) {
      log.error("Error deleting briefing:", e);
      this.error = formatError(e);
    }
  }
}

export const briefingStore = new BriefingStore();
