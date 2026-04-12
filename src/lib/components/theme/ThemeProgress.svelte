<script lang="ts">
  import { _ } from "svelte-i18n";

  interface ThemeProgressData {
    report_id: number;
    themes_complete: number;
    themes_total: number;
    current_theme: string;
  }

  let { progress }: { progress: ThemeProgressData | null } = $props();

  let percentage = $derived(
    progress && progress.themes_total > 0
      ? Math.round((progress.themes_complete / progress.themes_total) * 100)
      : 0,
  );
</script>

{#if progress}
  <div class="tr-progress">
    <div class="tr-progress-header">
      <i class="fa-solid fa-spinner fa-spin tr-progress-icon"></i>
      <span class="tr-progress-label">
        {$_("themeReport.progress", {
          values: { current: progress.themes_complete, total: progress.themes_total },
        })}
      </span>
      <span class="tr-progress-pct">{percentage}%</span>
    </div>
    <div class="tr-progress-bar-track">
      <div class="tr-progress-bar-fill" style:width="{percentage}%"></div>
    </div>
    {#if progress.current_theme}
      <div class="tr-progress-current">
        <i class="fa-solid fa-circle tr-pulse"></i>
        {progress.current_theme}
      </div>
    {/if}
  </div>
{/if}

<style>
  .tr-progress {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1.25rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    margin: 1rem;
  }

  .tr-progress-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .tr-progress-icon {
    color: var(--accent-primary);
    font-size: 0.9rem;
  }

  .tr-progress-label {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
  }

  .tr-progress-pct {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--accent-primary);
    font-variant-numeric: tabular-nums;
  }

  .tr-progress-bar-track {
    height: 6px;
    background: var(--bg-tertiary, var(--bg-primary));
    border-radius: 3px;
    overflow: hidden;
  }

  .tr-progress-bar-fill {
    height: 100%;
    background: var(--accent-primary);
    border-radius: 3px;
    transition: width 0.4s ease;
  }

  .tr-progress-current {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
    font-style: italic;
  }

  .tr-pulse {
    font-size: 0.4rem;
    color: var(--accent-primary);
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.3;
    }
  }
</style>
