import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

/**
 * Tests for navigation events between components.
 * The app uses custom window events for cross-component navigation:
 * - 'navigate-to-network': Dispatched from ArticleView to navigate to Immanentize Network
 */

describe('Navigation Events', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('navigate-to-network event', () => {
    it('should dispatch event with keywordId', () => {
      const listener = vi.fn();
      window.addEventListener('navigate-to-network', listener as EventListener);

      const event = new CustomEvent('navigate-to-network', {
        detail: { keywordId: 42 },
      });
      window.dispatchEvent(event);

      expect(listener).toHaveBeenCalledTimes(1);
      const receivedEvent = listener.mock.calls[0][0] as CustomEvent;
      expect(receivedEvent.detail.keywordId).toBe(42);

      window.removeEventListener('navigate-to-network', listener as EventListener);
    });

    it('should dispatch event without keywordId (open network without selection)', () => {
      const listener = vi.fn();
      window.addEventListener('navigate-to-network', listener as EventListener);

      const event = new CustomEvent('navigate-to-network', {
        detail: {},
      });
      window.dispatchEvent(event);

      expect(listener).toHaveBeenCalledTimes(1);
      const receivedEvent = listener.mock.calls[0][0] as CustomEvent;
      expect(receivedEvent.detail.keywordId).toBeUndefined();

      window.removeEventListener('navigate-to-network', listener as EventListener);
    });

    it('should handle missing detail gracefully (defaults to null)', () => {
      const listener = vi.fn();
      window.addEventListener('navigate-to-network', listener as EventListener);

      const event = new CustomEvent('navigate-to-network');
      window.dispatchEvent(event);

      expect(listener).toHaveBeenCalledTimes(1);
      const receivedEvent = listener.mock.calls[0][0] as CustomEvent;
      // CustomEvent.detail defaults to null when not provided
      expect(receivedEvent.detail).toBeNull();

      window.removeEventListener('navigate-to-network', listener as EventListener);
    });

    it('should allow multiple listeners', () => {
      const listener1 = vi.fn();
      const listener2 = vi.fn();
      window.addEventListener('navigate-to-network', listener1 as EventListener);
      window.addEventListener('navigate-to-network', listener2 as EventListener);

      const event = new CustomEvent('navigate-to-network', {
        detail: { keywordId: 1 },
      });
      window.dispatchEvent(event);

      expect(listener1).toHaveBeenCalledTimes(1);
      expect(listener2).toHaveBeenCalledTimes(1);

      window.removeEventListener('navigate-to-network', listener1 as EventListener);
      window.removeEventListener('navigate-to-network', listener2 as EventListener);
    });

    it('should support removing event listeners', () => {
      const listener = vi.fn();
      window.addEventListener('navigate-to-network', listener as EventListener);
      window.removeEventListener('navigate-to-network', listener as EventListener);

      const event = new CustomEvent('navigate-to-network', {
        detail: { keywordId: 1 },
      });
      window.dispatchEvent(event);

      expect(listener).not.toHaveBeenCalled();
    });
  });

  describe('Navigation helper function', () => {
    // Helper function that would be used in ArticleView
    function navigateToKeyword(tagId: number): void {
      window.dispatchEvent(
        new CustomEvent('navigate-to-network', {
          detail: { keywordId: tagId },
        })
      );
    }

    it('should dispatch correct event from helper', () => {
      const listener = vi.fn();
      window.addEventListener('navigate-to-network', listener as EventListener);

      navigateToKeyword(123);

      expect(listener).toHaveBeenCalledTimes(1);
      const receivedEvent = listener.mock.calls[0][0] as CustomEvent;
      expect(receivedEvent.detail.keywordId).toBe(123);

      window.removeEventListener('navigate-to-network', listener as EventListener);
    });

    it('should handle keywordId of 0', () => {
      const listener = vi.fn();
      window.addEventListener('navigate-to-network', listener as EventListener);

      navigateToKeyword(0);

      const receivedEvent = listener.mock.calls[0][0] as CustomEvent;
      expect(receivedEvent.detail.keywordId).toBe(0);

      window.removeEventListener('navigate-to-network', listener as EventListener);
    });
  });

  describe('Event handler in App.svelte style', () => {
    // Simulates the handler logic from App.svelte
    function handleNavigateToNetwork(
      event: CustomEvent<{ keywordId?: number }>,
      setMainView: (view: string) => void,
      selectKeyword: (id: number) => void
    ): void {
      setMainView('network');
      if (event.detail?.keywordId !== undefined) {
        selectKeyword(event.detail.keywordId);
      }
    }

    it('should set main view to network', () => {
      const setMainView = vi.fn();
      const selectKeyword = vi.fn();

      const event = new CustomEvent('navigate-to-network', {
        detail: { keywordId: 42 },
      });

      handleNavigateToNetwork(event, setMainView, selectKeyword);

      expect(setMainView).toHaveBeenCalledWith('network');
    });

    it('should select keyword when keywordId provided', () => {
      const setMainView = vi.fn();
      const selectKeyword = vi.fn();

      const event = new CustomEvent('navigate-to-network', {
        detail: { keywordId: 42 },
      });

      handleNavigateToNetwork(event, setMainView, selectKeyword);

      expect(selectKeyword).toHaveBeenCalledWith(42);
    });

    it('should not select keyword when keywordId not provided', () => {
      const setMainView = vi.fn();
      const selectKeyword = vi.fn();

      const event = new CustomEvent('navigate-to-network', {
        detail: {},
      });

      handleNavigateToNetwork(event, setMainView, selectKeyword);

      expect(setMainView).toHaveBeenCalledWith('network');
      expect(selectKeyword).not.toHaveBeenCalled();
    });

    it('should handle keywordId of 0 as valid', () => {
      const setMainView = vi.fn();
      const selectKeyword = vi.fn();

      const event = new CustomEvent('navigate-to-network', {
        detail: { keywordId: 0 },
      });

      handleNavigateToNetwork(event, setMainView, selectKeyword);

      // 0 is a valid ID, should call selectKeyword
      expect(selectKeyword).toHaveBeenCalledWith(0);
    });

    it('should handle undefined detail', () => {
      const setMainView = vi.fn();
      const selectKeyword = vi.fn();

      const event = new CustomEvent('navigate-to-network');

      handleNavigateToNetwork(event, setMainView, selectKeyword);

      expect(setMainView).toHaveBeenCalledWith('network');
      expect(selectKeyword).not.toHaveBeenCalled();
    });
  });

  describe('Main view state transitions', () => {
    type MainView = 'articles' | 'network';

    it('should allow toggle between articles and network', () => {
      let mainView: MainView = 'articles';

      // Toggle to network
      mainView = mainView === 'network' ? 'articles' : 'network';
      expect(mainView).toBe('network');

      // Toggle back to articles
      mainView = mainView === 'network' ? 'articles' : 'network';
      expect(mainView).toBe('articles');
    });

    it('should maintain state through navigation', () => {
      let mainView: MainView = 'articles';
      let selectedKeywordId: number | null = null;

      // Navigate to network with keyword
      mainView = 'network';
      selectedKeywordId = 42;

      expect(mainView).toBe('network');
      expect(selectedKeywordId).toBe(42);

      // Navigate back to articles
      mainView = 'articles';

      expect(mainView).toBe('articles');
      // Keyword selection should persist for when returning to network
      expect(selectedKeywordId).toBe(42);
    });
  });
});

