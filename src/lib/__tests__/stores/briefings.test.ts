import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { BriefingStore } from "../../stores/briefings.svelte";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("BriefingStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("keeps generating state and applies the new briefing when the async call resolves", async () => {
    let resolveGenerate!: (value: unknown) => void;
    const generatePromise = new Promise((resolve) => {
      resolveGenerate = resolve;
    });

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "generate_briefing") {
        return generatePromise as Promise<unknown>;
      }
      return Promise.resolve([]);
    });

    const store = new BriefingStore();
    const pendingGeneration = store.generateBriefing("daily");

    expect(store.generating).toBe(true);
    expect(store.briefings).toEqual([]);

    resolveGenerate({
      id: 7,
      period_type: "daily",
      period_start: "2026-04-15T00:00:00",
      period_end: "2026-04-15T23:59:59",
      content: "Generated",
      top_keywords: null,
      article_count: 4,
      model_used: "test-model",
      created_at: "2026-04-15T10:00:00",
      article_refs: null,
    });

    await pendingGeneration;

    expect(store.generating).toBe(false);
    expect(store.briefings).toHaveLength(1);
    expect(store.briefings[0].id).toBe(7);
    expect(store.expandedId).toBe(7);
  });

  it("preserves a locally generated briefing when a stale load result arrives later", async () => {
    let resolveLoad!: (value: unknown) => void;
    let resolveGenerate!: (value: unknown) => void;

    const loadPromise = new Promise((resolve) => {
      resolveLoad = resolve;
    });
    const generatePromise = new Promise((resolve) => {
      resolveGenerate = resolve;
    });

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "get_briefings") {
        return loadPromise as Promise<unknown>;
      }
      if (command === "generate_briefing") {
        return generatePromise as Promise<unknown>;
      }
      return Promise.resolve(undefined);
    });

    const store = new BriefingStore();

    const pendingLoad = store.loadBriefings();
    const pendingGeneration = store.generateBriefing("weekly");

    resolveGenerate({
      id: 99,
      period_type: "weekly",
      period_start: "2026-04-08T00:00:00",
      period_end: "2026-04-15T23:59:59",
      content: "Fresh",
      top_keywords: null,
      article_count: 8,
      model_used: "test-model",
      created_at: "2026-04-15T10:00:00",
      article_refs: null,
    });

    await pendingGeneration;

    resolveLoad([
      {
        id: 1,
        period_type: "daily",
        period_start: "2026-04-14T00:00:00",
        period_end: "2026-04-14T23:59:59",
        content: "Older",
        top_keywords: null,
        article_count: 2,
        model_used: "test-model",
        created_at: "2026-04-14T10:00:00",
        article_refs: null,
      },
    ]);

    await pendingLoad;

    expect(store.briefings.map((briefing) => briefing.id)).toEqual([99, 1]);
  });
});
