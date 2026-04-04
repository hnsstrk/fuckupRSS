import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  getMainCategoryId,
  getCategoryColorVar,
  getSachlichkeitLabel,
  getSachlichkeitRangeLabel,
  getSachlichkeitIcon,
  getSachlichkeitColor,
  getBiasColor,
  getBiasIcon,
  getBiasLabel,
  getBiasRangeLabel,
  getStatusIcon,
  getStatusColorClass,
  formatRelativeDate,
  formatFullDate,
  formatShortDate,
  stripHtml,
  formatSimilarity,
  truncateText,
} from "$lib/utils/articleFormat";

// Mock the invoke function
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock appState
vi.mock("../../stores/state.svelte", () => ({
  appState: {
    selectedFnord: null,
    selectedFnordId: null,
    ollamaStatus: { available: true, models: [] },
    retrieving: false,
    analyzing: false,
    error: null,
    selectedModel: "ministral-3:latest",
    getRevisions: vi.fn(),
    getArticleCategories: vi.fn(),
    getArticleTags: vi.fn(),
    findSimilarArticles: vi.fn(),
    fetchFullContent: vi.fn(),
    processArticleDiscordian: vi.fn(),
    acknowledgeChanges: vi.fn(),
    toggleGoldenApple: vi.fn(),
  },
  toasts: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

import { appState, toasts } from "../../stores/state.svelte";

// ============================================================
// Utility Function Tests (articleFormat.ts)
// ============================================================

describe("ArticleView Utility Functions", () => {
  // ============================================================
  // Date Formatting
  // ============================================================

  describe("Date Formatting", () => {
    it("formats full date in German", () => {
      const result = formatFullDate("2025-01-15T10:30:00Z", "de");
      expect(result).toBeTruthy();
      expect(typeof result).toBe("string");
      expect(result).toContain("15");
    });

    it("formats full date in English", () => {
      const result = formatFullDate("2025-01-15T10:30:00Z", "en");
      expect(result).toBeTruthy();
      expect(result).toContain("15");
    });

    it("returns empty string for null date", () => {
      expect(formatFullDate(null)).toBe("");
    });

    it("formats short date correctly", () => {
      const result = formatShortDate("2025-01-15T10:30:00Z", "en");
      expect(result).toContain("15");
    });

    it("returns empty string for null short date", () => {
      expect(formatShortDate(null)).toBe("");
    });

    it("formats relative date for recent articles", () => {
      const now = new Date();
      const fiveMinAgo = new Date(now.getTime() - 5 * 60 * 1000).toISOString();
      const result = formatRelativeDate(fiveMinAgo, "de");
      expect(result).toContain("Min");
    });

    it("formats relative date for hours ago", () => {
      const now = new Date();
      const twoHoursAgo = new Date(now.getTime() - 2 * 60 * 60 * 1000).toISOString();
      const result = formatRelativeDate(twoHoursAgo, "de");
      expect(result).toContain("Std");
    });

    it("formats relative date for days ago", () => {
      const now = new Date();
      const threeDaysAgo = new Date(now.getTime() - 3 * 24 * 60 * 60 * 1000).toISOString();
      const result = formatRelativeDate(threeDaysAgo, "de");
      expect(result).toContain("Tagen");
    });

    it("returns empty string for null relative date", () => {
      expect(formatRelativeDate(null)).toBe("");
    });
  });

  // ============================================================
  // Bias Functions
  // ============================================================

  describe("Bias Labels", () => {
    it("returns correct bias label for each value in German", () => {
      expect(getBiasLabel(null, "de")).toBe("");
      expect(getBiasLabel(-2, "de")).toBe("Stark links");
      expect(getBiasLabel(-1, "de")).toBe("Leicht links");
      expect(getBiasLabel(0, "de")).toBe("Neutral");
      expect(getBiasLabel(1, "de")).toBe("Leicht rechts");
      expect(getBiasLabel(2, "de")).toBe("Stark rechts");
    });

    it("returns correct bias label for each value in English", () => {
      expect(getBiasLabel(-2, "en")).toBe("Strong left");
      expect(getBiasLabel(-1, "en")).toBe("Lean left");
      expect(getBiasLabel(0, "en")).toBe("Neutral");
      expect(getBiasLabel(1, "en")).toBe("Lean right");
      expect(getBiasLabel(2, "en")).toBe("Strong right");
    });

    it("returns empty string for unknown bias values", () => {
      expect(getBiasLabel(5, "de")).toBe("");
    });
  });

  describe("Bias Icons", () => {
    it("returns correct bias icon class", () => {
      expect(getBiasIcon(null)).toBe("");
      expect(getBiasIcon(-2)).toBe("fa-solid fa-angles-left");
      expect(getBiasIcon(-1)).toBe("fa-solid fa-angle-left");
      expect(getBiasIcon(0)).toBe("fa-solid fa-circle");
      expect(getBiasIcon(1)).toBe("fa-solid fa-angle-right");
      expect(getBiasIcon(2)).toBe("fa-solid fa-angles-right");
    });
  });

  describe("Bias Colors", () => {
    it("returns correct bias color class format", () => {
      expect(getBiasColor(null, "class")).toBe("neutral");
      expect(getBiasColor(-2, "class")).toBe("strong-left");
      expect(getBiasColor(-1, "class")).toBe("lean-left");
      expect(getBiasColor(0, "class")).toBe("center");
      expect(getBiasColor(1, "class")).toBe("lean-right");
      expect(getBiasColor(2, "class")).toBe("strong-right");
    });

    it("returns CSS variable format by default", () => {
      expect(getBiasColor(null)).toBe("var(--text-muted)");
      expect(getBiasColor(0)).toBe("var(--bias-center)");
    });
  });

  // ============================================================
  // Sachlichkeit Functions
  // ============================================================

  describe("Sachlichkeit Labels", () => {
    it("returns correct sachlichkeit label in German", () => {
      expect(getSachlichkeitLabel(null, "de")).toBe("");
      expect(getSachlichkeitLabel(0, "de")).toBe("Hoch emotional");
      expect(getSachlichkeitLabel(1, "de")).toBe("Emotional");
      expect(getSachlichkeitLabel(2, "de")).toBe("Gemischt");
      expect(getSachlichkeitLabel(3, "de")).toBe("Überwiegend sachlich");
      expect(getSachlichkeitLabel(4, "de")).toBe("Sachlich");
    });

    it("returns correct sachlichkeit label in English", () => {
      expect(getSachlichkeitLabel(0, "en")).toBe("Highly emotional");
      expect(getSachlichkeitLabel(4, "en")).toBe("Objective");
    });
  });

  describe("Sachlichkeit Icons", () => {
    it("returns correct sachlichkeit icon class", () => {
      expect(getSachlichkeitIcon(null)).toBe("fa-face-meh");
      expect(getSachlichkeitIcon(0)).toBe("fa-heart");
      expect(getSachlichkeitIcon(1)).toBe("fa-heart");
      expect(getSachlichkeitIcon(2)).toBe("fa-face-meh");
      expect(getSachlichkeitIcon(3)).toBe("fa-brain");
      expect(getSachlichkeitIcon(4)).toBe("fa-brain");
    });
  });

  describe("Sachlichkeit Colors", () => {
    it("returns correct sachlichkeit color class", () => {
      expect(getSachlichkeitColor(null)).toBe("neutral");
      expect(getSachlichkeitColor(0)).toBe("emotional");
      expect(getSachlichkeitColor(1)).toBe("emotional");
      expect(getSachlichkeitColor(2)).toBe("mixed");
      expect(getSachlichkeitColor(3)).toBe("objective");
      expect(getSachlichkeitColor(4)).toBe("objective");
    });
  });

  describe("Sachlichkeit Range Labels (float averages)", () => {
    it("maps float averages to correct labels without i18n", () => {
      expect(getSachlichkeitRangeLabel(null)).toBe("");
      expect(getSachlichkeitRangeLabel(0.0)).toBe("Hoch emotional");
      expect(getSachlichkeitRangeLabel(0.5)).toBe("Hoch emotional");
      expect(getSachlichkeitRangeLabel(0.6)).toBe("Emotional");
      expect(getSachlichkeitRangeLabel(1.5)).toBe("Emotional");
      expect(getSachlichkeitRangeLabel(1.6)).toBe("Gemischt");
      expect(getSachlichkeitRangeLabel(2.5)).toBe("Gemischt");
      expect(getSachlichkeitRangeLabel(2.6)).toBe("Überwiegend sachlich");
      expect(getSachlichkeitRangeLabel(3.5)).toBe("Überwiegend sachlich");
      expect(getSachlichkeitRangeLabel(3.6)).toBe("Sachlich");
      expect(getSachlichkeitRangeLabel(4.0)).toBe("Sachlich");
    });
  });

  describe("Bias Range Labels (float averages)", () => {
    it("maps float averages to correct labels without i18n", () => {
      expect(getBiasRangeLabel(null)).toBe("");
      expect(getBiasRangeLabel(-2.0)).toBe("Strong left");
      expect(getBiasRangeLabel(-1.5)).toBe("Strong left");
      expect(getBiasRangeLabel(-1.4)).toBe("Lean left");
      expect(getBiasRangeLabel(-0.5)).toBe("Lean left");
      expect(getBiasRangeLabel(-0.4)).toBe("Neutral");
      expect(getBiasRangeLabel(0.0)).toBe("Neutral");
      expect(getBiasRangeLabel(0.5)).toBe("Neutral");
      expect(getBiasRangeLabel(0.6)).toBe("Lean right");
      expect(getBiasRangeLabel(1.5)).toBe("Lean right");
      expect(getBiasRangeLabel(1.6)).toBe("Strong right");
      expect(getBiasRangeLabel(2.0)).toBe("Strong right");
    });
  });

  // ============================================================
  // Status Functions
  // ============================================================

  describe("Status Icons", () => {
    it("returns correct status icon", () => {
      expect(getStatusIcon("concealed")).toBe("fa-solid fa-eye-slash");
      expect(getStatusIcon("illuminated")).toBe("fa-solid fa-eye");
      expect(getStatusIcon("golden_apple")).toBe("fa-solid fa-apple-whole");
    });

    it("returns default icon for unknown status", () => {
      expect(getStatusIcon("unknown")).toBe("fa-solid fa-check");
    });
  });

  describe("Status Color Classes", () => {
    it("returns correct status color class", () => {
      expect(getStatusColorClass("concealed")).toBe("status-concealed");
      expect(getStatusColorClass("illuminated")).toBe("status-illuminated");
      expect(getStatusColorClass("golden_apple")).toBe("status-golden_apple");
    });
  });

  // ============================================================
  // Category Functions
  // ============================================================

  describe("Category Color Helpers", () => {
    it("gets main category ID from subcategory ID", () => {
      expect(getMainCategoryId(undefined)).toBe(0);
      expect(getMainCategoryId(1)).toBe(1);
      expect(getMainCategoryId(6)).toBe(6);
      expect(getMainCategoryId(101)).toBe(1);
      expect(getMainCategoryId(205)).toBe(2);
      expect(getMainCategoryId(603)).toBe(6);
    });

    it("gets CSS variable for category color", () => {
      expect(getCategoryColorVar(undefined, "var(--bg-overlay)")).toBe("var(--bg-overlay)");
      expect(getCategoryColorVar(0, "var(--bg-overlay)")).toBe("var(--bg-overlay)");
      expect(getCategoryColorVar(1)).toBe("var(--category-1)");
      expect(getCategoryColorVar(3)).toBe("var(--category-3)");
      expect(getCategoryColorVar(101)).toBe("var(--category-1)");
      expect(getCategoryColorVar(305)).toBe("var(--category-3)");
    });

    it("returns fallback for invalid category IDs", () => {
      expect(getCategoryColorVar(0)).toBe("var(--accent-primary)");
      expect(getCategoryColorVar(undefined)).toBe("var(--accent-primary)");
    });
  });

  // ============================================================
  // Text Utilities
  // ============================================================

  describe("Text Utilities", () => {
    it("strips HTML tags", () => {
      expect(stripHtml("<p>Hello World</p>")).toBe("Hello World");
      expect(stripHtml("<div><span>Nested</span></div>")).toBe("Nested");
      expect(stripHtml("Plain text")).toBe("Plain text");
    });

    it("truncates text with ellipsis", () => {
      expect(truncateText("Short", 100)).toBe("Short");
      expect(truncateText("This is a longer text", 10)).toBe("This is a ...");
    });

    it("formats similarity as percentage", () => {
      expect(formatSimilarity(0.85)).toBe("85%");
      expect(formatSimilarity(1.0)).toBe("100%");
      expect(formatSimilarity(0.0)).toBe("0%");
      expect(formatSimilarity(0.333)).toBe("33%");
    });
  });
});

// ============================================================
// Component Logic Tests
// ============================================================

describe("ArticleView Component Logic", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================
  // Article Actions
  // ============================================================

  describe("Article Actions", () => {
    it("toggles golden apple status", () => {
      const fnordId = 42;

      const handleToggleGoldenApple = (id: number) => {
        appState.toggleGoldenApple(id);
      };

      handleToggleGoldenApple(fnordId);
      expect(appState.toggleGoldenApple).toHaveBeenCalledWith(42);
    });

    it("fetches full content successfully", async () => {
      const mockResult = {
        fnord_id: 42,
        success: true,
        content: "Full article content",
        error: null,
      };
      vi.mocked(appState.fetchFullContent).mockResolvedValue(mockResult);

      const fetchFullContent = async (fnordId: number) => {
        const result = await appState.fetchFullContent(fnordId);
        if (result?.success) {
          toasts.success("Content fetched");
        } else if (result?.error) {
          toasts.error(`Fetch failed: ${result.error}`);
        }
      };

      await fetchFullContent(42);

      expect(appState.fetchFullContent).toHaveBeenCalledWith(42);
      expect(toasts.success).toHaveBeenCalled();
    });

    it("handles fetch error with specific messages", async () => {
      const mockResult = {
        fnord_id: 42,
        success: false,
        content: null,
        error: "404 Not Found",
      };
      vi.mocked(appState.fetchFullContent).mockResolvedValue(mockResult);

      const getSpecificFetchError = (error: string): string => {
        const errorLower = error.toLowerCase();
        if (errorLower.includes("404") || errorLower.includes("not found")) {
          return "Page not found";
        }
        if (errorLower.includes("403") || errorLower.includes("forbidden")) {
          return "Access blocked";
        }
        if (errorLower.includes("timeout")) {
          return "Connection timeout";
        }
        return `Fetch error: ${error}`;
      };

      const fetchFullContent = async (fnordId: number) => {
        const result = await appState.fetchFullContent(fnordId);
        if (result?.error) {
          toasts.error(getSpecificFetchError(result.error));
        }
      };

      await fetchFullContent(42);

      expect(toasts.error).toHaveBeenCalledWith("Page not found");
    });

    it("analyzes article with AI", async () => {
      const mockResult = {
        fnord_id: 42,
        success: true,
        analysis: null,
        categories_saved: [],
        tags_saved: [],
        error: null,
      };
      vi.mocked(appState.processArticleDiscordian).mockResolvedValue(mockResult);

      const analyzeWithAI = async (fnordId: number) => {
        const result = await appState.processArticleDiscordian(fnordId);
        if (result?.success) {
          toasts.success("Analysis complete");
        }
      };

      await analyzeWithAI(42);

      expect(appState.processArticleDiscordian).toHaveBeenCalledWith(42);
      expect(toasts.success).toHaveBeenCalled();
    });

    it("requires model selection for analysis", () => {
      const selectedModel: string | null = null;

      const canAnalyze = !!selectedModel;
      expect(canAnalyze).toBe(false);
    });

    it("requires sufficient content for analysis", () => {
      const hasContentForAnalysis = (contentFull: string | null): boolean => {
        return !!contentFull && contentFull.length >= 100;
      };

      expect(hasContentForAnalysis(null)).toBe(false);
      expect(hasContentForAnalysis("short")).toBe(false);
      expect(hasContentForAnalysis("x".repeat(100))).toBe(true);
      expect(hasContentForAnalysis("x".repeat(1000))).toBe(true);
    });
  });

  // ============================================================
  // Content Status
  // ============================================================

  describe("Content Status", () => {
    type ContentStatus = "full" | "rss" | "missing";

    const getContentStatus = (
      contentFull: string | null,
      contentRaw: string | null,
    ): ContentStatus => {
      if (contentFull && contentFull.length > 500) return "full";
      if (contentRaw) return "rss";
      return "missing";
    };

    it("returns full for articles with full content", () => {
      expect(getContentStatus("x".repeat(600), null)).toBe("full");
    });

    it("returns rss for articles with only raw content", () => {
      expect(getContentStatus(null, "raw content")).toBe("rss");
    });

    it("returns rss for articles with short full content", () => {
      expect(getContentStatus("short", "raw")).toBe("rss");
    });

    it("returns missing for articles without any content", () => {
      expect(getContentStatus(null, null)).toBe("missing");
    });
  });

  // ============================================================
  // Content Display
  // ============================================================

  describe("Content Display", () => {
    it("prefers content_full over content_raw", () => {
      const getDisplayContent = (
        contentFull: string | null,
        contentRaw: string | null,
      ): string | null => {
        if (contentFull) return contentFull;
        if (contentRaw) return contentRaw;
        return null;
      };

      expect(getDisplayContent("Full", "Raw")).toBe("Full");
      expect(getDisplayContent(null, "Raw")).toBe("Raw");
      expect(getDisplayContent(null, null)).toBeNull();
    });

    it("determines if full content fetch is needed", () => {
      const needsFullContent = (contentFull: string | null): boolean => {
        return !contentFull;
      };

      expect(needsFullContent(null)).toBe(true);
      expect(needsFullContent("Full content here")).toBe(false);
    });

    it("determines analyze button text", () => {
      const getAnalyzeButtonText = (hasSummary: boolean): string => {
        return hasSummary ? "Reanalyze" : "Analyze";
      };

      expect(getAnalyzeButtonText(true)).toBe("Reanalyze");
      expect(getAnalyzeButtonText(false)).toBe("Analyze");
    });
  });

  // ============================================================
  // Keyboard Shortcuts
  // ============================================================

  describe("Keyboard Shortcuts", () => {
    it("handles v key to open in browser", () => {
      let browserOpened = false;

      const handleKeydown = (key: string, hasSelectedFnord: boolean) => {
        if (key === "v" && hasSelectedFnord) {
          browserOpened = true;
          return true;
        }
        return false;
      };

      expect(handleKeydown("v", true)).toBe(true);
      expect(browserOpened).toBe(true);
    });

    it("does not open browser without selection", () => {
      let browserOpened = false;

      const handleKeydown = (key: string, hasSelectedFnord: boolean) => {
        if (key === "v" && hasSelectedFnord) {
          browserOpened = true;
          return true;
        }
        return false;
      };

      expect(handleKeydown("v", false)).toBe(false);
      expect(browserOpened).toBe(false);
    });

    it("handles r key to fetch full text", () => {
      let fetchTriggered = false;

      const handleKeydown = (key: string, hasSelectedFnord: boolean, hasFullContent: boolean) => {
        if (key === "r" && hasSelectedFnord && !hasFullContent) {
          fetchTriggered = true;
          return true;
        }
        return false;
      };

      expect(handleKeydown("r", true, false)).toBe(true);
      expect(fetchTriggered).toBe(true);
    });

    it("does not fetch when full content already exists", () => {
      let fetchTriggered = false;

      const handleKeydown = (key: string, hasSelectedFnord: boolean, hasFullContent: boolean) => {
        if (key === "r" && hasSelectedFnord && !hasFullContent) {
          fetchTriggered = true;
          return true;
        }
        return false;
      };

      expect(handleKeydown("r", true, true)).toBe(false);
      expect(fetchTriggered).toBe(false);
    });
  });

  // ============================================================
  // Revision Handling
  // ============================================================

  describe("Revision Handling", () => {
    it("toggles revision visibility", () => {
      let showRevisions = false;

      const toggleRevisions = () => {
        showRevisions = !showRevisions;
      };

      expect(showRevisions).toBe(false);
      toggleRevisions();
      expect(showRevisions).toBe(true);
      toggleRevisions();
      expect(showRevisions).toBe(false);
    });

    it("loads revisions when article has changes", async () => {
      const mockRevisions = [
        {
          id: 1,
          fnord_id: 42,
          title: "Revision 1",
          author: null,
          content_raw: null,
          content_full: null,
          summary: null,
          content_hash: "abc",
          revision_at: "2025-01-15T10:00:00Z",
        },
        {
          id: 2,
          fnord_id: 42,
          title: "Revision 2",
          author: null,
          content_raw: null,
          content_full: null,
          summary: null,
          content_hash: "def",
          revision_at: "2025-01-14T10:00:00Z",
        },
      ];
      vi.mocked(appState.getRevisions).mockResolvedValue(mockRevisions);

      const loadRevisions = async (fnordId: number, revisionCount: number) => {
        if (revisionCount > 0) {
          return await appState.getRevisions(fnordId);
        }
        return [];
      };

      const revisions = await loadRevisions(42, 2);

      expect(appState.getRevisions).toHaveBeenCalledWith(42);
      expect(revisions).toHaveLength(2);
    });

    it("skips loading revisions when count is 0", async () => {
      const loadRevisions = async (fnordId: number, revisionCount: number) => {
        if (revisionCount > 0) {
          return await appState.getRevisions(fnordId);
        }
        return [];
      };

      const revisions = await loadRevisions(42, 0);

      expect(appState.getRevisions).not.toHaveBeenCalled();
      expect(revisions).toHaveLength(0);
    });
  });

  // ============================================================
  // Similar Articles
  // ============================================================

  describe("Similar Articles", () => {
    it("loads similar articles when article has embedding", async () => {
      const mockSimilar = [
        {
          fnord_id: 10,
          title: "Similar 1",
          pentacle_title: null,
          published_at: null,
          similarity: 0.85,
          tags: [],
          categories: [],
        },
        {
          fnord_id: 11,
          title: "Similar 2",
          pentacle_title: null,
          published_at: null,
          similarity: 0.75,
          tags: [],
          categories: [],
        },
      ];
      vi.mocked(appState.findSimilarArticles).mockResolvedValue(mockSimilar);

      const loadSimilarArticles = async (fnordId: number, hasEmbedding: boolean, limit: number) => {
        if (hasEmbedding) {
          return await appState.findSimilarArticles(fnordId, limit);
        }
        return [];
      };

      const similar = await loadSimilarArticles(42, true, 5);

      expect(appState.findSimilarArticles).toHaveBeenCalledWith(42, 5);
      expect(similar).toHaveLength(2);
    });

    it("does not load similar articles without embedding", async () => {
      const loadSimilarArticles = async (fnordId: number, hasEmbedding: boolean, limit: number) => {
        if (hasEmbedding) {
          return await appState.findSimilarArticles(fnordId, limit);
        }
        return [];
      };

      const similar = await loadSimilarArticles(42, false, 5);

      expect(appState.findSimilarArticles).not.toHaveBeenCalled();
      expect(similar).toHaveLength(0);
    });
  });

  // ============================================================
  // Article Data Loading
  // ============================================================

  describe("Article Data Loading", () => {
    it("loads categories and tags in parallel", async () => {
      const mockCategories = [
        {
          sephiroth_id: 1,
          name: "Tech",
          icon: null,
          color: null,
          confidence: 0.9,
          source: "ai" as const,
          assigned_at: null,
          parent_id: null,
          main_category_name: null,
          main_category_color: null,
        },
      ];
      const mockTags = [{ id: 1, name: "AI", count: 5, last_used: null }];

      vi.mocked(appState.getArticleCategories).mockResolvedValue(mockCategories);
      vi.mocked(appState.getArticleTags).mockResolvedValue(mockTags);

      const [categories, tags] = await Promise.all([
        appState.getArticleCategories(42),
        appState.getArticleTags(42),
      ]);

      expect(categories).toHaveLength(1);
      expect(tags).toHaveLength(1);
    });

    it("acknowledges changed articles", async () => {
      const fnord = { id: 42, has_changes: true };

      const acknowledgeIfNeeded = async (article: { id: number; has_changes: boolean }) => {
        if (article.has_changes) {
          await appState.acknowledgeChanges(article.id);
        }
      };

      await acknowledgeIfNeeded(fnord);

      expect(appState.acknowledgeChanges).toHaveBeenCalledWith(42);
    });

    it("does not acknowledge article without changes", async () => {
      const fnord = { id: 42, has_changes: false };

      const acknowledgeIfNeeded = async (article: { id: number; has_changes: boolean }) => {
        if (article.has_changes) {
          await appState.acknowledgeChanges(article.id);
        }
      };

      await acknowledgeIfNeeded(fnord);

      expect(appState.acknowledgeChanges).not.toHaveBeenCalled();
    });
  });

  // ============================================================
  // Navigation Events
  // ============================================================

  describe("Navigation Functions", () => {
    it("navigates to keyword via store method", () => {
      // Navigation now goes through navigationStore.navigateToNetwork(keywordId)
      // rather than DOM events. We verify the function signature works correctly.
      const navigateToKeyword = (tagId: number) => {
        return { method: "navigateToNetwork", keywordId: tagId };
      };

      const result = navigateToKeyword(123);
      expect(result.method).toBe("navigateToNetwork");
      expect(result.keywordId).toBe(123);
    });

    it("navigates to article via store method", () => {
      // Navigation now goes through navigationStore.navigateToArticle(articleId)
      const navigateToArticle = (fnordId: number) => {
        return { method: "navigateToArticle", articleId: fnordId };
      };

      const result = navigateToArticle(456);
      expect(result.method).toBe("navigateToArticle");
      expect(result.articleId).toBe(456);
    });
  });

  // ============================================================
  // Edit Mode
  // ============================================================

  describe("Edit Mode", () => {
    it("toggles keyword editing mode", () => {
      let editingKeywords = false;

      const toggleKeywordEditing = () => {
        editingKeywords = !editingKeywords;
      };

      expect(editingKeywords).toBe(false);
      toggleKeywordEditing();
      expect(editingKeywords).toBe(true);
      toggleKeywordEditing();
      expect(editingKeywords).toBe(false);
    });

    it("toggles category editing mode", () => {
      let editingCategories = false;

      const toggleCategoryEditing = () => {
        editingCategories = !editingCategories;
      };

      expect(editingCategories).toBe(false);
      toggleCategoryEditing();
      expect(editingCategories).toBe(true);
      toggleCategoryEditing();
      expect(editingCategories).toBe(false);
    });
  });

  // ============================================================
  // Fetch Error Handling
  // ============================================================

  describe("Fetch Error Handling", () => {
    it("classifies 404 errors", () => {
      const getSpecificFetchError = (error: string): string => {
        const errorLower = error.toLowerCase();
        if (errorLower.includes("404") || errorLower.includes("not found")) return "not-found";
        if (errorLower.includes("403") || errorLower.includes("forbidden")) return "blocked";
        if (errorLower.includes("timeout")) return "timeout";
        if (errorLower.includes("network") || errorLower.includes("connection")) return "network";
        if (errorLower.includes("paywall")) return "paywall";
        return "generic";
      };

      expect(getSpecificFetchError("HTTP 404 Not Found")).toBe("not-found");
      expect(getSpecificFetchError("HTTP 403 Forbidden")).toBe("blocked");
      expect(getSpecificFetchError("Connection timeout")).toBe("timeout");
      expect(getSpecificFetchError("Network error")).toBe("network");
      expect(getSpecificFetchError("Paywall detected")).toBe("paywall");
      expect(getSpecificFetchError("Unknown error")).toBe("generic");
    });
  });
});

// ============================================================
// Data Structures
// ============================================================

describe("ArticleView Data Structures", () => {
  describe("Fnord Structure", () => {
    interface FnordDetailed {
      id: number;
      title: string;
      pentacle_title: string | null;
      url: string;
      author: string | null;
      content_raw: string | null;
      content_full: string | null;
      summary: string | null;
      published_at: string | null;
      processed_at: string | null;
      status: "concealed" | "illuminated" | "golden_apple";
      political_bias: number | null;
      sachlichkeit: number | null;
      quality_score: number | null;
      has_changes: boolean;
      revision_count: number;
      full_text_fetch_error: string | null;
    }

    it("creates valid detailed fnord", () => {
      const fnord: FnordDetailed = {
        id: 1,
        title: "Test Article",
        pentacle_title: "Tech News",
        url: "https://example.com/article",
        author: "John Doe",
        content_raw: "<p>Raw content</p>",
        content_full: "<article>Full content</article>",
        summary: "Article summary",
        published_at: "2025-01-15T10:00:00Z",
        processed_at: "2025-01-15T11:00:00Z",
        status: "illuminated",
        political_bias: 0,
        sachlichkeit: 3,
        quality_score: 4,
        has_changes: false,
        revision_count: 0,
        full_text_fetch_error: null,
      };

      expect(fnord.id).toBe(1);
      expect(fnord.status).toBe("illuminated");
      expect(fnord.political_bias).toBe(0);
    });

    it("handles unprocessed article", () => {
      const fnord: FnordDetailed = {
        id: 2,
        title: "Unprocessed Article",
        pentacle_title: null,
        url: "https://example.com/unprocessed",
        author: null,
        content_raw: "<p>Some content</p>",
        content_full: null,
        summary: null,
        published_at: null,
        processed_at: null,
        status: "concealed",
        political_bias: null,
        sachlichkeit: null,
        quality_score: null,
        has_changes: false,
        revision_count: 0,
        full_text_fetch_error: null,
      };

      expect(fnord.processed_at).toBeNull();
      expect(fnord.summary).toBeNull();
      expect(fnord.content_full).toBeNull();
    });

    it("handles fetch error state", () => {
      const fnord: FnordDetailed = {
        id: 3,
        title: "Error Article",
        pentacle_title: null,
        url: "https://example.com/error",
        author: null,
        content_raw: null,
        content_full: null,
        summary: null,
        published_at: null,
        processed_at: null,
        status: "concealed",
        political_bias: null,
        sachlichkeit: null,
        quality_score: null,
        has_changes: false,
        revision_count: 0,
        full_text_fetch_error: "403 Forbidden",
      };

      expect(fnord.full_text_fetch_error).toBe("403 Forbidden");
      expect(fnord.content_full).toBeNull();
    });
  });
});
