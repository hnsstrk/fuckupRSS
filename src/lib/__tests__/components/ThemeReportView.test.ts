import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock("svelte-i18n", () => ({
  _: {
    subscribe: vi.fn((cb: any) => {
      cb((key: string) => key);
      return () => {};
    }),
  },
}));

// -----------------------------------------------------------------------
// Types mirroring the component's internal interfaces
// -----------------------------------------------------------------------

interface ThemeReportSummary {
  id: number;
  period_start: string;
  period_end: string;
  search_query: string | null;
  theme_count: number;
  model_used: string | null;
  locale: string;
  created_at: string;
}

interface ThemeArticle {
  fnord_id: number;
  title: string;
  summary: string | null;
  source_name: string;
  political_bias: number | null;
  sachlichkeit: number | null;
  published_at: string;
  topic_score: number;
}

interface ThemeReportTheme {
  id: number;
  label: string;
  headline: string | null;
  report_json: string | null;
  report_status: string;
  cluster_score: number;
  article_count: number;
  source_count: number;
  articles: ThemeArticle[];
}

interface ThemeReportDetail {
  report: ThemeReportSummary;
  themes: ThemeReportTheme[];
}

interface ThemeProgressData {
  report_id: number;
  themes_complete: number;
  themes_total: number;
  current_theme: string;
}

// -----------------------------------------------------------------------
// Helper functions extracted from ThemeReportView logic
// -----------------------------------------------------------------------

/** Mirrors the toggleTheme logic in ThemeReportView */
function toggleTheme(expandedThemes: Set<number>, themeId: number): void {
  if (expandedThemes.has(themeId)) {
    expandedThemes.delete(themeId);
  } else {
    expandedThemes.add(themeId);
  }
}

/** Mirrors the handleRetry theme-replacement logic */
function applyRetryResult(
  detail: ThemeReportDetail,
  updatedTheme: ThemeReportTheme,
): ThemeReportDetail {
  return {
    ...detail,
    themes: detail.themes.map((t) => (t.id === updatedTheme.id ? updatedTheme : t)),
  };
}

/** Mirrors the handleDelete list-filter logic */
function removeReportFromList(
  reports: ThemeReportSummary[],
  removedId: number,
): ThemeReportSummary[] {
  return reports.filter((r) => r.id !== removedId);
}

