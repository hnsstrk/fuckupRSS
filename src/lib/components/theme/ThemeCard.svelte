<script lang="ts">
  import { _ } from "svelte-i18n";
  import { renderMarkdown } from "$lib/utils/sanitizer";
  import { getBiasColor } from "$lib/utils/articleFormat";
  import { navigationStore } from "$lib/stores/navigation.svelte";

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

  interface ThemeReportJson {
    tldr: { core_message: string; key_divergence: string };
    headline: string;
    period: string;
    summary: string;
    consensus: string[];
    timeline: { date: string; development: string }[];
    divergences: {
      topic: string;
      positions: { stance: string; sources: string[]; article_indices: number[] }[];
    }[];
    sources: {
      name: string;
      article_count: number;
      bias_label: string;
      avg_sachlichkeit: number;
    }[];
    article_indices: number[];
  }

  let {
    theme,
    expanded,
    ontoggle,
    onretry,
    onarticlenavigate,
  }: {
    theme: ThemeReportTheme;
    expanded: boolean;
    ontoggle: () => void;
    onretry: (themeId: number) => void;
    onarticlenavigate: (fnordId: number) => void;
  } = $props();

  let parsedReport = $derived.by(() => {
    if (!theme.report_json) return null;
    try {
      return JSON.parse(theme.report_json) as ThemeReportJson;
    } catch {
      return null;
    }
  });

  function statusBadgeClass(status: string): string {
    switch (status) {
      case "complete":
        return "tr-badge-complete";
      case "failed":
        return "tr-badge-failed";
      default:
        return "tr-badge-pending";
    }
  }

  function statusLabel(status: string): string {
    switch (status) {
      case "complete":
        return $_("themeReport.complete");
      case "failed":
        return $_("themeReport.failed");
      default:
        return $_("themeReport.pending");
    }
  }

  function biasIndicator(bias: number | null): string {
    if (bias === null) return "";
    const labels: Record<number, string> = {
      "-2": "<<",
      "-1": "<",
      0: "=",
      1: ">",
      2: ">>",
    };
    return labels[bias] ?? "";
  }

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return new Intl.DateTimeFormat(undefined, {
        day: "2-digit",
        month: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      }).format(date);
    } catch {
      return dateStr;
    }
  }

  function navigateToArticle(fnordId: number) {
    navigationStore.navigateToArticle(fnordId);
    onarticlenavigate(fnordId);
  }
</script>

