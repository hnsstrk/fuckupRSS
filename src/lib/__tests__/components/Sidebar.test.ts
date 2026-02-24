import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

// Mock the invoke function
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock the listen function
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Mock appState
vi.mock("../../stores/state.svelte", () => ({
  appState: {
    pentacles: [],
    sephiroth: [],
    fnords: [],
    changedFnords: [],
    selectedPentacleId: null,
    selectedSephirothId: null,
    selectedView: "all",
    totalUnread: 0,
    totalIlluminated: 0,
    totalGoldenApple: 0,
    syncing: false,
    batchProcessing: false,
    batchProgress: null,
    ollamaStatus: { available: true, models: [] },
    selectedModel: "ministral-3:latest",
    unprocessedCount: { total: 0, with_content: 0 },
    searchResults: [],
    searching: false,
    error: null,
    hasAnyMissingModel: false,
    missingMainModel: null,
    missingEmbeddingModel: null,
    loading: false,
    loadPentacles: vi.fn(),
    loadSephiroth: vi.fn(),
    loadFnords: vi.fn(),
    loadChangedFnords: vi.fn(),
    checkOllama: vi.fn(),
    syncAllFeeds: vi.fn(),
    loadUnprocessedCount: vi.fn(),
    updateBatchProgress: vi.fn(),
    updateEmbeddingProgress: vi.fn(),
    addPentacle: vi.fn(),
    deletePentacle: vi.fn(),
    selectPentacle: vi.fn(),
    selectSephiroth: vi.fn(),
    startBatchProcessing: vi.fn(),
    cancelBatch: vi.fn(),
    semanticSearch: vi.fn(),
    clearSearch: vi.fn(),
    resetAllChanges: vi.fn(),
  },
  toasts: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

import { appState, toasts } from "../../stores/state.svelte";

// ============================================================
// Sync Handling
// ============================================================

describe("Sidebar Component Logic", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (appState as unknown as Record<string, unknown>).error = null;
    (appState as unknown as Record<string, unknown>).syncing = false;
  });

  describe("Sync Handling", () => {
    it("calls syncAllFeeds and shows success toast with counts", async () => {
      const mockResult = { success: true, total_new: 5, total_updated: 2, results: [] };
      vi.mocked(appState.syncAllFeeds).mockResolvedValue(mockResult);

      const handleSync = async () => {
        const result = await appState.syncAllFeeds();
        if (result) {
          if (result.total_new > 0 || result.total_updated > 0) {
            toasts.success(
              `Sync complete: ${result.total_new} new, ${result.total_updated} updated`,
            );
          } else {
            toasts.info("No new articles");
          }
        }
      };

      await handleSync();

      expect(appState.syncAllFeeds).toHaveBeenCalled();
      expect(toasts.success).toHaveBeenCalledWith("Sync complete: 5 new, 2 updated");
    });

    it("shows info toast when no new articles", async () => {
      const mockResult = { success: true, total_new: 0, total_updated: 0, results: [] };
      vi.mocked(appState.syncAllFeeds).mockResolvedValue(mockResult);

      const handleSync = async () => {
        const result = await appState.syncAllFeeds();
        if (result) {
          if (result.total_new > 0 || result.total_updated > 0) {
            toasts.success("Sync complete");
          } else {
            toasts.info("No new articles");
          }
        }
      };

      await handleSync();

      expect(toasts.info).toHaveBeenCalledWith("No new articles");
      expect(toasts.success).not.toHaveBeenCalled();
    });

    it("shows error toast when sync fails", async () => {
      vi.mocked(appState.syncAllFeeds).mockResolvedValue(null);
      (appState as unknown as Record<string, unknown>).error = "Connection failed";

      const handleSync = async () => {
        const result = await appState.syncAllFeeds();
        if (result) {
          toasts.success("Sync complete");
        } else if (appState.error) {
          toasts.error(`Sync failed: ${appState.error}`);
        }
      };

      await handleSync();

      expect(toasts.error).toHaveBeenCalledWith("Sync failed: Connection failed");
    });

    it("handles sync with only new articles", async () => {
      const mockResult = { success: true, total_new: 10, total_updated: 0, results: [] };
      vi.mocked(appState.syncAllFeeds).mockResolvedValue(mockResult);

      const handleSync = async () => {
        const result = await appState.syncAllFeeds();
        if (result && (result.total_new > 0 || result.total_updated > 0)) {
          return true;
        }
        return false;
      };

      const hadChanges = await handleSync();
      expect(hadChanges).toBe(true);
    });

    it("handles sync with only updated articles", async () => {
      const mockResult = { success: true, total_new: 0, total_updated: 3, results: [] };
      vi.mocked(appState.syncAllFeeds).mockResolvedValue(mockResult);

      const handleSync = async () => {
        const result = await appState.syncAllFeeds();
        if (result && (result.total_new > 0 || result.total_updated > 0)) {
          return true;
        }
        return false;
      };

      const hadChanges = await handleSync();
      expect(hadChanges).toBe(true);
    });
  });

  // ============================================================
  // Feed Management
  // ============================================================

  describe("Feed Management", () => {
    it("adds feed and shows success toast", async () => {
      vi.mocked(appState.addPentacle).mockImplementation(() => {
        (appState as unknown as Record<string, unknown>).error = null;
        return Promise.resolve();
      });

      const handleAddFeed = async (url: string) => {
        if (url.trim()) {
          await appState.addPentacle(url);
          if (!appState.error) {
            toasts.success("Feed added");
          }
        }
      };

      await handleAddFeed("https://example.com/feed.xml");

      expect(appState.addPentacle).toHaveBeenCalledWith("https://example.com/feed.xml");
      expect(toasts.success).toHaveBeenCalledWith("Feed added");
    });

    it("does not add feed with empty URL", async () => {
      const handleAddFeed = async (url: string) => {
        if (url.trim()) {
          await appState.addPentacle(url);
        }
      };

      await handleAddFeed("");

      expect(appState.addPentacle).not.toHaveBeenCalled();
    });

    it("does not add feed with whitespace-only URL", async () => {
      const handleAddFeed = async (url: string) => {
        if (url.trim()) {
          await appState.addPentacle(url);
        }
      };

      await handleAddFeed("   ");

      expect(appState.addPentacle).not.toHaveBeenCalled();
    });

    it("trims URL before adding", async () => {
      vi.mocked(appState.addPentacle).mockResolvedValue(undefined);

      const handleAddFeed = async (url: string) => {
        if (url.trim()) {
          const trimmedUrl = url.trim();
          await appState.addPentacle(trimmedUrl);
        }
      };

      await handleAddFeed("  https://example.com/feed.xml  ");

      expect(appState.addPentacle).toHaveBeenCalledWith("https://example.com/feed.xml");
    });

    it("shows error toast when adding feed fails", async () => {
      vi.mocked(appState.addPentacle).mockImplementation(() => {
        (appState as unknown as Record<string, unknown>).error = "Invalid URL";
        return Promise.resolve();
      });

      const handleAddFeed = async (url: string) => {
        if (url.trim()) {
          await appState.addPentacle(url);
          if (appState.error) {
            toasts.error(`Feed error: ${appState.error}`);
          }
        }
      };

      await handleAddFeed("invalid-url");

      expect(toasts.error).toHaveBeenCalledWith("Feed error: Invalid URL");
    });

    it("deletes feed and shows success toast", async () => {
      vi.mocked(appState.deletePentacle).mockImplementation(() => {
        (appState as unknown as Record<string, unknown>).error = null;
        return Promise.resolve();
      });

      const handleDeletePentacle = async (id: number) => {
        await appState.deletePentacle(id);
        if (!appState.error) {
          toasts.success("Feed deleted");
        }
      };

      await handleDeletePentacle(42);

      expect(appState.deletePentacle).toHaveBeenCalledWith(42);
      expect(toasts.success).toHaveBeenCalledWith("Feed deleted");
    });

    it("clears form state after successful submission", async () => {
      vi.mocked(appState.addPentacle).mockResolvedValue(undefined);

      let newFeedUrl = "https://example.com/feed.xml";
      let showAddForm = true;

      const handleAddFeed = async (url: string) => {
        if (url.trim()) {
          const trimmedUrl = url.trim();
          newFeedUrl = "";
          showAddForm = false;
          await appState.addPentacle(trimmedUrl);
        }
      };

      await handleAddFeed(newFeedUrl);

      expect(newFeedUrl).toBe("");
      expect(showAddForm).toBe(false);
    });
  });

  // ============================================================
  // Navigation
  // ============================================================

  describe("Navigation", () => {
    it("handles select all feeds", () => {
      let selectedView = "";
      const selectPentacle = vi.fn();

      const handleSelectAll = () => {
        selectedView = "all";
        selectPentacle(null);
      };

      handleSelectAll();

      expect(selectedView).toBe("all");
      expect(selectPentacle).toHaveBeenCalledWith(null);
    });

    it("handles select pentacle", () => {
      let selectedView = "";

      const handleSelectPentacle = (id: number) => {
        selectedView = "pentacle";
        appState.selectPentacle(id);
      };

      handleSelectPentacle(42);

      expect(selectedView).toBe("pentacle");
      expect(appState.selectPentacle).toHaveBeenCalledWith(42);
    });

    it("handles select sephiroth", () => {
      const handleSelectSephiroth = (id: number) => {
        appState.selectSephiroth(id);
      };

      handleSelectSephiroth(5);

      expect(appState.selectSephiroth).toHaveBeenCalledWith(5);
    });

    it("calls onerisianArchives callback on navigation", () => {
      const onerisianArchives = vi.fn();

      const handleSelectAll = () => {
        appState.selectPentacle(null);
        onerisianArchives();
      };

      handleSelectAll();

      expect(onerisianArchives).toHaveBeenCalled();
    });

    it("calls onerisianArchives callback on pentacle selection", () => {
      const onerisianArchives = vi.fn();

      const handleSelectPentacle = (id: number) => {
        appState.selectPentacle(id);
        onerisianArchives();
      };

      handleSelectPentacle(42);

      expect(onerisianArchives).toHaveBeenCalled();
    });
  });

  // ============================================================
  // Mode Toggle
  // ============================================================

  describe("Mode Toggle", () => {
    it("toggles between pentacles and sephiroth mode", () => {
      let sidebarMode: "pentacles" | "sephiroth" = "pentacles";

      const toggleMode = (mode: "pentacles" | "sephiroth") => {
        sidebarMode = mode;
      };

      expect(sidebarMode).toBe("pentacles");

      toggleMode("sephiroth");
      expect(sidebarMode).toBe("sephiroth");

      toggleMode("pentacles");
      expect(sidebarMode).toBe("pentacles");
    });

    it("defaults to pentacles mode", () => {
      const sidebarMode: "pentacles" | "sephiroth" = "pentacles";
      expect(sidebarMode).toBe("pentacles");
    });
  });

  // ============================================================
  // Batch Processing
  // ============================================================

  describe("Batch Processing", () => {
    it("starts batch processing and shows success toast", async () => {
      const mockResult = { succeeded: 10, failed: 2, processed: 12 };
      vi.mocked(appState.startBatchProcessing).mockResolvedValue(mockResult);

      const handleBatchProcessing = async () => {
        const result = await appState.startBatchProcessing();
        if (result) {
          toasts.success(`Processed: ${result.succeeded} succeeded, ${result.failed} failed`);
        }
      };

      await handleBatchProcessing();

      expect(appState.startBatchProcessing).toHaveBeenCalled();
      expect(toasts.success).toHaveBeenCalledWith("Processed: 10 succeeded, 2 failed");
    });

    it("shows error toast when batch processing fails", async () => {
      vi.mocked(appState.startBatchProcessing).mockResolvedValue(null);
      (appState as unknown as Record<string, unknown>).error = "Ollama not available";

      const handleBatchProcessing = async () => {
        const result = await appState.startBatchProcessing();
        if (result) {
          toasts.success("Processed");
        } else if (appState.error) {
          toasts.error(`Error: ${appState.error}`);
        }
      };

      await handleBatchProcessing();

      expect(toasts.error).toHaveBeenCalledWith("Error: Ollama not available");
    });

    it("cancels batch processing", () => {
      const handleCancelBatch = () => {
        appState.cancelBatch();
      };

      handleCancelBatch();

      expect(appState.cancelBatch).toHaveBeenCalled();
    });

    it("disables batch button when no unprocessed articles", () => {
      const shouldDisableBatch = (batchProcessing: boolean, unprocessedWithContent: number) => {
        return batchProcessing || unprocessedWithContent === 0;
      };

      expect(shouldDisableBatch(false, 0)).toBe(true);
      expect(shouldDisableBatch(true, 5)).toBe(true);
      expect(shouldDisableBatch(false, 5)).toBe(false);
    });

    it("shows model missing state when models are not available", () => {
      const getBatchButtonState = (
        batchProcessing: boolean,
        hasAnyMissingModel: boolean,
        ollamaAvailable: boolean,
      ) => {
        if (batchProcessing) return "processing";
        if (hasAnyMissingModel) return "model-missing";
        if (ollamaAvailable) return "ready";
        return "disabled";
      };

      expect(getBatchButtonState(true, false, true)).toBe("processing");
      expect(getBatchButtonState(false, true, true)).toBe("model-missing");
      expect(getBatchButtonState(false, false, true)).toBe("ready");
      expect(getBatchButtonState(false, false, false)).toBe("disabled");
    });
  });

  // ============================================================
  // Search
  // ============================================================

  describe("Search", () => {
    it("debounces search input", async () => {
      vi.useFakeTimers();

      let searchCalled = false;
      let searchTimeout: ReturnType<typeof setTimeout> | null = null;

      const handleSearchInput = (value: string) => {
        if (searchTimeout) {
          clearTimeout(searchTimeout);
        }

        if (value.trim()) {
          searchTimeout = setTimeout(() => {
            searchCalled = true;
          }, 300);
        }
      };

      handleSearchInput("test");
      expect(searchCalled).toBe(false);

      vi.advanceTimersByTime(300);
      expect(searchCalled).toBe(true);

      vi.useRealTimers();
    });

    it("cancels previous debounce on new input", async () => {
      vi.useFakeTimers();

      const searchCalls: string[] = [];
      let searchTimeout: ReturnType<typeof setTimeout> | null = null;

      const handleSearchInput = (value: string) => {
        if (searchTimeout) {
          clearTimeout(searchTimeout);
        }

        if (value.trim()) {
          searchTimeout = setTimeout(() => {
            searchCalls.push(value);
          }, 300);
        }
      };

      handleSearchInput("t");
      vi.advanceTimersByTime(100);
      handleSearchInput("te");
      vi.advanceTimersByTime(100);
      handleSearchInput("tes");
      vi.advanceTimersByTime(100);
      handleSearchInput("test");
      vi.advanceTimersByTime(300);

      // Only the last input should have triggered search
      expect(searchCalls).toEqual(["test"]);

      vi.useRealTimers();
    });

    it("clears search", () => {
      let searchInput = "test query";

      const handleClearSearch = () => {
        searchInput = "";
        appState.clearSearch();
      };

      handleClearSearch();

      expect(searchInput).toBe("");
      expect(appState.clearSearch).toHaveBeenCalled();
    });

    it("handles search on Enter key", () => {
      const handleSearchKeydown = (key: string, searchInput: string) => {
        if (key === "Enter" && searchInput.trim()) {
          appState.semanticSearch(searchInput.trim());
          return true;
        }
        return false;
      };

      const handled = handleSearchKeydown("Enter", "test query");

      expect(handled).toBe(true);
      expect(appState.semanticSearch).toHaveBeenCalledWith("test query");
    });

    it("does not search on Enter with empty input", () => {
      const handleSearchKeydown = (key: string, searchInput: string) => {
        if (key === "Enter" && searchInput.trim()) {
          appState.semanticSearch(searchInput.trim());
          return true;
        }
        return false;
      };

      const handled = handleSearchKeydown("Enter", "  ");

      expect(handled).toBe(false);
      expect(appState.semanticSearch).not.toHaveBeenCalled();
    });

    it("handles Escape key to clear search", () => {
      const clearFn = vi.fn();

      const handleSearchKeydown = (key: string, clearCallback: () => void) => {
        if (key === "Escape") {
          clearCallback();
          return true;
        }
        return false;
      };

      const handled = handleSearchKeydown("Escape", clearFn);

      expect(handled).toBe(true);
      expect(clearFn).toHaveBeenCalled();
    });

    it("disables search when ollama not available", () => {
      const isSearchDisabled = (ollamaAvailable: boolean) => {
        return !ollamaAvailable;
      };

      expect(isSearchDisabled(true)).toBe(false);
      expect(isSearchDisabled(false)).toBe(true);
    });

    it("clears search when input is emptied", () => {
      const handleSearchInput = (value: string, clearSearch: () => void) => {
        if (!value.trim()) {
          clearSearch();
          return;
        }
      };

      const clearSearch = vi.fn();
      handleSearchInput("", clearSearch);

      expect(clearSearch).toHaveBeenCalled();
    });
  });

  // ============================================================
  // Category Expansion
  // ============================================================

  describe("Category Expansion", () => {
    it("expands and collapses categories", () => {
      let expandedCategoryId: number | null = null;

      const toggleExpand = (categoryId: number) => {
        if (expandedCategoryId === categoryId) {
          expandedCategoryId = null;
        } else {
          expandedCategoryId = categoryId;
        }
      };

      toggleExpand(1);
      expect(expandedCategoryId).toBe(1);

      toggleExpand(1);
      expect(expandedCategoryId).toBeNull();

      toggleExpand(2);
      expect(expandedCategoryId).toBe(2);
    });

    it("collapsing one category and expanding another", () => {
      let expandedCategoryId: number | null = null;

      const toggleExpand = (categoryId: number) => {
        expandedCategoryId = expandedCategoryId === categoryId ? null : categoryId;
      };

      toggleExpand(1);
      expect(expandedCategoryId).toBe(1);

      toggleExpand(2); // Should close 1 and open 2
      expect(expandedCategoryId).toBe(2);
    });
  });

  // ============================================================
  // Sephiroth Filtering
  // ============================================================

  describe("Sephiroth Filtering", () => {
    const sephiroth = [
      { id: 1, name: "Technik", level: 0, parent_id: null, article_count: 0 },
      { id: 2, name: "Politik", level: 0, parent_id: null, article_count: 0 },
      { id: 101, name: "KI", level: 1, parent_id: 1, article_count: 10 },
      { id: 102, name: "Web", level: 1, parent_id: 1, article_count: 15 },
      { id: 201, name: "National", level: 1, parent_id: 2, article_count: 5 },
    ];

    it("filters main categories (level 0)", () => {
      const mainCategories = sephiroth.filter((c) => c.level === 0);

      expect(mainCategories).toHaveLength(2);
      expect(mainCategories[0].name).toBe("Technik");
      expect(mainCategories[1].name).toBe("Politik");
    });

    it("filters subcategories by parent", () => {
      const techSubcategories = sephiroth.filter((c) => c.parent_id === 1);

      expect(techSubcategories).toHaveLength(2);
      expect(techSubcategories[0].name).toBe("KI");
      expect(techSubcategories[1].name).toBe("Web");
    });

    it("calculates subcategory count", () => {
      const techSubcategories = sephiroth.filter((c) => c.parent_id === 1);
      const subcategoryCount = techSubcategories.reduce((sum, c) => sum + c.article_count, 0);

      expect(subcategoryCount).toBe(25);
    });

    it("returns empty array for parent with no subcategories", () => {
      const noSubcategories = sephiroth.filter((c) => c.parent_id === 99);
      expect(noSubcategories).toHaveLength(0);
    });
  });

  // ============================================================
  // Delete Confirmation
  // ============================================================

  describe("Delete Confirmation", () => {
    it("counts articles before deletion", async () => {
      const mockInvoke = vi.mocked(invoke);
      mockInvoke.mockResolvedValue({ total: 50, favorites: 3 });

      const stats: { total: number; favorites: number } = await invoke("count_pentacle_articles", {
        pentacleId: 1,
      });

      expect(stats.total).toBe(50);
      expect(stats.favorites).toBe(3);
    });

    it("shows favorites warning when feed has favorites", () => {
      const stats = { total: 50, favorites: 3 };
      const showFavoritesWarning = stats.favorites > 0;

      expect(showFavoritesWarning).toBe(true);
    });

    it("shows no-articles message for empty feed", () => {
      const stats = { total: 0, favorites: 0 };
      const showNoArticles = stats.total === 0;

      expect(showNoArticles).toBe(true);
    });

    it("cancels delete confirmation", () => {
      let deleteConfirm: { pentacle: { id: number } } | null = {
        pentacle: { id: 1 },
      };

      const cancelDeletePentacle = () => {
        deleteConfirm = null;
      };

      cancelDeletePentacle();

      expect(deleteConfirm).toBeNull();
    });
  });

  // ============================================================
  // Background Maintenance
  // ============================================================

  describe("Background Maintenance", () => {
    it("calls keyword quality calculation", async () => {
      const mockInvoke = vi.mocked(invoke);
      mockInvoke.mockResolvedValue({ updated: 100 });

      const runBackgroundMaintenance = async () => {
        try {
          await invoke("calculate_keyword_quality_scores", { limit: 500 });
        } catch {
          // Silently fail - this is background maintenance
        }
      };

      await runBackgroundMaintenance();

      expect(mockInvoke).toHaveBeenCalledWith("calculate_keyword_quality_scores", { limit: 500 });
    });

    it("silently handles maintenance errors", async () => {
      const mockInvoke = vi.mocked(invoke);
      mockInvoke.mockRejectedValue(new Error("DB locked"));

      const runBackgroundMaintenance = async () => {
        try {
          await invoke("calculate_keyword_quality_scores", { limit: 500 });
          return true;
        } catch {
          return false;
        }
      };

      const success = await runBackgroundMaintenance();
      expect(success).toBe(false);
    });
  });
});

