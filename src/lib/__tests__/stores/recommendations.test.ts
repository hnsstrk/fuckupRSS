import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  RecommendationStore,
  getRecommendationEmptyReason,
  parseRecommendationError,
} from "../../stores/recommendations.svelte";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("Recommendation helpers", () => {
  it("classifies retryable and non-retryable recommendation errors", () => {
    expect(parseRecommendationError("database is locked")).toEqual({
      code: "DB_LOCKED",
      retryable: true,
    });
    expect(parseRecommendationError("no such table: recommendations")).toEqual({
      code: "SCHEMA_ERROR",
      retryable: false,
    });
  });

  it("detects the empty-state reason from stats", () => {
    expect(getRecommendationEmptyReason(null)).toBe("no_stats");
    expect(
      getRecommendationEmptyReason({
        total_saved: 0,
        total_hidden: 0,
        total_clicks: 0,
        articles_read: 1,
        articles_with_embedding: 10,
        profile_strength: "Cold",
        top_keywords: [],
        top_categories: [],
        candidate_pool_size: 10,
      }),
    ).toBe("not_enough_articles");
  });
});

describe("RecommendationStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("keeps loading state and applies recommendations when the request resolves", async () => {
    let resolveRecommendations!: (value: unknown) => void;

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "get_recommendations") {
        return new Promise((resolve) => {
          resolveRecommendations = resolve;
        }) as Promise<unknown>;
      }
      if (command === "get_recommendation_stats") {
        return Promise.resolve({
          total_saved: 2,
          total_hidden: 0,
          total_clicks: 0,
          articles_read: 12,
          articles_with_embedding: 12,
          profile_strength: "Warm",
          top_keywords: [],
          top_categories: [],
          candidate_pool_size: 8,
        });
      }
      return Promise.resolve(undefined);
    });

    const store = new RecommendationStore();
    const pendingLoad = store.loadRecommendations();

    expect(store.loadState.status).toBe("loading");

    resolveRecommendations([
      {
        fnord_id: 11,
        title: "Recommendation",
        summary: null,
        url: "https://example.com",
        image_url: null,
        pentacle_id: 1,
        pentacle_title: "Feed",
        pentacle_icon: null,
        published_at: "2026-04-15T10:00:00Z",
        relevance_score: 0.8,
        freshness_score: 0.7,
        political_bias: null,
        sachlichkeit: null,
        categories: [],
        matching_keywords: [],
        explanation: "Because",
        is_saved: false,
      },
    ]);

    await pendingLoad;

    expect(store.loadState.status).toBe("success");
    expect(store.recommendations).toHaveLength(1);
    expect(store.recommendations[0].fnord_id).toBe(11);
  });

  it("keeps the resolved result after ensureLoaded is called again", async () => {
    vi.mocked(invoke).mockResolvedValue([
      {
        fnord_id: 21,
        title: "Persisted recommendation",
        summary: null,
        url: "https://example.com",
        image_url: null,
        pentacle_id: 1,
        pentacle_title: "Feed",
        pentacle_icon: null,
        published_at: "2026-04-15T10:00:00Z",
        relevance_score: 0.8,
        freshness_score: 0.7,
        political_bias: null,
        sachlichkeit: null,
        categories: [],
        matching_keywords: [],
        explanation: "Because",
        is_saved: false,
      },
    ]);

    const store = new RecommendationStore();
    await store.loadRecommendations();

    expect(store.recommendations).toHaveLength(1);

    await store.ensureLoaded();

    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === "get_recommendations"),
    ).toHaveLength(1);
    expect(store.recommendations[0].fnord_id).toBe(21);
  });
});
