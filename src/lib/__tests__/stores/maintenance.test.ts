import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { appState } from "../../stores/state.svelte";
import { settings } from "../../stores/settings.svelte";
import { MaintenanceStore } from "../../stores/maintenance.svelte";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
  emit: vi.fn(),
}));

vi.mock("../../stores/state.svelte", () => ({
  appState: {
    batchProcessing: false,
    selectedModel: "test-model",
    ollamaStatus: {
      available: true,
      models: ["test-model"],
      recommended_main: "test-model",
      recommended_embedding: "embed-model",
      has_recommended_main: true,
      has_recommended_embedding: true,
    },
    loadFnords: vi.fn().mockResolvedValue(undefined),
    loadPentacles: vi.fn().mockResolvedValue(undefined),
    loadUnprocessedCount: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("../../stores/settings.svelte", () => ({
  settings: {
    enableHeadlessBrowser: true,
  },
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

describe("MaintenanceStore", () => {
  let progressHandler: ((event: { payload: unknown }) => void) | null;
  let unlistenMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    progressHandler = null;
    unlistenMock = vi.fn();

    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      progressHandler = handler as (event: { payload: unknown }) => void;
      return unlistenMock as unknown as () => void;
    });

    const appStateMock = appState as typeof appState & {
      batchProcessing: boolean;
      selectedModel: string | null;
      ollamaStatus: typeof appState.ollamaStatus;
      loadFnords: ReturnType<typeof vi.fn>;
      loadPentacles: ReturnType<typeof vi.fn>;
      loadUnprocessedCount: ReturnType<typeof vi.fn>;
    };

    appStateMock.batchProcessing = false;
    appStateMock.selectedModel = "test-model";
    appStateMock.ollamaStatus.available = true;
    appStateMock.loadFnords.mockResolvedValue(undefined);
    appStateMock.loadPentacles.mockResolvedValue(undefined);
    appStateMock.loadUnprocessedCount.mockResolvedValue(undefined);

    (settings as { enableHeadlessBrowser: boolean }).enableHeadlessBrowser = true;
  });

  it("keeps reanalysis progress alive until the batch finishes", async () => {
    const batch = deferred<{ succeeded: number; failed: number }>();
    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "process_batch") {
        return batch.promise as Promise<unknown>;
      }
      return Promise.resolve(undefined);
    });

    const store = new MaintenanceStore();
    const dispatchSpy = vi.spyOn(window, "dispatchEvent");

    const pending = store.startReanalyze(4, "test-model");

    expect(store.maintenanceRunning).toBe("reanalyze");
    expect(store.reanalyzeRunning).toBe(true);
    expect(store.reanalyzeProgress).toMatchObject({ current: 0, total: 4 });

    progressHandler?.({
      payload: {
        current: 2,
        total: 4,
        fnord_id: 11,
        title: "Halfway",
        success: true,
        error: null,
      },
    });

    expect(store.reanalyzeProgress).toMatchObject({ current: 2, total: 4, title: "Halfway" });

    batch.resolve({ succeeded: 3, failed: 1 });
    await pending;

    expect(store.reanalyzeRunning).toBe(false);
    expect(store.maintenanceRunning).toBeNull();
    expect(store.reanalyzeProgress).toBeNull();
    expect(store.resultMessage).toBe("settings.maintenance.reanalyzeComplete");

    expect(vi.mocked(appState.loadFnords)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(appState.loadPentacles)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(appState.loadUnprocessedCount)).toHaveBeenCalledTimes(1);
    expect(unlistenMock).toHaveBeenCalledTimes(1);
    expect(dispatchSpy).toHaveBeenCalledTimes(1);

    dispatchSpy.mockRestore();
  });

  it("keeps short-content refetch progress and result across completion", async () => {
    const refetch = deferred<{
      total_found: number;
      processed: number;
      improved: number;
      unchanged: number;
      failed: number;
    }>();

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "refetch_short_articles") {
        return refetch.promise as Promise<unknown>;
      }
      if (command === "get_short_content_stats") {
        return Promise.resolve({
          total_fetched: 6,
          content_null_or_empty: 1,
          content_under_200: 2,
          content_200_to_500: 1,
          content_over_500: 2,
          by_feed: [],
        });
      }
      return Promise.resolve(undefined);
    });

    const store = new MaintenanceStore();
    const pending = store.refetchShortContent();

    expect(store.maintenanceRunning).toBe("shortContentRefetch");
    expect(store.shortContentRefetching).toBe(true);

    progressHandler?.({
      payload: {
        current: 3,
        total: 6,
        fnord_id: 22,
        title: "Refetching",
        success: true,
        error: null,
      },
    });

    expect(store.shortContentProgress).toMatchObject({ current: 3, total: 6, title: "Refetching" });

    refetch.resolve({ total_found: 6, processed: 6, improved: 4, unchanged: 1, failed: 1 });
    await pending;

    expect(store.shortContentRefetching).toBe(false);
    expect(store.maintenanceRunning).toBeNull();
    expect(store.shortContentProgress).toBeNull();
    expect(store.shortContentRefetchResult).toMatchObject({ improved: 4, failed: 1 });
    expect(store.shortContentStats).toMatchObject({ total_fetched: 6, content_over_500: 2 });
    expect(unlistenMock).toHaveBeenCalledTimes(1);
  });

  it("tracks the active feed during a feed-specific refetch", async () => {
    const refetch = deferred<{
      total_found: number;
      processed: number;
      improved: number;
      unchanged: number;
      failed: number;
    }>();

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "refetch_feed_short_articles") {
        return refetch.promise as Promise<unknown>;
      }
      if (command === "get_short_content_stats") {
        return Promise.resolve({
          total_fetched: 1,
          content_null_or_empty: 0,
          content_under_200: 1,
          content_200_to_500: 0,
          content_over_500: 0,
          by_feed: [],
        });
      }
      return Promise.resolve(undefined);
    });

    const store = new MaintenanceStore();
    const pending = store.refetchFeedShortContent(42);

    expect(store.maintenanceRunning).toBe("shortContentFeedRefetch");
    expect(store.refetchingFeed).toBe(42);

    progressHandler?.({
      payload: {
        current: 1,
        total: 1,
        fnord_id: 42,
        title: "Feed refetch",
        success: true,
        error: null,
      },
    });

    expect(store.shortContentProgress).toMatchObject({
      current: 1,
      total: 1,
      title: "Feed refetch",
    });

    refetch.resolve({ total_found: 1, processed: 1, improved: 1, unchanged: 0, failed: 0 });
    await pending;

    expect(store.refetchingFeed).toBeNull();
    expect(store.maintenanceRunning).toBeNull();
    expect(store.shortContentProgress).toBeNull();
    expect(store.shortContentRefetchResult).toMatchObject({ improved: 1, failed: 0 });
  });

  it("keeps statistical analysis progress alive until the batch finishes", async () => {
    const statistical = deferred<{
      processed: number;
      total: number;
      errors: string[];
    }>();

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "get_unprocessed_statistical_count") {
        return Promise.resolve(5);
      }
      if (command === "process_statistical_batch") {
        return statistical.promise as Promise<unknown>;
      }
      return Promise.resolve(undefined);
    });

    const store = new MaintenanceStore();
    const pending = store.processStatisticalAnalysis();
    await Promise.resolve();
    await Promise.resolve();

    expect(store.maintenanceRunning).toBe("statistical");
    expect(store.statisticalRunning).toBe(true);
    expect(store.statisticalProgress).toMatchObject({ current: 0, total: 5 });

    progressHandler?.({
      payload: {
        current: 4,
        total: 5,
        fnord_id: 9,
        title: "Almost done",
        success: true,
        error: null,
      },
    });

    expect(store.statisticalProgress).toMatchObject({ current: 4, total: 5, title: "Almost done" });

    statistical.resolve({ processed: 5, total: 5, errors: [] });
    await pending;

    expect(store.statisticalRunning).toBe(false);
    expect(store.maintenanceRunning).toBeNull();
    expect(store.statisticalProgress).toBeNull();
    expect(store.resultMessage).toContain("settings.maintenance.articlesAnalyzed");
    expect(unlistenMock).toHaveBeenCalledTimes(1);
  });
});
