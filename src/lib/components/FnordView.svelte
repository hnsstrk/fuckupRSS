<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import { appState, type Fnord, type FnordStats, type CategoryRevisionStats } from '../stores/state.svelte';

  // Get CSS variable for category color (theme-aware)
  function getCategoryColorVar(id: number | undefined): string {
    if (id && id >= 1 && id <= 6) {
      return `var(--category-${id})`;
    }
    return 'var(--accent-primary)';
  }
  import type {
    ArticleTimeline,
    GreyfaceIndex,
    KeywordStats,
    FeedActivity,
    BiasHeatmapEntry,
    KeywordCloudEntry
  } from '../types';
  import Tooltip from './Tooltip.svelte';
  import Tabs, { type Tab } from './Tabs.svelte';
  import { ArticleItemCompact } from './article';

  // State
  let stats = $state<FnordStats | null>(null);
  let changedFnords = $state<Fnord[]>([]);
  let loading = $state(true);
  let selectedFnordId = $state<number | null>(null);

  // Extended statistics state
  let timeline = $state<ArticleTimeline | null>(null);
  let greyfaceIndex = $state<GreyfaceIndex | null>(null);
  let topKeywords = $state<KeywordStats[]>([]);
  let feedActivity = $state<FeedActivity[]>([]);
  let biasHeatmap = $state<BiasHeatmapEntry[]>([]);
  let keywordCloud = $state<KeywordCloudEntry[]>([]);

  // Loading/error states for extended stats
  let extendedStatsLoading = $state(false);
  let extendedStatsError = $state<string | null>(null);

  // Period selector
  let selectedPeriod = $state<7 | 30 | 90>(7);

  // Tab state
  let activeTab = $state<string>('stats');

  // Expanded category state
  let expandedCategoryId = $state<number | null>(null);
  let subcategories = $state<CategoryRevisionStats[]>([]);
  let loadingSubcategories = $state(false);

  // Easter egg state
  let show23EasterEgg = $state(false);

  // Tabs definition
  let tabs = $derived<Tab[]>([
    { id: 'stats', label: $_('fnordView.statsTab') || 'Statistiken' },
    { id: 'articles', label: $_('fnordView.articlesTab') || 'Geänderte Artikel', badge: changedFnords.length || undefined }
  ]);

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    try {
      // Load basic stats
      const statsData = await appState.getFnordStats();
      stats = statsData;

      await appState.loadChangedFnords();
      changedFnords = appState.changedFnords;

      // Load extended statistics
      await loadExtendedStats();
    } catch (e) {
      console.error('[FnordView] Error loading data:', e);
    } finally {
      loading = false;
    }
  }

  async function loadExtendedStats() {
    extendedStatsLoading = true;
    extendedStatsError = null;
    try {
      const [timelineData, greyfaceData, keywordsData, feedData, heatmapData, cloudData] = await Promise.all([
        invoke<ArticleTimeline>('get_article_timeline', { days: selectedPeriod }),
        invoke<GreyfaceIndex>('get_greyface_index'),
        invoke<KeywordStats[]>('get_top_keywords_stats', { days: selectedPeriod, limit: 5 }),
        invoke<FeedActivity[]>('get_feed_activity', { days: selectedPeriod, limit: 5 }),
        invoke<BiasHeatmapEntry[]>('get_bias_heatmap'),
        invoke<KeywordCloudEntry[]>('get_keyword_cloud', { days: selectedPeriod, limit: 50 })
      ]);

      timeline = timelineData;
      greyfaceIndex = greyfaceData;
      topKeywords = keywordsData;
      feedActivity = feedData;
      biasHeatmap = heatmapData;
      keywordCloud = cloudData;

      // Easter egg: Check if selectedPeriod is 23 (user must manually enter 23 via browser console)
      if (selectedPeriod === 23 as any) {
        show23EasterEgg = true;
      }
    } catch (e) {
      console.error('[FnordView] Error loading extended stats:', e);
      extendedStatsError = e instanceof Error ? e.message : String(e);
    } finally {
      extendedStatsLoading = false;
    }
  }

  async function changePeriod(days: 7 | 30 | 90) {
    selectedPeriod = days;
    await loadExtendedStats();
  }

  function selectFnord(id: number) {
    selectedFnordId = id;
    appState.selectFnord(id);
    window.dispatchEvent(new CustomEvent('navigate-to-article', { detail: { articleId: id } }));
  }

  async function toggleCategory(categoryId: number) {
    if (expandedCategoryId === categoryId) {
      expandedCategoryId = null;
      subcategories = [];
    } else {
      expandedCategoryId = categoryId;
      loadingSubcategories = true;
      try {
        subcategories = await invoke<CategoryRevisionStats[]>('get_subcategory_stats', {
          mainCategoryId: categoryId,
        });
      } catch (e) {
        console.error('Failed to load subcategories:', e);
        subcategories = [];
      } finally {
        loadingSubcategories = false;
      }
    }
  }

  function getTrendIcon(trend: number): string {
    if (trend > 10) return 'fa-solid fa-arrow-trend-up';
    if (trend < -10) return 'fa-solid fa-arrow-trend-down';
    if (trend === 100) return 'fa-solid fa-sparkles';
    return 'fa-solid fa-minus';
  }

  function getTrendClass(trend: number): string {
    if (trend > 10) return 'trend-up';
    if (trend < -10) return 'trend-down';
    if (trend === 100) return 'trend-new';
    return 'trend-stable';
  }

  function getGreyfaceLevel(index: number): string {
    if (index < 20) return 'excellent';
    if (index < 40) return 'good';
    if (index < 60) return 'moderate';
    if (index < 80) return 'concerning';
    return 'critical';
  }

  function getBiasColor(bias: number): string {
    if (bias <= -1.5) return 'var(--category-2)';
    if (bias <= -0.5) return 'color-mix(in srgb, var(--category-2) 50%, var(--text-muted))';
    if (bias < 0.5) return 'var(--text-muted)';
    if (bias < 1.5) return 'color-mix(in srgb, var(--category-5) 50%, var(--text-muted))';
    return 'var(--category-5)';
  }

  // Keyword type color mapping
  function getKeywordTypeColor(type: string | null): string {
    switch (type) {
      case 'person': return 'var(--category-2)';
      case 'organization': return 'var(--category-3)';
      case 'location': return 'var(--category-4)';
      case 'acronym': return 'var(--category-5)';
      default: return 'var(--category-1)';
    }
  }

  // Check if timeline has any meaningful data (not all zeros)
  function hasTimelineData(data: ArticleTimeline | null): boolean {
    if (!data || data.data.length === 0) return false;
    return data.data.some(d => d.articles > 0 || d.revisions > 0);
  }

  // Check if we have any trend data at all
  let hasTrendData = $derived(
    hasTimelineData(timeline) ||
    topKeywords.length > 0 ||
    feedActivity.length > 0 ||
    keywordCloud.length > 0
  );