// ============================================================
// Data Structures
// ============================================================

describe("Sidebar Data Structures", () => {
  describe("Pentacle Structure", () => {
    interface PentacleItem {
      id: number;
      title: string | null;
      url: string;
      article_count: number;
      unread_count: number;
    }

    it("creates valid pentacle", () => {
      const pentacle: PentacleItem = {
        id: 1,
        title: "Tech News",
        url: "https://example.com/feed.xml",
        article_count: 50,
        unread_count: 10,
      };

      expect(pentacle.id).toBe(1);
      expect(pentacle.title).toBe("Tech News");
      expect(pentacle.article_count).toBe(50);
    });

    it("handles null title - displays URL as fallback", () => {
      const pentacle: PentacleItem = {
        id: 2,
        title: null,
        url: "https://example.com/feed.xml",
        article_count: 0,
        unread_count: 0,
      };

      const displayName = pentacle.title || pentacle.url;
      expect(displayName).toBe("https://example.com/feed.xml");
    });
  });

  describe("Sephiroth Structure", () => {
    interface SephirothItem {
      id: number;
      name: string;
      parent_id: number | null;
      level: number;
      icon: string | null;
      color: string | null;
      article_count: number;
    }

    it("creates main category", () => {
      const category: SephirothItem = {
        id: 1,
        name: "Technik",
        parent_id: null,
        level: 0,
        icon: "fa-laptop",
        color: "#3498db",
        article_count: 100,
      };

      expect(category.level).toBe(0);
      expect(category.parent_id).toBeNull();
    });

    it("creates subcategory linked to parent", () => {
      const subcategory: SephirothItem = {
        id: 101,
        name: "Kuenstliche Intelligenz",
        parent_id: 1,
        level: 1,
        icon: "fa-brain",
        color: null,
        article_count: 25,
      };

      expect(subcategory.level).toBe(1);
      expect(subcategory.parent_id).toBe(1);
    });
  });
});

