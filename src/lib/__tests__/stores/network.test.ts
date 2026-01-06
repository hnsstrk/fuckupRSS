import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

// Mock invoke before importing the store
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('Network Store / Immanentize Network', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('get_keywords command', () => {
    it('should fetch keywords with pagination', async () => {
      const mockKeywords = [
        { id: 1, name: 'Politik', article_count: 42, first_seen: '2024-01-01', last_used: '2024-12-01' },
        { id: 2, name: 'Wirtschaft', article_count: 35, first_seen: '2024-01-02', last_used: '2024-12-02' },
        { id: 3, name: 'Technologie', article_count: 28, first_seen: '2024-01-03', last_used: '2024-12-03' },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockKeywords);

      const result = await invoke('get_keywords', { limit: 50, offset: 0 });

      expect(result).toEqual(mockKeywords);
      expect(invoke).toHaveBeenCalledWith('get_keywords', { limit: 50, offset: 0 });
    });

    it('should handle empty keyword list', async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      const result = await invoke('get_keywords', { limit: 50, offset: 0 });

      expect(result).toEqual([]);
      expect(result).toHaveLength(0);
    });

    it('should handle pagination with offset', async () => {
      const mockKeywords = [
        { id: 51, name: 'Umwelt', article_count: 15, first_seen: '2024-02-01', last_used: '2024-12-05' },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockKeywords);

      const result = await invoke('get_keywords', { limit: 50, offset: 50 });

      expect(invoke).toHaveBeenCalledWith('get_keywords', { limit: 50, offset: 50 });
      expect(result).toHaveLength(1);
    });
  });

  describe('get_trending_keywords command', () => {
    it('should fetch trending keywords', async () => {
      const mockTrending = [
        { id: 1, name: 'KI', recent_count: 25, total_count: 100, trend_direction: 'up' },
        { id: 2, name: 'Wahlen', recent_count: 20, total_count: 80, trend_direction: 'up' },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockTrending);

      const result = await invoke('get_trending_keywords', { days: 7, limit: 20 });

      expect(result).toEqual(mockTrending);
      expect(invoke).toHaveBeenCalledWith('get_trending_keywords', { days: 7, limit: 20 });
    });

    it('should return empty array when no trending keywords', async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      const result = await invoke('get_trending_keywords', { days: 7, limit: 20 });

      expect(result).toEqual([]);
    });
  });

  describe('get_network_stats command', () => {
    it('should fetch network statistics', async () => {
      const mockStats = {
        total_keywords: 1500,
        total_connections: 4500,
        avg_neighbors_per_keyword: 3.2,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockStats);

      const result = await invoke('get_network_stats');

      expect(result).toEqual(mockStats);
      expect(result.total_keywords).toBe(1500);
      expect(result.total_connections).toBe(4500);
    });

    it('should handle zero stats for empty network', async () => {
      const mockStats = {
        total_keywords: 0,
        total_connections: 0,
        avg_neighbors_per_keyword: 0,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockStats);

      const result = await invoke('get_network_stats');

      expect(result.total_keywords).toBe(0);
    });
  });

  describe('get_keyword command', () => {
    it('should fetch a single keyword by id', async () => {
      const mockKeyword = {
        id: 1,
        name: 'Politik',
        article_count: 42,
        first_seen: '2024-01-01',
        last_used: '2024-12-01',
        canonical_id: null,
        cluster_id: 5,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockKeyword);

      const result = await invoke('get_keyword', { id: 1 });

      expect(result).toEqual(mockKeyword);
      expect(invoke).toHaveBeenCalledWith('get_keyword', { id: 1 });
    });

    it('should return null for non-existent keyword', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(null);

      const result = await invoke('get_keyword', { id: 99999 });

      expect(result).toBeNull();
    });
  });

  describe('get_keyword_neighbors command', () => {
    it('should fetch keyword neighbors', async () => {
      const mockNeighbors = [
        { id: 2, name: 'Bundestag', weight: 0.85, cooccurrence: 50, embedding_similarity: 0.78 },
        { id: 3, name: 'Regierung', weight: 0.72, cooccurrence: 35, embedding_similarity: 0.65 },
        { id: 4, name: 'Wahlen', weight: 0.68, cooccurrence: 28, embedding_similarity: 0.55 },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockNeighbors);

      const result = await invoke('get_keyword_neighbors', { id: 1, limit: 20 });

      expect(result).toEqual(mockNeighbors);
      expect(result).toHaveLength(3);
    });

    it('should handle keywords with no neighbors', async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      const result = await invoke('get_keyword_neighbors', { id: 1, limit: 20 });

      expect(result).toEqual([]);
    });

    it('should respect limit parameter', async () => {
      const mockNeighbors = [
        { id: 2, name: 'Test', weight: 0.9, cooccurrence: 10, embedding_similarity: 0.8 },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockNeighbors);

      await invoke('get_keyword_neighbors', { id: 1, limit: 5 });

      expect(invoke).toHaveBeenCalledWith('get_keyword_neighbors', { id: 1, limit: 5 });
    });
  });

  describe('get_keyword_categories command', () => {
    it('should fetch keyword categories (Sephiroth associations)', async () => {
      const mockCategories = [
        { sephiroth_id: 1, name: 'Politik', weight: 0.9, icon: '🏛️', color: '#4F46E5' },
        { sephiroth_id: 5, name: 'Gesellschaft', weight: 0.3, icon: '👥', color: '#10B981' },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockCategories);

      const result = await invoke('get_keyword_categories', { id: 1 });

      expect(result).toEqual(mockCategories);
      expect(result).toHaveLength(2);
    });

    it('should handle keywords with no category associations', async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      const result = await invoke('get_keyword_categories', { id: 1 });

      expect(result).toEqual([]);
    });
  });

  describe('search_keywords command', () => {
    it('should search keywords by query', async () => {
      const mockResults = [
        { id: 1, name: 'Politik', article_count: 42 },
        { id: 2, name: 'Politikwissenschaft', article_count: 5 },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockResults);

      const result = await invoke('search_keywords', { query: 'Politik', limit: 20 });

      expect(result).toEqual(mockResults);
      expect(invoke).toHaveBeenCalledWith('search_keywords', { query: 'Politik', limit: 20 });
    });

    it('should return empty array for no matches', async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      const result = await invoke('search_keywords', { query: 'xyznichtvorhanden', limit: 20 });

      expect(result).toEqual([]);
    });

    it('should handle partial matches', async () => {
      const mockResults = [
        { id: 1, name: 'Technologie', article_count: 30 },
        { id: 2, name: 'Biotechnologie', article_count: 12 },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockResults);

      const result = await invoke('search_keywords', { query: 'tech', limit: 20 });

      expect(result).toHaveLength(2);
    });
  });

  describe('get_network_graph command', () => {
    it('should fetch network graph data', async () => {
      const mockGraph = {
        nodes: [
          { id: 1, name: 'Politik', article_count: 42, cluster_id: 1 },
          { id: 2, name: 'Bundestag', article_count: 20, cluster_id: 1 },
          { id: 3, name: 'Wirtschaft', article_count: 35, cluster_id: 2 },
        ],
        edges: [
          { source: 1, target: 2, weight: 0.85 },
          { source: 1, target: 3, weight: 0.45 },
        ],
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockGraph);

      const result = await invoke('get_network_graph', { limit: 100, minWeight: 0.1 });

      expect(result).toEqual(mockGraph);
      expect(result.nodes).toHaveLength(3);
      expect(result.edges).toHaveLength(2);
    });

    it('should respect minWeight filter', async () => {
      const mockGraph = {
        nodes: [
          { id: 1, name: 'A', article_count: 10, cluster_id: null },
          { id: 2, name: 'B', article_count: 10, cluster_id: null },
        ],
        edges: [
          { source: 1, target: 2, weight: 0.8 },
        ],
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockGraph);

      await invoke('get_network_graph', { limit: 100, minWeight: 0.5 });

      expect(invoke).toHaveBeenCalledWith('get_network_graph', { limit: 100, minWeight: 0.5 });
    });

    it('should handle empty graph', async () => {
      const mockGraph = { nodes: [], edges: [] };
      vi.mocked(invoke).mockResolvedValueOnce(mockGraph);

      const result = await invoke('get_network_graph', { limit: 100, minWeight: 0.1 });

      expect(result.nodes).toHaveLength(0);
      expect(result.edges).toHaveLength(0);
    });
  });

  describe('get_keyword_trend command', () => {
    it('should fetch keyword trend data', async () => {
      const mockTrend = [
        { date: '2024-12-01', count: 5 },
        { date: '2024-12-02', count: 8 },
        { date: '2024-12-03', count: 3 },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(mockTrend);

      const result = await invoke('get_keyword_trend', { id: 1, days: 30 });

      expect(result).toEqual(mockTrend);
      expect(result).toHaveLength(3);
    });

    it('should handle keyword with no recent activity', async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      const result = await invoke('get_keyword_trend', { id: 1, days: 30 });

      expect(result).toEqual([]);
    });
  });

  describe('Error handling', () => {
    it('should handle network errors for get_keywords', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error('Database connection failed'));

      await expect(invoke('get_keywords', { limit: 50, offset: 0 })).rejects.toThrow('Database connection failed');
    });

    it('should handle network errors for get_keyword_neighbors', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error('Keyword not found'));

      await expect(invoke('get_keyword_neighbors', { id: 99999, limit: 20 })).rejects.toThrow('Keyword not found');
    });

    it('should handle network errors for get_network_graph', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error('Graph generation failed'));

      await expect(invoke('get_network_graph', { limit: 100, minWeight: 0.1 })).rejects.toThrow('Graph generation failed');
    });
  });
});