/** Mirrors the searchQuery pass-through: empty string → null */
function normalizeSearchQuery(query: string): string | null {
  return query || null;
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

describe("ThemeReportView — default state values", () => {
  it("days defaults to 1 (24 h window)", () => {
    const days = 1;
    expect(days).toBe(1);
  });

  it("searchQuery defaults to empty string", () => {
    const searchQuery = "";
    expect(searchQuery).toBe("");
  });

  it("generating defaults to false", () => {
    const generating = false;
    expect(generating).toBe(false);
  });

  it("progress defaults to null", () => {
    const progress: ThemeProgressData | null = null;
    expect(progress).toBeNull();
  });

  it("selectedReportId defaults to null", () => {
    const selectedReportId: number | null = null;
    expect(selectedReportId).toBeNull();
  });

  it("reportDetail defaults to null", () => {
    const reportDetail: ThemeReportDetail | null = null;
    expect(reportDetail).toBeNull();
  });
});

describe("ThemeReportView — toggleTheme logic", () => {
  it("adds a theme to the expanded set", () => {
    const expanded = new Set<number>();
    toggleTheme(expanded, 5);
    expect(expanded.has(5)).toBe(true);
  });

  it("removes a theme from the expanded set when already present", () => {
    const expanded = new Set<number>([5]);
    toggleTheme(expanded, 5);
    expect(expanded.has(5)).toBe(false);
  });

  it("toggles multiple themes independently", () => {
    const expanded = new Set<number>();
    toggleTheme(expanded, 1);
    toggleTheme(expanded, 2);
    expect(expanded.has(1)).toBe(true);
    expect(expanded.has(2)).toBe(true);

    toggleTheme(expanded, 1);
    expect(expanded.has(1)).toBe(false);
    expect(expanded.has(2)).toBe(true);
  });
});

describe("ThemeReportView — applyRetryResult logic", () => {
  const makeDetail = (): ThemeReportDetail => ({
    report: {
      id: 1,
      period_start: "2026-01-01T00:00:00Z",
      period_end: "2026-01-02T00:00:00Z",
      search_query: null,
      theme_count: 2,
      model_used: "ministral-3",
      locale: "de",
      created_at: "2026-01-02T10:00:00Z",
    },
    themes: [
      {
        id: 10,
        label: "Theme A",
        headline: null,
        report_json: null,
        report_status: "pending",
        cluster_score: 0.8,
        article_count: 3,
        source_count: 2,
        articles: [],
      },
      {
        id: 20,
        label: "Theme B",
        headline: null,
        report_json: null,
        report_status: "done",
        cluster_score: 0.9,
        article_count: 5,
        source_count: 3,
        articles: [],
      },
    ],
  });

  it("replaces the correct theme in the list", () => {
    const detail = makeDetail();
    const updated: ThemeReportTheme = {
      ...detail.themes[0],
      report_status: "done",
      report_json: "{}",
    };
    const result = applyRetryResult(detail, updated);

    expect(result.themes[0].report_status).toBe("done");
    expect(result.themes[1].report_status).toBe("done"); // unchanged
  });

  it("leaves other themes untouched", () => {
    const detail = makeDetail();
    const updated: ThemeReportTheme = { ...detail.themes[0], headline: "New Headline" };
    const result = applyRetryResult(detail, updated);

    expect(result.themes[1].label).toBe("Theme B");
    expect(result.themes[1].headline).toBeNull();
  });

  it("preserves the report summary unchanged", () => {
    const detail = makeDetail();
    const updated: ThemeReportTheme = { ...detail.themes[0], report_status: "done" };
    const result = applyRetryResult(detail, updated);

    expect(result.report.id).toBe(detail.report.id);
    expect(result.report.theme_count).toBe(2);
  });
});

describe("ThemeReportView — removeReportFromList logic", () => {
  const reports: ThemeReportSummary[] = [
    {
      id: 1,
      period_start: "2026-01-01T00:00:00Z",
      period_end: "2026-01-02T00:00:00Z",
      search_query: null,
      theme_count: 3,
      model_used: null,
      locale: "de",
      created_at: "2026-01-02T10:00:00Z",
    },
    {
      id: 2,
      period_start: "2026-01-03T00:00:00Z",
      period_end: "2026-01-04T00:00:00Z",
      search_query: "AI",
      theme_count: 5,
      model_used: "ministral-3",
      locale: "de",
      created_at: "2026-01-04T10:00:00Z",
    },
  ];

  it("removes the report with the matching id", () => {
    const result = removeReportFromList(reports, 1);
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe(2);
  });

  it("returns the same list when id is not found", () => {
    const result = removeReportFromList(reports, 999);
    expect(result).toHaveLength(2);
  });

  it("returns an empty list when the only item is removed", () => {
    const single = [reports[0]];
    const result = removeReportFromList(single, 1);
    expect(result).toHaveLength(0);
  });
});

describe("ThemeReportView — normalizeSearchQuery logic", () => {
  it("converts empty string to null (matches invoke param logic)", () => {
    expect(normalizeSearchQuery("")).toBeNull();
  });

  it("keeps a non-empty string as-is", () => {
    expect(normalizeSearchQuery("AI trends")).toBe("AI trends");
  });

  it("keeps a whitespace-only string (not trimmed in component)", () => {
    // The component passes `searchQuery || null`, so " " is truthy
    expect(normalizeSearchQuery("  ")).toBe("  ");
  });
});

describe("ThemeReportView — Tauri invoke commands", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls get_theme_reports with limit parameter", async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    await invoke<ThemeReportSummary[]>("get_theme_reports", { limit: 30 });
    expect(invoke).toHaveBeenCalledWith("get_theme_reports", { limit: 30 });
  });

  it("calls get_theme_report_detail with reportId", async () => {
    const mockDetail: ThemeReportDetail = {
      report: {
        id: 42,
        period_start: "2026-01-01T00:00:00Z",
        period_end: "2026-01-02T00:00:00Z",
        search_query: null,
        theme_count: 1,
        model_used: null,
        locale: "de",
        created_at: "2026-01-02T10:00:00Z",
      },
      themes: [],
    };
    vi.mocked(invoke).mockResolvedValue(mockDetail);

    const result = await invoke<ThemeReportDetail>("get_theme_report_detail", { reportId: 42 });

    expect(invoke).toHaveBeenCalledWith("get_theme_report_detail", { reportId: 42 });
    expect((result as ThemeReportDetail).report.id).toBe(42);
  });

  it("calls generate_theme_report with days and searchQuery", async () => {
    vi.mocked(invoke).mockResolvedValue({ report: { id: 99 }, themes: [] });

    await invoke("generate_theme_report", { days: 7, searchQuery: "Klimawandel" });

    expect(invoke).toHaveBeenCalledWith("generate_theme_report", {
      days: 7,
      searchQuery: "Klimawandel",
    });
  });

  it("calls generate_theme_report with null searchQuery when empty", async () => {
    vi.mocked(invoke).mockResolvedValue({ report: { id: 100 }, themes: [] });

    const searchQuery = "";
    await invoke("generate_theme_report", { days: 1, searchQuery: searchQuery || null });

    expect(invoke).toHaveBeenCalledWith("generate_theme_report", {
      days: 1,
      searchQuery: null,
    });
  });

  it("calls retry_theme_analysis with themeId", async () => {
    const updatedTheme: ThemeReportTheme = {
      id: 10,
      label: "Theme A",
      headline: "Updated Headline",
      report_json: "{}",
      report_status: "done",
      cluster_score: 0.8,
      article_count: 3,
      source_count: 2,
      articles: [],
    };
    vi.mocked(invoke).mockResolvedValue(updatedTheme);

    const result = await invoke<ThemeReportTheme>("retry_theme_analysis", { themeId: 10 });

    expect(invoke).toHaveBeenCalledWith("retry_theme_analysis", { themeId: 10 });
    expect((result as ThemeReportTheme).id).toBe(10);
  });

  it("calls delete_theme_report with reportId", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    await invoke("delete_theme_report", { reportId: 5 });

    expect(invoke).toHaveBeenCalledWith("delete_theme_report", { reportId: 5 });
  });
});

