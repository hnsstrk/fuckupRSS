<script lang="ts">
  import { _ } from "svelte-i18n";
  import { renderMarkdown, renderMarkdownInline } from "$lib/utils/sanitizer";
  import {
    type Briefing,
    type ArticleRef,
    type StructuredBriefing,
  } from "$lib/stores/briefings.svelte";

  let {
    briefing,
    ondelete,
    onarticlenavigate,
    onkeywordnavigate,
  }: {
    briefing: Briefing;
    ondelete: (id: number) => void;
    onarticlenavigate: (fnordId: number) => void;
    onkeywordnavigate: (keyword: string) => void;
  } = $props();

  let expanded = $state(false);

  function formatPeriod(start: string, end: string): string {
    const startDate = new Date(start + "Z");
    const endDate = new Date(end + "Z");
    const fmt = new Intl.DateTimeFormat(undefined, {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
    return `${fmt.format(startDate)} — ${fmt.format(endDate)}`;
  }

  function formatCreatedAt(dateStr: string): string {
    const date = new Date(dateStr + "Z");
    return new Intl.DateTimeFormat(undefined, {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(date);
  }

  function getPeriodBadgeClass(periodType: string): string {
    return periodType === "daily" ? "badge-daily" : "badge-weekly";
  }

  function parseStructuredContent(content: string): StructuredBriefing | null {
    try {
      const parsed = JSON.parse(content);
      if (parsed.tldr && parsed.topics) return parsed as StructuredBriefing;
    } catch {
      /* Legacy markdown briefing */
    }
    return null;
  }

  function parseArticleRefs(refsJson: string | null): ArticleRef[] {
    if (!refsJson) return [];
    try {
      return JSON.parse(refsJson) as ArticleRef[];
    } catch {
      return [];
    }
  }
</script>

<div class="briefing-card" class:expanded>
  <button class="briefing-card-header" onclick={() => (expanded = !expanded)}>
    <div class="card-left">
      <span class="period-badge {getPeriodBadgeClass(briefing.period_type)}">
        {briefing.period_type === "daily" ? $_("briefing.daily") : $_("briefing.weekly")}
      </span>
      <span class="card-meta">
        <i class="fa-solid fa-newspaper"></i>
        {briefing.article_count}
        {$_("briefing.articles")}
      </span>
      {#if briefing.model_used}
        <span class="card-meta model-meta">
          <i class="fa-solid fa-robot"></i>
          {briefing.model_used}
        </span>
      {/if}
    </div>
    <div class="card-right">
      <span class="card-date">
        {formatCreatedAt(briefing.created_at)}
      </span>
      <i class="fa-solid {expanded ? 'fa-chevron-up' : 'fa-chevron-down'} expand-icon"></i>
    </div>
  </button>

  {#if expanded}
    {@const structured = parseStructuredContent(briefing.content)}
    {@const articleRefs = parseArticleRefs(briefing.article_refs)}
    <div class="briefing-card-body">
      <div class="briefing-period-info">
        <span class="period-label">
          <i class="fa-solid fa-clock"></i>
          {$_("briefing.period")}:
        </span>
        <span class="period-value">
          {formatPeriod(briefing.period_start, briefing.period_end)}
        </span>
      </div>

      <!-- eslint-disable svelte/no-at-html-tags -->
      {#if structured}
        <div class="briefing-tldr">
          <h3 class="tldr-title">
            <i class="fa-solid fa-bolt"></i>
            TL;DR
          </h3>
          <div class="tldr-content">
            <div class="tldr-overview markdown-content">
              {@html renderMarkdown(structured.tldr.overview)}
            </div>
            {#if structured.tldr.trends}
              <div class="tldr-trends markdown-content">
                <strong>{$_("briefing.trends")}:</strong>
                {@html renderMarkdown(structured.tldr.trends)}
              </div>
            {/if}
            {#if structured.tldr.conclusion}
              <div class="tldr-conclusion markdown-content">
                <strong>{$_("briefing.conclusion")}:</strong>
                {@html renderMarkdown(structured.tldr.conclusion)}
              </div>
            {/if}
          </div>
        </div>

        <div class="briefing-topics">
          {#each structured.topics as topic, topicIdx (topicIdx)}
            <div class="briefing-topic">
              <h4 class="topic-title">{@html renderMarkdownInline(topic.title)}</h4>
              <div class="topic-body markdown-content">
                {@html renderMarkdown(topic.body)}
              </div>

              {#if topic.article_indices.length > 0 && articleRefs.length > 0}
                <div class="topic-articles">
                  <i class="fa-solid fa-newspaper topic-articles-icon"></i>
                  {#each topic.article_indices as idx (idx)}
                    {@const ref = articleRefs.find((r) => r.index === idx)}
                    {#if ref}
                      <button
                        class="article-link"
                        onclick={() => onarticlenavigate(ref.fnord_id)}
                        title={ref.source}
                      >
                        {ref.title}
                      </button>
                    {/if}
                  {/each}
                </div>
              {/if}

              {#if topic.keywords.length > 0}
                {@const uniqueKeywords = [...new Set(topic.keywords)]}
                <div class="topic-keywords">
                  {#each uniqueKeywords as keyword (keyword)}
                    <button class="keyword-badge" onclick={() => onkeywordnavigate(keyword)}>
                      {keyword}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        {#if briefing.top_keywords}
          <div class="briefing-keywords">
            <span class="keywords-label">
              <i class="fa-solid fa-tags"></i>
              {$_("briefing.topKeywords")}:
            </span>
            <span class="keywords-value">{briefing.top_keywords}</span>
          </div>
        {/if}

        <div class="briefing-text markdown-content">
          {@html renderMarkdown(briefing.content)}
        </div>
      {/if}

      <div class="briefing-card-actions">
        <button
          class="btn btn-danger btn-sm"
          onclick={(e: MouseEvent) => {
            e.stopPropagation();
            if (confirm($_("briefing.deleteConfirm"))) {
              ondelete(briefing.id);
            }
          }}
        >
          <i class="fa-solid fa-trash"></i>
          {$_("briefing.delete")}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .briefing-card {
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    background-color: var(--bg-elevated);
    overflow: clip;
    transition: border-color 0.15s;
  }

  .briefing-card:hover {
    border-color: var(--border-hover);
  }

  .briefing-card.expanded {
    border-color: var(--accent-primary);
  }

  .briefing-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.75rem 1rem;
    border: none;
    background: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-size: 0.875rem;
    gap: 0.75rem;
  }

  .briefing-card-header:hover {
    background-color: var(--bg-overlay);
  }

  .card-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .period-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.1875rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .badge-daily {
    background-color: color-mix(in srgb, var(--accent-warning) 20%, transparent);
    color: var(--accent-warning);
  }

  .badge-weekly {
    background-color: color-mix(in srgb, var(--accent-primary) 20%, transparent);
    color: var(--accent-primary);
  }

  .card-meta {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--text-muted);
    font-size: 0.8125rem;
  }

  .model-meta {
    font-size: 0.75rem;
    opacity: 0.8;
  }

  .card-right {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-shrink: 0;
  }

  .card-date {
    color: var(--text-muted);
    font-size: 0.75rem;
    white-space: nowrap;
  }

  .expand-icon {
    color: var(--text-muted);
    font-size: 0.75rem;
    transition: transform 0.15s;
  }

  .briefing-card-body {
    padding: 0 1rem 1rem;
    border-top: 1px solid var(--border-default);
  }

  .briefing-period-info,
  .briefing-keywords {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.75rem 0 0.5rem;
    font-size: 0.8125rem;
  }

  .period-label,
  .keywords-label {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--text-muted);
    font-weight: 500;
    white-space: nowrap;
  }

  .period-value,
  .keywords-value {
    color: var(--text-secondary);
  }

  .keywords-value {
    line-height: 1.5;
  }

  .briefing-text {
    padding: 0.75rem 0;
    line-height: 1.7;
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .briefing-text :global(p) {
    margin: 0 0 0.625rem;
  }

  .briefing-text :global(p:last-child) {
    margin-bottom: 0;
  }

  .briefing-card-actions {
    display: flex;
    justify-content: flex-end;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-default);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 0.875rem;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color 0.15s,
      filter 0.15s;
  }

  .btn-danger {
    background-color: var(--accent-error);
    color: white;
  }

  .btn-danger:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-sm {
    padding: 0.375rem 0.625rem;
    font-size: 0.75rem;
  }

  /* TL;DR Block */
  .briefing-tldr {
    margin: 0.75rem 0;
    padding: 0.875rem 1rem;
    background-color: color-mix(in srgb, var(--accent-primary) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent-primary) 25%, transparent);
    border-radius: 0.5rem;
  }

  .tldr-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.625rem;
    font-size: 0.9375rem;
    font-weight: 700;
    color: var(--accent-primary);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .tldr-content :global(p) {
    margin: 0 0 0.5rem;
    font-size: 0.875rem;
    line-height: 1.6;
    color: var(--text-primary);
  }

  .tldr-content :global(p:last-child) {
    margin-bottom: 0;
  }

  .tldr-content :global(ol),
  .tldr-content :global(ul) {
    margin: 0.375rem 0 0.5rem;
    padding-left: 1.5rem;
    font-size: 0.875rem;
    line-height: 1.6;
  }

  .tldr-content :global(li) {
    margin-bottom: 0.25rem;
  }

  .tldr-content :global(li:last-child) {
    margin-bottom: 0;
  }

  .tldr-trends,
  .tldr-conclusion {
    color: var(--text-secondary);
  }

  /* Topics */
  .briefing-topics {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .briefing-topic {
    padding: 0.75rem 0;
    border-bottom: 1px solid var(--border-default);
  }

  .briefing-topic:last-child {
    border-bottom: none;
  }

  .topic-title {
    margin: 0 0 0.375rem;
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .topic-body {
    font-size: 0.875rem;
    line-height: 1.6;
    color: var(--text-primary);
  }

  /* Artikel-Links */
  .topic-articles {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.375rem;
    margin-top: 0.5rem;
  }

  .topic-articles-icon {
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .article-link {
    display: inline-flex;
    align-items: center;
    padding: 0.1875rem 0.5rem;
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    background: none;
    color: var(--accent-primary);
    font-size: 0.75rem;
    cursor: pointer;
    transition:
      background-color 0.15s,
      border-color 0.15s;
    max-width: 20rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .article-link:hover {
    background-color: color-mix(in srgb, var(--accent-primary) 10%, transparent);
    border-color: var(--accent-primary);
  }

  /* Keyword-Badges */
  .topic-keywords {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.375rem;
  }

  .keyword-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.4375rem;
    border: 1px solid color-mix(in srgb, var(--accent-primary) 30%, transparent);
    border-radius: 1rem;
    background: color-mix(in srgb, var(--accent-primary) 8%, transparent);
    color: var(--text-secondary);
    font-size: 0.6875rem;
    cursor: pointer;
    transition:
      background-color 0.15s,
      color 0.15s;
  }

  .keyword-badge:hover {
    background: color-mix(in srgb, var(--accent-primary) 18%, transparent);
    color: var(--text-primary);
  }
</style>