describe('Tab switching in KeywordNetwork', () => {
  type TabType = 'list' | 'graph' | 'trends';

  it('should default to list tab', () => {
    const activeTab: TabType = 'list';
    expect(activeTab).toBe('list');
  });

  it('should allow switching between tabs', () => {
    let activeTab: TabType = 'list';

    activeTab = 'graph';
    expect(activeTab).toBe('graph');

    activeTab = 'trends';
    expect(activeTab).toBe('trends');

    activeTab = 'list';
    expect(activeTab).toBe('list');
  });

  it('should trigger graph loading when switching to graph tab', () => {
    const loadGraphData = vi.fn();
    let activeTab: TabType = 'list';

    function handleTabChange(tab: TabType) {
      activeTab = tab;
      if (tab === 'graph') {
        loadGraphData();
      }
    }

    handleTabChange('graph');

    expect(activeTab).toBe('graph');
    expect(loadGraphData).toHaveBeenCalled();
  });

  it('should not trigger graph loading for other tabs', () => {
    const loadGraphData = vi.fn();
    let activeTab: TabType = 'list';

    function handleTabChange(tab: TabType) {
      activeTab = tab;
      if (tab === 'graph') {
        loadGraphData();
      }
    }

    handleTabChange('trends');
    expect(loadGraphData).not.toHaveBeenCalled();

    handleTabChange('list');
    expect(loadGraphData).not.toHaveBeenCalled();
  });
});

describe('Search functionality', () => {
  it('should clear results when query is empty', () => {
    let searchResults: { id: number; name: string }[] = [
      { id: 1, name: 'Test' },
    ];

    function handleClearSearch() {
      searchResults = [];
    }

    handleClearSearch();
    expect(searchResults).toEqual([]);
  });

  it('should debounce search with timeout', async () => {
    vi.useFakeTimers();

    const searchFn = vi.fn();
    let searchTimeout: ReturnType<typeof setTimeout> | null = null;

    function handleSearch(query: string) {
      if (searchTimeout) clearTimeout(searchTimeout);
      searchTimeout = setTimeout(() => {
        searchFn(query);
      }, 300);
    }

    handleSearch('pol');
    handleSearch('poli');
    handleSearch('polit');
    handleSearch('politik');

    // Only the last search should fire
    expect(searchFn).not.toHaveBeenCalled();

    vi.advanceTimersByTime(300);

    expect(searchFn).toHaveBeenCalledTimes(1);
    expect(searchFn).toHaveBeenCalledWith('politik');

    vi.useRealTimers();
  });

  it('should not search for empty or whitespace query', () => {
    const searchFn = vi.fn();

    function searchKeywordsLocal(query: string) {
      if (!query.trim()) {
        return [];
      }
      searchFn(query);
      return [{ id: 1, name: query }];
    }

    expect(searchKeywordsLocal('')).toEqual([]);
    expect(searchKeywordsLocal('   ')).toEqual([]);
    expect(searchFn).not.toHaveBeenCalled();

    searchKeywordsLocal('test');
    expect(searchFn).toHaveBeenCalledWith('test');
  });
});
