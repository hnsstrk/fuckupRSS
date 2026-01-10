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

  // Color palette: main keyword first (Mauve), then 7 co-occurring keywords
  const colors = [
    { border: '#cba6f7', bg: 'rgba(203, 166, 247, 0.3)' }, // Mauve - Main keyword
    { border: '#f9e2af', bg: 'rgba(249, 226, 175, 0.15)' }, // Yellow
    { border: '#a6e3a1', bg: 'rgba(166, 227, 161, 0.15)' }, // Green
    { border: '#89b4fa', bg: 'rgba(137, 180, 250, 0.15)' }, // Blue
    { border: '#f5c2e7', bg: 'rgba(245, 194, 231, 0.15)' }, // Pink
    { border: '#94e2d5', bg: 'rgba(148, 226, 213, 0.15)' }, // Teal
    { border: '#fab387', bg: 'rgba(250, 179, 135, 0.15)' }, // Peach
    { border: '#89dceb', bg: 'rgba(137, 220, 235, 0.15)' }, // Sky
  ];

  async function loadTrendData() {
    if (!keywordId) return;

    loading = true;
    error = null;

    try {
      // Build list of all IDs (main keyword + up to 7 co-occurring)
      const allIds = [keywordId, ...neighborIds.slice(0, 7)];

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
              color: '#cdd6f4',
              font: {
                size: 11,
              },
              usePointStyle: true,
              pointStyle: 'line',
              padding: 15,
            },
          },
          tooltip: {
            backgroundColor: '#1e1e2e',
            titleColor: '#cdd6f4',
            bodyColor: '#cdd6f4',
            borderColor: '#585b70',
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
            title: {
              display: true,
              text: $_('network.articleCount') || 'Artikel',
              color: '#a6adc8',
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
  });

  onDestroy(() => {
    mounted = false;
    if (chart) {
      chart.destroy();
      chart = null;
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
    min-height: 280px;
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