</script>

<div class="fnord-view">
  <!-- Header -->
  <div class="fnord-header">
    <div class="header-top">
      <h2 class="fnord-title">
        <Tooltip termKey="fnord">{$_('fnordView.title') || 'Fnord-Statistiken'}</Tooltip>
      </h2>
      {#if stats}
        <div class="fnord-summary">
          <span class="summary-item">
            <span class="summary-value">{stats.total_revisions}</span>
            <span class="summary-label">{$_('fnordView.totalRevisions') || 'Revisionen'}</span>
          </span>
          <span class="summary-item">
            <span class="summary-value">{stats.articles_with_changes}</span>
            <span class="summary-label">{$_('fnordView.articlesWithChanges') || 'Geänderte Artikel'}</span>
          </span>
        </div>
      {/if}
    </div>

    <!-- Tabs -->
    <Tabs {tabs} bind:activeTab />
  </div>

  <!-- Content -->
  <div class="fnord-content">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <span>{$_('fnordView.loading') || 'Laden...'}</span>
      </div>
    {:else if activeTab === 'stats' && stats}
      <div class="stats-view">
        <!-- GESAMT-ÜBERSICHT (ohne Zeitfilter) -->
        <div class="stats-section-header">
          <h3 class="section-header-title">
            <i class="fa-solid fa-chart-pie"></i>
            {$_('fnordView.overallStats') || 'Gesamt-Übersicht'}
          </h3>
        </div>

        <!-- Top Row: Greyface Index + By Source -->
        <div class="stats-row top-row">
          <!-- Greyface Index Card -->
          {#if greyfaceIndex && greyfaceIndex.articles_with_bias > 0}
            <div class="stats-card greyface-card">
              <h3 class="card-title">
                <i class="fa-solid fa-triangle-exclamation"></i>
                <Tooltip termKey="greyface">{$_('fnordView.greyface.title') || 'Greyface-Index'}</Tooltip>
              </h3>
              <div class="greyface-content">
                <div class="greyface-gauge">
                  <div
                    class="gauge-fill {getGreyfaceLevel(greyfaceIndex.index)}"
                    style="--gauge-value: {greyfaceIndex.index}%"
                  ></div>
                  <span class="gauge-value">{greyfaceIndex.index.toFixed(0)}</span>
                </div>
                <div class="greyface-details">
                  <div class="detail-row">
                    <span class="detail-label">{$_('fnordView.greyface.avgBias') || 'Durchschn. Tendenz'}</span>
                    <span class="detail-value" style="color: {getBiasColor(greyfaceIndex.avg_political_bias)}">
                      {greyfaceIndex.avg_political_bias.toFixed(2)}
                    </span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">{$_('fnordView.greyface.avgSachlichkeit') || 'Durchschn. Sachlichkeit'}</span>
                    <span class="detail-value">{greyfaceIndex.avg_sachlichkeit.toFixed(2)}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">{$_('fnordView.greyface.articlesWithBias') || 'Mit Bias-Daten'}</span>
                    <span class="detail-value">{greyfaceIndex.articles_with_bias} / {greyfaceIndex.total_articles}</span>
                  </div>
                </div>
                <!-- Bias Distribution Bar -->
                <div class="bias-distribution">
                  <div class="distribution-label">{$_('fnordView.greyface.distribution') || 'Bias-Verteilung'}</div>
                  <div class="distribution-bar">
                    {#if greyfaceIndex.articles_with_bias > 0}
                      {@const total = greyfaceIndex.articles_with_bias}
                      <div
                        class="dist-segment left-extreme"
                        style="width: {(greyfaceIndex.bias_distribution.left_extreme / total) * 100}%"
                        title="{$_('fnordView.greyface.leftExtreme')}: {greyfaceIndex.bias_distribution.left_extreme}"
                      ></div>
                      <div
                        class="dist-segment left-leaning"
                        style="width: {(greyfaceIndex.bias_distribution.left_leaning / total) * 100}%"
                        title="{$_('fnordView.greyface.leftLeaning')}: {greyfaceIndex.bias_distribution.left_leaning}"
                      ></div>
                      <div
                        class="dist-segment neutral"
                        style="width: {(greyfaceIndex.bias_distribution.neutral / total) * 100}%"
                        title="{$_('fnordView.greyface.neutral')}: {greyfaceIndex.bias_distribution.neutral}"
                      ></div>
                      <div
                        class="dist-segment right-leaning"
                        style="width: {(greyfaceIndex.bias_distribution.right_leaning / total) * 100}%"
                        title="{$_('fnordView.greyface.rightLeaning')}: {greyfaceIndex.bias_distribution.right_leaning}"
                      ></div>
                      <div
                        class="dist-segment right-extreme"
                        style="width: {(greyfaceIndex.bias_distribution.right_extreme / total) * 100}%"
                        title="{$_('fnordView.greyface.rightExtreme')}: {greyfaceIndex.bias_distribution.right_extreme}"
                      ></div>
                    {/if}
                  </div>
                  <div class="distribution-legend">
                    <span class="legend-item"><span class="legend-dot left-extreme"></span> -2</span>
                    <span class="legend-item"><span class="legend-dot left-leaning"></span> -1</span>
                    <span class="legend-item"><span class="legend-dot neutral"></span> 0</span>
                    <span class="legend-item"><span class="legend-dot right-leaning"></span> +1</span>
                    <span class="legend-item"><span class="legend-dot right-extreme"></span> +2</span>
                  </div>
                </div>
              </div>
            </div>
          {:else}
            <!-- Greyface Placeholder when no data -->
            <div class="stats-card greyface-card greyface-empty">
              <h3 class="card-title">
                <i class="fa-solid fa-triangle-exclamation"></i>
                <Tooltip termKey="greyface">{$_('fnordView.greyface.title') || 'Greyface-Index'}</Tooltip>
              </h3>
              <div class="empty-placeholder">
                <i class="fa-solid fa-chart-pie empty-icon"></i>
                <p>{$_('fnordView.greyface.noData') || 'Keine Bias-Daten vorhanden'}</p>
                <span class="empty-hint">{$_('fnordView.greyface.noDataHint') || 'Führe die KI-Analyse durch, um Bias-Werte zu erhalten.'}</span>
              </div>
            </div>
          {/if}

          <!-- By Source Card (Restored) -->
          {#if stats.by_source.length > 0}
            {@const maxSourceRevisions = Math.max(...stats.by_source.map(s => s.revision_count), 1)}
            <div class="stats-card source-card">
              <h3 class="card-title">
                <i class="fa-solid fa-rss"></i>
                <Tooltip termKey="pentacle">{$_('fnordView.bySource') || 'Nach Quelle'}</Tooltip>
              </h3>
              <div class="source-list">
                {#each stats.by_source.slice(0, 6) as source (source.pentacle_id)}
                  {@const barWidth = (source.revision_count / maxSourceRevisions) * 100}
                  <div class="source-item">
                    <span class="source-name" title={source.title}>{source.title || `Feed #${source.pentacle_id}`}</span>
                    <div class="source-bar-wrapper">
                      <div class="source-progress">
                        <div class="source-progress-fill" style="width: {barWidth}%"></div>
                      </div>
                    </div>
                    <span class="source-stats">
                      <span class="source-count">{source.revision_count}</span>
                      <span class="source-articles">({source.article_count})</span>
                    </span>
                  </div>
                {/each}
                {#if stats.by_source.length > 6}
                  <div class="source-more">
                    +{stats.by_source.length - 6} {$_('fnordView.moreSources') || 'weitere'}
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>

        <!-- Bias Heatmap (Gesamt) -->
        {#if biasHeatmap.length > 0}
          <div class="stats-card full-width">
            <h3 class="card-title">
              <i class="fa-solid fa-table-cells"></i>
              {$_('fnordView.biasHeatmap') || 'Bias-Heatmap'}
            </h3>
            <div class="heatmap-container">
              <table class="heatmap-table">
                <thead>
                  <tr>
                    <th>{$_('fnordView.source') || 'Quelle'}</th>
                    <th class="bias-col">-2</th>
                    <th class="bias-col">-1</th>
                    <th class="bias-col">0</th>
                    <th class="bias-col">+1</th>
                    <th class="bias-col">+2</th>
                    <th>{$_('fnordView.avgBias') || 'Avg'}</th>
                  </tr>
                </thead>
                <tbody>
                  {#each biasHeatmap.slice(0, 10) as entry (entry.pentacle_id)}
                    {@const maxCell = Math.max(entry.bias_minus_2, entry.bias_minus_1, entry.bias_0, entry.bias_plus_1, entry.bias_plus_2, 1)}
                    <tr>
                      <td class="source-cell">{entry.pentacle_title || `Feed #${entry.pentacle_id}`}</td>
                      <td class="heatmap-cell" style="--intensity: {entry.bias_minus_2 / maxCell}">
                        {entry.bias_minus_2 || ''}
                      </td>
                      <td class="heatmap-cell" style="--intensity: {entry.bias_minus_1 / maxCell}">
                        {entry.bias_minus_1 || ''}
                      </td>
                      <td class="heatmap-cell neutral-cell" style="--intensity: {entry.bias_0 / maxCell}">
                        {entry.bias_0 || ''}
                      </td>
                      <td class="heatmap-cell" style="--intensity: {entry.bias_plus_1 / maxCell}">
                        {entry.bias_plus_1 || ''}
                      </td>
                      <td class="heatmap-cell" style="--intensity: {entry.bias_plus_2 / maxCell}">
                        {entry.bias_plus_2 || ''}
                      </td>
                      <td class="avg-cell" style="color: {getBiasColor(entry.avg_bias)}">
                        {entry.avg_bias.toFixed(2)}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/if}

        <!-- By Category (Gesamt) -->
        {#if stats.by_category.length > 0}
          {@const maxRevisions = Math.max(...stats.by_category.map(c => c.revision_count), 1)}
          <div class="stats-section">
            <h3 class="section-title">
              <Tooltip termKey="sephiroth">{$_('fnordView.byCategory') || 'Nach Kategorie'}</Tooltip>
            </h3>
            <div class="category-cards">
              {#each stats.by_category as cat (cat.sephiroth_id)}
                {@const barWidth = (cat.revision_count / maxRevisions) * 100}
                {@const isExpanded = expandedCategoryId === cat.sephiroth_id}
                <button
                  class="category-card {isExpanded ? 'expanded' : ''}"
                  style="--cat-color: {getCategoryColorVar(cat.sephiroth_id)}"
                  data-category-id={cat.sephiroth_id}
                  onclick={() => toggleCategory(cat.sephiroth_id)}
                >
                  <div class="card-header">
                    <div class="card-icon-wrapper">
                      {#if cat.icon}
                        <i class="{cat.icon}"></i>
                      {:else}
                        <i class="fa-solid fa-folder"></i>
                      {/if}
                    </div>
                    <span class="card-title-text">{cat.name}</span>
                    <i class="fa-solid fa-chevron-down expand-icon {isExpanded ? 'rotated' : ''}"></i>
                  </div>
                  <div class="card-stats">
                    <div class="stat-row">
                      <span class="stat-label">{$_('fnordView.revisions') || 'Revisionen'}</span>
                      <span class="stat-value">{cat.revision_count}</span>
                    </div>
                    <div class="progress-bar">
                      <div class="progress-fill" style="width: {barWidth}%"></div>
                    </div>
                    <div class="stat-row secondary">
                      <span class="stat-label">{$_('fnordView.articles') || 'Artikel'}</span>
                      <span class="stat-value">{cat.article_count}</span>
                    </div>
                  </div>

                  {#if isExpanded}
                    <div class="subcategories">
                      {#if loadingSubcategories}
                        <div class="subcategory-loading">
                          <div class="spinner small"></div>
                        </div>
                      {:else if subcategories.length > 0}
                        {#each subcategories as sub (sub.sephiroth_id)}
                          <div class="subcategory-item">
                            <div class="subcategory-info">
                              {#if sub.icon}
                                <i class="{sub.icon} subcategory-icon"></i>
                              {/if}
                              <span class="subcategory-name">{sub.name}</span>
                            </div>
                            <div class="subcategory-stats">
                              <span class="subcategory-count">{sub.revision_count}</span>
                              <span class="subcategory-divider">/</span>
                              <span class="subcategory-count">{sub.article_count}</span>
                            </div>
                          </div>
                        {/each}
                      {:else}
                        <div class="subcategory-empty">
                          {$_('fnordView.noSubcategories') || 'Keine Unterkategorien'}
                        </div>
                      {/if}
                    </div>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <!-- TRENDS & AKTIVITÄT (mit Zeitfilter) -->
        <div class="stats-section-header trends-header">
          <h3 class="section-header-title">
            <i class="fa-solid fa-chart-line"></i>
            {$_('fnordView.trendsAndActivity') || 'Trends & Aktivität'}
          </h3>
          <div class="period-selector">
            <span class="period-label">{$_('fnordView.period') || 'Zeitraum'}:</span>
            <div class="period-buttons">
              <button
                class="period-btn"
                class:active={selectedPeriod === 7}
                onclick={() => changePeriod(7)}
                disabled={extendedStatsLoading}
              >
                {$_('fnordView.days7') || '7 Tage'}
              </button>
              <button
                class="period-btn"
                class:active={selectedPeriod === 30}
                onclick={() => changePeriod(30)}
                disabled={extendedStatsLoading}
              >
                {$_('fnordView.days30') || '30 Tage'}
              </button>
              <button
                class="period-btn"
                class:active={selectedPeriod === 90}
                onclick={() => changePeriod(90)}
                disabled={extendedStatsLoading}
              >
                {$_('fnordView.days90') || '90 Tage'}
              </button>
            </div>
          </div>
        </div>

        <!-- Loading state for trends section -->
        {#if extendedStatsLoading}
          <div class="trends-loading-state">
            <div class="spinner"></div>
            <span>{$_('fnordView.loadingTrends') || 'Trend-Daten werden geladen...'}</span>
          </div>
        {:else if extendedStatsError}
          <!-- Error state for trends section -->
          <div class="trends-error-state">
            <i class="fa-solid fa-exclamation-triangle error-icon"></i>
            <p>{$_('fnordView.trendsError') || 'Fehler beim Laden der Trend-Daten'}</p>
            <span class="error-message">{extendedStatsError}</span>
            <button class="retry-btn" onclick={() => loadExtendedStats()}>
              <i class="fa-solid fa-refresh"></i>
              {$_('fnordView.retry') || 'Erneut versuchen'}
            </button>
          </div>
        {:else if !hasTrendData}
          <!-- Empty state when no trend data available -->
          <div class="trends-empty-state">
            <i class="fa-solid fa-chart-line-down empty-icon"></i>
            <p>{$_('fnordView.noTrendData') || 'Keine Trend-Daten vorhanden'}</p>
            <span class="empty-hint">{$_('fnordView.noTrendDataHint') || 'Trend-Daten werden verfügbar, sobald Artikel synchronisiert und analysiert wurden.'}</span>
          </div>
        {:else}
        <!-- Timeline Card -->
        {#if timeline && hasTimelineData(timeline)}
          {@const timelineData = timeline.data}
          {@const maxArticles = Math.max(...timelineData.map(d => d.articles), 1)}
          {@const maxRevisions = Math.max(...timelineData.map(d => d.revisions), 1)}
          {@const maxVal = Math.max(maxArticles, maxRevisions)}
          <div class="stats-card full-width timeline-card">
            <h3 class="card-title">
              <i class="fa-solid fa-chart-area"></i>
              {$_('fnordView.timeline') || 'Eris-Chronik'}
            </h3>
            <div class="timeline-chart">
              <div class="chart-bars">
                {#each timelineData as day, i (day.date)}
                  <div class="chart-day" title="{day.date}">
                    <div class="bar-container">
                      <div
                        class="bar articles-bar"
                        style="height: {(day.articles / maxVal) * 100}%"
                        title="{$_('fnordView.articles')}: {day.articles}"
                      ></div>
                      <div
                        class="bar revisions-bar"
                        style="height: {(day.revisions / maxVal) * 100}%"
                        title="{$_('fnordView.revisions')}: {day.revisions}"
                      ></div>
                    </div>
                    {#if i === 0 || i === timelineData.length - 1 || i === Math.floor(timelineData.length / 2)}
                      <span class="day-label">{day.date.slice(5)}</span>
                    {/if}
                  </div>
                {/each}
              </div>
              <div class="chart-legend">
                <span class="legend-item"><span class="legend-bar articles"></span> {$_('fnordView.articles')}</span>
                <span class="legend-item"><span class="legend-bar revisions"></span> {$_('fnordView.revisions')}</span>
              </div>
            </div>
          </div>
        {/if}

        <!-- Keywords + Feed Activity Row -->
        <div class="stats-row">
          <!-- Top Keywords (im Zeitraum) -->
          {#if topKeywords.length > 0}
            <div class="stats-card">
              <h3 class="card-title">
                <i class="fa-solid fa-hashtag"></i>
                {$_('fnordView.topKeywords') || 'Top Keywords'}
              </h3>
              <div class="keyword-list">
                {#each topKeywords as kw, i (kw.id)}
                  <div class="keyword-item">
                    <span class="keyword-rank">#{i + 1}</span>
                    <span
                      class="keyword-name"
                      style="border-left: 3px solid {getKeywordTypeColor(kw.keyword_type)}; padding-left: 0.5rem;"
                    >
                      {kw.name}
                    </span>
                    <span class="keyword-count">{kw.count}</span>
                    <span class="keyword-trend {getTrendClass(kw.trend)}">
                      <i class="{getTrendIcon(kw.trend)}"></i>
                      {#if kw.trend !== 0 && kw.trend !== 100}
                        {Math.abs(kw.trend).toFixed(0)}%
                      {/if}
                    </span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Feed Activity (im Zeitraum) -->
          {#if feedActivity.length > 0}
            <div class="stats-card">
              <h3 class="card-title">
                <i class="fa-solid fa-bolt"></i>
                {$_('fnordView.feedActivity') || 'Feed-Aktivität'}
              </h3>
              <div class="feed-list">
                {#each feedActivity as feed (feed.pentacle_id)}
                  <div class="feed-item">
                    <span class="feed-name">{feed.title || `Feed #${feed.pentacle_id}`}</span>
                    <div class="feed-stats">
                      <span class="feed-stat" title="{$_('fnordView.articlesInPeriod')}">
                        <i class="fa-solid fa-newspaper"></i> {feed.articles_period}
                      </span>
                      <span class="feed-stat" title="{$_('fnordView.revisions')}">
                        <i class="fa-solid fa-code-compare"></i> {feed.revisions_period}
                      </span>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>

        <!-- Keyword Cloud (im Zeitraum) -->
        {#if keywordCloud.length > 0}
          <div class="stats-card full-width">
            <h3 class="card-title">
              <i class="fa-solid fa-cloud"></i>
              {$_('fnordView.keywordCloud') || 'Keyword-Wolke'}
            </h3>
            <div class="keyword-cloud">
              {#each keywordCloud as kw (kw.id)}
                <span
                  class="cloud-word"
                  style="
                    font-size: {0.75 + kw.weight * 1.25}rem;
                    opacity: {0.5 + kw.weight * 0.5};
                    color: {getKeywordTypeColor(kw.keyword_type)};
                  "
                  title="{kw.name}: {kw.count} {$_('fnordView.articles')}"
                >
                  {kw.name}
                </span>
              {/each}
              <!-- Hidden fnord Easter Egg -->
              <span class="cloud-word fnord-hidden" title="fnord">fnord</span>
            </div>
          </div>
        {/if}
        {/if}
      </div>
    {:else if activeTab === 'articles'}
      <div class="articles-view">
        {#if changedFnords.length === 0}
          <div class="empty-state">
            <i class="empty-icon fa-solid fa-check"></i>
            <p>{$_('fnordView.noChangedArticles') || 'Keine geänderten Artikel'}</p>
          </div>
        {:else}
          <div class="articles-list">
            {#each changedFnords as fnord (fnord.id)}
              <ArticleItemCompact
                id={fnord.id}
                title={fnord.title}
                status={fnord.status}
                pentacle_title={fnord.pentacle_title}
                changed_at={fnord.changed_at}
                revision_count={fnord.revision_count}
                categories={fnord.categories}
                active={selectedFnordId === fnord.id}
                showIndicators={false}
                onclick={() => selectFnord(fnord.id)}
              />
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Easter Egg -->
  {#if show23EasterEgg}
    <div class="easter-egg-23">
      <span>{$_('fnordView.easterEgg23') || 'Du hast das Geheimnis der 23 entdeckt!'}</span>
    </div>
  {/if}
</div>

<style>
  .fnord-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1;
    background-color: var(--bg-base);
  }

  .fnord-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-default);
    background-color: var(--bg-surface);
  }

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .fnord-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .fnord-summary {
    display: flex;
    gap: 1.5rem;
  }

  .summary-item {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .summary-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .summary-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Section Headers */
  .stats-section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 0;
    margin-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .stats-section-header.trends-header {
    margin-top: 1.5rem;
    padding-top: 1.5rem;
    border-top: 2px solid var(--border-default);
    border-bottom: none;
  }

  .section-header-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .section-header-title i {
    color: var(--accent-primary);
  }

  /* Period Selector */
  .period-selector {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .period-label {
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }

  .period-buttons {
    display: flex;
    gap: 0.25rem;
  }

  .period-btn {
    padding: 0.25rem 0.75rem;
    font-size: 0.75rem;
    background: var(--bg-base);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .period-btn:hover {
    background: var(--bg-overlay);
    border-color: var(--border-active);
  }

  .period-btn.active {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }

  .fnord-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.5rem;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 1rem;
  }

  .empty-icon {
    font-size: 3rem;
    opacity: 0.5;
  }

  .spinner {
    width: 2rem;
    height: 2rem;
    border: 3px solid var(--border-default);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .spinner.small {
    width: 1rem;
    height: 1rem;
    border-width: 2px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Stats View */
  .stats-view {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
  }

  .stats-row.top-row {
    grid-template-columns: 1fr 2fr;
  }

  @media (max-width: 900px) {
    .stats-row.top-row {
      grid-template-columns: 1fr;
    }
  }

  .stats-card {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    padding: 1rem;
    border: 1px solid var(--border-default);
  }

  .stats-card.full-width {
    grid-column: 1 / -1;
  }

  .card-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 1rem 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .card-title i {
    color: var(--accent-primary);
  }

  /* Source Card (Nach Quelle) */
  .source-card {
    overflow: hidden;
  }

  .source-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .source-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .source-name {
    font-size: 0.75rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 140px;
    flex-shrink: 0;
  }

  .source-bar-wrapper {
    flex: 1;
    min-width: 80px;
  }

  .source-progress {
    height: 6px;
    background-color: var(--bg-base);
    border-radius: 3px;
    overflow: hidden;
  }

  .source-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-primary), var(--category-3));
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .source-stats {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
    min-width: 50px;
    justify-content: flex-end;
  }

  .source-count {
    font-size: 0.8125rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .source-articles {
    font-size: 0.625rem;
    color: var(--text-muted);
  }

  .source-more {
    text-align: center;
    font-size: 0.6875rem;
    color: var(--text-muted);
    padding-top: 0.375rem;
    border-top: 1px solid var(--border-default);
    margin-top: 0.25rem;
  }

  /* Empty Placeholder */
  .empty-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 1.5rem 1rem;
    text-align: center;
    color: var(--text-muted);
  }

  .empty-placeholder .empty-icon {
    font-size: 2rem;
    opacity: 0.3;
    margin-bottom: 0.75rem;
  }

  .empty-placeholder p {
    margin: 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .empty-placeholder .empty-hint {
    font-size: 0.75rem;
    margin-top: 0.25rem;
    opacity: 0.7;
  }

  .greyface-empty {
    display: flex;
    flex-direction: column;
  }

  /* Greyface Index Card */
  .greyface-content {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .greyface-gauge {
    position: relative;
    height: 1rem;
    background: linear-gradient(90deg,
      var(--category-4) 0%,
      var(--category-3) 40%,
      var(--category-5) 70%,
      var(--category-2) 100%
    );
    border-radius: 0.5rem;
    overflow: hidden;
  }

  .gauge-fill {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    width: var(--gauge-value);
    background: var(--bg-base);
    opacity: 0.3;
    transition: width 0.3s ease;
  }

  .gauge-value {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.75rem;
    font-weight: 700;
    color: white;
    text-shadow: 0 1px 2px rgba(0,0,0,0.5);
  }

  .greyface-details {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
  }

  .detail-label {
    color: var(--text-muted);
  }

  .detail-value {
    font-weight: 600;
    color: var(--text-primary);
  }

  /* Bias Distribution */
  .bias-distribution {
    margin-top: 0.5rem;
  }

  .distribution-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
    margin-bottom: 0.375rem;
  }

  .distribution-bar {
    display: flex;
    height: 0.5rem;
    border-radius: 0.25rem;
    overflow: hidden;
    background: var(--bg-base);
  }

  .dist-segment {
    height: 100%;
    min-width: 2px;
    transition: width 0.3s ease;
  }

  .dist-segment.left-extreme { background: var(--category-2); }
  .dist-segment.left-leaning { background: color-mix(in srgb, var(--category-2) 60%, var(--bg-surface)); }
  .dist-segment.neutral { background: var(--text-muted); }
  .dist-segment.right-leaning { background: color-mix(in srgb, var(--category-5) 60%, var(--bg-surface)); }
  .dist-segment.right-extreme { background: var(--category-5); }

  .distribution-legend {
    display: flex;
    justify-content: space-between;
    margin-top: 0.25rem;
    font-size: 0.625rem;
    color: var(--text-muted);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .legend-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
  }

  .legend-dot.left-extreme { background: var(--category-2); }
  .legend-dot.left-leaning { background: color-mix(in srgb, var(--category-2) 60%, var(--bg-surface)); }
  .legend-dot.neutral { background: var(--text-muted); }
  .legend-dot.right-leaning { background: color-mix(in srgb, var(--category-5) 60%, var(--bg-surface)); }
  .legend-dot.right-extreme { background: var(--category-5); }

  /* Timeline Chart */
  .timeline-chart {
    height: 150px;
    display: flex;
    flex-direction: column;
  }

  .chart-bars {
    flex: 1;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding-bottom: 1.5rem;
  }

  .chart-day {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
    position: relative;
  }

  .bar-container {
    flex: 1;
    width: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 1px;
  }

  .bar {
    width: 45%;
    border-radius: 2px 2px 0 0;
    transition: height 0.3s ease;
    min-height: 2px;
  }

  .bar.articles-bar {
    background: var(--accent-primary);
  }

  .bar.revisions-bar {
    background: var(--category-5);
  }

  .day-label {
    position: absolute;
    bottom: 0;
    font-size: 0.625rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .chart-legend {
    display: flex;
    justify-content: center;
    gap: 1rem;
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .legend-bar {
    display: inline-block;
    width: 1rem;
    height: 0.5rem;
    border-radius: 2px;
    margin-right: 0.25rem;
  }

  .legend-bar.articles { background: var(--accent-primary); }
  .legend-bar.revisions { background: var(--category-5); }

  /* Keyword List */
  .keyword-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .keyword-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--bg-base);
    border-radius: 0.375rem;
  }

  .keyword-rank {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    width: 1.5rem;
  }

  .keyword-name {
    flex: 1;
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .keyword-count {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .keyword-trend {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.6875rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }

  .keyword-trend.trend-up {
    color: var(--category-4);
    background: color-mix(in srgb, var(--category-4) 15%, transparent);
  }

  .keyword-trend.trend-down {
    color: var(--category-5);
    background: color-mix(in srgb, var(--category-5) 15%, transparent);
  }

  .keyword-trend.trend-new {
    color: var(--category-3);
    background: color-mix(in srgb, var(--category-3) 15%, transparent);
  }

  .keyword-trend.trend-stable {
    color: var(--text-muted);
  }

  /* Feed List */
  .feed-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .feed-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    background: var(--bg-base);
    border-radius: 0.375rem;
  }

  .feed-name {
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    margin-right: 0.5rem;
  }

  .feed-stats {
    display: flex;
    gap: 0.75rem;
  }

  .feed-stat {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .feed-stat i {
    font-size: 0.625rem;
  }

  /* Heatmap */
  .heatmap-container {
    overflow-x: auto;
  }

  .heatmap-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.75rem;
  }

  .heatmap-table th {
    padding: 0.5rem;
    text-align: center;
    color: var(--text-muted);
    font-weight: 500;
    border-bottom: 1px solid var(--border-default);
  }

  .heatmap-table th:first-child {
    text-align: left;
  }

  .bias-col {
    width: 3rem;
  }

  .heatmap-table td {
    padding: 0.375rem;
    text-align: center;
  }

  .source-cell {
    text-align: left;
    color: var(--text-primary);
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .heatmap-cell {
    background: color-mix(in srgb, var(--accent-primary) calc(var(--intensity) * 50%), transparent);
    color: var(--text-primary);
    font-weight: 500;
  }

  .heatmap-cell.neutral-cell {
    background: color-mix(in srgb, var(--text-muted) calc(var(--intensity) * 30%), transparent);
  }

  .avg-cell {
    font-weight: 600;
  }

  /* Keyword Cloud */
  .keyword-cloud {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    justify-content: center;
    padding: 1rem;
  }

  .cloud-word {
    cursor: default;
    transition: transform 0.15s ease;
  }

  .cloud-word:hover {
    transform: scale(1.1);
  }

  .cloud-word.fnord-hidden {
    font-size: 0.5rem !important;
    opacity: 0.05 !important;
    color: var(--text-muted) !important;
    cursor: help;
  }

  .cloud-word.fnord-hidden:hover {
    opacity: 0.3 !important;
  }

  /* Category Cards (existing styles) */
  .stats-section {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    padding: 1.25rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 1rem 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 1rem;
  }

  /* Category colors are set via inline style using getCategoryColorVar() for theme-awareness */

  .category-card {
    background: linear-gradient(135deg, color-mix(in srgb, var(--cat-color) 25%, var(--bg-base)) 0%, color-mix(in srgb, var(--cat-color) 8%, var(--bg-base)) 100%);
    border: 1px solid color-mix(in srgb, var(--cat-color) 50%, transparent);
    border-left: 3px solid var(--cat-color);
    border-radius: 0.625rem;
    padding: 1rem;
    transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
    cursor: pointer;
    text-align: left;
    width: 100%;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 15%, transparent);
  }

  .category-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-color: color-mix(in srgb, var(--cat-color) 70%, transparent);
  }

  .category-card.expanded {
    grid-column: 1 / -1;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.875rem;
  }

  .card-icon-wrapper {
    width: 2.25rem;
    height: 2.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 70%, black));
    border-radius: 0.5rem;
    color: white;
    font-size: 1rem;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 40%, transparent);
  }

  .card-title-text {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
    line-height: 1.2;
  }

  .expand-icon {
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: transform 0.2s ease;
    flex-shrink: 0;
  }

  .expand-icon.rotated {
    transform: rotate(180deg);
  }

  .card-stats {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .stat-row.secondary {
    margin-top: 0.25rem;
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .stat-value {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .stat-row.secondary .stat-label,
  .stat-row.secondary .stat-value {
    font-size: 0.6875rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .progress-bar {
    height: 6px;
    background-color: color-mix(in srgb, var(--cat-color) 20%, transparent);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--cat-color), color-mix(in srgb, var(--cat-color) 80%, white));
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  /* Subcategories */
  .subcategories {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid color-mix(in srgb, var(--cat-color) 20%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .subcategory-loading {
    display: flex;
    justify-content: center;
    padding: 0.5rem;
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.375rem;
    font-size: 0.8125rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .subcategory-icon {
    font-size: 0.75rem;
    color: var(--cat-color);
    opacity: 0.8;
  }

  .subcategory-name {
    color: var(--text-primary);
  }

  .subcategory-stats {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .subcategory-count {
    font-weight: 500;
  }

  .subcategory-divider {
    opacity: 0.5;
  }

  .subcategory-empty {
    text-align: center;
    color: var(--text-muted);
    font-size: 0.75rem;
    padding: 0.5rem;
  }

  /* Articles View */
  .articles-view {
    height: 100%;
  }

  .articles-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  /* Trends Section States */
  .trends-loading-state,
  .trends-error-state,
  .trends-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1.5rem;
    text-align: center;
    color: var(--text-muted);
    gap: 0.75rem;
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    border: 1px dashed var(--border-default);
    margin-top: 1rem;
  }

  .trends-loading-state span,
  .trends-empty-state p,
  .trends-error-state p {
    font-size: 0.9375rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .trends-empty-state .empty-icon,
  .trends-error-state .error-icon {
    font-size: 2.5rem;
    opacity: 0.4;
  }

  .trends-error-state .error-icon {
    color: var(--category-5);
    opacity: 0.7;
  }

  .trends-empty-state .empty-hint,
  .trends-error-state .error-message {
    font-size: 0.8125rem;
    opacity: 0.7;
    max-width: 350px;
  }

  .trends-error-state .error-message {
    font-family: monospace;
    font-size: 0.75rem;
    padding: 0.5rem 1rem;
    background-color: var(--bg-base);
    border-radius: 0.375rem;
    color: var(--text-muted);
  }

  .retry-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    margin-top: 0.5rem;
    font-size: 0.875rem;
    background: var(--accent-primary);
    border: none;
    border-radius: 0.5rem;
    color: white;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .retry-btn:hover {
    background: color-mix(in srgb, var(--accent-primary) 85%, black);
    transform: translateY(-1px);
  }

  .period-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Easter Egg */
  .easter-egg-23 {
    position: fixed;
    bottom: 2rem;
    left: 50%;
    transform: translateX(-50%);
    background: var(--category-3);
    color: var(--bg-base);
    padding: 0.75rem 1.5rem;
    border-radius: 2rem;
    font-weight: 600;
    animation: pulse 2s ease-in-out infinite;
    z-index: 1000;
  }

  @keyframes pulse {
    0%, 100% { transform: translateX(-50%) scale(1); }
    50% { transform: translateX(-50%) scale(1.05); }
  }
</style>
