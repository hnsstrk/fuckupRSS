<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Recommendation, RecommendationStats } from '../../types';
  import RecommendationCard from './RecommendationCard.svelte';

  interface Props {
    onArticleClick?: (fnordId: number) => void;
  }

  let { onArticleClick }: Props = $props();

  let recommendations = $state<Recommendation[]>([]);
  let stats = $state<RecommendationStats | null>(null);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    await loadRecommendations();
    await loadStats();
  });

  async function loadRecommendations() {
    isLoading = true;
    error = null;

    try {
      recommendations = await invoke<Recommendation[]>('get_recommendations', { limit: 10 });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      console.error('Failed to load recommendations:', e);
    } finally {
      isLoading = false;
    }
  }

  async function loadStats() {
    try {
      stats = await invoke<RecommendationStats>('get_recommendation_stats');
    } catch (e) {
      console.error('Failed to load recommendation stats:', e);
    }
  }

  async function handleSave(fnordId: number) {
    try {
      await invoke('save_article', { fnordId });
      recommendations = recommendations.map(r =>
        r.fnord_id === fnordId ? { ...r, is_saved: true } : r
      );
      // Reload stats
      await loadStats();
    } catch (e) {
      console.error('Failed to save article:', e);
    }
  }

  async function handleUnsave(fnordId: number) {
    try {
      await invoke('unsave_article', { fnordId });
      recommendations = recommendations.map(r =>
        r.fnord_id === fnordId ? { ...r, is_saved: false } : r
      );
      await loadStats();
    } catch (e) {
      console.error('Failed to unsave article:', e);
    }
  }

  async function handleHide(fnordId: number) {
    try {
      await invoke('hide_recommendation', { fnordId });
      // Optimistic update: Remove from list
      recommendations = recommendations.filter(r => r.fnord_id !== fnordId);

      // Load new recommendations if we're running low
      if (recommendations.length < 5) {
        const newRecs = await invoke<Recommendation[]>('get_recommendations', { limit: 5 });
        const existingIds = new Set(recommendations.map(r => r.fnord_id));
        const toAdd = newRecs.filter(r => !existingIds.has(r.fnord_id));
        recommendations = [...recommendations, ...toAdd.slice(0, 5 - recommendations.length)];
      }
    } catch (e) {
      console.error('Failed to hide recommendation:', e);
    }
  }

  function handleClick(fnordId: number) {
    onArticleClick?.(fnordId);
  }
</script>

