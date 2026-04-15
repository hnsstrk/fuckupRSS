import { invoke } from "@tauri-apps/api/core";
import { createLogger } from "../logger";
import { formatError } from "../utils/formatError";

const log = createLogger("modelDownloadStore");

export interface PullModelResult {
  success: boolean;
  error: string | null;
}

export class ModelDownloadStore {
  downloadingModel = $state<string | null>(null);
  downloadError = $state<string | null>(null);

  private pullPromise: Promise<PullModelResult> | null = null;

  pullModel(model: string): Promise<PullModelResult> {
    if (this.pullPromise) {
      return this.pullPromise;
    }

    this.downloadingModel = model;
    this.downloadError = null;

    this.pullPromise = (async () => {
      try {
        const result = await invoke<PullModelResult>("pull_model", { model });
        if (!result.success) {
          this.downloadError = result.error || "Unknown error";
        }
        return result;
      } catch (e) {
        log.error("Failed to pull model:", e);
        this.downloadError = formatError(e);
        return { success: false, error: this.downloadError };
      } finally {
        this.downloadingModel = null;
        this.pullPromise = null;
      }
    })();

    return this.pullPromise;
  }

  clearDownloadError(): void {
    this.downloadError = null;
  }
}

export const modelDownloadStore = new ModelDownloadStore();
