<script lang="ts">
  import { _ } from "svelte-i18n";

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

  let {
    reports,
    selectedReportId,
    onselectreport,
  }: {
    reports: ThemeReportSummary[];
    selectedReportId: number | null;
    onselectreport: (id: number) => void;
  } = $props();

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr + "Z");
      return new Intl.DateTimeFormat(undefined, {
        day: "2-digit",
        month: "2-digit",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      }).format(date);
    } catch {
      return dateStr;
    }
  }

  function formatPeriod(start: string, end: string): string {
    try {
      const s = new Date(start + "Z");
      const e = new Date(end + "Z");
      const diffMs = e.getTime() - s.getTime();
      const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));
      if (diffDays <= 1) return "24h";
      return `${diffDays}d`;
    } catch {
      return "";
    }
  }
</script>

<div class="tr-list-panel">
  {#if reports.length === 0}
    <div class="tr-empty">
      <i class="fa-light fa-newspaper"></i>
      <p>{$_("themeReport.noReports")}</p>
    </div>
  {:else}
    <div class="tr-report-list">
      {#each reports as report (report.id)}
        <button
          class="tr-report-card"
          class:selected={selectedReportId === report.id}
          onclick={() => onselectreport(report.id)}
        >
          <div class="tr-card-top">
            <span class="tr-card-date">{formatDate(report.created_at)}</span>
            <span class="tr-period-badge">{formatPeriod(report.period_start, report.period_end)}</span>
          </div>
          <div class="tr-card-meta">
            <span class="tr-meta-item">
              <i class="fa-solid fa-layer-group"></i>
              {report.theme_count}
            </span>
            {#if report.model_used}
              <span class="tr-meta-item tr-model">
                <i class="fa-solid fa-robot"></i>
                {report.model_used}
              </span>
            {/if}
          </div>
          {#if report.search_query}
            <div class="tr-card-query">
              <i class="fa-solid fa-filter"></i>
              {report.search_query}
            </div>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tr-list-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
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

  .tr-report-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
  }

  .tr-report-card {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.6rem 0.75rem;
    background: var(--bg-primary);
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
    width: 100%;
    font-family: inherit;
    color: inherit;
  }

  .tr-report-card:hover {
    background: var(--bg-hover);
    border-color: var(--border-color);
  }

  .tr-report-card.selected {
    background: var(--bg-active, var(--bg-secondary));
    border-color: var(--accent-color);
  }

  .tr-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .tr-card-date {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .tr-period-badge {
    display: inline-block;
    padding: 0.1rem 0.4rem;
    background: color-mix(in srgb, var(--accent-color) 15%, transparent);
    color: var(--accent-color);
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 600;
  }

  .tr-card-meta {
    display: flex;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .tr-meta-item {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .tr-model {
    font-size: 0.7rem;
    opacity: 0.8;
  }

  .tr-card-query {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.72rem;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
