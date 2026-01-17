<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import { Chart, registerables } from 'chart.js';

  // Register all Chart.js components
  Chart.register(...registerables);

  interface Props {
    keywordId: number;
    keywordName: string;
    neighborIds?: number[];
    ondayschange?: (days: number) => void;
  }

  let { keywordId, keywordName, neighborIds = [], ondayschange }: Props = $props();

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;
  let days = $state(30);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Track previous values to prevent unnecessary re-renders
  let prevKeywordId: number | null = null;
  let prevNeighborIdsKey = '';
  let mounted = false;
  let themeObserver: MutationObserver | null = null;

  // Chart color type
  interface ChartColor {
    border: string;
    bg: string;
  }

  // Get theme-aware colors from CSS variables (no hardcoded fallbacks)
  function getThemeColors(): ChartColor[] {
    const style = getComputedStyle(document.documentElement);

    // Main keyword uses accent-primary
    const accentPrimary = style.getPropertyValue('--accent-primary').trim();

    // Category colors for the 4 co-occurring keywords (indices 1-4)
    // Generate bg color dynamically from border color
    const categoryColors: ChartColor[] = [];
    for (let i = 1; i <= 4; i++) {
      const border = style.getPropertyValue(`--category-${i}`).trim();
      if (border) {
        categoryColors.push({ border, bg: hexToRgba(border, 0.15) });
      }
    }

    return [
      { border: accentPrimary, bg: hexToRgba(accentPrimary, 0.3) },
      ...categoryColors
    ];
  }

  // Helper to convert hex to rgba (no hardcoded fallback)
  function hexToRgba(hex: string, alpha: number): string {
    if (!hex) return `rgba(128, 128, 128, ${alpha})`; // Neutral gray if no color
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    if (result) {
      const r = parseInt(result[1], 16);
      const g = parseInt(result[2], 16);
      const b = parseInt(result[3], 16);
      return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    }
    return `rgba(128, 128, 128, ${alpha})`; // Neutral gray for invalid hex
  }

  // Get theme-aware chart styling (no hardcoded fallbacks)
  function getChartThemeStyles() {
    const style = getComputedStyle(document.documentElement);
    return {
      textColor: style.getPropertyValue('--text-muted').trim(),
      gridColor: style.getPropertyValue('--border-default').trim(),
      tooltipBg: style.getPropertyValue('--bg-surface').trim(),
      tooltipText: style.getPropertyValue('--text-primary').trim(),
      tooltipBorder: style.getPropertyValue('--border-default').trim(),
    };
  }

  async function loadTrendData() {
    if (!keywordId) return;

    loading = true;
    error = null;

    try {
      // Build list of all IDs (main keyword + up to 4 co-occurring = 5 total)
      const allIds = [keywordId, ...neighborIds.slice(0, 4)];

      const response = await invoke<{
        keywords: { id: number; name: string; counts: number[] }[];
        dates: string[];
      }>('get_trending_comparison', {
        ids: allIds,
        days,
      });

      updateChart(response);
    } catch (e) {
      error = String(e);
      console.error('Failed to load trend data:', e);
    } finally {
      loading = false;
    }
  }

  function updateChart(data: {
    keywords: { id: number; name: string; counts: number[] }[];
    dates: string[];
  }) {
    if (!canvas) return;

    // Destroy existing chart
    if (chart) {
      chart.destroy();
      chart = null;
    }

    if (data.dates.length === 0 || data.keywords.length === 0) return;

    // Get theme-aware colors and styles
    const colors = getThemeColors();
    const themeStyles = getChartThemeStyles();

    // Format dates for display
    const labels = data.dates.map(d => {
      const date = new Date(d);
      return date.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit' });
    });

    // Build datasets - main keyword gets filled area, neighbors get lines only
    const datasets = data.keywords.map((kw, idx) => {
      const isMain = idx === 0;
      const color = colors[idx % colors.length];

      return {
        label: kw.name,
        data: kw.counts,
        borderColor: color.border,
        backgroundColor: color.bg,
        borderWidth: isMain ? 3 : 2,
        fill: isMain, // Only fill the main keyword
        tension: 0.3,
        pointRadius: isMain ? 4 : 2,
        pointHoverRadius: 6,
        borderDash: isMain ? [] : [5, 5], // Dashed lines for neighbors
      };
    });

    chart = new Chart(canvas, {
      type: 'line',
      data: {
        labels,
        datasets,
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: {
          mode: 'index',
          intersect: false,
        },
        plugins: {
          legend: {
            display: true,
            position: 'top',
            align: 'start',
            labels: {
              color: themeStyles.textColor,
              font: {
                size: 11,
              },
              usePointStyle: true,
              pointStyle: 'line',
              padding: 15,
            },
          },
          tooltip: {
            backgroundColor: themeStyles.tooltipBg,
            titleColor: themeStyles.tooltipText,
            bodyColor: themeStyles.tooltipText,
            borderColor: themeStyles.tooltipBorder,
            borderWidth: 1,
            padding: 12,
            callbacks: {
              title: (items) => {
                if (items.length > 0) {
                  const idx = items[0].dataIndex;
                  return data.dates[idx];
                }
                return '';
              },
              label: (item) => {
                return `${item.dataset.label}: ${item.parsed.y} Artikel`;
              },
            },
          },
        },
        scales: {
          x: {
            grid: {
              color: hexToRgba(themeStyles.gridColor, 0.3),
            },
            ticks: {
              color: themeStyles.textColor,
              maxRotation: 45,
              minRotation: 45,
              font: {
                size: 10,
              },
            },
          },
          y: {
            beginAtZero: true,
            grid: {
              color: hexToRgba(themeStyles.gridColor, 0.3),
            },
            ticks: {
              color: themeStyles.textColor,
              precision: 0,
              font: {
                size: 10,
              },
            },
            title: {
              display: true,
              text: $_('network.articleCount') || 'Artikel',
              color: themeStyles.textColor,
              font: {
                size: 11,
              },
            },
          },
        },
      },
    });
  }

  function handleDaysChange(newDays: number) {
    days = newDays;
    ondayschange?.(newDays);
    loadTrendData();
  }

  onMount(() => {
    mounted = true;
    // Initialize tracking values
    prevKeywordId = keywordId;
    prevNeighborIdsKey = neighborIds.join(',');
    ondayschange?.(days);
    loadTrendData();

    // Watch for theme changes on html element
    themeObserver = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
          // Theme changed, reload chart with new colors
          if (chart && canvas) {
            loadTrendData();
          }
        }
      }
    });

    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class']
    });
  });

  onDestroy(() => {
    mounted = false;
    if (chart) {
      chart.destroy();
      chart = null;
    }
    if (themeObserver) {
      themeObserver.disconnect();
      themeObserver = null;
    }
  });

  // Reload when keywordId or neighborIds change (with stability check)
  $effect(() => {
    // Only run after mount
    if (!mounted) return;

    // Create a stable key for neighbor comparison
    const currentNeighborIdsKey = neighborIds.join(',');

    // Only reload if something actually changed
    if (keywordId && canvas) {
      if (keywordId !== prevKeywordId || currentNeighborIdsKey !== prevNeighborIdsKey) {
        prevKeywordId = keywordId;
        prevNeighborIdsKey = currentNeighborIdsKey;
        loadTrendData();
      }
    }
  });
