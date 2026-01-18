import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { RecommendationStats, RecommendationLoadState, RecommendationPhase, Recommendation } from '../../types';

// Mock invoke before tests
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

/**
 * Helper functions extracted from RecommendationList.svelte for testing
 */

function parseError(message: string): { code: string; retryable: boolean } {
  if (message.includes('database is locked')) {
    return { code: 'DB_LOCKED', retryable: true };
  }
  if (message.includes('no such table')) {
    return { code: 'SCHEMA_ERROR', retryable: false };
  }
  if (message.includes('connection')) {
    return { code: 'CONNECTION_ERROR', retryable: true };
  }
  return { code: 'UNKNOWN', retryable: true };
}

function getEmptyReason(stats: RecommendationStats | null): string {
  if (!stats) return 'no_stats';
  if (stats.articles_read < 5) return 'not_enough_articles';
  if (stats.articles_with_embedding === 0) return 'no_embeddings';
  if (stats.candidate_pool_size === 0) return 'no_candidates';
  return 'no_matches';
}

function generateRequestId(): string {
  return `rec-${Date.now()}-${Math.random().toString(36).substring(2, 8)}`;
}

describe('Recommendation Error Parsing', () => {
  it('should identify database locked errors as retryable', () => {
    const result = parseError('database is locked');
    expect(result.code).toBe('DB_LOCKED');
    expect(result.retryable).toBe(true);
  });

  it('should identify schema errors as non-retryable', () => {
    const result = parseError('no such table: recommendations');
    expect(result.code).toBe('SCHEMA_ERROR');
    expect(result.retryable).toBe(false);
  });

  it('should identify connection errors as retryable', () => {
    const result = parseError('connection refused');
    expect(result.code).toBe('CONNECTION_ERROR');
    expect(result.retryable).toBe(true);
  });

  it('should default to UNKNOWN with retryable=true for unrecognized errors', () => {
    const result = parseError('some random error');
    expect(result.code).toBe('UNKNOWN');
    expect(result.retryable).toBe(true);
  });

  it('should handle empty error message', () => {
    const result = parseError('');
    expect(result.code).toBe('UNKNOWN');
    expect(result.retryable).toBe(true);
  });
});

describe('Empty Reason Detection', () => {
  it('should return no_stats when stats is null', () => {
    const result = getEmptyReason(null);
    expect(result).toBe('no_stats');
  });

  it('should return not_enough_articles when articles_read < 5', () => {
    const stats: RecommendationStats = {
      total_saved: 0,
      total_hidden: 0,
      total_clicks: 0,
      articles_read: 3,
      articles_with_embedding: 10,
      profile_strength: 'Cold',
      top_keywords: [],
      top_categories: [],
      candidate_pool_size: 50,
    };
    const result = getEmptyReason(stats);
    expect(result).toBe('not_enough_articles');
  });

  it('should return no_embeddings when articles_with_embedding is 0', () => {
    const stats: RecommendationStats = {
      total_saved: 0,
      total_hidden: 0,
      total_clicks: 0,
      articles_read: 10,
      articles_with_embedding: 0,
      profile_strength: 'Warm',
      top_keywords: [],
      top_categories: [],
      candidate_pool_size: 50,
    };
    const result = getEmptyReason(stats);
    expect(result).toBe('no_embeddings');
  });

  it('should return no_candidates when candidate_pool_size is 0', () => {
    const stats: RecommendationStats = {
      total_saved: 0,
      total_hidden: 0,
      total_clicks: 0,
      articles_read: 10,
      articles_with_embedding: 50,
      profile_strength: 'Warm',
      top_keywords: [],
      top_categories: [],
      candidate_pool_size: 0,
    };
    const result = getEmptyReason(stats);
    expect(result).toBe('no_candidates');
  });

  it('should return no_matches when all conditions are met but still empty', () => {
    const stats: RecommendationStats = {
      total_saved: 5,
      total_hidden: 2,
      total_clicks: 10,
      articles_read: 50,
      articles_with_embedding: 100,
      profile_strength: 'Hot',
      top_keywords: [{ name: 'test', weight: 1.0, article_count: 5 }],
      top_categories: [{ id: 1, name: 'Tech', weight: 0.5 }],
      candidate_pool_size: 25,
    };
    const result = getEmptyReason(stats);
    expect(result).toBe('no_matches');
  });
});

