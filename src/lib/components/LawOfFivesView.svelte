<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import Tooltip from './Tooltip.svelte';

  // Types from backend
  interface TopKeyword {
    id: number;
    name: string;
    keyword_type: string | null;
    article_count: number;
    trend_direction: 'Rising' | 'Stable' | 'Falling';
    trend_percent: number;
  }

  interface TopFeed {
    id: number;
    title: string;
    article_count: number;
    unread_count: number;
    articles_today: number;
    articles_week: number;
  }

  interface TopCategory {
    id: number;
    name: string;
    icon: string | null;
    color: string | null;
    article_count: number;
    trend_direction: 'Rising' | 'Stable' | 'Falling';
  }

  interface TrendPoint {
    date: string;
    article_count: number;
    keyword_count: number;
  }

  interface FnordIndexComponents {
    change_rate: number;
    bias_intensity: number;
    activity_rate: number;
    keyword_diversity: number;
    reading_coverage: number;
  }

  interface FnordIndex {
    level: number;
    description: string;
    components: FnordIndexComponents;
  }

  interface LawOfFivesStats {
    top_5_keywords: TopKeyword[];
    top_5_feeds: TopFeed[];
    top_5_categories: TopCategory[];
    five_day_trend: TrendPoint[];
    fnord_index: FnordIndex;
    calculated_at: string;
  }

  let stats = $state<LawOfFivesStats | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    await loadStats();
  });

  async function loadStats() {
    loading = true;
    error = null;
    try {
      stats = await invoke<LawOfFivesStats>('get_law_of_fives_stats');
    } catch (e) {
      console.error('[LawOfFives] Error loading stats:', e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function getTrendIcon(direction: 'Rising' | 'Stable' | 'Falling'): string {
    switch (direction) {
      case 'Rising': return 'fa-solid fa-arrow-trend-up';
      case 'Falling': return 'fa-solid fa-arrow-trend-down';
      default: return 'fa-solid fa-minus';
    }
  }

  function getTrendClass(direction: 'Rising' | 'Stable' | 'Falling'): string {
    switch (direction) {
      case 'Rising': return 'trend-up';
      case 'Falling': return 'trend-down';
      default: return 'trend-stable';
    }
  }

  function getKeywordTypeIcon(type: string | null): string {
    switch (type) {
      case 'person': return 'fa-solid fa-user';
      case 'organization': return 'fa-solid fa-building';
      case 'location': return 'fa-solid fa-location-dot';
      case 'acronym': return 'fa-solid fa-font';
      default: return 'fa-solid fa-lightbulb';
    }
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('de-DE', { weekday: 'short', day: 'numeric', month: 'short' });
  }

  function navigateToKeyword(keywordId: number) {
    window.dispatchEvent(new CustomEvent('navigate-to-network', { detail: { keywordId } }));
  }

  // Pentagram points calculation
  function getPentagramPoints(cx: number, cy: number, r: number): string {
    const points: [number, number][] = [];
    for (let i = 0; i < 5; i++) {
      const angle = (i * 144 - 90) * (Math.PI / 180);
      points.push([cx + r * Math.cos(angle), cy + r * Math.sin(angle)]);
    }
    return points.map(p => p.join(',')).join(' ');
  }

  // Get fnord level description
  function getLevelDescription(level: number): string {
    const descriptions: Record<number, string> = {
      1: $_('lawOfFives.levels.novice'),
      2: $_('lawOfFives.levels.initiate'),
      3: $_('lawOfFives.levels.adept'),
      4: $_('lawOfFives.levels.illuminated'),
      5: $_('lawOfFives.levels.pope'),
    };
    return descriptions[level] || '';
  }
</script>

<div class="law-of-fives-view">
  <!-- Header with Pentagram -->
  <div class="view-header">
    <div class="header-content">
      <div class="pentagram-badge">
        <svg viewBox="0 0 100 100" class="pentagram">
          <polygon
            points={getPentagramPoints(50, 50, 45)}
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          />
          <circle cx="50" cy="50" r="48" fill="none" stroke="currentColor" stroke-width="1" opacity="0.3" />
        </svg>
      </div>
      <div class="header-text">
        <h1 class="view-title">
          <Tooltip termKey="lawOfFives">{$_('lawOfFives.title')}</Tooltip>
        </h1>
        <p class="view-subtitle">{$_('lawOfFives.subtitle')}</p>
      </div>
    </div>
    <button class="refresh-btn" onclick={loadStats} disabled={loading} aria-label={$_('actions.refresh')}>
      <i class="fa-solid fa-rotate {loading ? 'fa-spin' : ''}"></i>
    </button>
  </div>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <span>{$_('lawOfFives.loading')}</span>
    </div>
  {:else if error}
    <div class="error-state">
      <i class="fa-solid fa-exclamation-triangle"></i>
      <p>{error}</p>
      <button onclick={loadStats}>{$_('actions.refresh')}</button>
    </div>
  {:else if stats}
    <div class="dashboard-content">
      <!-- Fnord Index (5-Level Gauge) -->
      <div class="fnord-index-section">
        <div class="section-header">
          <h2>{$_('lawOfFives.fnordIndex')}</h2>
        </div>
        <div class="fnord-gauge">
          <div class="gauge-pentagram">
            <svg viewBox="0 0 200 200" class="gauge-svg">
              <!-- Background pentagon -->
              <polygon
                points={getPentagramPoints(100, 100, 90)}
                fill="none"
                stroke="var(--border-default)"
                stroke-width="2"
              />
              <!-- Filled levels -->
              {#each [1, 2, 3, 4, 5] as level (level)}
                <polygon
                  points={getPentagramPoints(100, 100, 18 * level)}
                  fill={level <= stats.fnord_index.level ? 'var(--accent-primary)' : 'transparent'}
                  opacity={level <= stats.fnord_index.level ? 0.2 + (level * 0.1) : 0.05}
                  stroke={level <= stats.fnord_index.level ? 'var(--accent-primary)' : 'var(--border-muted)'}
                  stroke-width="1"
                />
              {/each}
              <!-- Center level number -->
              <text x="100" y="105" text-anchor="middle" class="gauge-level">{stats.fnord_index.level}</text>
            </svg>
          </div>
          <div class="gauge-info">
            <span class="gauge-description">{stats.fnord_index.description}</span>
            <div class="gauge-components">
              <div class="component" title="{$_('lawOfFives.components.changeRate')}">
                <i class="fa-solid fa-code-compare"></i>
                <span>{(stats.fnord_index.components.change_rate * 100).toFixed(0)}%</span>
              </div>
              <div class="component" title="{$_('lawOfFives.components.biasIntensity')}">
                <i class="fa-solid fa-scale-balanced"></i>
                <span>{(stats.fnord_index.components.bias_intensity * 100).toFixed(0)}%</span>
              </div>
              <div class="component" title="{$_('lawOfFives.components.activityRate')}">
                <i class="fa-solid fa-bolt"></i>
                <span>{(stats.fnord_index.components.activity_rate * 100).toFixed(0)}%</span>
              </div>
              <div class="component" title="{$_('lawOfFives.components.keywordDiversity')}">
                <i class="fa-solid fa-tags"></i>
                <span>{(stats.fnord_index.components.keyword_diversity * 100).toFixed(0)}%</span>
              </div>
              <div class="component" title="{$_('lawOfFives.components.readingCoverage')}">
                <i class="fa-solid fa-book-open"></i>
                <span>{(stats.fnord_index.components.reading_coverage * 100).toFixed(0)}%</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 5-Day Trend -->
      <div class="trend-section">
        <div class="section-header">
          <h2>{$_('lawOfFives.fiveDayTrend')}</h2>
        </div>
        <div class="trend-chart">
          {#each stats.five_day_trend as point, i (point.date)}
            {@const maxArticles = Math.max(...stats.five_day_trend.map(p => p.article_count), 1)}
            {@const height = (point.article_count / maxArticles) * 100}
            <div class="trend-bar-container">
              <div class="trend-bar" style="height: {height}%">
                <span class="trend-value">{point.article_count}</span>
              </div>
              <span class="trend-date">{formatDate(point.date)}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Three columns grid -->
      <div class="three-columns">
        <!-- Top 5 Keywords -->
        <div class="card keywords-card">
          <div class="card-header">
            <i class="fa-solid fa-hashtag"></i>
            <h3>{$_('lawOfFives.topKeywords')}</h3>
          </div>
          <div class="card-content">
            {#each stats.top_5_keywords as keyword, i (keyword.id)}
              <button class="list-item" onclick={() => navigateToKeyword(keyword.id)}>
                <span class="item-rank">{i + 1}</span>
                <i class="{getKeywordTypeIcon(keyword.keyword_type)} item-type-icon"></i>
                <span class="item-name">{keyword.name}</span>
                <span class="item-count">{keyword.article_count}</span>
                <span class="item-trend {getTrendClass(keyword.trend_direction)}">
                  <i class={getTrendIcon(keyword.trend_direction)}></i>
                </span>
              </button>
            {:else}
              <div class="empty-list">{$_('lawOfFives.noData')}</div>
            {/each}
          </div>
        </div>

        <!-- Top 5 Feeds -->
        <div class="card feeds-card">
          <div class="card-header">
            <i class="fa-solid fa-rss"></i>
            <h3>{$_('lawOfFives.topFeeds')}</h3>
          </div>
          <div class="card-content">
            {#each stats.top_5_feeds as feed, i (feed.id)}
              <div class="list-item">
                <span class="item-rank">{i + 1}</span>
                <span class="item-name">{feed.title}</span>
                <div class="feed-stats">
                  <span class="feed-stat" title="{$_('lawOfFives.articlesThisWeek')}">
                    <i class="fa-solid fa-calendar-week"></i>
                    {feed.articles_week}
                  </span>
                  {#if feed.unread_count > 0}
                    <span class="feed-stat unread" title="{$_('lawOfFives.unread')}">
                      <i class="fa-solid fa-eye-slash"></i>
                      {feed.unread_count}
                    </span>
                  {/if}
                </div>
              </div>
            {:else}
              <div class="empty-list">{$_('lawOfFives.noData')}</div>
            {/each}
          </div>
        </div>

        <!-- Top 5 Categories -->
        <div class="card categories-card">
          <div class="card-header">
            <i class="fa-solid fa-folder-tree"></i>
            <h3>{$_('lawOfFives.topCategories')}</h3>
          </div>
          <div class="card-content">
            {#each stats.top_5_categories as category, i (category.id)}
              <div class="list-item" style="--cat-color: {category.color || 'var(--accent-primary)'}">
                <span class="item-rank">{i + 1}</span>
                {#if category.icon}
                  <i class="{category.icon} item-category-icon"></i>
                {/if}
                <span class="item-name">{category.name}</span>
                <span class="item-count">{category.article_count}</span>
                <span class="item-trend {getTrendClass(category.trend_direction)}">
                  <i class={getTrendIcon(category.trend_direction)}></i>
                </span>
              </div>
            {:else}
              <div class="empty-list">{$_('lawOfFives.noData')}</div>
            {/each}
          </div>
        </div>
      </div>

      <!-- Footer timestamp -->
      <div class="dashboard-footer">
        <span class="timestamp">
          {$_('lawOfFives.calculatedAt')}: {new Date(stats.calculated_at).toLocaleString()}
        </span>
      </div>
    </div>
  {/if}
</div>

<style>
  .law-of-fives-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1;
    background-color: var(--bg-base);
    overflow-y: auto;
  }

  /* Header */
  .view-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem 2rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .pentagram-badge {
    width: 3rem;
    height: 3rem;
    color: var(--accent-primary);
  }

  .pentagram {
    width: 100%;
    height: 100%;
  }

  .header-text {
    display: flex;
    flex-direction: column;
  }

  .view-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .view-subtitle {
    font-size: 0.875rem;
    color: var(--text-muted);
    margin: 0.25rem 0 0 0;
  }

  .refresh-btn {
    background: none;
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    padding: 0.5rem 0.75rem;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .refresh-btn:hover:not(:disabled) {
    background-color: var(--bg-overlay);
    color: var(--text-primary);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Loading & Error */
  .loading-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 1rem;
    color: var(--text-muted);
  }

  .spinner {
    width: 2.5rem;
    height: 2.5rem;
    border: 3px solid var(--border-default);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-state i {
    font-size: 2rem;
    color: var(--accent-error);
  }

  .error-state button {
    padding: 0.5rem 1rem;
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
  }

  /* Dashboard Content */
  .dashboard-content {
    padding: 1.5rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  /* Section Headers */
  .section-header {
    margin-bottom: 1rem;
  }

  .section-header h2 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Fnord Index Section */
  .fnord-index-section {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    padding: 1.5rem;
    border: 1px solid var(--border-default);
  }

  .fnord-gauge {
    display: flex;
    align-items: center;
    gap: 2rem;
  }

  .gauge-pentagram {
    width: 12rem;
    height: 12rem;
    flex-shrink: 0;
  }

  .gauge-svg {
    width: 100%;
    height: 100%;
  }

  .gauge-level {
    font-size: 3rem;
    font-weight: 700;
    fill: var(--accent-primary);
  }

  .gauge-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .gauge-description {
    font-size: 1.125rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .gauge-components {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
  }

  .component {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .component i {
    color: var(--accent-primary);
    opacity: 0.8;
  }

  /* 5-Day Trend */
  .trend-section {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    padding: 1.5rem;
    border: 1px solid var(--border-default);
  }

  .trend-chart {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    height: 8rem;
    gap: 0.5rem;
    padding-top: 1rem;
  }

  .trend-bar-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
  }

  .trend-bar {
    width: 100%;
    max-width: 4rem;
    background: linear-gradient(180deg, var(--accent-primary), var(--accent-secondary, var(--accent-primary)));
    border-radius: 0.375rem 0.375rem 0 0;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    min-height: 1.5rem;
    transition: height 0.3s ease;
  }

  .trend-value {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-on-accent);
    padding: 0.25rem;
  }

  .trend-date {
    font-size: 0.6875rem;
    color: var(--text-muted);
    margin-top: 0.5rem;
    text-align: center;
    white-space: nowrap;
  }

  /* Three Columns */
  .three-columns {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1rem;
  }

  @media (max-width: 1200px) {
    .three-columns {
      grid-template-columns: 1fr;
    }
  }

  /* Cards */
  .card {
    background-color: var(--bg-surface);
    border-radius: 0.75rem;
    border: 1px solid var(--border-default);
    overflow: hidden;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-default);
    background-color: var(--bg-overlay);
  }

  .card-header i {
    color: var(--accent-primary);
    font-size: 1rem;
  }

  .card-header h3 {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .card-content {
    padding: 0.5rem;
  }

  /* List Items */
  .list-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 0.5rem;
    transition: background-color 0.15s;
    width: 100%;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    color: inherit;
  }

  .list-item:hover {
    background-color: var(--bg-overlay);
  }

  .item-rank {
    width: 1.5rem;
    height: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--bg-muted);
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
  }

  .item-type-icon {
    font-size: 0.875rem;
    color: var(--text-muted);
    width: 1rem;
    text-align: center;
  }

  .item-category-icon {
    font-size: 0.875rem;
    color: var(--cat-color, var(--accent-primary));
    width: 1rem;
    text-align: center;
  }

  .item-name {
    flex: 1;
    font-size: 0.875rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-count {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-muted);
    background-color: var(--bg-muted);
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
  }

  .item-trend {
    width: 1.25rem;
    text-align: center;
    font-size: 0.75rem;
  }

  .item-trend.trend-up {
    color: var(--accent-success, #22c55e);
  }

  .item-trend.trend-down {
    color: var(--accent-error, #ef4444);
  }

  .item-trend.trend-stable {
    color: var(--text-muted);
  }

  /* Feed stats */
  .feed-stats {
    display: flex;
    gap: 0.5rem;
    align-items: center;
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
    opacity: 0.7;
  }

  .feed-stat.unread {
    color: var(--accent-warning, #f59e0b);
  }

  /* Empty state */
  .empty-list {
    padding: 2rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  /* Footer */
  .dashboard-footer {
    padding-top: 1rem;
    border-top: 1px solid var(--border-default);
    text-align: center;
  }

  .timestamp {
    font-size: 0.75rem;
    color: var(--text-faint);
  }
</style>
