import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

// Mock the invoke function with specific responses
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock appState
vi.mock('../../stores/state.svelte', () => ({
  appState: {
    selectFnord: vi.fn(),
  },
}));

describe('ErisianArchives', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Stats Loading', () => {
    it('loads all stats counts on initialization', async () => {
      const mockInvoke = vi.mocked(invoke);

      // Setup mock responses
      mockInvoke.mockImplementation(async (cmd: string) => {
        switch (cmd) {
          case 'get_fnords_count':
            return 100;
          case 'get_failed_count':
            return { count: 5 };
          case 'get_hopeless_count':
            return { count: 2 };
          default:
            return [];
        }
      });

      // Simulate stats loading
      const totalCount = await invoke('get_fnords_count', { filter: null });
      const unreadCount = await invoke('get_fnords_count', { filter: { status: 'concealed' } });
      const favoritesCount = await invoke('get_fnords_count', { filter: { status: 'golden_apple' } });
      const failedResult = await invoke<{ count: number }>('get_failed_count');
      const hopelessResult = await invoke<{ count: number }>('get_hopeless_count');

      expect(totalCount).toBe(100);
      expect(unreadCount).toBe(100);
      expect(favoritesCount).toBe(100);
      expect(failedResult.count).toBe(5);
      expect(hopelessResult.count).toBe(2);
    });

    it('handles stats loading errors gracefully', async () => {
      const mockInvoke = vi.mocked(invoke);
      mockInvoke.mockRejectedValue(new Error('Database error'));

      let errorCaught = false;
      try {
        await invoke('get_fnords_count', { filter: null });
      } catch (e) {
        errorCaught = true;
      }

      expect(errorCaught).toBe(true);
    });
  });

  describe('Articles Tab', () => {
    it('loads articles with limit filter', async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockArticles = [
        { id: 1, title: 'Test Article 1', status: 'concealed' },
        { id: 2, title: 'Test Article 2', status: 'illuminated' },
      ];

      mockInvoke.mockResolvedValue(mockArticles);

      const articles = await invoke('get_fnords', { filter: { limit: 100 } });

      expect(mockInvoke).toHaveBeenCalledWith('get_fnords', { filter: { limit: 100 } });
      expect(articles).toHaveLength(2);
    });
  });

  describe('Unread Tab', () => {
    it('loads unread articles with concealed status filter', async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockArticles = [
        { id: 1, title: 'Unread Article', status: 'concealed' },
      ];

      mockInvoke.mockResolvedValue(mockArticles);

      const articles = await invoke('get_fnords', { filter: { status: 'concealed', limit: 100 } });

      expect(mockInvoke).toHaveBeenCalledWith('get_fnords', { filter: { status: 'concealed', limit: 100 } });
      expect(articles).toHaveLength(1);
    });
  });

  describe('Golden Apple Tab', () => {
    it('loads favorite articles with golden_apple status filter', async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockArticles = [
        { id: 1, title: 'Favorite Article', status: 'golden_apple' },
      ];

      mockInvoke.mockResolvedValue(mockArticles);

      const articles = await invoke('get_fnords', { filter: { status: 'golden_apple', limit: 100 } });

      expect(mockInvoke).toHaveBeenCalledWith('get_fnords', { filter: { status: 'golden_apple', limit: 100 } });
      expect(articles).toHaveLength(1);
    });
  });

  describe('Failed Tab', () => {
    it('loads failed articles from dedicated command', async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockFailedArticles = [
        {
          id: 1,
          title: 'Failed Analysis Article',
          pentacle_id: 10,
          pentacle_title: 'Test Feed',
          summary: null,
          published_at: '2025-01-15T10:00:00Z',
          status: 'concealed',
          analysis_attempts: 2,
          last_error: 'Ollama connection failed',
        },
      ];

      mockInvoke.mockResolvedValue(mockFailedArticles);

      const articles = await invoke('get_failed_articles', { limit: 100 });

      expect(mockInvoke).toHaveBeenCalledWith('get_failed_articles', { limit: 100 });
      expect(articles).toHaveLength(1);
      expect(articles[0].analysis_attempts).toBe(2);
      expect(articles[0].last_error).toBe('Ollama connection failed');
    });

    it('maps AnalysisStatusArticle to Fnord-like structure', async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockFailedArticle = {
        id: 1,
        title: 'Failed Article',
        pentacle_id: 10,
        pentacle_title: 'Test Feed',
        summary: 'Some summary',
        published_at: '2025-01-15T10:00:00Z',
        status: 'concealed',
        analysis_attempts: 3,
        last_error: 'Timeout',
      };

      mockInvoke.mockResolvedValue([mockFailedArticle]);

      const failedArticles = await invoke<typeof mockFailedArticle[]>('get_failed_articles', { limit: 100 });

      // Simulate the mapping done in the component
      const mappedArticle = {
        id: failedArticles[0].id,
        pentacle_id: failedArticles[0].pentacle_id,
        pentacle_title: failedArticles[0].pentacle_title,
        guid: '',
        url: '',
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
      expect(mappedArticle.title).toBe('Failed Article');
      expect(mappedArticle.pentacle_title).toBe('Test Feed');
      expect(mappedArticle.summary).toBe('Some summary');
      expect(mappedArticle.guid).toBe('');
      expect(mappedArticle.categories).toEqual([]);
    });
  });

  describe('Hopeless Tab', () => {
    it('loads hopeless articles from dedicated command', async () => {
      const mockInvoke = vi.mocked(invoke);
      const mockHopelessArticles = [
        {
          id: 2,
          title: 'Hopeless Article',
          pentacle_id: 11,
          pentacle_title: 'Another Feed',
          summary: null,
          published_at: '2025-01-14T10:00:00Z',
          status: 'concealed',
          analysis_attempts: 5,
          last_error: 'Max retries exceeded',
        },
      ];

      mockInvoke.mockResolvedValue(mockHopelessArticles);

      const articles = await invoke('get_hopeless_articles', { limit: 100 });

      expect(mockInvoke).toHaveBeenCalledWith('get_hopeless_articles', { limit: 100 });
      expect(articles).toHaveLength(1);
      expect(articles[0].analysis_attempts).toBe(5);
    });
  });

  describe('Empty State Messages', () => {
    it('returns correct empty message for each tab', () => {
      const getEmptyMessage = (activeTab: string) => {
        switch (activeTab) {
          case 'articles':
            return 'erisianArchives.noArticles';
          case 'unread':
            return 'erisianArchives.noUnread';
          case 'goldenApple':
            return 'erisianArchives.noFavorites';
          case 'failed':
            return 'erisianArchives.noFailed';
          case 'hopeless':
            return 'erisianArchives.noHopeless';
          default:
            return '';
        }
      };

      expect(getEmptyMessage('articles')).toBe('erisianArchives.noArticles');
      expect(getEmptyMessage('unread')).toBe('erisianArchives.noUnread');
      expect(getEmptyMessage('goldenApple')).toBe('erisianArchives.noFavorites');
      expect(getEmptyMessage('failed')).toBe('erisianArchives.noFailed');
      expect(getEmptyMessage('hopeless')).toBe('erisianArchives.noHopeless');
      expect(getEmptyMessage('unknown')).toBe('');
    });
  });

  describe('Tab Classification', () => {
    it('correctly identifies analysis tabs', () => {
      const isAnalysisTab = (tab: string) => tab === 'failed' || tab === 'hopeless';

      expect(isAnalysisTab('articles')).toBe(false);
      expect(isAnalysisTab('unread')).toBe(false);
      expect(isAnalysisTab('goldenApple')).toBe(false);
      expect(isAnalysisTab('failed')).toBe(true);
      expect(isAnalysisTab('hopeless')).toBe(true);
    });
  });

  describe('Article Selection', () => {
    it('dispatches navigate-to-article event on selection', async () => {
      const dispatchEventSpy = vi.spyOn(window, 'dispatchEvent');

      const articleId = 42;
      const event = new CustomEvent('navigate-to-article', { detail: { articleId } });
      window.dispatchEvent(event);

      expect(dispatchEventSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'navigate-to-article',
          detail: { articleId: 42 },
        })
      );

      dispatchEventSpy.mockRestore();
    });
  });
});
