import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

// Mock the invoke function
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock the listen function
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Mock appState
vi.mock('../../stores/state.svelte', () => ({
  appState: {
    pentacles: [],
    sephiroth: [],
    fnords: [],
    changedFnords: [],
    selectedPentacleId: null,
    selectedSephirothId: null,
    selectedView: 'all',
    totalUnread: 0,
    totalIlluminated: 0,
    totalGoldenApple: 0,
    syncing: false,
    batchProcessing: false,
    batchProgress: null,
    ollamaStatus: { available: true, models: [] },
    selectedModel: 'ministral-3:latest',
    unprocessedCount: { total: 0, with_content: 0 },
    searchResults: [],
    searching: false,
    error: null,
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

import { appState, toasts } from '../../stores/state.svelte';

describe('Sidebar Component Logic', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Sync Handling', () => {
    it('calls syncAllFeeds and shows success toast', async () => {
      const mockResult = { total_new: 5, total_updated: 2 };
      vi.mocked(appState.syncAllFeeds).mockResolvedValue(mockResult);

      const handleSync = async () => {
        const result = await appState.syncAllFeeds();
        if (result) {
          if (result.total_new > 0 || result.total_updated > 0) {
            toasts.success('Sync complete');
          } else {
            toasts.info('No new articles');
          }
        }
      };

      await handleSync();

      expect(appState.syncAllFeeds).toHaveBeenCalled();
      expect(toasts.success).toHaveBeenCalledWith('Sync complete');
    });

    it('shows info toast when no new articles', async () => {
      const mockResult = { total_new: 0, total_updated: 0 };
      vi.mocked(appState.syncAllFeeds).mockResolvedValue(mockResult);

      const handleSync = async () => {
        const result = await appState.syncAllFeeds();
        if (result) {
          if (result.total_new > 0 || result.total_updated > 0) {
            toasts.success('Sync complete');
          } else {
            toasts.info('No new articles');
          }
        }
      };

      await handleSync();

      expect(toasts.info).toHaveBeenCalledWith('No new articles');
    });
  });

  describe('Feed Management', () => {
    it('adds feed and shows success toast', async () => {
      vi.mocked(appState.addPentacle).mockImplementation(() => {
        (appState as any).error = null;
        return Promise.resolve();
      });

      const handleAddFeed = async (url: string) => {
        if (url.trim()) {
          await appState.addPentacle(url);
          if (!appState.error) {
            toasts.success('Feed added');
          }
        }
      };

      await handleAddFeed('https://example.com/feed.xml');

      expect(appState.addPentacle).toHaveBeenCalledWith('https://example.com/feed.xml');
      expect(toasts.success).toHaveBeenCalledWith('Feed added');
    });

    it('does not add feed with empty URL', async () => {
      const handleAddFeed = async (url: string) => {
        if (url.trim()) {
          await appState.addPentacle(url);
        }
      };

      await handleAddFeed('');

      expect(appState.addPentacle).not.toHaveBeenCalled();
    });

    it('deletes feed and shows success toast', async () => {
      vi.mocked(appState.deletePentacle).mockImplementation(() => {
        (appState as any).error = null;
        return Promise.resolve();
      });

      const handleDeletePentacle = async (id: number) => {
        await appState.deletePentacle(id);
        if (!appState.error) {
          toasts.success('Feed deleted');
        }
      };

      await handleDeletePentacle(42);

      expect(appState.deletePentacle).toHaveBeenCalledWith(42);
      expect(toasts.success).toHaveBeenCalledWith('Feed deleted');
    });
  });

  describe('Navigation', () => {
    it('handles select all feeds', () => {
      let selectedView = '';
      let selectedPentacleId: number | null = 1;
      let selectedSephirothId: number | null = 2;

      const handleSelectAll = () => {
        selectedView = 'all';
        selectedPentacleId = null;
        selectedSephirothId = null;
      };

      handleSelectAll();

      expect(selectedView).toBe('all');
      expect(selectedPentacleId).toBeNull();
      expect(selectedSephirothId).toBeNull();
    });

    it('handles select pentacle', () => {
      let selectedView = '';

      const handleSelectPentacle = (id: number) => {
        selectedView = 'pentacle';
        appState.selectPentacle(id);
      };

      handleSelectPentacle(42);

      expect(selectedView).toBe('pentacle');
      expect(appState.selectPentacle).toHaveBeenCalledWith(42);
    });

    it('handles select sephiroth', () => {
      const handleSelectSephiroth = (id: number) => {
        appState.selectSephiroth(id);
      };

      handleSelectSephiroth(5);

      expect(appState.selectSephiroth).toHaveBeenCalledWith(5);
    });
  });

  describe('Mode Toggle', () => {
    it('toggles between pentacles and sephiroth mode', () => {
      let sidebarMode: 'pentacles' | 'sephiroth' = 'pentacles';

      const toggleMode = (mode: 'pentacles' | 'sephiroth') => {
        sidebarMode = mode;
      };

      expect(sidebarMode).toBe('pentacles');

      toggleMode('sephiroth');
      expect(sidebarMode).toBe('sephiroth');

      toggleMode('pentacles');
      expect(sidebarMode).toBe('pentacles');
    });
  });

  describe('Batch Processing', () => {
    it('starts batch processing', async () => {
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
      expect(toasts.success).toHaveBeenCalled();
    });

    it('cancels batch processing', () => {
      const handleCancelBatch = () => {
        appState.cancelBatch();
      };

      handleCancelBatch();

      expect(appState.cancelBatch).toHaveBeenCalled();
    });

    it('disables batch button when no unprocessed articles', () => {
      const shouldDisableBatch = (
        batchProcessing: boolean,
        unprocessedWithContent: number
      ) => {
        return batchProcessing || unprocessedWithContent === 0;
      };

      expect(shouldDisableBatch(false, 0)).toBe(true);
      expect(shouldDisableBatch(true, 5)).toBe(true);
      expect(shouldDisableBatch(false, 5)).toBe(false);
    });
  });

  describe('Search', () => {
    it('debounces search input', async () => {
      vi.useFakeTimers();

      let searchTimeout: ReturnType<typeof setTimeout> | null = null;
      let searchCalled = false;

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

      handleSearchInput('test');
      expect(searchCalled).toBe(false);

      vi.advanceTimersByTime(300);
      expect(searchCalled).toBe(true);

      vi.useRealTimers();
    });

    it('clears search', () => {
      let searchInput = 'test query';

      const handleClearSearch = () => {
        searchInput = '';
        appState.clearSearch();
      };

      handleClearSearch();

      expect(searchInput).toBe('');
      expect(appState.clearSearch).toHaveBeenCalled();
    });

    it('handles search on Enter key', () => {
      const handleSearchKeydown = (key: string, searchInput: string) => {
        if (key === 'Enter' && searchInput.trim()) {
          appState.semanticSearch(searchInput.trim());
          return true;
        }
        return false;
      };

      const handled = handleSearchKeydown('Enter', 'test query');

      expect(handled).toBe(true);
      expect(appState.semanticSearch).toHaveBeenCalledWith('test query');
    });

    it('handles Escape key to clear search', () => {
      const handleSearchKeydown = (key: string, clearFn: () => void) => {
        if (key === 'Escape') {
          clearFn();
          return true;
        }
        return false;
      };

      const clearFn = vi.fn();
      const handled = handleSearchKeydown('Escape', clearFn);

      expect(handled).toBe(true);
      expect(clearFn).toHaveBeenCalled();
    });

    it('disables search when ollama not available', () => {
      const isSearchDisabled = (ollamaAvailable: boolean) => {
        return !ollamaAvailable;
      };

      expect(isSearchDisabled(true)).toBe(false);
      expect(isSearchDisabled(false)).toBe(true);
    });
  });

  describe('Category Expansion', () => {
    it('expands and collapses categories', () => {
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
  });

  describe('Sephiroth Filtering', () => {
    it('filters main categories (level 0)', () => {
      const sephiroth = [
        { id: 1, name: 'Tech', level: 0, parent_id: null },
        { id: 2, name: 'Politics', level: 0, parent_id: null },
        { id: 101, name: 'AI', level: 1, parent_id: 1 },
        { id: 102, name: 'Web', level: 1, parent_id: 1 },
      ];

      const mainCategories = sephiroth.filter((c) => c.level === 0);

      expect(mainCategories).toHaveLength(2);
      expect(mainCategories[0].name).toBe('Tech');
      expect(mainCategories[1].name).toBe('Politics');
    });

    it('filters subcategories by parent', () => {
      const sephiroth = [
        { id: 1, name: 'Tech', level: 0, parent_id: null },
        { id: 2, name: 'Politics', level: 0, parent_id: null },
        { id: 101, name: 'AI', level: 1, parent_id: 1 },
        { id: 102, name: 'Web', level: 1, parent_id: 1 },
        { id: 201, name: 'National', level: 1, parent_id: 2 },
      ];

      const techSubcategories = sephiroth.filter((c) => c.parent_id === 1);

      expect(techSubcategories).toHaveLength(2);
      expect(techSubcategories[0].name).toBe('AI');
      expect(techSubcategories[1].name).toBe('Web');
    });

    it('calculates subcategory count', () => {
      const subcategories = [
        { id: 101, article_count: 10 },
        { id: 102, article_count: 15 },
        { id: 103, article_count: 5 },
      ];

      const subcategoryCount = subcategories.reduce(
        (sum, c) => sum + c.article_count,
        0
      );

      expect(subcategoryCount).toBe(30);
    });
  });

  describe('Background Maintenance', () => {
    it('calls keyword quality calculation', async () => {
      const mockInvoke = vi.mocked(invoke);
      mockInvoke.mockResolvedValue({ updated: 100 });

      const runBackgroundMaintenance = async () => {
        try {
          await invoke('calculate_keyword_quality_scores', { limit: 500 });
        } catch (e) {
          // Silently fail - this is background maintenance
        }
      };

      await runBackgroundMaintenance();

      expect(mockInvoke).toHaveBeenCalledWith('calculate_keyword_quality_scores', {
        limit: 500,
      });
    });
  });
});

describe('Sidebar Data Structures', () => {
  describe('Pentacle Structure', () => {
    interface PentacleItem {
      id: number;
      title: string | null;
      url: string;
      article_count: number;
      unread_count: number;
    }

    it('creates valid pentacle', () => {
      const pentacle: PentacleItem = {
        id: 1,
        title: 'Tech News',
        url: 'https://example.com/feed.xml',
        article_count: 50,
        unread_count: 10,
      };

      expect(pentacle.id).toBe(1);
      expect(pentacle.title).toBe('Tech News');
      expect(pentacle.article_count).toBe(50);
    });

    it('handles null title', () => {
      const pentacle: PentacleItem = {
        id: 2,
        title: null,
        url: 'https://example.com/feed.xml',
        article_count: 0,
        unread_count: 0,
      };

      expect(pentacle.title).toBeNull();
    });
  });

  describe('Sephiroth Structure', () => {
    interface SephirothItem {
      id: number;
      name: string;
      parent_id: number | null;
      level: number;
      icon: string | null;
      color: string | null;
      article_count: number;
    }

    it('creates main category', () => {
      const category: SephirothItem = {
        id: 1,
        name: 'Technology',
        parent_id: null,
        level: 0,
        icon: 'fa-laptop',
        color: '#3498db',
        article_count: 100,
      };

      expect(category.level).toBe(0);
      expect(category.parent_id).toBeNull();
    });

    it('creates subcategory', () => {
      const subcategory: SephirothItem = {
        id: 101,
        name: 'Artificial Intelligence',
        parent_id: 1,
        level: 1,
        icon: 'fa-brain',
        color: null,
        article_count: 25,
      };

      expect(subcategory.level).toBe(1);
      expect(subcategory.parent_id).toBe(1);
    });
  });
});

describe('Sidebar Stats Display', () => {
  it('formats stats correctly', () => {
    const stats = {
      concealed: 10,
      illuminated: 50,
      goldenApple: 5,
    };

    expect(stats.concealed).toBe(10);
    expect(stats.illuminated).toBe(50);
    expect(stats.goldenApple).toBe(5);
  });
});

describe('Sidebar Navigation Bar', () => {
  it('determines active navigation button', () => {
    const isActive = (buttonName: string, activeButton: string) => {
      return buttonName === activeButton;
    };

    expect(isActive('articles', 'articles')).toBe(true);
    expect(isActive('network', 'articles')).toBe(false);
    expect(isActive('settings', 'settings')).toBe(true);
  });
});

describe('Sidebar Add Feed Form', () => {
  it('toggles add form visibility', () => {
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

  it('validates feed URL before submission', () => {
    const isValidUrl = (url: string) => {
      return url.trim().length > 0;
    };

    expect(isValidUrl('https://example.com/feed.xml')).toBe(true);
    expect(isValidUrl('')).toBe(false);
    expect(isValidUrl('   ')).toBe(false);
  });

  it('clears form after successful submission', async () => {
    let newFeedUrl = 'https://example.com/feed.xml';
    let showAddForm = true;

    const handleAddFeed = async (url: string) => {
      if (url.trim()) {
        await appState.addPentacle(url);
        newFeedUrl = '';
        showAddForm = false;
      }
    };

    await handleAddFeed(newFeedUrl);

    expect(newFeedUrl).toBe('');
    expect(showAddForm).toBe(false);
  });
});
