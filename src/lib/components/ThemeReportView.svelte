<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { themeReportStore } from "$lib/stores/themeReports.svelte";
  import ThemeReportHeader from "./theme/ThemeReportHeader.svelte";
  import ThemeReportList from "./theme/ThemeReportList.svelte";
  import ThemeCard from "./theme/ThemeCard.svelte";
  import ThemeProgress from "./theme/ThemeProgress.svelte";

  // State
  let detailPanelRef = $state<HTMLDivElement | null>(null);

  onMount(async () => {
    detailPanelRef?.scrollTo({ top: 0 });
    await themeReportStore.ensureLoaded();
  });
</script>

<div class="theme-report-view">
  <ThemeReportHeader
    days={themeReportStore.days}
    searchQuery={themeReportStore.searchQuery}
    minSources={themeReportStore.minSources}
    generating={themeReportStore.generating}
    ongenerate={() => themeReportStore.generateReport()}
    ondayschange={(d) => (themeReportStore.days = d)}
    onsearchchange={(q) => (themeReportStore.searchQuery = q)}
    onminsourceschange={(v) => (themeReportStore.minSources = v)}
  />

  <div class="tr-panels">
    <!-- Left: Report list -->
    <div class="tr-list-column">
      {#if themeReportStore.loading}
        <div class="tr-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
        </div>
      {:else}
        <ThemeReportList
          reports={themeReportStore.reports}
          selectedReportId={themeReportStore.selectedReportId}
          onselectreport={(id) => themeReportStore.selectReport(id)}
        />
      {/if}
    </div>

    <!-- Right: Detail panel -->
    <div class="tr-detail-panel" bind:this={detailPanelRef}>
      {#if themeReportStore.generating && themeReportStore.progress}
        <ThemeProgress progress={themeReportStore.progress} />
      {:else if themeReportStore.generating}
        <div class="tr-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
          <span>{$_("themeReport.generating")}</span>
        </div>
      {:else if !themeReportStore.selectedReportId}
        <div class="tr-empty">
          <i class="fa-solid fa-newspaper"></i>
          <p>{$_("themeReport.selectReport")}</p>
        </div>
      {:else if themeReportStore.detailLoading}
        <div class="tr-loading">
          <i class="fa-solid fa-spinner fa-spin"></i>
        </div>
      {:else if themeReportStore.reportDetail}
        <div class="tr-detail">
          <!-- Detail header with delete -->
          <div class="tr-detail-header">
            <div class="tr-detail-info">
              <span class="tr-detail-themes">
                {$_("themeReport.themesFound", {
                  values: { count: themeReportStore.reportDetail.themes.length },
                })}
              </span>
              {#if themeReportStore.reportDetail.report.model_used}
                <span class="tr-detail-model">
                  <i class="fa-solid fa-robot"></i>
                  {themeReportStore.reportDetail.report.model_used}
                </span>
              {/if}
            </div>
            <button
              class="tr-btn-danger"
              onclick={async () => {
                if (!confirm($_("themeReport.deleteConfirm"))) return;
                await themeReportStore.deleteReport();
              }}
              title={$_("themeReport.delete")}
            >
              <i class="fa-solid fa-trash"></i>
              {$_("themeReport.delete")}
            </button>
          </div>

          <!-- Theme cards -->
          <div class="tr-themes-list">
            {#each themeReportStore.reportDetail.themes as theme (theme.id)}
              <ThemeCard
                {theme}
                onretry={(themeId) => themeReportStore.retryTheme(themeId)}
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
