<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { SvelteSet } from "svelte/reactivity";
  import { createLogger } from "$lib/logger";
  import ThemeReportHeader from "./theme/ThemeReportHeader.svelte";
  import ThemeReportList from "./theme/ThemeReportList.svelte";
  import ThemeCard from "./theme/ThemeCard.svelte";
  import ThemeProgress from "./theme/ThemeProgress.svelte";

  const log = createLogger("ThemeReportView");

  // Types matching backend
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

  // State
  let reports = $state<ThemeReportSummary[]>([]);
  let selectedReportId = $state<number | null>(null);
  let reportDetail = $state<ThemeReportDetail | null>(null);
  let generating = $state(false);
  let progress = $state<ThemeProgressData | null>(null);
  let days = $state(1);
  let searchQuery = $state("");
  let expandedThemes = new SvelteSet<number>();
  let loading = $state(false);
  let detailLoading = $state(false);

  let detailPanelRef = $state<HTMLDivElement | null>(null);

  // Tauri event unlisten function
  let unlistenProgress: (() => void) | null = null;

  onMount(async () => {
    detailPanelRef?.scrollTo({ top: 0 });
    try {
      unlistenProgress = await listen<ThemeProgressData>(
        "theme-report-progress",
        (event) => {
          progress = event.payload;
        },
      );
    } catch (e) {
      log.error("Failed to listen for theme-report-progress:", e);
    }
    await loadReports();
  });

  onDestroy(() => {
    if (unlistenProgress) {
      unlistenProgress();
      unlistenProgress = null;
    }
  });

  async function loadReports() {
    loading = true;
    try {
      reports = await invoke<ThemeReportSummary[]>("get_theme_reports", {
        limit: 30,
      });
    } catch (e) {
      log.error("Error loading theme reports:", e);
      reports = [];
    } finally {
      loading = false;
    }
  }

  async function selectReport(reportId: number) {
    selectedReportId = reportId;
    detailLoading = true;
    expandedThemes.clear();
    try {
      reportDetail = await invoke<ThemeReportDetail>("get_theme_report_detail", {
        reportId: reportId,
      });
    } catch (e) {
      log.error("Error loading report detail:", e);
      reportDetail = null;
    } finally {
      detailLoading = false;
    }
  }

  async function handleGenerate() {
    generating = true;
    progress = null;
    try {
      const detail = await invoke<ThemeReportDetail>("generate_theme_report", {
        days,
        searchQuery: searchQuery || null,
      });
      // Refresh reports list and select new one
      await loadReports();
      reportDetail = detail;
      selectedReportId = detail.report.id;
      expandedThemes.clear();
    } catch (e) {
      log.error("Error generating theme report:", e);
    } finally {
      generating = false;
      progress = null;
    }
  }

  async function handleRetry(themeId: number) {
    try {
      const updatedTheme = await invoke<ThemeReportTheme>("retry_theme_analysis", {
        themeId: themeId,
      });
      // Update the theme in the detail view
      if (reportDetail) {
        reportDetail = {
          ...reportDetail,
          themes: reportDetail.themes.map((t) =>
            t.id === themeId ? updatedTheme : t,
          ),
        };
      }
    } catch (e) {
      log.error("Error retrying theme analysis:", e);
    }
  }

  async function handleDelete() {
    if (!selectedReportId) return;
    if (!confirm($_("themeReport.deleteConfirm"))) return;
    try {
      await invoke("delete_theme_report", {
        reportId: selectedReportId,
      });
      reports = reports.filter((r) => r.id !== selectedReportId);
      selectedReportId = null;
      reportDetail = null;
      expandedThemes.clear();
    } catch (e) {
      log.error("Error deleting theme report:", e);
    }
  }

  function toggleTheme(themeId: number) {
    if (expandedThemes.has(themeId)) {
      expandedThemes.delete(themeId);
    } else {
      expandedThemes.add(themeId);
    }
  }
</script>

<div class="theme-report-view">
  <ThemeReportHeader
    {days}
    {searchQuery}
    {generating}
    ongenerate={handleGenerate}
    ondayschange={(d) => (days = d)}
    onsearchchange={(q) => (searchQuery = q)}
  />

  <div class="tr-panels">
    <!-- Left: Report list -->
    <div class="tr-list-column">
      {#if loading}
        <div class="tr-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
        </div>
      {:else}
        <ThemeReportList
          {reports}
          {selectedReportId}
          onselectreport={selectReport}
        />
      {/if}
    </div>

    <!-- Right: Detail panel -->
    <div class="tr-detail-panel" bind:this={detailPanelRef}>
      {#if generating && progress}
        <ThemeProgress {progress} />
      {:else if generating}
        <div class="tr-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
          <span>{$_("themeReport.generating")}</span>
        </div>
      {:else if !selectedReportId}
        <div class="tr-empty">
          <i class="fa-solid fa-newspaper"></i>
          <p>{$_("themeReport.selectReport")}</p>
        </div>
      {:else if detailLoading}
        <div class="tr-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
        </div>
      {:else if reportDetail}
        <div class="tr-detail">
          <!-- Detail header with delete -->
          <div class="tr-detail-header">
            <div class="tr-detail-info">
              <span class="tr-detail-themes">
                {$_("themeReport.themesFound", {
                  values: { count: reportDetail.themes.length },
                })}
              </span>
              {#if reportDetail.report.model_used}
                <span class="tr-detail-model">
                  <i class="fa-solid fa-robot"></i>
                  {reportDetail.report.model_used}
                </span>
              {/if}
            </div>
            <button
              class="tr-btn-danger"
              onclick={handleDelete}
              title={$_("themeReport.delete")}
            >
              <i class="fa-solid fa-trash"></i>
              {$_("themeReport.delete")}
            </button>
          </div>

          <!-- Theme cards -->
          <div class="tr-themes-list">
            {#each reportDetail.themes as theme (theme.id)}
              <ThemeCard
                {theme}
                expanded={expandedThemes.has(theme.id)}
                ontoggle={() => toggleTheme(theme.id)}
                onretry={handleRetry}
                onarticlenavigate={() => {}}
              />
            {/each}
          </div>
        </div>
      {:else}
        <div class="tr-empty">
          <i class="fa-solid fa-triangle-exclamation"></i>
          <p>{$_("themeReport.noResults")}</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .theme-report-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
  }

  /* Two-panel layout */
  .tr-panels {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .tr-list-column {
    width: 300px;
    min-width: 240px;
    border-right: 1px solid var(--border-color);
    overflow-y: auto;
    flex-shrink: 0;
  }

  .tr-detail-panel {
    flex: 1;
    overflow-y: auto;
    min-width: 0;
  }

  /* Detail area */
  .tr-detail {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .tr-detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-color);
  }

  .tr-detail-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .tr-detail-themes {
    font-size: 0.92rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .tr-detail-model {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .tr-btn-danger {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.6rem;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
    transition: all 0.15s;
  }

  .tr-btn-danger:hover {
    background: var(--red, #e06c75);
    color: white;
    border-color: var(--red, #e06c75);
  }

  .tr-themes-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  /* Loading / Empty states */
  .tr-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: var(--text-muted);
    gap: 0.75rem;
    height: 100%;
    font-size: 0.9rem;
  }

  .tr-loading i {
    font-size: 1.5rem;
  }

  .tr-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: var(--text-muted);
    gap: 0.75rem;
    height: 100%;
  }

  .tr-empty i {
    font-size: 2.5rem;
    opacity: 0.4;
  }

  .tr-empty p {
    margin: 0;
    font-size: 0.9rem;
  }
</style>