<div class="tr-theme-card" class:expanded>
  <!-- Collapsed header (always visible) -->
  <button class="tr-theme-header" onclick={ontoggle}>
    <div class="tr-theme-title-row">
      <span class="tr-theme-label">{theme.headline || theme.label}</span>
      <span class="tr-status-badge {statusBadgeClass(theme.report_status)}">
        {statusLabel(theme.report_status)}
      </span>
    </div>
    <div class="tr-theme-meta">
      <span class="tr-meta-item">
        <i class="fa-solid fa-newspaper"></i>
        {theme.article_count} {$_("themeReport.articles")}
      </span>
      <span class="tr-meta-item">
        <i class="fa-solid fa-rss"></i>
        {theme.source_count} {$_("themeReport.sources")}
      </span>
      <i
        class="fa-solid tr-expand-icon"
        class:fa-chevron-down={!expanded}
        class:fa-chevron-up={expanded}
      ></i>
    </div>
  </button>

  <!-- Expanded detail -->
  {#if expanded}
    <div class="tr-theme-body">
      {#if theme.report_status === "failed"}
        <div class="tr-failed-banner">
          <i class="fa-solid fa-triangle-exclamation"></i>
          {$_("themeReport.failed")}
          <button class="tr-retry-btn" onclick={() => onretry(theme.id)}>
            <i class="fa-solid fa-rotate-right"></i>
            {$_("themeReport.retry")}
          </button>
        </div>
      {:else if theme.report_status === "pending"}
        <div class="tr-pending-indicator">
          <i class="fa-solid fa-spinner fa-spin"></i>
          {$_("themeReport.pending")}
        </div>
      {:else if parsedReport}
        <!-- TL;DR Block -->
        <div class="tr-tldr">
          <h4 class="tr-tldr-title">
            <i class="fa-solid fa-bolt"></i>
            {$_("themeReport.tldr")}
          </h4>
          <div class="tr-tldr-content">
            <div class="tr-tldr-core markdown-content">
              {@html renderMarkdown(parsedReport.tldr.core_message)}
            </div>
            {#if parsedReport.tldr.key_divergence}
              <div class="tr-tldr-divergence">
                {parsedReport.tldr.key_divergence}
              </div>
            {/if}
          </div>
        </div>

        <!-- Konsens -->
        {#if parsedReport.consensus && parsedReport.consensus.length > 0}
          <div class="tr-section">
            <h4 class="tr-section-title">
              <i class="fa-solid fa-handshake"></i>
              {$_("themeReport.consensus")}
            </h4>
            <ul class="tr-consensus-list">
              {#each parsedReport.consensus as point, i (i)}
                <li>{point}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <!-- Timeline -->
        {#if parsedReport.timeline && parsedReport.timeline.length > 0}
          <div class="tr-section">
            <h4 class="tr-section-title">
              <i class="fa-solid fa-clock-rotate-left"></i>
              {$_("themeReport.timeline")}
            </h4>
            <div class="tr-timeline">
              {#each parsedReport.timeline as entry, i (i)}
                <div class="tr-timeline-entry">
                  <span class="tr-timeline-date">{entry.date}</span>
                  <span class="tr-timeline-desc">{entry.development}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Divergenzen -->
        {#if parsedReport.divergences && parsedReport.divergences.length > 0}
          <div class="tr-section">
            <h4 class="tr-section-title">
              <i class="fa-solid fa-code-branch"></i>
              {$_("themeReport.divergences")}
            </h4>
            {#each parsedReport.divergences as div, di (di)}
              <div class="tr-divergence-group">
                <h5 class="tr-divergence-topic">{div.topic}</h5>
                <div class="tr-positions">
                  {#each div.positions as pos, pi (pi)}
                    <div class="tr-position-card">
                      <div class="tr-position-stance">{pos.stance}</div>
                      <div class="tr-position-sources">
                        {#each pos.sources as source (source)}
                          <span class="tr-source-badge">{source}</span>
                        {/each}
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <!-- Quellen -->
        {#if parsedReport.sources && parsedReport.sources.length > 0}
          <div class="tr-section">
            <h4 class="tr-section-title">
              <i class="fa-solid fa-rss"></i>
              {$_("themeReport.sources")}
            </h4>
            <div class="tr-sources-table">
              {#each parsedReport.sources as src (src.name)}
                <div class="tr-source-row">
                  <span class="tr-source-name">{src.name}</span>
                  <span class="tr-source-count">{src.article_count}</span>
                  <span class="tr-source-bias">{src.bias_label}</span>
                  <span class="tr-source-sach">{src.avg_sachlichkeit.toFixed(1)}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Artikel -->
        {#if theme.articles && theme.articles.length > 0}
          <div class="tr-section">
            <h4 class="tr-section-title">
              <i class="fa-solid fa-newspaper"></i>
              {theme.articles.length} {$_("themeReport.articles")}
            </h4>
            <div class="tr-articles">
              {#each theme.articles as article (article.fnord_id)}
                <button
                  class="tr-article-link"
                  onclick={() => navigateToArticle(article.fnord_id)}
                  title={article.source_name}
                >
                  <span class="tr-article-source">{article.source_name}</span>
                  {#if article.political_bias !== null}
                    <span class="tr-bias-indicator bias-{getBiasColor(article.political_bias, 'class')}">
                      {biasIndicator(article.political_bias)}
                    </span>
                  {/if}
                  <span class="tr-article-title">{article.title}</span>
                  <span class="tr-article-date">{formatDate(article.published_at)}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .tr-theme-card {
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-primary);
    overflow: hidden;
    transition: border-color 0.15s;
  }

  .tr-theme-card:hover {
    border-color: var(--border-hover, var(--border-color));
  }

  .tr-theme-card.expanded {
    border-color: var(--accent-color);
  }

  /* Header (toggle) */
  .tr-theme-header {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    width: 100%;
    padding: 0.75rem 1rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-family: inherit;
    transition: background 0.1s;
  }

  .tr-theme-header:hover {
    background: var(--bg-hover);
  }

  .tr-theme-title-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .tr-theme-label {
    font-size: 0.92rem;
    font-weight: 600;
    line-height: 1.3;
    color: var(--text-primary);
  }

  .tr-status-badge {
    display: inline-block;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-size: 0.68rem;
    font-weight: 600;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tr-badge-complete {
    background: color-mix(in srgb, var(--green, #98c379) 15%, transparent);
    color: var(--green, #98c379);
  }

  .tr-badge-failed {
    background: color-mix(in srgb, var(--red, #e06c75) 15%, transparent);
    color: var(--red, #e06c75);
  }

  .tr-badge-pending {
    background: color-mix(in srgb, var(--yellow, #e5c07b) 15%, transparent);
    color: var(--yellow, #e5c07b);
  }

  .tr-theme-meta {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .tr-meta-item {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .tr-expand-icon {
    margin-left: auto;
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  /* Body (expanded) */
  .tr-theme-body {
    padding: 0 1rem 1rem;
    border-top: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  /* Failed / Pending states */
  .tr-failed-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 0.75rem;
    margin-top: 0.75rem;
    background: color-mix(in srgb, var(--red, #e06c75) 10%, transparent);
    border: 1px solid var(--red, #e06c75);
    border-radius: 6px;
    color: var(--red, #e06c75);
    font-size: 0.85rem;
  }

  .tr-retry-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    margin-left: auto;
    padding: 0.3rem 0.6rem;
    background: var(--red, #e06c75);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.78rem;
    font-weight: 500;
    transition: opacity 0.15s;
  }

  .tr-retry-btn:hover {
    opacity: 0.85;
  }

  .tr-pending-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    margin-top: 0.75rem;
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  /* TL;DR Block */
  .tr-tldr {
    margin-top: 0.75rem;
    padding: 0.875rem 1rem;
    background: color-mix(in srgb, var(--accent-color) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent-color) 25%, transparent);
    border-radius: 8px;
  }

  .tr-tldr-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.5rem;
    font-size: 0.92rem;
    font-weight: 700;
    color: var(--accent-color);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .tr-tldr-content :global(p) {
    margin: 0 0 0.4rem;
    font-size: 0.875rem;
    line-height: 1.6;
    color: var(--text-primary);
  }

  .tr-tldr-content :global(p:last-child) {
    margin-bottom: 0;
  }

  .tr-tldr-core {
    font-weight: 500;
  }

  .tr-tldr-divergence {
    font-style: italic;
    color: var(--text-secondary);
    font-size: 0.84rem;
    margin-top: 0.4rem;
    line-height: 1.5;
  }

  /* Generic section */
  .tr-section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .tr-section-title {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0;
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .tr-section-title i {
    color: var(--accent-color);
    font-size: 0.82rem;
  }

  /* Consensus */
  .tr-consensus-list {
    margin: 0;
    padding-left: 1.25rem;
    font-size: 0.84rem;
    line-height: 1.6;
    color: var(--text-primary);
  }

  .tr-consensus-list li {
    margin-bottom: 0.2rem;
  }

  /* Timeline */
  .tr-timeline {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding-left: 0.5rem;
    border-left: 2px solid color-mix(in srgb, var(--accent-color) 30%, transparent);
  }

  .tr-timeline-entry {
    display: flex;
    gap: 0.5rem;
    font-size: 0.84rem;
    padding-left: 0.5rem;
  }

  .tr-timeline-date {
    font-weight: 600;
    color: var(--text-secondary);
    white-space: nowrap;
    min-width: 5rem;
  }

  .tr-timeline-desc {
    color: var(--text-primary);
    line-height: 1.5;
  }

  /* Divergences */
  .tr-divergence-group {
    margin-bottom: 0.5rem;
  }

  .tr-divergence-topic {
    margin: 0 0 0.3rem;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .tr-positions {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .tr-position-card {
    padding: 0.5rem 0.65rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
  }

  .tr-position-stance {
    font-size: 0.84rem;
    color: var(--text-primary);
    line-height: 1.5;
    margin-bottom: 0.3rem;
  }

  .tr-position-sources {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .tr-source-badge {
    display: inline-block;
    padding: 0.1rem 0.4rem;
    background: var(--bg-tertiary, var(--bg-secondary));
    color: var(--text-secondary);
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 500;
  }

  /* Sources table */
  .tr-sources-table {
    display: flex;
    flex-direction: column;
    gap: 1px;
    background: var(--border-color);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    overflow: hidden;
  }

  .tr-source-row {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    gap: 0.75rem;
    padding: 0.4rem 0.65rem;
    background: var(--bg-primary);
    font-size: 0.8rem;
    align-items: center;
  }

  .tr-source-name {
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tr-source-count {
    color: var(--text-secondary);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .tr-source-bias {
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .tr-source-sach {
    color: var(--text-muted);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }

  /* Articles list */
  .tr-articles {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tr-article-link {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.5rem;
    background: none;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    color: var(--text-primary);
    font-size: 0.82rem;
    transition: all 0.15s;
  }

  .tr-article-link:hover {
    background: color-mix(in srgb, var(--accent-color) 8%, transparent);
    border-color: var(--accent-color);
  }

  .tr-article-source {
    display: inline-block;
    padding: 0.1rem 0.35rem;
    background: var(--bg-tertiary, var(--bg-secondary));
    color: var(--text-secondary);
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 500;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tr-bias-indicator {
    font-size: 0.75rem;
    font-weight: 700;
    font-family: monospace;
    flex-shrink: 0;
  }

  .tr-article-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--accent-color);
  }

  .tr-article-date {
    font-size: 0.7rem;
    color: var(--text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }
</style>
