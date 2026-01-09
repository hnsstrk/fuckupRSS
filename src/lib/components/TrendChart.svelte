<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import { Chart, registerables } from 'chart.js';

  // Register all Chart.js components
  Chart.register(...registerables);

  interface Props {
    keywordId: number | null;
    keywordName?: string;
    comparisonIds?: number[];
  }

  let { keywordId, keywordName = '', comparisonIds = [] }: Props = $props();

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;
  let days = $state(30);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Track previous values to prevent unnecessary re-renders
  let prevKeywordId: number | null = null;
  let mounted = false;

  async function loadTrendData() {
    if (!keywordId) return;

    loading = true;
    error = null;

    try {
      // Get all keyword IDs to fetch
      const allIds = [keywordId, ...comparisonIds];

      // Fetch comparison data for all keywords
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

    if (data.dates.length === 0) return;

    // Color palette for multiple lines
    const colors = [
      { border: '#cba6f7', bg: 'rgba(203, 166, 247, 0.2)' }, // Mauve
      { border: '#f9e2af', bg: 'rgba(249, 226, 175, 0.2)' }, // Yellow
      { border: '#a6e3a1', bg: 'rgba(166, 227, 161, 0.2)' }, // Green
      { border: '#89b4fa', bg: 'rgba(137, 180, 250, 0.2)' }, // Blue
      { border: '#f5c2e7', bg: 'rgba(245, 194, 231, 0.2)' }, // Pink
    ];

    // Format dates for display
    const labels = data.dates.map(d => {
      const date = new Date(d);
      return date.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit' });
    });

    // Build datasets
    const datasets = data.keywords.map((kw, idx) => ({
      label: kw.name,
      data: kw.counts,
      borderColor: colors[idx % colors.length].border,
      backgroundColor: colors[idx % colors.length].bg,
      borderWidth: 2,
      fill: idx === 0, // Only fill the first (main) keyword
      tension: 0.3,
      pointRadius: 3,
      pointHoverRadius: 5,
    }));

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
            display: datasets.length > 1,
            position: 'top',
            labels: {
              color: '#cdd6f4',
              font: {
                size: 11,
              },
            },
          },
          tooltip: {
            backgroundColor: '#1e1e2e',
            titleColor: '#cdd6f4',
            bodyColor: '#cdd6f4',
            borderColor: '#585b70',
            borderWidth: 1,
            padding: 10,
            callbacks: {
              title: (items) => {
                if (items.length > 0) {
                  const idx = items[0].dataIndex;
                  return data.dates[idx];
                }
                return '';
              },
            },
          },
        },
        scales: {
          x: {
            grid: {
              color: 'rgba(88, 91, 112, 0.3)',
            },
            ticks: {
              color: '#a6adc8',
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
              color: 'rgba(88, 91, 112, 0.3)',
            },
            ticks: {
              color: '#a6adc8',
              precision: 0,
              font: {
                size: 10,
              },
            },
          },
        },
      },
    });
  }

  function handleDaysChange(newDays: number) {
    days = newDays;
    loadTrendData();
  }

  onMount(() => {
    mounted = true;
    if (keywordId) {
      prevKeywordId = keywordId;
      loadTrendData();
    }
  });

  onDestroy(() => {
    mounted = false;
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });

  // Reload when keywordId changes (with stability check)
  $effect(() => {
    // Only run after mount and if keywordId actually changed
    if (mounted && keywordId && keywordId !== prevKeywordId) {
      prevKeywordId = keywordId;
      loadTrendData();
    }
  });
</script>

<div class="trend-chart-container">
  <div class="trend-header">
    <h4 class="trend-title">
      {#if keywordName}
        {$_('network.trendFor') || 'Trend for'}: <span class="keyword-name">{keywordName}</span>
      {:else}
        {$_('network.trendsTab') || 'Trends'}
      {/if}
    </h4>

    <div class="time-range-selector">
      <button
        class:active={days === 7}
        onclick={() => handleDaysChange(7)}
      >
        {$_('network.days7') || '7 days'}
      </button>
      <button
        class:active={days === 30}
        onclick={() => handleDaysChange(30)}
      >
        {$_('network.days30') || '30 days'}
      </button>
      <button
        class:active={days === 90}
        onclick={() => handleDaysChange(90)}
      >
        {$_('network.days90') || '90 days'}
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
  {:else if !keywordId}
    <div class="chart-empty">
      <span>{$_('network.selectKeywordForTrend') || 'Select a keyword to see trends'}</span>
    </div>
  {/if}

  <div class="chart-wrapper" class:hidden={loading || error || !keywordId}>
    <canvas bind:this={canvas}></canvas>
  </div>
</div>

<style>
  .trend-chart-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 300px;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    padding: 1rem;
  }

  .trend-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .trend-title {
    margin: 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .keyword-name {
    color: var(--accent-primary);
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
    min-height: 200px;
  }

  .chart-wrapper.hidden {
    display: none;
  }

  .chart-wrapper canvas {
    width: 100% !important;
    height: 100% !important;
  }

  .chart-loading,
  .chart-error,
  .chart-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 0.875rem;
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
