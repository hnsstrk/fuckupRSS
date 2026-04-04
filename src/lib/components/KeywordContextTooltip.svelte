<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { formatChangedDate } from "$lib/utils/articleFormat";

  // Type for keyword context from backend
  interface KeywordContext {
    sentence: string;
    article_title: string;
    article_date: string | null;
    article_id: number;
  }

  interface Props {
    keywordId: number;
    keywordName: string;
    children: import("svelte").Snippet;
    onclick?: () => void;
  }

  let { keywordId, keywordName, children, onclick }: Props = $props();

  let showTooltip = $state(false);
  let tooltipEl: HTMLElement | null = $state(null);
  let x = $state(0);
  let y = $state(0);
  let loading = $state(false);
  let context = $state<KeywordContext | null>(null);
  let loadError = $state<string | null>(null);

  import { SvelteMap } from "svelte/reactivity";
  import { createLogger } from "$lib/logger";

  const log = createLogger("KeywordContextTooltip");
  // Cache for keyword contexts (persists across hovers)
  const contextCache = new SvelteMap<number, KeywordContext>();

  async function loadContext() {
    // Check cache first
    if (contextCache.has(keywordId)) {
      context = contextCache.get(keywordId) || null;
      return;
    }

    loading = true;
    loadError = null;

    try {
      const result = await invoke<KeywordContext | null>("get_keyword_context", { keywordId });
      if (result) {
        context = result;
        contextCache.set(keywordId, result);
      } else {
        loadError = $_("compound.noContextAvailable") || "No context available";
      }
    } catch (e) {
      log.error("Failed to load keyword context:", e);
      loadError = $_("compound.contextLoadError") || "Failed to load context";
    } finally {
      loading = false;
    }
  }

  let hoverTimeout: ReturnType<typeof setTimeout> | null = null;

  function handleMouseEnter(event: MouseEvent) {
    // Delay showing tooltip to avoid flicker on quick mouse movements
    hoverTimeout = setTimeout(() => {
      showTooltip = true;
      updatePosition(event);
      // Load context if not cached
      if (!contextCache.has(keywordId)) {
        loadContext();
      } else {
        context = contextCache.get(keywordId) || null;
      }
    }, 300);
  }

  function handleMouseMove(event: MouseEvent) {
    if (showTooltip) {
      updatePosition(event);
    }
  }

  function handleMouseLeave() {
    if (hoverTimeout) {
      clearTimeout(hoverTimeout);
      hoverTimeout = null;
    }
    showTooltip = false;
  }

  function updatePosition(event: MouseEvent) {
    x = event.clientX + 10;
    y = event.clientY + 10;

    // Ensure tooltip stays within viewport
    if (tooltipEl) {
      const rect = tooltipEl.getBoundingClientRect();
      if (x + rect.width > window.innerWidth) {
        x = event.clientX - rect.width - 10;
      }
      if (y + rect.height > window.innerHeight) {
        y = event.clientY - rect.height - 10;
      }
    }
  }

  function handleClick() {
    if (onclick) {
      onclick();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleClick();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<span
  class="keyword-context-trigger"
  class:clickable={!!onclick}
  role={onclick ? "button" : "note"}
  tabindex={onclick ? 0 : -1}
  onmouseenter={handleMouseEnter}
  onmousemove={handleMouseMove}
  onmouseleave={handleMouseLeave}
  onclick={handleClick}
  onkeydown={handleKeydown}
>
  {@render children()}
</span>

{#if showTooltip}
  <div
    bind:this={tooltipEl}
    class="keyword-tooltip"
    style="left: {x}px; top: {y}px;"
    role="tooltip"
  >
    <div class="tooltip-header">
      <i class="fa-solid fa-quote-left"></i>
      <span class="tooltip-keyword">{keywordName}</span>
    </div>

    {#if loading}
      <div class="tooltip-loading">
        <i class="fa-solid fa-spinner fa-spin"></i>
        {$_("network.loading") || "Loading..."}
      </div>
    {:else if loadError}
      <div class="tooltip-error">
        <i class="fa-solid fa-exclamation-circle"></i>
        {loadError}
      </div>
    {:else if context}
      {#if context.sentence}
        <div class="tooltip-sentence">"{context.sentence}"</div>
      {:else}
        <div class="tooltip-no-sentence">
          <i class="fa-solid fa-quote-left"></i>
          <em>{$_("compound.noSentenceAvailable") || "No sentence context available"}</em>
        </div>
      {/if}
      <div class="tooltip-meta">
        <div class="tooltip-article-title">
          <i class="fa-solid fa-newspaper"></i>
          {context.article_title}
        </div>
        {#if context.article_date}
          <div class="tooltip-date">
            <i class="fa-solid fa-calendar"></i>
            {formatChangedDate(context.article_date)}
          </div>
        {/if}
      </div>
    {:else}
      <div class="tooltip-empty">
        {$_("compound.noContextAvailable") || "No context available"}
      </div>
    {/if}

    {#if onclick}
      <div class="tooltip-hint">
        <i class="fa-solid fa-arrow-up-right-from-square"></i>
        {$_("compound.clickToOpenNetwork") || "Click to open in Network"}
      </div>
    {/if}
  </div>
{/if}

<style>
  .keyword-context-trigger {
    cursor: help;
  }

  .keyword-context-trigger.clickable {
    cursor: pointer;
  }

  .keyword-context-trigger.clickable:hover {
    color: var(--accent-primary);
  }

  .keyword-tooltip {
    position: fixed;
    z-index: 1000;
    max-width: 350px;
    min-width: 200px;
    padding: 0.75rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    pointer-events: none;
  }

  .tooltip-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    color: var(--accent-primary);
  }

  .tooltip-header i {
    font-size: 0.75rem;
    opacity: 0.7;
  }

  .tooltip-keyword {
    font-weight: 600;
    font-size: 0.875rem;
  }

  .tooltip-loading,
  .tooltip-error,
  .tooltip-empty {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .tooltip-error {
    color: var(--status-error);
  }

  .tooltip-no-sentence {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
    margin-bottom: 0.5rem;
    padding: 0.5rem;
    background-color: var(--bg-overlay);
    border-radius: 0.25rem;
    border-left: 3px solid var(--border-muted);
  }

  .tooltip-no-sentence i {
    font-size: 0.75rem;
    opacity: 0.5;
  }

  .tooltip-sentence {
    font-size: 0.8125rem;
    color: var(--text-primary);
    line-height: 1.5;
    font-style: italic;
    margin-bottom: 0.5rem;
    padding: 0.5rem;
    background-color: var(--bg-overlay);
    border-radius: 0.25rem;
    border-left: 3px solid var(--accent-primary);
  }

  .tooltip-meta {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .tooltip-article-title {
    display: flex;
    align-items: flex-start;
    gap: 0.375rem;
    line-height: 1.3;
  }

  .tooltip-article-title i {
    flex-shrink: 0;
    margin-top: 0.125rem;
  }

  .tooltip-date {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .tooltip-meta i {
    font-size: 0.625rem;
    opacity: 0.7;
  }

  .tooltip-hint {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-muted);
    font-size: 0.6875rem;
    color: var(--accent-primary);
    opacity: 0.8;
  }

  .tooltip-hint i {
    font-size: 0.5625rem;
  }
</style>
