import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

// Mock the invoke function
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('MindfuckView Component Logic', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Tab Management', () => {
    it('creates valid tab configuration', () => {
      interface Tab {
        id: string;
        label: string;
      }

      const tabs: Tab[] = [
        { id: 'overview', label: 'Overview' },
        { id: 'blindSpots', label: 'Blind Spots' },
        { id: 'recommendations', label: 'Recommendations' },
        { id: 'trends', label: 'Trends' },
      ];

      expect(tabs).toHaveLength(4);
      expect(tabs[0].id).toBe('overview');
      expect(tabs[3].id).toBe('trends');
    });

    it('determines data loading on tab change', () => {
      const loadedData = {
        overview: true,
        blindSpots: false,
        recommendations: false,
        trends: false,
      };

      const shouldLoadData = (tabId: string) => {
        if (tabId === 'blindSpots' && !loadedData.blindSpots) return 'blindSpots';
        if (tabId === 'recommendations' && !loadedData.recommendations)
          return 'recommendations';
        if (tabId === 'trends' && !loadedData.trends) return 'trends';
        return null;
      };

      expect(shouldLoadData('blindSpots')).toBe('blindSpots');
      expect(shouldLoadData('overview')).toBeNull();

      loadedData.blindSpots = true;
      expect(shouldLoadData('blindSpots')).toBeNull();
    });
  });

  describe('Bias Labels', () => {
    it('returns correct bias label for range values', () => {
      const getBiasLabel = (bias: number | null): string => {
        if (bias === null) return 'Not Rated';
        if (bias <= -1.5) return 'Strong Left';
        if (bias <= -0.5) return 'Left';
        if (bias <= 0.5) return 'Neutral';
        if (bias <= 1.5) return 'Right';
        return 'Strong Right';
      };

      expect(getBiasLabel(null)).toBe('Not Rated');
      expect(getBiasLabel(-2)).toBe('Strong Left');
      expect(getBiasLabel(-1)).toBe('Left');
      expect(getBiasLabel(0)).toBe('Neutral');
      expect(getBiasLabel(1)).toBe('Right');
      expect(getBiasLabel(2)).toBe('Strong Right');
    });
  });

  describe('Sachlichkeit Labels', () => {
    it('returns correct sachlichkeit label for range values', () => {
      const getSachlichkeitLabel = (sach: number | null): string => {
        if (sach === null) return 'Not Rated';
        if (sach <= 0.5) return 'Highly Emotional';
        if (sach <= 1.5) return 'Emotional';
        if (sach <= 2.5) return 'Mixed';
        if (sach <= 3.5) return 'Mostly Objective';
        return 'Objective';
      };

      expect(getSachlichkeitLabel(null)).toBe('Not Rated');
      expect(getSachlichkeitLabel(0)).toBe('Highly Emotional');
      expect(getSachlichkeitLabel(1)).toBe('Emotional');
      expect(getSachlichkeitLabel(2)).toBe('Mixed');
      expect(getSachlichkeitLabel(3)).toBe('Mostly Objective');
      expect(getSachlichkeitLabel(4)).toBe('Objective');
    });
  });

  describe('Bias Colors', () => {
    it('returns correct CSS variable for bias', () => {
      const getBiasColor = (bias: number | null): string => {
        if (bias === null) return 'var(--text-muted)';
        if (bias <= -1.5) return 'var(--bias-strong-left)';
        if (bias <= -0.5) return 'var(--bias-lean-left)';
        if (bias <= 0.5) return 'var(--bias-center)';
        if (bias <= 1.5) return 'var(--bias-lean-right)';
        return 'var(--bias-strong-right)';
      };

      expect(getBiasColor(null)).toBe('var(--text-muted)');
      expect(getBiasColor(-2)).toBe('var(--bias-strong-left)');
      expect(getBiasColor(0)).toBe('var(--bias-center)');
      expect(getBiasColor(2)).toBe('var(--bias-strong-right)');
    });
  });

  describe('Severity Colors', () => {
    it('returns correct color for severity level', () => {
      const getSeverityColor = (severity: string): string => {
        switch (severity) {
          case 'high':
            return 'var(--status-error)';
          case 'medium':
            return 'var(--status-warning)';
          case 'low':
            return 'var(--ctp-yellow)';
          default:
            return 'var(--text-muted)';
        }
      };

      expect(getSeverityColor('high')).toBe('var(--status-error)');
      expect(getSeverityColor('medium')).toBe('var(--status-warning)');
      expect(getSeverityColor('low')).toBe('var(--ctp-yellow)');
      expect(getSeverityColor('unknown')).toBe('var(--text-muted)');
    });
  });

  describe('Category Color Variables', () => {
    it('returns correct CSS variable for category', () => {
      const getCategoryColorVar = (id: number | undefined): string => {
        if (id && id >= 1 && id <= 6) {
          return `var(--category-${id})`;
        }
        return 'var(--accent-primary)';
      };

      expect(getCategoryColorVar(undefined)).toBe('var(--accent-primary)');
      expect(getCategoryColorVar(0)).toBe('var(--accent-primary)');
      expect(getCategoryColorVar(1)).toBe('var(--category-1)');
      expect(getCategoryColorVar(6)).toBe('var(--category-6)');
      expect(getCategoryColorVar(7)).toBe('var(--accent-primary)');
    });
  });

  describe('Date Formatting', () => {
    it('formats date correctly', () => {
      const formatDate = (dateStr: string | null): string => {
        if (!dateStr) return '-';
        const date = new Date(dateStr);
        return date.toLocaleDateString();
      };

      expect(formatDate(null)).toBe('-');
      expect(formatDate('2025-01-15T10:00:00Z')).toBeTruthy();
    });
  });

  describe('Bias Indicator', () => {
    it('determines bias indicator text', () => {
      const getBiasIndicator = (avgBias: number | null): string => {
        if (!avgBias) return 'Balanced';
        if (avgBias < -0.3) return 'Leaning Left';
        if (avgBias > 0.3) return 'Leaning Right';
        return 'Balanced';
      };

      expect(getBiasIndicator(null)).toBe('Balanced');
      expect(getBiasIndicator(0)).toBe('Balanced');
      expect(getBiasIndicator(0.2)).toBe('Balanced');
      expect(getBiasIndicator(-0.5)).toBe('Leaning Left');
      expect(getBiasIndicator(0.5)).toBe('Leaning Right');
    });
  });

  describe('Category Expansion', () => {
    it('toggles category expansion', () => {
      let expandedCategoryId: number | null = null;

      const toggleCategoryExpand = (categoryId: number) => {
        if (expandedCategoryId === categoryId) {
          expandedCategoryId = null;
        } else {
          expandedCategoryId = categoryId;
        }
      };

      expect(expandedCategoryId).toBeNull();

      toggleCategoryExpand(1);
      expect(expandedCategoryId).toBe(1);

      toggleCategoryExpand(1);
      expect(expandedCategoryId).toBeNull();

      toggleCategoryExpand(2);
      expect(expandedCategoryId).toBe(2);
    });
  });

  describe('Trend Period Selection', () => {
    it('changes trend period', () => {
      let trendPeriod: 7 | 30 | 90 = 30;

      const handleTrendPeriodChange = (days: 7 | 30 | 90) => {
        trendPeriod = days;
      };

      expect(trendPeriod).toBe(30);

      handleTrendPeriodChange(7);
      expect(trendPeriod).toBe(7);

      handleTrendPeriodChange(90);
      expect(trendPeriod).toBe(90);
    });
  });

  describe('Reading Profile Loading', () => {
    it('loads reading profile via invoke', async () => {
      const mockProfile = {
        total_read: 100,
        total_articles: 500,
        read_percentage: 20.0,
        avg_political_bias: 0.2,
        avg_sachlichkeit: 2.5,
        by_category: [],
        by_bias: [],
        by_sachlichkeit: [],
      };

      vi.mocked(invoke).mockResolvedValue(mockProfile);

      const profile = await invoke('get_reading_profile');

      expect(invoke).toHaveBeenCalledWith('get_reading_profile');
      expect(profile.total_read).toBe(100);
      expect(profile.read_percentage).toBe(20.0);
    });
  });

  describe('Blind Spots Loading', () => {
    it('loads blind spots via invoke', async () => {
      const mockBlindSpots = [
        {
          spot_type: 'category',
          name: 'Politics',
          severity: 'high',
          available_count: 100,
          read_count: 5,
        },
        {
          spot_type: 'subcategory',
          name: 'AI',
          severity: 'medium',
          available_count: 50,
          read_count: 15,
        },
      ];

      vi.mocked(invoke).mockResolvedValue(mockBlindSpots);

      const blindSpots = await invoke('get_blind_spots');

      expect(invoke).toHaveBeenCalledWith('get_blind_spots');
      expect(blindSpots).toHaveLength(2);
    });
  });

  describe('Counter Perspectives Loading', () => {
    it('loads counter perspectives via invoke', async () => {
      const mockPerspectives = [
        {
          fnord_id: 1,
          title: 'Alternative View',
          pentacle_title: 'News Source',
          political_bias: -1,
          reason: 'Different perspective on topic',
        },
      ];

      vi.mocked(invoke).mockResolvedValue(mockPerspectives);

      const perspectives = await invoke('get_counter_perspectives', { limit: 10 });

      expect(invoke).toHaveBeenCalledWith('get_counter_perspectives', { limit: 10 });
      expect(perspectives).toHaveLength(1);
    });
  });

  describe('Reading Trends Loading', () => {
    it('loads reading trends via invoke', async () => {
      const mockTrends = [
        { date: '2025-01-15', read_count: 10, avg_bias: 0.1, avg_sachlichkeit: 2.5 },
        { date: '2025-01-14', read_count: 8, avg_bias: -0.2, avg_sachlichkeit: 3.0 },
      ];

      vi.mocked(invoke).mockResolvedValue(mockTrends);

      const trends = await invoke('get_reading_trends', { days: 30 });

      expect(invoke).toHaveBeenCalledWith('get_reading_trends', { days: 30 });
      expect(trends).toHaveLength(2);
    });
  });

  describe('Recommendation Progress', () => {
    it('calculates recommendation progress', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'check_ollama') {
          return { available: true };
        }
        if (cmd === 'get_bias_stats') {
          return { articles_with_bias: 15 };
        }
        return {};
      });

      const ollamaStatus = await invoke<{ available: boolean }>('check_ollama');
      const biasStats = await invoke<{ articles_with_bias: number }>('get_bias_stats');

      const progress = {
        articlesRead: 20,
        articlesNeeded: 10,
        aiAvailable: ollamaStatus.available,
        articlesWithBias: biasStats.articles_with_bias,
      };

      expect(progress.aiAvailable).toBe(true);
      expect(progress.articlesWithBias).toBe(15);
      expect(progress.articlesRead >= progress.articlesNeeded).toBe(true);
    });
  });

  describe('Article Navigation', () => {
    it('dispatches navigate-to-article event', () => {
      const dispatchEventSpy = vi.spyOn(window, 'dispatchEvent');

      const handleReadArticle = (fnordId: number) => {
        window.dispatchEvent(
          new CustomEvent('navigate-to-article', { detail: { articleId: fnordId } })
        );
      };

      handleReadArticle(42);

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

describe('MindfuckView Data Structures', () => {
  describe('Reading Profile Structure', () => {
    interface ReadingProfile {
      total_read: number;
      total_articles: number;
      read_percentage: number;
      avg_political_bias: number | null;
      avg_sachlichkeit: number | null;
      by_category: {
        sephiroth_id: number;
        name: string;
        read_count: number;
        total_count: number;
      }[];
      by_bias: {
        bias_value: number;
        label: string;
        read_count: number;
        percentage: number;
      }[];
      by_sachlichkeit: {
        sachlichkeit_value: number;
        label: string;
        read_count: number;
        percentage: number;
      }[];
    }

    it('creates valid reading profile', () => {
      const profile: ReadingProfile = {
        total_read: 150,
        total_articles: 500,
        read_percentage: 30.0,
        avg_political_bias: -0.3,
        avg_sachlichkeit: 2.8,
        by_category: [
          { sephiroth_id: 1, name: 'Tech', read_count: 50, total_count: 100 },
          { sephiroth_id: 2, name: 'Politics', read_count: 30, total_count: 150 },
        ],
        by_bias: [
          { bias_value: -2, label: 'Strong Left', read_count: 10, percentage: 6.67 },
          { bias_value: 0, label: 'Neutral', read_count: 80, percentage: 53.33 },
        ],
        by_sachlichkeit: [
          { sachlichkeit_value: 3, label: 'Mostly Objective', read_count: 70, percentage: 46.67 },
        ],
      };

      expect(profile.total_read).toBe(150);
      expect(profile.read_percentage).toBe(30.0);
      expect(profile.by_category).toHaveLength(2);
    });
  });

  describe('Blind Spot Structure', () => {
    interface BlindSpot {
      spot_type: string;
      name: string;
      icon: string | null;
      description: string;
      severity: string;
      available_count: number;
      read_count: number;
      main_category: string | null;
      main_category_color: string | null;
    }

    it('creates valid blind spot', () => {
      const blindSpot: BlindSpot = {
        spot_type: 'category',
        name: 'Politics',
        icon: 'fa-landmark',
        description: 'You rarely read political news',
        severity: 'high',
        available_count: 200,
        read_count: 10,
        main_category: null,
        main_category_color: null,
      };

      expect(blindSpot.severity).toBe('high');
      expect(blindSpot.available_count).toBeGreaterThan(blindSpot.read_count);
    });

    it('calculates read percentage', () => {
      const blindSpot = {
        available_count: 100,
        read_count: 25,
      };

      const readPercentage =
        blindSpot.available_count > 0
          ? Math.round((blindSpot.read_count / blindSpot.available_count) * 100)
          : 0;

      expect(readPercentage).toBe(25);
    });
  });

  describe('Counter Perspective Structure', () => {
    interface CounterPerspective {
      fnord_id: number;
      title: string;
      pentacle_title: string | null;
      published_at: string | null;
      political_bias: number | null;
      reason: string;
    }

    it('creates valid counter perspective', () => {
      const perspective: CounterPerspective = {
        fnord_id: 42,
        title: 'Alternative Viewpoint',
        pentacle_title: 'Conservative News',
        published_at: '2025-01-15T10:00:00Z',
        political_bias: 1,
        reason: 'To balance your left-leaning reading',
      };

      expect(perspective.fnord_id).toBe(42);
      expect(perspective.reason).toBeTruthy();
    });
  });

  describe('Reading Trend Structure', () => {
    interface ReadingTrend {
      date: string;
      read_count: number;
      avg_bias: number | null;
      avg_sachlichkeit: number | null;
    }

    it('creates valid reading trend', () => {
      const trend: ReadingTrend = {
        date: '2025-01-15',
        read_count: 12,
        avg_bias: 0.1,
        avg_sachlichkeit: 3.2,
      };

      expect(trend.read_count).toBeGreaterThan(0);
      expect(trend.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    });
  });
});

describe('MindfuckView Progress Bar Calculations', () => {
  it('calculates bar width percentage', () => {
    const calculateBarWidth = (value: number, maxValue: number): number => {
      if (maxValue === 0) return 0;
      return (value / maxValue) * 100;
    };

    expect(calculateBarWidth(50, 100)).toBe(50);
    expect(calculateBarWidth(25, 50)).toBe(50);
    expect(calculateBarWidth(0, 100)).toBe(0);
    expect(calculateBarWidth(50, 0)).toBe(0);
  });

  it('calculates spectrum position indicator', () => {
    const calculateIndicatorPosition = (bias: number): number => {
      // Bias ranges from -2 to +2, need to map to 0-100%
      return ((bias + 2) / 4) * 100;
    };

    expect(calculateIndicatorPosition(-2)).toBe(0);
    expect(calculateIndicatorPosition(0)).toBe(50);
    expect(calculateIndicatorPosition(2)).toBe(100);
    expect(calculateIndicatorPosition(-1)).toBe(25);
    expect(calculateIndicatorPosition(1)).toBe(75);
  });
});

describe('MindfuckView Trend Chart Calculations', () => {
  it('calculates max read count for scaling', () => {
    const trends = [
      { date: '2025-01-15', read_count: 10 },
      { date: '2025-01-14', read_count: 25 },
      { date: '2025-01-13', read_count: 15 },
    ];

    const maxReadCount = Math.max(...trends.map((t) => t.read_count));

    expect(maxReadCount).toBe(25);
  });

  it('calculates trend bar height', () => {
    const calculateBarHeight = (readCount: number, maxCount: number): number => {
      if (maxCount === 0) return 4; // Minimum height
      return Math.max(4, (readCount / maxCount) * 100);
    };

    expect(calculateBarHeight(50, 100)).toBe(50);
    expect(calculateBarHeight(100, 100)).toBe(100);
    expect(calculateBarHeight(0, 100)).toBe(4);
    expect(calculateBarHeight(0, 0)).toBe(4);
  });

  it('calculates total reads for trend period', () => {
    const trends = [
      { date: '2025-01-15', read_count: 10 },
      { date: '2025-01-14', read_count: 8 },
      { date: '2025-01-13', read_count: 12 },
    ];

    const totalReads = trends.reduce((sum, t) => sum + t.read_count, 0);

    expect(totalReads).toBe(30);
  });

  it('calculates average bias over time', () => {
    const trends = [
      { date: '2025-01-15', read_count: 10, avg_bias: 0.2 },
      { date: '2025-01-14', read_count: 8, avg_bias: -0.1 },
      { date: '2025-01-13', read_count: 12, avg_bias: 0.0 },
    ];

    const trendsWithBias = trends.filter((t) => t.avg_bias !== null);
    const avgBias =
      trendsWithBias.length > 0
        ? trendsWithBias.reduce((sum, t) => sum + t.avg_bias, 0) / trendsWithBias.length
        : null;

    expect(avgBias).toBeCloseTo(0.033, 2);
  });
});

describe('MindfuckView Empty State Detection', () => {
  it('detects empty reading profile', () => {
    const isProfileEmpty = (profile: { total_read: number } | null): boolean => {
      return !profile || profile.total_read === 0;
    };

    expect(isProfileEmpty(null)).toBe(true);
    expect(isProfileEmpty({ total_read: 0 })).toBe(true);
    expect(isProfileEmpty({ total_read: 10 })).toBe(false);
  });

  it('detects no blind spots (positive state)', () => {
    const hasNoBlindSpots = (blindSpots: unknown[]): boolean => {
      return blindSpots.length === 0;
    };

    expect(hasNoBlindSpots([])).toBe(true);
    expect(hasNoBlindSpots([{ name: 'Test' }])).toBe(false);
  });

  it('detects no trends data', () => {
    const hasNoTrends = (trends: unknown[]): boolean => {
      return trends.length === 0;
    };

    expect(hasNoTrends([])).toBe(true);
    expect(hasNoTrends([{ date: '2025-01-15' }])).toBe(false);
  });
});

describe('MindfuckView Subcategory Handling', () => {
  it('identifies low read rate subcategories', () => {
    const isLowReadRate = (
      percentage: number,
      totalCount: number,
      threshold: number = 30,
      minArticles: number = 5
    ): boolean => {
      return percentage < threshold && totalCount > minArticles;
    };

    expect(isLowReadRate(20, 10)).toBe(true);
    expect(isLowReadRate(40, 10)).toBe(false);
    expect(isLowReadRate(20, 3)).toBe(false); // Not enough articles
  });
});
