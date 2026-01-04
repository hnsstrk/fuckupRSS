<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { settings } from '../stores/settings.svelte';

  interface Props {
    termKey: string;
    children: any;
  }

  let { termKey, children }: Props = $props();

  let showTooltip = $state(false);
  let tooltipEl: HTMLElement | null = $state(null);
  let x = $state(0);
  let y = $state(0);

  function handleMouseEnter(event: MouseEvent) {
    if (!settings.showTerminologyTooltips) return;
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
  role="note"
  onmouseenter={handleMouseEnter}
  onmousemove={handleMouseMove}
  onmouseleave={handleMouseLeave}
>
  {@render children()}
</span>

{#if showTooltip && settings.showTerminologyTooltips}
  <div
    bind:this={tooltipEl}
    class="tooltip"
    style="left: {x}px; top: {y}px;"
    role="tooltip"
  >
    <div class="tooltip-term">{$_(`terminology.${termKey}.term`)}</div>
    <div class="tooltip-description">{$_(`terminology.${termKey}.description`)}</div>
  </div>
{/if}

<style>
  .tooltip-trigger {
    cursor: help;
    border-bottom: 1px dotted currentColor;
  }

  .tooltip {
    position: fixed;
    z-index: 1000;
    max-width: 300px;
    padding: 8px 12px;
    background: #1a1a2e;
    border: 1px solid #4a4a6a;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    pointer-events: none;
  }

  .tooltip-term {
    font-weight: 600;
    color: #ffd700;
    margin-bottom: 4px;
  }

  .tooltip-description {
    font-size: 0.875rem;
    color: #e0e0e0;
    line-height: 1.4;
  }

  :global(.light) .tooltip {
    background: #ffffff;
    border-color: #d0d0d0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  :global(.light) .tooltip-term {
    color: #b8860b;
  }

  :global(.light) .tooltip-description {
    color: #333333;
  }
</style>