</script>

<div class="trend-chart-container">
  <div class="trend-header">
    <div class="time-range-selector">
      <button
        class:active={days === 7}
        onclick={() => handleDaysChange(7)}
      >
        7 {$_('network.days') || 'Tage'}
      </button>
      <button
        class:active={days === 30}
        onclick={() => handleDaysChange(30)}
      >
        30 {$_('network.days') || 'Tage'}
      </button>
      <button
        class:active={days === 90}
        onclick={() => handleDaysChange(90)}
      >
        90 {$_('network.days') || 'Tage'}
      </button>
    </div>
  </div>

  {#if loading}
    <div class="chart-loading">
      <div class="spinner"></div>
    </div>
  {:else if error}
    <div class="chart-error">
      <span>{error}</span>
    </div>
  {/if}

  <div class="chart-wrapper" class:hidden={loading || !!error}>
    <canvas bind:this={canvas}></canvas>
  </div>
</div>

<style>
  .trend-chart-container {
    display: flex;
    flex-direction: column;
    height: 320px;
    min-height: 320px;
    max-height: 320px;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    padding: 1rem;
  }

  .trend-header {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 0.75rem;
  }

  .time-range-selector {
    display: flex;
    gap: 0.25rem;
  }

  .time-range-selector button {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    background: none;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .time-range-selector button:hover {
    border-color: var(--accent-primary);
    color: var(--text-primary);
  }

  .time-range-selector button.active {
    background-color: var(--accent-primary);
    border-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .chart-wrapper {
    flex: 1;
    position: relative;
    height: 240px;
    min-height: 240px;
  }

  .chart-wrapper.hidden {
    display: none;
  }

  .chart-wrapper canvas {
    width: 100% !important;
    height: 100% !important;
  }

  .chart-loading,
  .chart-error {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 0.875rem;
    min-height: 200px;
  }

  .chart-error {
    color: var(--accent-error);
  }

  .spinner {
    width: 1.5rem;
    height: 1.5rem;
    border: 2px solid var(--border-default);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
