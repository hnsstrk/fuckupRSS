import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createLogger } from "../logger";

const log = createLogger("themeReportsStore");

export interface ThemeReportSummary {
  id: number;
  period_start: string;
  period_end: string;
  search_query: string | null;
  theme_count: number;
  model_used: string | null;
  locale: string;
  created_at: string;
}

export interface ThemeArticle {
  fnord_id: number;
  title: string;
  summary: string | null;
  source_name: string;
  political_bias: number | null;
  sachlichkeit: number | null;
  published_at: string;
  topic_score: number;
}

export interface ThemeReportTheme {
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

export interface ThemeReportDetail {
  report: ThemeReportSummary;
  themes: ThemeReportTheme[];
}

export interface ThemeProgressData {
  report_id: number;
  themes_complete: number;
  themes_total: number;
  current_theme: string;
}

export class ThemeReportStore {
  reports = $state<ThemeReportSummary[]>([]);
  selectedReportId = $state<number | null>(null);
  reportDetail = $state<ThemeReportDetail | null>(null);
  generating = $state(false);
  progress = $state<ThemeProgressData | null>(null);
  days = $state(1);
  searchQuery = $state("");
  minSources = $state(2);
  loading = $state(false);
  detailLoading = $state(false);

  private initialized = false;
  private loadPromise: Promise<void> | null = null;
  private generatePromise: Promise<void> | null = null;
  private progressListenerPromise: Promise<void> | null = null;

  constructor() {
    void this.ensureProgressListener();
  }

  async ensureLoaded(): Promise<void> {
    if (this.initialized || this.loading) {
      return this.loadPromise ?? Promise.resolve();
    }

    return this.loadReports();
  }

  async ensureProgressListener(): Promise<void> {
    if (this.progressListenerPromise) {
      return this.progressListenerPromise;
    }

    this.progressListenerPromise = (async () => {
      try {
        await listen<ThemeProgressData>("theme-report-progress", (event) => {
          this.progress = event.payload;
        });
      } catch (e) {
        log.error("Failed to listen for theme-report-progress:", e);
      }
    })();

    return this.progressListenerPromise;
  }

  async loadReports(): Promise<void> {
    if (this.loadPromise) {
      return this.loadPromise;
    }

    this.loading = true;
    this.loadPromise = (async () => {
      try {
        const fetchedReports = await invoke<ThemeReportSummary[]>("get_theme_reports", {
          limit: 30,
        });
        const fetchedIds = fetchedReports.map((report) => report.id);
        const localOnlyReports = this.reports.filter((report) => !fetchedIds.includes(report.id));

        // Preserve locally inserted reports while a stale fetch is still in flight.
        this.reports = [...localOnlyReports, ...fetchedReports];
        this.initialized = true;
      } catch (e) {
        log.error("Error loading theme reports:", e);
      } finally {
        this.loading = false;
        this.loadPromise = null;
      }
    })();

    return this.loadPromise;
  }

  async selectReport(reportId: number): Promise<void> {
    if (this.detailLoading) return;

    this.selectedReportId = reportId;
    this.detailLoading = true;

    try {
      this.reportDetail = await invoke<ThemeReportDetail>("get_theme_report_detail", {
        reportId,
      });
    } catch (e) {
      log.error("Error loading report detail:", e);
      this.reportDetail = null;
    } finally {
      this.detailLoading = false;
    }
  }

  async generateReport(): Promise<void> {
    if (this.generatePromise) {
      return this.generatePromise;
    }

    this.generating = true;
    this.progress = null;

    this.generatePromise = (async () => {
      try {
        const detail = await invoke<ThemeReportDetail>("generate_theme_report", {
          days: this.days,
          searchQuery: this.searchQuery || null,
          minSources: this.minSources,
        });

        this.reportDetail = detail;
        this.selectedReportId = detail.report.id;
        this.reports = [
          detail.report,
          ...this.reports.filter((report) => report.id !== detail.report.id),
        ];
        this.initialized = true;

        void this.loadReports();
      } catch (e) {
        log.error("Error generating theme report:", e);
      } finally {
        this.generating = false;
        this.progress = null;
        this.generatePromise = null;
      }
    })();

    return this.generatePromise;
  }

  async retryTheme(themeId: number): Promise<void> {
    try {
      const updatedTheme = await invoke<ThemeReportTheme>("retry_theme_analysis", {
        themeId,
      });

      if (this.reportDetail) {
        this.reportDetail = {
          ...this.reportDetail,
          themes: this.reportDetail.themes.map((theme) =>
            theme.id === themeId ? updatedTheme : theme,
          ),
        };
      }
    } catch (e) {
      log.error("Error retrying theme analysis:", e);
    }
  }

  async deleteReport(): Promise<void> {
    if (!this.selectedReportId) return;

    try {
      await invoke("delete_theme_report", {
        reportId: this.selectedReportId,
      });
      this.reports = this.reports.filter((report) => report.id !== this.selectedReportId);
      this.selectedReportId = null;
      this.reportDetail = null;
    } catch (e) {
      log.error("Error deleting theme report:", e);
    }
  }
}

export const themeReportStore = new ThemeReportStore();
