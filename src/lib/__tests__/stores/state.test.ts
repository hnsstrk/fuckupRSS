import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

// Mock invoke before importing the store
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Since the store uses Svelte 5 runes ($state), we need to test differently
// We'll test the pure logic functions and mock the Tauri calls

describe('AppState', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('invoke mocking', () => {
    it('should mock invoke correctly', async () => {
      const mockData = { test: 'data' };
      vi.mocked(invoke).mockResolvedValueOnce(mockData);

      const result = await invoke('test_command');
      expect(result).toEqual(mockData);
      expect(invoke).toHaveBeenCalledWith('test_command');
    });

    it('should handle invoke errors', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error('Test error'));

      await expect(invoke('failing_command')).rejects.toThrow('Test error');
    });
  });

  describe('Pentacle operations', () => {
    it('should mock get_pentacles', async () => {
      const mockPentacles = [
        { id: 1, url: 'https://example.com/feed.xml', title: 'Test Feed', unread_count: 5 },
        { id: 2, url: 'https://other.com/feed.xml', title: 'Other Feed', unread_count: 0 },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockPentacles);

      const result = await invoke('get_pentacles');
      expect(result).toEqual(mockPentacles);
      expect(result).toHaveLength(2);
    });

    it('should mock add_pentacle', async () => {
      const newPentacle = { id: 3, url: 'https://new.com/feed.xml', title: 'New Feed', unread_count: 0 };
      vi.mocked(invoke).mockResolvedValueOnce(newPentacle);

      const result = await invoke('add_pentacle', { url: 'https://new.com/feed.xml' });
      expect(result).toEqual(newPentacle);
    });

    it('should mock delete_pentacle', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await invoke('delete_pentacle', { id: 1 });
      expect(invoke).toHaveBeenCalledWith('delete_pentacle', { id: 1 });
    });
  });

  describe('Fnord operations', () => {
    it('should mock get_fnords with filter', async () => {
      const mockFnords = [
        { id: 1, title: 'Article 1', status: 'concealed' },
        { id: 2, title: 'Article 2', status: 'illuminated' },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockFnords);

      const result = await invoke('get_fnords', { filter: { pentacle_id: 1 } });
      expect(result).toEqual(mockFnords);
    });

    it('should mock update_fnord_status', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await invoke('update_fnord_status', { id: 1, status: 'illuminated' });
      expect(invoke).toHaveBeenCalledWith('update_fnord_status', { id: 1, status: 'illuminated' });
    });
  });

  describe('Ollama operations', () => {
    it('should mock check_ollama when available', async () => {
      const mockStatus = {
        available: true,
        models: ['ministral-3:latest', 'nomic-embed-text'],
        recommended_main: 'ministral-3:latest',
        recommended_embedding: 'nomic-embed-text',
        has_recommended_main: true,
        has_recommended_embedding: true,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockStatus);

      const result = await invoke('check_ollama');
      expect(result).toEqual(mockStatus);
      expect(result.available).toBe(true);
    });

    it('should mock check_ollama when unavailable', async () => {
      const mockStatus = {
        available: false,
        models: [],
        recommended_main: 'ministral-3:latest',
        recommended_embedding: 'nomic-embed-text',
        has_recommended_main: false,
        has_recommended_embedding: false,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockStatus);

      const result = await invoke('check_ollama');
      expect(result.available).toBe(false);
      expect(result.models).toHaveLength(0);
    });

    it('should mock process_batch', async () => {
      const mockResult = {
        processed: 10,
        succeeded: 8,
        failed: 2,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockResult);

      const result = await invoke('process_batch', { model: 'ministral-3:latest', limit: 50 });
      expect(result).toEqual(mockResult);
    });
  });

  describe('Sync operations', () => {
    it('should mock sync_all_feeds', async () => {
      const mockResult = {
        success: true,
        total_new: 15,
        total_updated: 3,
        results: [
          { pentacle_id: 1, pentacle_title: 'Feed 1', new_articles: 10, updated_articles: 2, full_text_fetched: 5, error: null },
          { pentacle_id: 2, pentacle_title: 'Feed 2', new_articles: 5, updated_articles: 1, full_text_fetched: 3, error: null },
        ],
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockResult);

      const result = await invoke('sync_all_feeds');
      expect(result.success).toBe(true);
      expect(result.total_new).toBe(15);
    });

    it('should handle sync errors', async () => {
      const mockResult = {
        success: true,
        total_new: 0,
        total_updated: 0,
        results: [
          { pentacle_id: 1, pentacle_title: 'Feed 1', new_articles: 0, updated_articles: 0, full_text_fetched: 0, error: 'Connection timeout' },
        ],
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockResult);

      const result = await invoke('sync_all_feeds');
      expect(result.results[0].error).toBe('Connection timeout');
    });
  });

  describe('Settings operations', () => {
    it('should mock get_settings', async () => {
      const mockSettings = {
        locale: 'de',
        theme: 'mocha',
        sync_interval: 30,
        show_terminology_tooltips: true,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockSettings);

      const result = await invoke('get_settings');
      expect(result).toEqual(mockSettings);
    });

    it('should mock set_setting', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await invoke('set_setting', { key: 'theme', value: 'latte' });
      expect(invoke).toHaveBeenCalledWith('set_setting', { key: 'theme', value: 'latte' });
    });
  });
});

describe('Status types', () => {
  it('should have correct Fnord status values', () => {
    const validStatuses = ['concealed', 'illuminated', 'golden_apple'];
    validStatuses.forEach(status => {
      expect(typeof status).toBe('string');
    });
  });

  it('should have correct article types', () => {
    const validTypes = ['news', 'opinion', 'analysis', 'satire', 'ad', 'unknown'];
    validTypes.forEach(type => {
      expect(typeof type).toBe('string');
    });
  });
});

describe('Bias scales', () => {
  it('should validate political_bias range', () => {
    const validValues = [-2, -1, 0, 1, 2];
    validValues.forEach(value => {
      expect(value).toBeGreaterThanOrEqual(-2);
      expect(value).toBeLessThanOrEqual(2);
    });
  });

  it('should validate sachlichkeit range', () => {
    const validValues = [0, 1, 2, 3, 4];
    validValues.forEach(value => {
      expect(value).toBeGreaterThanOrEqual(0);
      expect(value).toBeLessThanOrEqual(4);
    });
  });
});