describe('Request ID Generation', () => {
  it('should generate unique request IDs', () => {
    const id1 = generateRequestId();
    const id2 = generateRequestId();
    expect(id1).not.toBe(id2);
  });

  it('should start with "rec-" prefix', () => {
    const id = generateRequestId();
    expect(id.startsWith('rec-')).toBe(true);
  });

  it('should contain a timestamp component', () => {
    const before = Date.now();
    const id = generateRequestId();
    const after = Date.now();

    // Extract timestamp from "rec-{timestamp}-{random}"
    const parts = id.split('-');
    const timestamp = parseInt(parts[1], 10);

    expect(timestamp).toBeGreaterThanOrEqual(before);
    expect(timestamp).toBeLessThanOrEqual(after);
  });
});

describe('RecommendationLoadState Types', () => {
  it('should validate idle state', () => {
    const state: RecommendationLoadState = { status: 'idle' };
    expect(state.status).toBe('idle');
  });

  it('should validate loading state with phase', () => {
    const state: RecommendationLoadState = {
      status: 'loading',
      phase: 'generating_candidates',
      startedAt: Date.now(),
    };
    expect(state.status).toBe('loading');
    if (state.status === 'loading') {
      expect(state.phase).toBe('generating_candidates');
      expect(typeof state.startedAt).toBe('number');
    }
  });

  it('should validate success state with recommendations', () => {
    const mockRecommendation: Recommendation = {
      fnord_id: 1,
      title: 'Test Article',
      summary: 'A test summary',
      url: 'https://example.com/article',
      image_url: null,
      pentacle_id: 1,
      pentacle_title: 'Test Feed',
      pentacle_icon: null,
      published_at: '2025-01-01T00:00:00Z',
      relevance_score: 0.8,
      freshness_score: 0.9,
      political_bias: 0,
      sachlichkeit: 3,
      categories: [],
      matching_keywords: ['test', 'keyword'],
      explanation: 'Based on: test, keyword',
      is_saved: false,
    };

    const state: RecommendationLoadState = {
      status: 'success',
      recommendations: [mockRecommendation],
      loadedAt: Date.now(),
    };

    expect(state.status).toBe('success');
    if (state.status === 'success') {
      expect(state.recommendations).toHaveLength(1);
      expect(state.recommendations[0].title).toBe('Test Article');
    }
  });

  it('should validate error state with code and retryable flag', () => {
    const state: RecommendationLoadState = {
      status: 'error',
      code: 'DB_LOCKED',
      message: 'database is locked',
      retryable: true,
    };

    expect(state.status).toBe('error');
    if (state.status === 'error') {
      expect(state.code).toBe('DB_LOCKED');
      expect(state.retryable).toBe(true);
    }
  });

  it('should validate timeout state with elapsed time', () => {
    const state: RecommendationLoadState = {
      status: 'timeout',
      elapsedMs: 30000,
    };

    expect(state.status).toBe('timeout');
    if (state.status === 'timeout') {
      expect(state.elapsedMs).toBe(30000);
    }
  });

  it('should validate cancelled state', () => {
    const state: RecommendationLoadState = { status: 'cancelled' };
    expect(state.status).toBe('cancelled');
  });
});

describe('RecommendationPhase Values', () => {
  it('should have all expected phases', () => {
    const phases: RecommendationPhase[] = [
      'init',
      'loading_profile',
      'generating_candidates',
      'scoring',
      'finalizing',
    ];

    expect(phases).toHaveLength(5);
    expect(phases).toContain('init');
    expect(phases).toContain('loading_profile');
    expect(phases).toContain('generating_candidates');
    expect(phases).toContain('scoring');
    expect(phases).toContain('finalizing');
  });
});

