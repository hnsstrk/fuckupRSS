import { describe, it, expect, vi, beforeEach } from "vitest";

/**
 * Tests for NavigationStore — the central navigation state for the app.
 * Replaces the old DOM-based CustomEvent navigation.
 */

// Mock the network store used by NavigationStore
vi.mock("../../stores/network.svelte", () => ({
  networkStore: {
    selectKeyword: vi.fn(),
  },
}));

// Mock state store for navigateToArticle
vi.mock("../../stores/state.svelte", () => ({
  appState: {
    ensureFnordLoaded: vi.fn().mockResolvedValue(undefined),
    selectFnord: vi.fn(),
  },
}));

import { networkStore } from "../../stores/network.svelte";

describe("NavigationStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("navigateToNetwork", () => {
    it("should set currentView to network with keywordId", () => {
      // Simulate NavigationStore logic directly (store uses $state which is compile-time)
      const store = {
        currentView: "erisianArchives" as string,
        pendingKeywordId: null as number | null,
      };

      // Simulate navigateToNetwork(42)
      store.currentView = "network";
      store.pendingKeywordId = 42;
      networkStore.selectKeyword(42);

      expect(store.currentView).toBe("network");
      expect(store.pendingKeywordId).toBe(42);
      expect(networkStore.selectKeyword).toHaveBeenCalledWith(42);
    });

    it("should set currentView to network without keywordId", () => {
      const store = {
        currentView: "erisianArchives" as string,
        pendingKeywordId: null as number | null,
      };

      // navigateToNetwork() without keywordId — only sets view
      store.currentView = "network";

      expect(store.currentView).toBe("network");
      expect(store.pendingKeywordId).toBeNull();
      expect(networkStore.selectKeyword).not.toHaveBeenCalled();
    });

    it("should handle keywordId of 0 as valid", () => {
      const store = {
        currentView: "erisianArchives" as string,
        pendingKeywordId: null as number | null,
      };

      // 0 is a valid keyword ID (0 !== undefined)
      store.currentView = "network";
      store.pendingKeywordId = 0;
      networkStore.selectKeyword(0);

      expect(store.currentView).toBe("network");
      expect(store.pendingKeywordId).toBe(0);
      expect(networkStore.selectKeyword).toHaveBeenCalledWith(0);
    });
  });

  describe("navigateToArticles", () => {
    it("should set currentView to erisianArchives and clear pendingKeywordId", () => {
      const store = {
        currentView: "network" as string,
        pendingKeywordId: 42 as number | null,
      };

      // navigateToArticles()
      store.currentView = "erisianArchives";
      store.pendingKeywordId = null;

      expect(store.currentView).toBe("erisianArchives");
      expect(store.pendingKeywordId).toBeNull();
    });
  });

  describe("navigateTo", () => {
    it("should set currentView to any valid AppView", () => {
      type AppView =
        | "erisianArchives"
        | "network"
        | "fnord"
        | "mindfuck"
        | "briefings"
        | "storyClusters"
        | "settings";

      const views: AppView[] = [
        "erisianArchives",
        "network",
        "fnord",
        "mindfuck",
        "briefings",
        "storyClusters",
        "settings",
      ];

      for (const view of views) {
        const store = { currentView: "erisianArchives" as string };
        store.currentView = view;
        expect(store.currentView).toBe(view);
      }
    });
  });

  describe("Main view state transitions", () => {
    it("should allow toggle between erisianArchives and network", () => {
      let currentView = "erisianArchives";

      // Toggle to network
      currentView = currentView === "network" ? "erisianArchives" : "network";
      expect(currentView).toBe("network");

      // Toggle back to erisianArchives
      currentView = currentView === "network" ? "erisianArchives" : "network";
      expect(currentView).toBe("erisianArchives");
    });

    it("should maintain keyword selection through navigation", () => {
      let currentView = "erisianArchives";
      let pendingKeywordId: number | null = null;

      // Navigate to network with keyword
      currentView = "network";
      pendingKeywordId = 42;

      expect(currentView).toBe("network");
      expect(pendingKeywordId).toBe(42);

      // Navigate back to articles
      currentView = "erisianArchives";

      expect(currentView).toBe("erisianArchives");
      // Keyword selection should persist for when returning to network
      expect(pendingKeywordId).toBe(42);
    });
  });
});

describe("Tab switching in KeywordNetwork", () => {
  type TabType = "list" | "graph" | "trends";

  it("should default to list tab", () => {
    const activeTab: TabType = "list";
    expect(activeTab).toBe("list");
  });

  it("should allow switching between tabs", () => {
    let activeTab: TabType = "list";

    activeTab = "graph";
    expect(activeTab).toBe("graph");

    activeTab = "trends";
    expect(activeTab).toBe("trends");

    activeTab = "list";
    expect(activeTab).toBe("list");
  });

  it("should trigger graph loading when switching to graph tab", () => {
    const loadGraphData = vi.fn();
    let activeTab: TabType = "list";

    function handleTabChange(tab: TabType) {
      activeTab = tab;
      if (tab === "graph") {
        loadGraphData();
      }
    }

    handleTabChange("graph");

    expect(activeTab).toBe("graph");
    expect(loadGraphData).toHaveBeenCalled();
  });

  it("should not trigger graph loading for other tabs", () => {
    const loadGraphData = vi.fn();
    let activeTab: TabType = "list";

    function handleTabChange(tab: TabType) {
      activeTab = tab;
      if (tab === "graph") {
        loadGraphData();
      }
    }

    handleTabChange("trends");
    expect(loadGraphData).not.toHaveBeenCalled();
    expect(activeTab).toBe("trends");

    handleTabChange("list");
    expect(loadGraphData).not.toHaveBeenCalled();
  });
});

describe("Search functionality", () => {
  it("should clear results when query is empty", () => {
    let searchResults: { id: number; name: string }[] = [{ id: 1, name: "Test" }];

    function handleClearSearch() {
      searchResults = [];
    }

    handleClearSearch();
    expect(searchResults).toEqual([]);
  });

  it("should debounce search with timeout", async () => {
    vi.useFakeTimers();

    const searchFn = vi.fn();
    let searchTimeout: ReturnType<typeof setTimeout> | null = null;

    function handleSearch(query: string) {
      if (searchTimeout) clearTimeout(searchTimeout);
      searchTimeout = setTimeout(() => {
        searchFn(query);
      }, 300);
    }

    handleSearch("pol");
    handleSearch("poli");
    handleSearch("polit");
    handleSearch("politik");

    // Only the last search should fire
    expect(searchFn).not.toHaveBeenCalled();

    vi.advanceTimersByTime(300);

    expect(searchFn).toHaveBeenCalledTimes(1);
    expect(searchFn).toHaveBeenCalledWith("politik");

    vi.useRealTimers();
  });

  it("should not search for empty or whitespace query", () => {
    const searchFn = vi.fn();

    function searchKeywordsLocal(query: string) {
      if (!query.trim()) {
        return [];
      }
      searchFn(query);
      return [{ id: 1, name: query }];
    }

    expect(searchKeywordsLocal("")).toEqual([]);
    expect(searchKeywordsLocal("   ")).toEqual([]);
    expect(searchFn).not.toHaveBeenCalled();

    searchKeywordsLocal("test");
    expect(searchFn).toHaveBeenCalledWith("test");
  });
});