<div class="recommendation-list">
  {#if isLoading}
    <div class="loading-container">
      <i class="fa-solid fa-spinner fa-spin"></i>
      <p>{$_('recommendations.loading')}</p>
    </div>

  {:else if error}
    <div class="error-container">
      <i class="fa-solid fa-exclamation-triangle"></i>
      <p>{error}</p>
      <button type="button" onclick={loadRecommendations}>
        {$_('common.retry')}
      </button>
    </div>

  {:else if recommendations.length === 0}
    <div class="empty-state">
      <div class="empty-icon">
        <i class="fa-duotone fa-wand-magic-sparkles"></i>
      </div>
      <h3>{$_('recommendations.empty.title')}</h3>
      <p>{$_('recommendations.empty.description')}</p>

      <div class="empty-tips">
        <h4>{$_('recommendations.empty.tips_title')}</h4>
        <ul>
          <li>
            <i class="fa-solid fa-book-open"></i>
            {$_('recommendations.empty.tip_read')}
          </li>
          <li>
            <i class="fa-solid fa-rss"></i>
            {$_('recommendations.empty.tip_feeds')}
          </li>
          <li>
            <i class="fa-solid fa-robot"></i>
            {$_('recommendations.empty.tip_ollama')}
          </li>
        </ul>
      </div>

      {#if stats}
        <div class="empty-progress">
          <div class="progress-item">
            <span class="progress-label">{$_('recommendations.stats.articles_read')}</span>
            <span class="progress-value">{stats.articles_read}</span>
          </div>
          <div class="progress-item">
            <span class="progress-label">{$_('recommendations.stats.profile_strength')}</span>
            <span class="progress-value profile-{stats.profile_strength.toLowerCase()}">{stats.profile_strength}</span>
          </div>
        </div>
      {/if}
    </div>

  {:else}
    <!-- Stats Bar -->
    {#if stats && stats.articles_read > 0}
      <div class="stats-bar">
        <div class="stat-item">
          <i class="fa-solid fa-bookmark"></i>
          <span>{stats.total_saved} {$_('recommendations.stats.articles_saved')}</span>
        </div>
        <div class="stat-item">
          <span class="profile-badge profile-{stats.profile_strength.toLowerCase()}">
            {stats.profile_strength}
          </span>
        </div>
      </div>
    {/if}

    <!-- Recommendations Grid -->
    <div class="recommendations-grid">
      {#each recommendations as recommendation (recommendation.fnord_id)}
        <RecommendationCard
          {recommendation}
          onsave={handleSave}
          onunsave={handleUnsave}
          onhide={handleHide}
          onclick={handleClick}
        />
      {/each}
    </div>

    <button type="button" class="refresh-btn" onclick={loadRecommendations}>
      <i class="fa-solid fa-arrows-rotate"></i>
      {$_('recommendations.refresh')}
    </button>
  {/if}
</div>

<style>
  .recommendation-list {
    padding: 0;
  }

  .recommendations-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1rem;
  }

  .loading-container,
  .error-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    text-align: center;
    color: var(--text-muted);
    gap: 1rem;
  }

  .loading-container i {
    font-size: 2rem;
    color: var(--accent-primary);
  }

  .error-container i {
    font-size: 2rem;
    color: var(--status-error);
  }

  .error-container button {
    margin-top: 0.5rem;
    padding: 0.5rem 1rem;
    background: transparent;
    border: 1px solid var(--accent-primary);
    border-radius: 0.375rem;
    color: var(--accent-primary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .error-container button:hover {
    background: var(--accent-primary);
    color: var(--text-on-accent);
  }

  /* Empty State */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 3rem 1.5rem;
    text-align: center;
  }

  .empty-icon {
    width: 5rem;
    height: 5rem;
    margin-bottom: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--accent-primary), var(--ctp-mauve));
    border-radius: 50%;
    box-shadow: 0 8px 32px color-mix(in srgb, var(--accent-primary) 30%, transparent);
  }

  .empty-icon i {
    font-size: 2.5rem;
    color: white;
  }

  .empty-state h3 {
    margin: 0 0 0.75rem;
    font-size: 1.25rem;
    color: var(--text-primary);
  }

  .empty-state > p {
    margin: 0;
    font-size: 0.9375rem;
    color: var(--text-secondary);
    max-width: 400px;
  }

  .empty-tips {
    margin-top: 2rem;
    text-align: left;
    background: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.625rem;
    padding: 1.25rem;
    width: 100%;
    max-width: 400px;
  }

  .empty-tips h4 {
    margin: 0 0 0.75rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .empty-tips ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .empty-tips li {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .empty-tips li i {
    width: 1.25rem;
    color: var(--accent-primary);
  }

  .empty-progress {
    margin-top: 1.5rem;
    display: flex;
    gap: 2rem;
  }

  .progress-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }

  .progress-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .progress-value {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .progress-value.profile-cold {
    color: var(--ctp-blue);
  }

  .progress-value.profile-warm {
    color: var(--ctp-yellow);
  }

  .progress-value.profile-hot {
    color: var(--status-success);
  }

  /* Stats Bar */
  .stats-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    padding: 0.75rem 1rem;
    background: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
  }

  .stat-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .stat-item i {
    color: var(--accent-primary);
  }

  .profile-badge {
    padding: 0.25rem 0.625rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .profile-badge.profile-cold {
    background: color-mix(in srgb, var(--ctp-blue) 20%, transparent);
    color: var(--ctp-blue);
  }

  .profile-badge.profile-warm {
    background: color-mix(in srgb, var(--ctp-yellow) 20%, transparent);
    color: var(--ctp-yellow);
  }

  .profile-badge.profile-hot {
    background: color-mix(in srgb, var(--status-success) 20%, transparent);
    color: var(--status-success);
  }

  /* Refresh Button */
  .refresh-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    width: 100%;
    margin-top: 1.5rem;
    padding: 0.75rem 1rem;
    background: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .refresh-btn:hover {
    background: var(--bg-base);
    color: var(--text-primary);
    border-color: var(--accent-primary);
  }

  @media (max-width: 640px) {
    .recommendations-grid {
      grid-template-columns: 1fr;
    }

    .empty-progress {
      flex-direction: column;
      gap: 1rem;
    }

    .stats-bar {
      flex-direction: column;
      gap: 0.5rem;
      align-items: flex-start;
    }
  }
</style>