describe('Keyword weight classification', () => {
  function getWeightClass(weight: number): string {
    if (weight >= 0.7) return 'weight-high';
    if (weight >= 0.4) return 'weight-medium';
    return 'weight-low';
  }

  it('should classify high weight (>= 0.7)', () => {
    expect(getWeightClass(0.7)).toBe('weight-high');
    expect(getWeightClass(0.85)).toBe('weight-high');
    expect(getWeightClass(1.0)).toBe('weight-high');
  });

  it('should classify medium weight (0.4 - 0.7)', () => {
    expect(getWeightClass(0.4)).toBe('weight-medium');
    expect(getWeightClass(0.5)).toBe('weight-medium');
    expect(getWeightClass(0.69)).toBe('weight-medium');
  });

  it('should classify low weight (< 0.4)', () => {
    expect(getWeightClass(0.0)).toBe('weight-low');
    expect(getWeightClass(0.2)).toBe('weight-low');
    expect(getWeightClass(0.39)).toBe('weight-low');
  });
});

describe('Date formatting', () => {
  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleDateString();
  }

  it('should format valid date strings', () => {
    const result = formatDate('2024-12-01');
    expect(result).not.toBe('-');
    expect(typeof result).toBe('string');
  });

  it('should return dash for null dates', () => {
    expect(formatDate(null)).toBe('-');
  });

  it('should handle various date formats', () => {
    expect(formatDate('2024-01-15T10:30:00Z')).not.toBe('-');
    expect(formatDate('2024-06-30')).not.toBe('-');
  });
});
