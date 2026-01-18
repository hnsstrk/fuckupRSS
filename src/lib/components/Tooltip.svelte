<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { settings } from '../stores/settings.svelte';

  interface Props {
    /** For terminology tooltips - shows term + description from i18n */
    termKey?: string;
    /** For generic help tooltips - shows plain content text */
    content?: string;
    children: any;
  }

  let { termKey, content, children }: Props = $props();

  let showTooltip = $state(false);
  let tooltipEl: HTMLElement | null = $state(null);
  let x = $state(0);
  let y = $state(0);

  // termKey tooltips respect the settings toggle, content tooltips always show
  let isTerminology = $derived(!!termKey && !content);

  function handleMouseEnter(event: MouseEvent) {
    // termKey tooltips only show when enabled in settings
    if (isTerminology && !settings.showTerminologyTooltips) return;
    showTooltip = true;
    updatePosition(event);
  }

  function handleMouseMove(event: MouseEvent) {
    if (showTooltip) {
      updatePosition(event);
    }
  }

  function handleMouseLeave() {
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
</script>

<span
  class="tooltip-trigger"
  class:help-tooltip={!!content}
  class:term-tooltip={isTerminology}
  role="note"
  onmouseenter={handleMouseEnter}
  onmousemove={handleMouseMove}
  onmouseleave={handleMouseLeave}
>
  {@render children()}
</span>

{#if showTooltip && (content || (termKey && settings.showTerminologyTooltips))}
  <div
    bind:this={tooltipEl}
    class="tooltip"
    class:help-style={!!content}
    style="left: {x}px; top: {y}px;"
    role="tooltip"
  >
    {#if content}
      <div class="tooltip-content">{content}</div>
    {:else if termKey}
      <div class="tooltip-term">{$_(`terminology.${termKey}.term`)}</div>
      <div class="tooltip-description">{$_(`terminology.${termKey}.description`)}</div>
    {/if}
  </div>
{/if}

<style>
  /* Terminology tooltips have dotted underline */
  .tooltip-trigger.term-tooltip {
    cursor: help;
    border-bottom: 1px dotted currentColor;
  }

  /* Help tooltips (info icons) have no underline */
  .tooltip-trigger.help-tooltip {
    cursor: help;
  }

  .tooltip {
    position: fixed;
    z-index: 1000;
    max-width: 300px;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    pointer-events: none;
  }

  .tooltip-term {
    font-weight: 600;
    color: var(--accent-primary);
    margin-bottom: 0.25rem;
  }

  .tooltip-description {
    font-size: 0.875rem;
    color: var(--text-primary);
    line-height: 1.4;
  }

  /* Content-only tooltips have simpler styling */
  .tooltip-content {
    font-size: 0.8125rem;
    color: var(--text-primary);
    line-height: 1.4;
  }
</style>