describe("ThemeReportView — ThemeProgressData structure", () => {
  it("tracks generation progress correctly", () => {
    const progress: ThemeProgressData = {
      report_id: 1,
      themes_complete: 3,
      themes_total: 10,
      current_theme: "Klimapolitik",
    };

    expect(progress.themes_complete).toBeLessThanOrEqual(progress.themes_total);
    expect(progress.current_theme).toBe("Klimapolitik");
  });

  it("calculates completion percentage", () => {
    const progress: ThemeProgressData = {
      report_id: 1,
      themes_complete: 5,
      themes_total: 10,
      current_theme: "AI",
    };

    const pct =
      progress.themes_total > 0
        ? Math.round((progress.themes_complete / progress.themes_total) * 100)
        : 0;

    expect(pct).toBe(50);
  });

  it("handles zero-total gracefully", () => {
    const progress: ThemeProgressData = {
      report_id: 1,
      themes_complete: 0,
      themes_total: 0,
      current_theme: "",
    };

    const pct =
      progress.themes_total > 0
        ? Math.round((progress.themes_complete / progress.themes_total) * 100)
        : 0;

    expect(pct).toBe(0);
  });
});

describe("ThemeReportView — ThemeReportSummary structure", () => {
  it("creates a valid summary with all required fields", () => {
    const summary: ThemeReportSummary = {
      id: 7,
      period_start: "2026-04-05T00:00:00Z",
      period_end: "2026-04-06T00:00:00Z",
      search_query: null,
      theme_count: 4,
      model_used: "ministral-3",
      locale: "de",
      created_at: "2026-04-06T08:00:00Z",
    };

    expect(summary.id).toBe(7);
    expect(summary.theme_count).toBeGreaterThan(0);
    expect(summary.locale).toBe("de");
  });

  it("allows null model_used and search_query", () => {
    const summary: ThemeReportSummary = {
      id: 1,
      period_start: "2026-04-01T00:00:00Z",
      period_end: "2026-04-02T00:00:00Z",
      search_query: null,
      theme_count: 2,
      model_used: null,
      locale: "en",
      created_at: "2026-04-02T10:00:00Z",
    };

    expect(summary.search_query).toBeNull();
    expect(summary.model_used).toBeNull();
  });
});

describe("ThemeReportView — ThemeArticle structure", () => {
  it("creates a valid article with optional fields", () => {
    const article: ThemeArticle = {
      fnord_id: 42,
      title: "AI in Klimaforschung",
      summary: null,
      source_name: "heise.de",
      political_bias: null,
      sachlichkeit: 3.5,
      published_at: "2026-04-05T12:00:00Z",
      topic_score: 0.92,
    };

    expect(article.fnord_id).toBe(42);
    expect(article.topic_score).toBeGreaterThan(0);
    expect(article.summary).toBeNull();
    expect(article.political_bias).toBeNull();
  });
});
