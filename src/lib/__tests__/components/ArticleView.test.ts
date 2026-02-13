import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getMainCategoryId,
  getCategoryColorVar,
  getSachlichkeitLabel,
  getSachlichkeitIcon,
  getSachlichkeitColor,
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

describe("ArticleView Component Logic", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("HTML Stripping", () => {
    it("strips HTML tags from content", () => {
      // Simulate the stripHtml function
      const stripHtml = (html: string): string => {
        // In tests, we can't use document.createElement, so we use regex
        return html.replace(/<[^>]*>/g, "");
      };

      expect(stripHtml("<p>Hello World</p>")).toBe("Hello World");
      expect(stripHtml("<div><span>Nested</span></div>")).toBe("Nested");
      expect(stripHtml("Plain text")).toBe("Plain text");
    });
  });

  describe("Date Formatting", () => {
    it("formats date string correctly", () => {
      const formatDate = (dateStr: string | null, locale: string): string => {
        if (!dateStr) return "";
        const date = new Date(dateStr);
        return date.toLocaleDateString(locale === "de" ? "de-DE" : "en-US", {
          weekday: "long",
          year: "numeric",
          month: "long",
          day: "numeric",
          hour: "2-digit",
          minute: "2-digit",
        });
      };

      const result = formatDate("2025-01-15T10:30:00Z", "en");
      expect(result).toBeTruthy();
      expect(typeof result).toBe("string");
    });

    it("returns empty string for null date", () => {
      const formatDate = (dateStr: string | null): string => {
        if (!dateStr) return "";
        return new Date(dateStr).toLocaleDateString();
      };

      expect(formatDate(null)).toBe("");
    });

    it("formats short date", () => {
      const formatShortDate = (dateStr: string | null, locale: string): string => {
        if (!dateStr) return "";
        const date = new Date(dateStr);
        return date.toLocaleDateString(locale === "de" ? "de-DE" : "en-US", {
          day: "numeric",
          month: "short",
        });
      };

      const result = formatShortDate("2025-01-15T10:30:00Z", "en");
      expect(result).toContain("15");
    });
  });

  describe("Bias Labels", () => {
    it("returns correct bias label for each value", () => {
      const getBiasLabel = (bias: number | null): string => {
        if (bias === null) return "Not Rated";
        switch (bias) {
          case -2:
            return "Strong Left";
          case -1:
            return "Lean Left";
          case 0:
            return "Center";
          case 1:
            return "Lean Right";
          case 2:
            return "Strong Right";
          default:
            return "Unknown";
        }
      };

      expect(getBiasLabel(null)).toBe("Not Rated");
      expect(getBiasLabel(-2)).toBe("Strong Left");
      expect(getBiasLabel(-1)).toBe("Lean Left");
      expect(getBiasLabel(0)).toBe("Center");
      expect(getBiasLabel(1)).toBe("Lean Right");
      expect(getBiasLabel(2)).toBe("Strong Right");
      expect(getBiasLabel(5)).toBe("Unknown");
    });
  });

  describe("Sachlichkeit Labels", () => {
    it("returns correct sachlichkeit label for each value", () => {
      expect(getSachlichkeitLabel(null, "en")).toBe("");
      expect(getSachlichkeitLabel(0, "en")).toBe("Highly emotional");
      expect(getSachlichkeitLabel(1, "en")).toBe("Emotional");
      expect(getSachlichkeitLabel(2, "en")).toBe("Mixed");
      expect(getSachlichkeitLabel(3, "en")).toBe("Mostly objective");
      expect(getSachlichkeitLabel(4, "en")).toBe("Objective");
    });

    it("returns correct sachlichkeit label in German", () => {
      expect(getSachlichkeitLabel(0, "de")).toBe("Hoch emotional");
      expect(getSachlichkeitLabel(4, "de")).toBe("Sachlich");
    });
  });

  describe("Bias Icons", () => {
    it("returns correct bias icon class", () => {
      const getBiasIcon = (bias: number | null): string => {
        if (bias === null) return "fa-scale-balanced";
        if (bias < 0) return "fa-scale-unbalanced";
        if (bias > 0) return "fa-scale-unbalanced-flip";
        return "fa-scale-balanced";
      };

      expect(getBiasIcon(null)).toBe("fa-scale-balanced");
      expect(getBiasIcon(-2)).toBe("fa-scale-unbalanced");
      expect(getBiasIcon(-1)).toBe("fa-scale-unbalanced");
      expect(getBiasIcon(0)).toBe("fa-scale-balanced");
      expect(getBiasIcon(1)).toBe("fa-scale-unbalanced-flip");
      expect(getBiasIcon(2)).toBe("fa-scale-unbalanced-flip");
    });
  });

  describe("Bias Colors", () => {
    it("returns correct bias color class", () => {
      const getBiasColor = (bias: number | null): string => {
        if (bias === null) return "neutral";
        if (bias <= -2) return "strong-left";
        if (bias === -1) return "lean-left";
        if (bias === 0) return "center";
        if (bias === 1) return "lean-right";
        return "strong-right";
      };

      expect(getBiasColor(null)).toBe("neutral");
      expect(getBiasColor(-2)).toBe("strong-left");
      expect(getBiasColor(-1)).toBe("lean-left");
      expect(getBiasColor(0)).toBe("center");
      expect(getBiasColor(1)).toBe("lean-right");
      expect(getBiasColor(2)).toBe("strong-right");
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
  });

  describe("Article Actions", () => {
    it("toggles golden apple status", () => {
      const fnordId = 42;
      const currentStatus = "concealed";

      const handleToggleGoldenApple = (id: number) => {
        appState.toggleGoldenApple(id);
      };

      handleToggleGoldenApple(fnordId);
      expect(appState.toggleGoldenApple).toHaveBeenCalledWith(42);
    });

    it("opens article in browser", () => {
      const mockOpen = vi.fn();
      const originalOpen = global.window?.open;
      // @ts-ignore
      global.window = { open: mockOpen };

      const openInBrowser = (url: string) => {
        window.open(url, "_blank");
      };

      openInBrowser("https://example.com/article");
      expect(mockOpen).toHaveBeenCalledWith("https://example.com/article", "_blank");

      // @ts-ignore
      if (originalOpen) global.window.open = originalOpen;
    });

    it("fetches full content", async () => {
      const mockResult = { success: true, content: "Full article content" };
      vi.mocked(appState.fetchFullContent).mockResolvedValue(mockResult);

      const fetchFullContent = async (fnordId: number) => {
        const result = await appState.fetchFullContent(fnordId);
        if (result?.success) {
          toasts.success("Content fetched");
        }
      };

      await fetchFullContent(42);

      expect(appState.fetchFullContent).toHaveBeenCalledWith(42);
      expect(toasts.success).toHaveBeenCalled();
    });

    it("analyzes article with AI", async () => {
      const mockResult = { success: true, analysis: {} };
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
  });

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

      browserOpened = false;
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

      fetchTriggered = false;
      expect(handleKeydown("r", true, true)).toBe(false);
      expect(fetchTriggered).toBe(false);
    });
  });

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
        { id: 1, fnord_id: 42, title: "Revision 1", revision_at: "2025-01-15T10:00:00Z" },
        { id: 2, fnord_id: 42, title: "Revision 2", revision_at: "2025-01-14T10:00:00Z" },
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
  });

  describe("Similar Articles", () => {
    it("loads similar articles when article has embedding", async () => {
      const mockSimilar = [
        { fnord_id: 10, title: "Similar 1", similarity: 0.85 },
        { fnord_id: 11, title: "Similar 2", similarity: 0.75 },
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

  describe("Article Data Loading", () => {
    it("loads categories and tags", async () => {
      const mockCategories = [{ sephiroth_id: 1, name: "Tech" }];
      const mockTags = [{ id: 1, name: "AI" }];

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
  });

  describe("Navigation Events", () => {
    it("creates navigate-to-network custom event correctly", () => {
      const createNavigateToNetworkEvent = (tagId: number) => {
        return new CustomEvent("navigate-to-network", { detail: { keywordId: tagId } });
      };

      const event = createNavigateToNetworkEvent(123);

      expect(event.type).toBe("navigate-to-network");
      expect(event.detail.keywordId).toBe(123);
    });

    it("creates navigate-to-article custom event correctly", () => {
      const createNavigateToArticleEvent = (fnordId: number) => {
        return new CustomEvent("navigate-to-article", { detail: { articleId: fnordId } });
      };

      const event = createNavigateToArticleEvent(456);

      expect(event.type).toBe("navigate-to-article");
      expect(event.detail.articleId).toBe(456);
    });
  });
});

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
      };

      expect(fnord.processed_at).toBeNull();
      expect(fnord.summary).toBeNull();
      expect(fnord.content_full).toBeNull();
    });
  });

  describe("Revision Structure", () => {
    interface FnordRevision {
      id: number;
      fnord_id: number;
      title: string;
      content_raw: string | null;
      content_full: string | null;
      revision_at: string;
    }

    it("creates valid revision", () => {
      const revision: FnordRevision = {
        id: 1,
        fnord_id: 42,
        title: "Updated Title",
        content_raw: "<p>Updated content</p>",
        content_full: "<article>Full updated content</article>",
        revision_at: "2025-01-14T10:00:00Z",
      };

      expect(revision.fnord_id).toBe(42);
      expect(revision.revision_at).toBeTruthy();
    });
  });

  describe("Similar Article Structure", () => {
    interface SimilarArticleItem {
      fnord_id: number;
      title: string;
      pentacle_title: string | null;
      published_at: string | null;
      similarity: number;
      tags: { id: number; name: string }[];
      categories: { id: number; name: string }[];
    }

    it("creates valid similar article", () => {
      const similar: SimilarArticleItem = {
        fnord_id: 10,
        title: "Similar Article",
        pentacle_title: "News Feed",
        published_at: "2025-01-14T08:00:00Z",
        similarity: 0.85,
        tags: [{ id: 1, name: "AI" }],
        categories: [{ id: 1, name: "Technology" }],
      };

      expect(similar.similarity).toBeGreaterThanOrEqual(0);
      expect(similar.similarity).toBeLessThanOrEqual(1);
    });
  });
});

describe("ArticleView Content Display", () => {
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

describe("ArticleView Edit Mode", () => {
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
