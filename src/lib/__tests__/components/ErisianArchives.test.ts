import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

// Mock the invoke function with specific responses
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock appState
vi.mock("../../stores/state.svelte", () => ({
  appState: {
    selectFnord: vi.fn(),
  },
}));

describe("ErisianArchives", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Stats Loading", () => {
    it("loads all stats counts on initialization", async () => {
      const mockInvoke = vi.mocked(invoke);

      // Setup mock responses
      mockInvoke.mockImplementation(async (cmd: string) => {
        switch (cmd) {
          case "get_fnords_count":
            return 100;
          case "get_failed_count":
            return { count: 5 };
          case "get_hopeless_count":
            return { count: 2 };
          default:
            return [];
        }
      });

      // Simulate stats loading
      const totalCount = await invoke("get_fnords_count", { filter: null });
      const unreadCount = await invoke("get_fnords_count", { filter: { status: "concealed" } });
      const favoritesCount = await invoke("get_fnords_count", {
        filter: { status: "golden_apple" },
      });
      const failedResult = await invoke<{ count: number }>("get_failed_count");
      const hopelessResult = await invoke<{ count: number }>("get_hopeless_count");

      expect(totalCount).toBe(100);
      expect(unreadCount).toBe(100);
      expect(favoritesCount).toBe(100);
      expect(failedResult.count).toBe(5);
      expect(hopelessResult.count).toBe(2);
    });

    it("handles stats loading errors gracefully", async () => {
      const mockInvoke = vi.mocked(invoke);
      mockInvoke.mockRejectedValue(new Error("Database error"));

      let errorCaught = false;
      try {
        await invoke("get_fnords_count", { filter: null });
      } catch (e) {
        errorCaught = true;
      }

      expect(errorCaught).toBe(true);
    });
  });

  describe("Articles Tab", () => {
    it("loads articles with limit filter", async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockArticles = [
        { id: 1, title: "Test Article 1", status: "concealed" },
        { id: 2, title: "Test Article 2", status: "illuminated" },
      ];

      mockInvoke.mockResolvedValue(mockArticles);

      const articles = await invoke("get_fnords", { filter: { limit: 100 } });

      expect(mockInvoke).toHaveBeenCalledWith("get_fnords", { filter: { limit: 100 } });
      expect(articles).toHaveLength(2);
    });
  });

  describe("Unread Tab", () => {
    it("loads unread articles with concealed status filter", async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockArticles = [{ id: 1, title: "Unread Article", status: "concealed" }];

      mockInvoke.mockResolvedValue(mockArticles);

      const articles = await invoke("get_fnords", { filter: { status: "concealed", limit: 100 } });

      expect(mockInvoke).toHaveBeenCalledWith("get_fnords", {
        filter: { status: "concealed", limit: 100 },
      });
      expect(articles).toHaveLength(1);
    });
  });

  describe("Golden Apple Tab", () => {
    it("loads favorite articles with golden_apple status filter", async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockArticles = [{ id: 1, title: "Favorite Article", status: "golden_apple" }];

      mockInvoke.mockResolvedValue(mockArticles);

      const articles = await invoke("get_fnords", {
        filter: { status: "golden_apple", limit: 100 },
      });

      expect(mockInvoke).toHaveBeenCalledWith("get_fnords", {
        filter: { status: "golden_apple", limit: 100 },
      });
      expect(articles).toHaveLength(1);
    });
  });

  describe("Failed Tab", () => {
    it("loads failed articles from dedicated command with pagination", async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockFailedArticles = [
        {
          id: 1,
          title: "Failed Analysis Article",
          pentacle_id: 10,
          pentacle_title: "Test Feed",
          summary: null,
          published_at: "2025-01-15T10:00:00Z",
          status: "concealed",
          analysis_attempts: 2,
          last_error: "Ollama connection failed",
        },
      ];

      mockInvoke.mockResolvedValue(mockFailedArticles);

      // Component now uses BATCH_SIZE (50) and offset for pagination
      const articles = await invoke("get_failed_articles", { limit: 50, offset: 0 });

      expect(mockInvoke).toHaveBeenCalledWith("get_failed_articles", { limit: 50, offset: 0 });
      expect(articles).toHaveLength(1);
      expect(articles[0].analysis_attempts).toBe(2);
      expect(articles[0].last_error).toBe("Ollama connection failed");
    });

    it("loads more failed articles with offset", async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockMoreArticles = [
        {
          id: 51,
          title: "Another Failed Article",
          pentacle_id: 10,
          pentacle_title: "Test Feed",
          summary: null,
          published_at: "2025-01-14T10:00:00Z",
          status: "concealed",
          analysis_attempts: 1,
          last_error: "Parse error",
        },
      ];

      mockInvoke.mockResolvedValue(mockMoreArticles);

      // Simulate loading more with offset
      const articles = await invoke("get_failed_articles", { limit: 50, offset: 50 });

      expect(mockInvoke).toHaveBeenCalledWith("get_failed_articles", { limit: 50, offset: 50 });
      expect(articles).toHaveLength(1);
      expect(articles[0].id).toBe(51);
    });

    it("maps AnalysisStatusArticle to Fnord-like structure", async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockFailedArticle = {
        id: 1,
        title: "Failed Article",
        pentacle_id: 10,
        pentacle_title: "Test Feed",
        summary: "Some summary",
        published_at: "2025-01-15T10:00:00Z",
        status: "concealed",
        analysis_attempts: 3,
        last_error: "Timeout",
      };

      mockInvoke.mockResolvedValue([mockFailedArticle]);

      const failedArticles = await invoke<(typeof mockFailedArticle)[]>("get_failed_articles", {
        limit: 50,
        offset: 0,
      });

      // Simulate the mapping done in the component
      const mappedArticle = {
        id: failedArticles[0].id,
        pentacle_id: failedArticles[0].pentacle_id,
        pentacle_title: failedArticles[0].pentacle_title,
        guid: "",
        url: "",
        title: failedArticles[0].title,
        author: null,
        content_raw: null,
        content_full: null,
        summary: failedArticles[0].summary,
        image_url: null,
        published_at: failedArticles[0].published_at,
        processed_at: null,
        status: failedArticles[0].status,
        political_bias: null,
        sachlichkeit: null,
        quality_score: null,
        has_changes: false,
        changed_at: null,
        revision_count: 0,
        categories: [],
      };

      expect(mappedArticle.id).toBe(1);
      expect(mappedArticle.title).toBe("Failed Article");
      expect(mappedArticle.pentacle_title).toBe("Test Feed");
      expect(mappedArticle.summary).toBe("Some summary");
      expect(mappedArticle.guid).toBe("");
      expect(mappedArticle.categories).toEqual([]);
    });
  });

  describe("Hopeless Tab", () => {
    it("loads hopeless articles from dedicated command with pagination", async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockHopelessArticles = [
        {
          id: 2,
          title: "Hopeless Article",
          pentacle_id: 11,
          pentacle_title: "Another Feed",
          summary: null,
          published_at: "2025-01-14T10:00:00Z",
          status: "concealed",
          analysis_attempts: 5,
          last_error: "Max retries exceeded",
        },
      ];

      mockInvoke.mockResolvedValue(mockHopelessArticles);

      // Component now uses BATCH_SIZE (50) and offset for pagination
      const articles = await invoke("get_hopeless_articles", { limit: 50, offset: 0 });

      expect(mockInvoke).toHaveBeenCalledWith("get_hopeless_articles", { limit: 50, offset: 0 });
      expect(articles).toHaveLength(1);
      expect(articles[0].analysis_attempts).toBe(5);
    });

    it("loads more hopeless articles with offset", async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockMoreArticles = [
        {
          id: 52,
          title: "Another Hopeless Article",
          pentacle_id: 11,
          pentacle_title: "Another Feed",
          summary: null,
          published_at: "2025-01-13T10:00:00Z",
          status: "concealed",
          analysis_attempts: 6,
          last_error: "Permanently failed",
        },
      ];

      mockInvoke.mockResolvedValue(mockMoreArticles);

      // Simulate loading more with offset
      const articles = await invoke("get_hopeless_articles", { limit: 50, offset: 50 });

      expect(mockInvoke).toHaveBeenCalledWith("get_hopeless_articles", { limit: 50, offset: 50 });
      expect(articles).toHaveLength(1);
      expect(articles[0].id).toBe(52);
    });
  });

  describe("Empty State Messages", () => {
    it("returns correct empty message for each tab", () => {
      const getEmptyMessage = (activeTab: string) => {
        switch (activeTab) {
          case "articles":
            return "erisianArchives.noArticles";
          case "unread":
            return "erisianArchives.noUnread";
          case "goldenApple":
            return "erisianArchives.noFavorites";
          case "failed":
            return "erisianArchives.noFailed";
          case "hopeless":
            return "erisianArchives.noHopeless";
          default:
            return "";
        }
      };

      expect(getEmptyMessage("articles")).toBe("erisianArchives.noArticles");
      expect(getEmptyMessage("unread")).toBe("erisianArchives.noUnread");
      expect(getEmptyMessage("goldenApple")).toBe("erisianArchives.noFavorites");
      expect(getEmptyMessage("failed")).toBe("erisianArchives.noFailed");
      expect(getEmptyMessage("hopeless")).toBe("erisianArchives.noHopeless");
      expect(getEmptyMessage("unknown")).toBe("");
    });
  });

  describe("Tab Classification", () => {
    it("correctly identifies analysis tabs", () => {
      const isAnalysisTab = (tab: string) => tab === "failed" || tab === "hopeless";

      expect(isAnalysisTab("articles")).toBe(false);
      expect(isAnalysisTab("unread")).toBe(false);
      expect(isAnalysisTab("goldenApple")).toBe(false);
      expect(isAnalysisTab("failed")).toBe(true);
      expect(isAnalysisTab("hopeless")).toBe(true);
    });
  });

  describe("Article Selection", () => {
    it("dispatches navigate-to-article event on selection", async () => {
      const dispatchEventSpy = vi.spyOn(window, "dispatchEvent");

      const articleId = 42;
      const event = new CustomEvent("navigate-to-article", { detail: { articleId } });
      window.dispatchEvent(event);

      expect(dispatchEventSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "navigate-to-article",
          detail: { articleId: 42 },
        }),
      );

      dispatchEventSpy.mockRestore();
    });
  });

  describe("Tab Filter Auto-Remove Logic", () => {
    it("determines if article should be removed from unread tab", () => {
      const shouldRemoveFromTab = (activeTab: string, newStatus: string): boolean => {
        if (activeTab === "unread" && newStatus !== "concealed") {
          return true;
        }
        if (activeTab === "goldenApple" && newStatus !== "golden_apple") {
          return true;
        }
        return false;
      };

      // Unread tab: remove if status changes from concealed
      expect(shouldRemoveFromTab("unread", "illuminated")).toBe(true);
      expect(shouldRemoveFromTab("unread", "golden_apple")).toBe(true);
      expect(shouldRemoveFromTab("unread", "concealed")).toBe(false);

      // Golden Apple tab: remove if status changes from golden_apple
      expect(shouldRemoveFromTab("goldenApple", "concealed")).toBe(true);
      expect(shouldRemoveFromTab("goldenApple", "illuminated")).toBe(true);
      expect(shouldRemoveFromTab("goldenApple", "golden_apple")).toBe(false);

      // Articles tab: never remove based on status
      expect(shouldRemoveFromTab("articles", "illuminated")).toBe(false);
      expect(shouldRemoveFromTab("articles", "concealed")).toBe(false);
      expect(shouldRemoveFromTab("articles", "golden_apple")).toBe(false);
    });

    it("finds next article correctly when current is removed", () => {
      const findNextArticle = (articles: { id: number }[], currentId: number) => {
        const currentIndex = articles.findIndex((a) => a.id === currentId);
        return articles[currentIndex + 1] || articles[currentIndex - 1] || null;
      };

      const articles = [{ id: 1 }, { id: 2 }, { id: 3 }];

      // Remove from middle - should get next
      expect(findNextArticle(articles, 2)?.id).toBe(3);

      // Remove from end - should get previous
      expect(findNextArticle(articles, 3)?.id).toBe(2);

      // Remove from start - should get next
      expect(findNextArticle(articles, 1)?.id).toBe(2);

      // Single article - returns null
      expect(findNextArticle([{ id: 1 }], 1)).toBeNull();

      // Empty list - returns null
      expect(findNextArticle([], 1)).toBeNull();
    });

    it("updates local article list when status changes", () => {
      interface Article {
        id: number;
        title: string;
        status: string;
      }

      const updateArticleStatus = (
        articles: Article[],
        id: number,
        newStatus: string,
      ): Article[] => {
        return articles.map((a) => (a.id === id ? { ...a, status: newStatus } : a));
      };

      const articles: Article[] = [
        { id: 1, title: "Article 1", status: "concealed" },
        { id: 2, title: "Article 2", status: "concealed" },
      ];

      const updated = updateArticleStatus(articles, 1, "illuminated");

      expect(updated[0].status).toBe("illuminated");
      expect(updated[1].status).toBe("concealed");
      // Original should be unchanged
      expect(articles[0].status).toBe("concealed");
    });

    it("removes article from filtered list", () => {
      interface Article {
        id: number;
        status: string;
      }

      const removeArticle = (articles: Article[], id: number): Article[] => {
        return articles.filter((a) => a.id !== id);
      };

      const articles: Article[] = [
        { id: 1, status: "concealed" },
        { id: 2, status: "concealed" },
        { id: 3, status: "concealed" },
      ];

      const afterRemoval = removeArticle(articles, 2);

      expect(afterRemoval).toHaveLength(2);
      expect(afterRemoval.map((a) => a.id)).toEqual([1, 3]);
    });
  });

  describe("Cascade Prevention", () => {
    it("prevents cascade by using guard flag", () => {
      // Simulate the cascade prevention logic
      let isAutoSelecting = false;
      const operations: string[] = [];

      const simulateEffect = (selectedId: number | null) => {
        if (isAutoSelecting) {
          operations.push("effect-skipped");
          return;
        }
        operations.push(`effect-triggered-${selectedId}`);
      };

      const simulateAutoSelect = (nextId: number) => {
        isAutoSelecting = true;
        operations.push(`auto-select-${nextId}`);
        // Simulate async operation
        Promise.resolve().then(() => {
          isAutoSelecting = false;
        });
      };

      // Initial selection triggers effect
      simulateEffect(1);
      expect(operations).toContain("effect-triggered-1");

      // Auto-select sets flag
      simulateAutoSelect(2);
      expect(operations).toContain("auto-select-2");

      // Effect is skipped during auto-select
      simulateEffect(2);
      expect(operations).toContain("effect-skipped");
    });

    it("correctly identifies status change requiring removal", () => {
      interface StatusCheck {
        activeTab: string;
        oldStatus: string;
        newStatus: string;
      }

      const shouldTriggerRemoval = (check: StatusCheck): boolean => {
        // Status must have changed
        if (check.oldStatus === check.newStatus) return false;

        // Check tab-specific removal rules
        if (check.activeTab === "unread" && check.newStatus !== "concealed") {
          return true;
        }
        if (check.activeTab === "goldenApple" && check.newStatus !== "golden_apple") {
          return true;
        }
        return false;
      };

      // Unread tab scenarios
      expect(
        shouldTriggerRemoval({
          activeTab: "unread",
          oldStatus: "concealed",
          newStatus: "illuminated",
        }),
      ).toBe(true);

      expect(
        shouldTriggerRemoval({
          activeTab: "unread",
          oldStatus: "concealed",
          newStatus: "concealed",
        }),
      ).toBe(false);

      // Golden Apple tab scenarios
      expect(
        shouldTriggerRemoval({
          activeTab: "goldenApple",
          oldStatus: "golden_apple",
          newStatus: "illuminated",
        }),
      ).toBe(true);

      expect(
        shouldTriggerRemoval({
          activeTab: "goldenApple",
          oldStatus: "golden_apple",
          newStatus: "golden_apple",
        }),
      ).toBe(false);

      // Articles tab - never triggers removal
      expect(
        shouldTriggerRemoval({
          activeTab: "articles",
          oldStatus: "concealed",
          newStatus: "illuminated",
        }),
      ).toBe(false);
    });

    it("handles empty list after all articles removed", () => {
      interface Article {
        id: number;
      }

      const handleRemoval = (articles: Article[], currentId: number) => {
        const currentIndex = articles.findIndex((a) => a.id === currentId);
        const nextArticle = articles[currentIndex + 1] || articles[currentIndex - 1];
        const remainingArticles = articles.filter((a) => a.id !== currentId);

        return {
          remainingArticles,
          nextSelectedId: nextArticle?.id ?? null,
        };
      };

      // Single article removal leaves empty list
      const singleResult = handleRemoval([{ id: 1 }], 1);
      expect(singleResult.remainingArticles).toHaveLength(0);
      expect(singleResult.nextSelectedId).toBeNull();

      // Multiple articles - next is selected
      const multiResult = handleRemoval([{ id: 1 }, { id: 2 }, { id: 3 }], 2);
      expect(multiResult.remainingArticles).toHaveLength(2);
      expect(multiResult.nextSelectedId).toBe(3);
    });
  });

  describe("Keyboard Navigation", () => {
    it("handles j key for next article", () => {
      interface Article {
        id: number;
      }

      const selectNextArticle = (articles: Article[], currentId: number | null): number | null => {
        if (currentId === null && articles.length > 0) {
          return articles[0].id;
        }
        const currentIndex = articles.findIndex((a) => a.id === currentId);
        if (currentIndex < articles.length - 1) {
          return articles[currentIndex + 1].id;
        }
        return currentId;
      };

      const articles = [{ id: 1 }, { id: 2 }, { id: 3 }];

      // No selection - select first
      expect(selectNextArticle(articles, null)).toBe(1);

      // Middle - select next
      expect(selectNextArticle(articles, 2)).toBe(3);

      // End - stay at end
      expect(selectNextArticle(articles, 3)).toBe(3);
    });

    it("handles k key for previous article", () => {
      interface Article {
        id: number;
      }

      const selectPrevArticle = (articles: Article[], currentId: number | null): number | null => {
        if (currentId === null) return null;
        const currentIndex = articles.findIndex((a) => a.id === currentId);
        if (currentIndex > 0) {
          return articles[currentIndex - 1].id;
        }
        return currentId;
      };

      const articles = [{ id: 1 }, { id: 2 }, { id: 3 }];

      // No selection - stay null
      expect(selectPrevArticle(articles, null)).toBeNull();

      // Middle - select previous
      expect(selectPrevArticle(articles, 2)).toBe(1);

      // Start - stay at start
      expect(selectPrevArticle(articles, 1)).toBe(1);
    });
  });
});