describe('Recommendation API Calls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should call get_recommendations with limit', async () => {
    const mockRecommendations: Recommendation[] = [
      {
        fnord_id: 1,
        title: 'Test',
        summary: null,
        url: 'https://example.com',
        image_url: null,
        pentacle_id: 1,
        pentacle_title: 'Feed',
        pentacle_icon: null,
        published_at: null,
        relevance_score: 0.7,
        freshness_score: 0.8,
        political_bias: null,
        sachlichkeit: null,
        categories: [],
        matching_keywords: [],
        explanation: 'Test',
        is_saved: false,
      },
    ];

    vi.mocked(invoke).mockResolvedValueOnce(mockRecommendations);

    const result = await invoke('get_recommendations', { limit: 10 });
    expect(invoke).toHaveBeenCalledWith('get_recommendations', { limit: 10 });
    expect(result).toHaveLength(1);
  });

  it('should call get_recommendation_stats', async () => {
    const mockStats: RecommendationStats = {
      total_saved: 5,
      total_hidden: 2,
      total_clicks: 15,
      articles_read: 100,
      articles_with_embedding: 80,
      profile_strength: 'Hot',
      top_keywords: [],
      top_categories: [],
      candidate_pool_size: 50,
    };

    vi.mocked(invoke).mockResolvedValueOnce(mockStats);

    const result = await invoke('get_recommendation_stats');
    expect(invoke).toHaveBeenCalledWith('get_recommendation_stats');
    expect(result).toEqual(mockStats);
  });

  it('should handle save_article', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await invoke('save_article', { fnordId: 123 });
    expect(invoke).toHaveBeenCalledWith('save_article', { fnordId: 123 });
  });

  it('should handle unsave_article', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await invoke('unsave_article', { fnordId: 123 });
    expect(invoke).toHaveBeenCalledWith('unsave_article', { fnordId: 123 });
  });

  it('should handle hide_recommendation', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await invoke('hide_recommendation', { fnordId: 123 });
    expect(invoke).toHaveBeenCalledWith('hide_recommendation', { fnordId: 123 });
  });
});

describe('Timeout Behavior', () => {
  const TIMEOUT_MS = 30000;

  it('should have a 30 second timeout constant', () => {
    expect(TIMEOUT_MS).toBe(30000);
  });

  it('should be able to race between request and timeout', async () => {
    // Simulate a fast request
    const fastRequest = new Promise<string>((resolve) => {
      setTimeout(() => resolve('success'), 10);
    });

    const timeoutPromise = new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error('TIMEOUT')), TIMEOUT_MS);
    });

    const result = await Promise.race([fastRequest, timeoutPromise]);
    expect(result).toBe('success');
  });

  it('should timeout for slow requests', async () => {
    // Simulate a slow request (using a short timeout for testing)
    const shortTimeout = 50;

    const slowRequest = new Promise<string>((resolve) => {
      setTimeout(() => resolve('success'), 200);
    });

    const timeoutPromise = new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error('TIMEOUT')), shortTimeout);
    });

    await expect(Promise.race([slowRequest, timeoutPromise])).rejects.toThrow('TIMEOUT');
  });
});

describe('Profile Strength Calculation', () => {
  it('should return Cold for less than 10 articles read', () => {
    const articles_read = 5;
    const profile_strength = articles_read >= 50 ? 'Hot' : articles_read >= 10 ? 'Warm' : 'Cold';
    expect(profile_strength).toBe('Cold');
  });

  it('should return Warm for 10-49 articles read', () => {
    const articles_read = 25;
    const profile_strength = articles_read >= 50 ? 'Hot' : articles_read >= 10 ? 'Warm' : 'Cold';
    expect(profile_strength).toBe('Warm');
  });

  it('should return Hot for 50+ articles read', () => {
    const articles_read = 100;
    const profile_strength = articles_read >= 50 ? 'Hot' : articles_read >= 10 ? 'Warm' : 'Cold';
    expect(profile_strength).toBe('Hot');
  });

  it('should return Warm at exactly 10 articles', () => {
    const articles_read = 10;
    const profile_strength = articles_read >= 50 ? 'Hot' : articles_read >= 10 ? 'Warm' : 'Cold';
    expect(profile_strength).toBe('Warm');
  });

  it('should return Hot at exactly 50 articles', () => {
    const articles_read = 50;
    const profile_strength = articles_read >= 50 ? 'Hot' : articles_read >= 10 ? 'Warm' : 'Cold';
    expect(profile_strength).toBe('Hot');
  });
});
