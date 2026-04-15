import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ModelDownloadStore } from "../../stores/model-downloads.svelte";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("ModelDownloadStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("keeps the download state alive while pull_model is pending", async () => {
    let resolvePull: (value: unknown) => void = () => {};
    const pullPromise = new Promise<unknown>((resolve) => {
      resolvePull = resolve;
    });

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "pull_model") {
        return pullPromise as Promise<unknown>;
      }
      return Promise.resolve(undefined);
    });

    const store = new ModelDownloadStore();
    const pending = store.pullModel("ministral-3:latest");

    expect(store.downloadingModel).toBe("ministral-3:latest");
    expect(store.downloadError).toBeNull();

    resolvePull({ success: true, error: null });
    await expect(pending).resolves.toEqual({ success: true, error: null });

    expect(store.downloadingModel).toBeNull();
    expect(store.downloadError).toBeNull();
  });

  it("records backend failures and clears the pending state", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ success: false, error: "pull failed" });

    const store = new ModelDownloadStore();
    const result = await store.pullModel("ministral-3:latest");

    expect(result).toEqual({ success: false, error: "pull failed" });
    expect(store.downloadingModel).toBeNull();
    expect(store.downloadError).toBe("pull failed");
  });

  it("deduplicates concurrent pull attempts", async () => {
    let resolvePull: (value: unknown) => void = () => {};
    const pullPromise = new Promise<unknown>((resolve) => {
      resolvePull = resolve;
    });

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "pull_model") {
        return pullPromise as Promise<unknown>;
      }
      return Promise.resolve(undefined);
    });

    const store = new ModelDownloadStore();
    const first = store.pullModel("ministral-3:latest");
    const second = store.pullModel("qwen3:8b");

    expect(invoke).toHaveBeenCalledTimes(1);
    expect(store.downloadingModel).toBe("ministral-3:latest");

    resolvePull({ success: true, error: null });
    await expect(first).resolves.toEqual({ success: true, error: null });
    await expect(second).resolves.toEqual({ success: true, error: null });
    expect(store.downloadingModel).toBeNull();
  });
});
