import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  ThemeReportStore,
  type ThemeProgressData,
  type ThemeReportDetail,
  type ThemeReportSummary,
} from "../../stores/themeReports.svelte";

const mocks = vi.hoisted(() => ({
  progressListener: null as ((event: { payload: ThemeProgressData }) => void) | null,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((_eventName: string, callback: (event: { payload: ThemeProgressData }) => void) => {
    mocks.progressListener = callback;
    return Promise.resolve(() => {
      if (mocks.progressListener === callback) {
        mocks.progressListener = null;
      }
    });
  }),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

function makeDetail(id: number): ThemeReportDetail {
  return {
    report: {
      id,
      period_start: "2026-04-15T00:00:00Z",
      period_end: "2026-04-16T00:00:00Z",
      search_query: null,
      theme_count: 3,
      model_used: "ministral-3",
      locale: "de",
      created_at: "2026-04-16T09:00:00Z",
    },
    themes: [
      {
        id: 11,
        label: "Theme A",
        headline: null,
        report_json: null,
        report_status: "pending",
        cluster_score: 0.8,
        article_count: 4,
        source_count: 2,
        articles: [],
      },
    ],
  };
}

describe("ThemeReportStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.progressListener = null;
  });

  it("keeps generation state and progress alive until the report finishes", async () => {
    const generate = deferred<ThemeReportDetail>();

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "generate_theme_report") {
        return generate.promise as Promise<unknown>;
      }
      if (command === "get_theme_reports") {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
    });

    const store = new ThemeReportStore();
    const pending = store.generateReport();

    expect(store.generating).toBe(true);

    const progress: ThemeProgressData = {
      report_id: 7,
      themes_complete: 2,
      themes_total: 5,
      current_theme: "Europa",
    };
    mocks.progressListener?.({ payload: progress });

    expect(store.progress).toEqual(progress);

    generate.resolve(makeDetail(7));
    await pending;

    expect(store.generating).toBe(false);
    expect(store.progress).toBeNull();
    expect(store.selectedReportId).toBe(7);
    expect(store.reportDetail?.report.id).toBe(7);
    expect(store.reports.map((report) => report.id)).toEqual([7]);
  });

  it("preserves a generated report when a stale list reload resolves later", async () => {
    const load = deferred<ThemeReportSummary[]>();
    const generate = deferred<ThemeReportDetail>();

    vi.mocked(invoke).mockImplementation((command: string) => {
      if (command === "get_theme_reports") {
        return load.promise as Promise<unknown>;
      }
      if (command === "generate_theme_report") {
        return generate.promise as Promise<unknown>;
      }
      return Promise.resolve(undefined);
    });

    const store = new ThemeReportStore();
    const pendingLoad = store.loadReports();
    const pendingGenerate = store.generateReport();

    generate.resolve(makeDetail(99));
    await pendingGenerate;

    load.resolve([
      {
        id: 1,
        period_start: "2026-04-14T00:00:00Z",
        period_end: "2026-04-15T00:00:00Z",
        search_query: "AI",
        theme_count: 2,
        model_used: "ministral-3",
        locale: "de",
        created_at: "2026-04-15T08:00:00Z",
      },
    ]);

    await pendingLoad;

    expect(store.reports.map((report) => report.id)).toEqual([99, 1]);
    expect(store.selectedReportId).toBe(99);
    expect(store.reportDetail?.report.id).toBe(99);
  });
});