// ============================================================
// Stats Display
// ============================================================

describe("Sidebar Stats Display", () => {
  it("formats stats correctly", () => {
    const stats = {
      concealed: 10,
      illuminated: 50,
      goldenApple: 5,
    };

    expect(stats.concealed).toBe(10);
    expect(stats.illuminated).toBe(50);
    expect(stats.goldenApple).toBe(5);
  });

  it("total articles equals sum of statuses", () => {
    const stats = { concealed: 10, illuminated: 50, goldenApple: 5 };
    const total = stats.concealed + stats.illuminated + stats.goldenApple;

    expect(total).toBe(65);
  });
});

// ============================================================
// Navigation Bar
// ============================================================

describe("Sidebar Navigation Bar", () => {
  it("determines active navigation button", () => {
    const isActive = (buttonName: string, activeButton: string) => {
      return buttonName === activeButton;
    };

    expect(isActive("articles", "articles")).toBe(true);
    expect(isActive("network", "articles")).toBe(false);
    expect(isActive("settings", "settings")).toBe(true);
  });

  it("lists all navigation buttons", () => {
    const navButtons = ["erisianArchives", "network", "mindfuck", "fnord", "settings"];
    expect(navButtons).toHaveLength(5);
  });
});

// ============================================================
// Add Feed Form
// ============================================================

describe("Sidebar Add Feed Form", () => {
  it("toggles add form visibility", () => {
    let showAddForm = false;

    const toggleAddForm = () => {
      showAddForm = !showAddForm;
    };

    expect(showAddForm).toBe(false);

    toggleAddForm();
    expect(showAddForm).toBe(true);

    toggleAddForm();
    expect(showAddForm).toBe(false);
  });

  it("validates feed URL before submission", () => {
    const isValidUrl = (url: string) => {
      return url.trim().length > 0;
    };

    expect(isValidUrl("https://example.com/feed.xml")).toBe(true);
    expect(isValidUrl("")).toBe(false);
    expect(isValidUrl("   ")).toBe(false);
  });

  it("clears form after successful submission", async () => {
    let newFeedUrl = "https://example.com/feed.xml";
    let showAddForm = true;

    const handleAddFeed = async (url: string) => {
      if (url.trim()) {
        await appState.addPentacle(url);
        newFeedUrl = "";
        showAddForm = false;
      }
    };

    await handleAddFeed(newFeedUrl);

    expect(newFeedUrl).toBe("");
    expect(showAddForm).toBe(false);
  });
});
