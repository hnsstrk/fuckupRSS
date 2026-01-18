<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type {
    Recommendation,
    RecommendationStats,
    RecommendationLoadState,
    RecommendationPhase
  } from '../../types';
  import RecommendationCard from './RecommendationCard.svelte';

  // Configuration
  const TIMEOUT_MS = 30000; // 30 second timeout
  const PHASE_INTERVAL_MS = 2000; // Update phase every 2s during loading

  interface Props {
    onArticleClick?: (fnordId: number) => void;
  }

  let { onArticleClick }: Props = $props();

  // State machine
  let loadState = $state<RecommendationLoadState>({ status: 'idle' });
  let recommendations = $state<Recommendation[]>([]);
  let stats = $state<RecommendationStats | null>(null);

  // Request tracking
  let requestId = $state<string | null>(null);
  let abortController: AbortController | null = null;
  let timeoutHandle: ReturnType<typeof setTimeout> | null = null;
  let phaseHandle: ReturnType<typeof setInterval> | null = null;

  // Diagnostics
  let showDiagnostics = $state(false);
  let loadTimingMs = $state<number | null>(null);

  // Phase progression (simulated for UX)
  const phases: RecommendationPhase[] = [
    'init',
    'loading_profile',
    'generating_candidates',
    'scoring',
    'finalizing'
  ];

  function generateRequestId(): string {
    return `rec-${Date.now()}-${Math.random().toString(36).substring(2, 8)}`;
  }

  function cleanup() {
    if (timeoutHandle) {
      clearTimeout(timeoutHandle);
      timeoutHandle = null;
    }
    if (phaseHandle) {
      clearInterval(phaseHandle);
      phaseHandle = null;
    }
    abortController = null;
  }

  onMount(async () => {
    await loadRecommendations();
  });

  onDestroy(() => {
    cleanup();
  });

  async function loadRecommendations() {
    // Cleanup any previous request
    cleanup();

    const reqId = generateRequestId();
    requestId = reqId;
    abortController = new AbortController();

    const startTime = Date.now();
    let currentPhaseIndex = 0;

    // Start loading state
    loadState = {
      status: 'loading',
      phase: phases[0],
      startedAt: startTime
    };

    // Setup phase progression
    phaseHandle = setInterval(() => {
      if (loadState.status === 'loading' && currentPhaseIndex < phases.length - 1) {
        currentPhaseIndex++;
        loadState = {
          status: 'loading',
          phase: phases[currentPhaseIndex],
          startedAt: startTime
        };
      }
    }, PHASE_INTERVAL_MS);

    // Setup timeout
    const timeoutPromise = new Promise<never>((_, reject) => {
      timeoutHandle = setTimeout(() => {
        reject(new Error('TIMEOUT'));
      }, TIMEOUT_MS);
    });

    try {
      // Race between request and timeout
      const result = await Promise.race([
        invoke<Recommendation[]>('get_recommendations', { limit: 10 }),
        timeoutPromise
      ]);

      cleanup();
      loadTimingMs = Date.now() - startTime;

      if (result.length === 0) {
        // Load stats for empty state
        await loadStats();
        loadState = {
          status: 'empty',
          stats,
          reason: getEmptyReason(stats)
        };
      } else {
        recommendations = result;
        loadState = {
          status: 'success',
          recommendations: result,
          loadedAt: Date.now()
        };
        // Load stats in background
        loadStats();
      }

      console.log(`[${reqId}] Recommendations loaded in ${loadTimingMs}ms:`, {
        count: result.length,
        phase: 'complete'
      });

    } catch (e) {
      cleanup();
      loadTimingMs = Date.now() - startTime;

      const errorMessage = e instanceof Error ? e.message : String(e);

      if (errorMessage === 'TIMEOUT') {
        loadState = {
          status: 'timeout',
          elapsedMs: TIMEOUT_MS
        };
        console.error(`[${reqId}] Recommendation request timed out after ${TIMEOUT_MS}ms`);
      } else {
        // Parse error for specific codes
        const { code, retryable } = parseError(errorMessage);
        loadState = {
          status: 'error',
          code,
          message: errorMessage,
          retryable
        };
        console.error(`[${reqId}] Failed to load recommendations:`, e);
      }
    }
  }

  function parseError(message: string): { code: string; retryable: boolean } {
    if (message.includes('database is locked')) {
      return { code: 'DB_LOCKED', retryable: true };
    }
    if (message.includes('no such table')) {
      return { code: 'SCHEMA_ERROR', retryable: false };
    }
    if (message.includes('connection')) {
      return { code: 'CONNECTION_ERROR', retryable: true };
    }
    return { code: 'UNKNOWN', retryable: true };
  }

  function getEmptyReason(stats: RecommendationStats | null): string {
    if (!stats) return 'no_stats';
    if (stats.articles_read < 5) return 'not_enough_articles';
    if (stats.articles_with_embedding === 0) return 'no_embeddings';
    if (stats.candidate_pool_size === 0) return 'no_candidates';
    return 'no_matches';
  }

  async function loadStats() {
    try {
      stats = await invoke<RecommendationStats>('get_recommendation_stats');
    } catch (e) {
      console.error('Failed to load recommendation stats:', e);
    }
  }

  function handleCancel() {
    cleanup();
    loadState = { status: 'cancelled' };
    console.log(`[${requestId}] Request cancelled by user`);
  }

  async function handleRetry() {
    await loadRecommendations();
  }

  async function handleSave(fnordId: number) {
    try {
      await invoke('save_article', { fnordId });
      recommendations = recommendations.map(r =>
        r.fnord_id === fnordId ? { ...r, is_saved: true } : r
      );
      loadStats();
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
      loadStats();
    } catch (e) {
      console.error('Failed to unsave article:', e);
    }
  }

  function handleClick(fnordId: number) {
    onArticleClick?.(fnordId);
  }

  // Phase display text
  function getPhaseText(phase: RecommendationPhase): string {
    const phaseTexts: Record<RecommendationPhase, string> = {
      'init': $_('recommendations.phase.init') || 'Initialisiere...',
      'loading_profile': $_('recommendations.phase.loading_profile') || 'Lade Leseprofil...',
      'generating_candidates': $_('recommendations.phase.generating_candidates') || 'Ermittle Kandidaten...',
      'scoring': $_('recommendations.phase.scoring') || 'Berechne Relevanz...',
      'finalizing': $_('recommendations.phase.finalizing') || 'Bereite Empfehlungen auf...'
    };
    return phaseTexts[phase];
  }

  function getEmptyReasonText(reason: string): string {
    const reasons: Record<string, string> = {
      'not_enough_articles': $_('recommendations.empty.reason_not_enough') || 'Du hast noch nicht genug Artikel gelesen.',
      'no_embeddings': $_('recommendations.empty.reason_no_embeddings') || 'Artikel-Embeddings werden noch generiert.',
      'no_candidates': $_('recommendations.empty.reason_no_candidates') || 'Alle verfügbaren Artikel wurden bereits gelesen oder ausgeblendet.',
      'no_matches': $_('recommendations.empty.reason_no_matches') || 'Keine passenden Empfehlungen gefunden.',
      'no_stats': $_('recommendations.empty.reason_no_stats') || 'Statistiken konnten nicht geladen werden.'
    };
    return reasons[reason] || reasons['no_matches'];
  }

  // Elapsed time during loading
  let elapsedSeconds = $derived(
    loadState.status === 'loading'
      ? Math.floor((Date.now() - loadState.startedAt) / 1000)
      : 0
  );
</script>

<div class="recommendation-list">
  {#if loadState.status === 'loading'}
    <!-- Loading with Progress -->
    <div class="loading-container">
      <div class="loading-spinner">
        <i class="fa-solid fa-spinner fa-spin"></i>
      </div>
      <p class="loading-phase">{getPhaseText(loadState.phase)}</p>
      <div class="loading-progress">
        <div class="progress-bar">
          {#each phases as phase, i}
            <div
              class="progress-step"
              class:active={phases.indexOf(loadState.phase) >= i}
              class:current={loadState.phase === phase}
            ></div>
          {/each}
        </div>
        <span class="loading-time">{elapsedSeconds}s</span>
      </div>
      <button type="button" class="cancel-btn" onclick={handleCancel}>
        <i class="fa-solid fa-xmark"></i>
        {$_('common.cancel') || 'Abbrechen'}
      </button>
    </div>

  {:else if loadState.status === 'error'}
    <!-- Error State -->
    <div class="error-container">
      <i class="fa-solid fa-exclamation-triangle"></i>
      <p class="error-title">{$_('recommendations.error.title') || 'Fehler beim Laden'}</p>
      <p class="error-message">{loadState.message}</p>
      <code class="error-code">{loadState.code}</code>
      {#if loadState.retryable}
        <button type="button" class="retry-btn" onclick={handleRetry}>
          <i class="fa-solid fa-arrows-rotate"></i>
          {$_('common.retry') || 'Erneut versuchen'}
        </button>
      {/if}
      <button type="button" class="diagnose-btn" onclick={() => showDiagnostics = !showDiagnostics}>
        <i class="fa-solid fa-bug"></i>
        {$_('recommendations.show_diagnostics') || 'Diagnose anzeigen'}
      </button>
    </div>

  {:else if loadState.status === 'timeout'}
    <!-- Timeout State -->
    <div class="timeout-container">
      <i class="fa-solid fa-clock"></i>
      <p class="timeout-title">{$_('recommendations.timeout.title') || 'Zeitüberschreitung'}</p>
      <p class="timeout-message">
        {$_('recommendations.timeout.message') || 'Die Anfrage hat zu lange gedauert.'}
        ({(loadState.elapsedMs / 1000).toFixed(1)}s)
      </p>
      <button type="button" class="retry-btn" onclick={handleRetry}>
        <i class="fa-solid fa-arrows-rotate"></i>
        {$_('common.retry') || 'Erneut versuchen'}
      </button>
    </div>

  {:else if loadState.status === 'cancelled'}
    <!-- Cancelled State -->
    <div class="cancelled-container">
      <i class="fa-solid fa-ban"></i>
      <p>{$_('recommendations.cancelled') || 'Anfrage abgebrochen'}</p>
      <button type="button" class="retry-btn" onclick={handleRetry}>
        <i class="fa-solid fa-arrows-rotate"></i>
        {$_('recommendations.try_again') || 'Nochmal versuchen'}
      </button>
    </div>

  {:else if loadState.status === 'empty'}
    <!-- Empty State -->
    <div class="empty-state">
      <div class="empty-icon">
        <i class="fa-duotone fa-wand-magic-sparkles"></i>
      </div>
      <h3>{$_('recommendations.empty.title') || 'Noch keine Empfehlungen'}</h3>
      <p class="empty-reason">{getEmptyReasonText(loadState.reason)}</p>

      <div class="empty-tips">
        <h4>{$_('recommendations.empty.tips_title') || 'So bekommst du Empfehlungen:'}</h4>
        <ul>
          <li>
            <i class="fa-solid fa-book-open"></i>
            {$_('recommendations.empty.tip_read') || 'Lies mindestens 5-10 Artikel'}
          </li>
          <li>
            <i class="fa-solid fa-rss"></i>
            {$_('recommendations.empty.tip_feeds') || 'Füge mehr Feeds hinzu'}
          </li>
          <li>
            <i class="fa-solid fa-robot"></i>
            {$_('recommendations.empty.tip_ollama') || 'Aktiviere Ollama für KI-Analyse'}
          </li>
        </ul>
      </div>

      {#if loadState.stats}
        <div class="empty-progress">
          <div class="progress-item">
            <span class="progress-label">{$_('recommendations.stats.articles_read') || 'Gelesene Artikel'}</span>
            <span class="progress-value">{loadState.stats.articles_read}</span>
          </div>
          <div class="progress-item">
            <span class="progress-label">{$_('recommendations.stats.profile_strength') || 'Profilstärke'}</span>
            <span class="progress-value profile-{loadState.stats.profile_strength.toLowerCase()}">
              {loadState.stats.profile_strength}
            </span>
          </div>
        </div>
      {/if}
    </div>

  {:else if loadState.status === 'success' || recommendations.length > 0}
    <!-- Success State -->

    <!-- Stats Bar -->
    {#if stats && stats.articles_read > 0}
      <div class="stats-bar">
        <div class="stat-item">
          <i class="fa-solid fa-bookmark"></i>
          <span>{stats.total_saved} {$_('recommendations.stats.articles_saved') || 'gespeichert'}</span>
        </div>
        <div class="stat-item">
          <span class="profile-badge profile-{stats.profile_strength.toLowerCase()}">
            {stats.profile_strength}
          </span>
        </div>
        {#if loadTimingMs}
          <div class="stat-item timing">
            <i class="fa-solid fa-stopwatch"></i>
            <span>{(loadTimingMs / 1000).toFixed(2)}s</span>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Recommendations Grid -->
    <div class="recommendations-grid">
      {#each recommendations as recommendation (recommendation.fnord_id)}
        <RecommendationCard
          {recommendation}
          onsave={handleSave}
          onunsave={handleUnsave}
          onclick={handleClick}
        />
      {/each}
    </div>

    <button type="button" class="refresh-btn" onclick={handleRetry}>
      <i class="fa-solid fa-arrows-rotate"></i>
      {$_('recommendations.refresh') || 'Aktualisieren'}
    </button>
  {:else}
    <!-- Idle State -->
    <div class="idle-container">
      <button type="button" onclick={handleRetry}>
        {$_('recommendations.load') || 'Empfehlungen laden'}
      </button>
    </div>
  {/if}

  <!-- Diagnostics Panel -->
  {#if showDiagnostics}
    <div class="diagnostics-panel">
      <h4>
        <i class="fa-solid fa-bug"></i>
        Diagnose
        <button type="button" class="close-btn" onclick={() => showDiagnostics = false}>
          <i class="fa-solid fa-xmark"></i>
        </button>
      </h4>
      <dl>
        <dt>Request ID</dt>
        <dd><code>{requestId || 'none'}</code></dd>

        <dt>Status</dt>
        <dd><code>{loadState.status}</code></dd>

        <dt>Load Time</dt>
        <dd>{loadTimingMs ? `${loadTimingMs}ms` : 'N/A'}</dd>

        {#if stats}
          <dt>Articles Read</dt>
          <dd>{stats.articles_read}</dd>

          <dt>With Embedding</dt>
          <dd>{stats.articles_with_embedding}</dd>

          <dt>Candidate Pool</dt>
          <dd>{stats.candidate_pool_size}</dd>

          <dt>Top Keywords</dt>
          <dd>
            {#if stats.top_keywords.length > 0}
              {stats.top_keywords.slice(0, 5).map(k => k.name).join(', ')}
            {:else}
              none
            {/if}
          </dd>
        {/if}
      </dl>
    </div>
  {/if}
</div>

<style>
  .recommendation-list {
    padding: 0;
    position: relative;
  }

  .recommendations-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1rem;
  }

  /* Loading State */
  .loading-container,
  .error-container,
  .timeout-container,
  .cancelled-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    text-align: center;
    color: var(--text-muted);
    gap: 1rem;
  }

  .loading-spinner i {
    font-size: 2.5rem;
    color: var(--accent-primary);
  }

  .loading-phase {
    font-size: 0.9375rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .loading-progress {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-top: 0.5rem;
  }

  .progress-bar {
    display: flex;
    gap: 0.375rem;
  }

  .progress-step {
    width: 2rem;
    height: 0.375rem;
    background: var(--border-default);
    border-radius: 0.1875rem;
    transition: all 0.3s ease;
  }

  .progress-step.active {
    background: var(--accent-primary);
  }

  .progress-step.current {
    background: var(--accent-primary);
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .loading-time {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .cancel-btn {
    margin-top: 1rem;
    padding: 0.5rem 1rem;
    background: transparent;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.875rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition: all 0.2s;
  }

  .cancel-btn:hover {
    border-color: var(--status-error);
    color: var(--status-error);
  }

  /* Error State */
  .error-container i,
  .timeout-container i {
    font-size: 2.5rem;
    color: var(--status-error);
  }

  .error-title,
  .timeout-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .error-message,
  .timeout-message {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin: 0;
    max-width: 400px;
  }

  .error-code {
    font-size: 0.75rem;
    background: var(--bg-overlay);
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    color: var(--text-muted);
  }

  .retry-btn,
  .diagnose-btn {
    margin-top: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition: all 0.2s;
  }

  .retry-btn {
    background: var(--accent-primary);
    border: none;
    color: var(--text-on-accent);
  }

  .retry-btn:hover {
    filter: brightness(1.1);
  }

  .diagnose-btn {
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--text-muted);
  }

  .diagnose-btn:hover {
    border-color: var(--text-secondary);
    color: var(--text-secondary);
  }

  /* Cancelled State */
  .cancelled-container i {
    font-size: 2rem;
    color: var(--text-muted);
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

  .empty-reason {
    margin: 0 0 1.5rem;
    font-size: 0.9375rem;
    color: var(--text-secondary);
    max-width: 400px;
  }

  .empty-tips {
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

  .stat-item.timing {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
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

  /* Idle State */
  .idle-container {
    display: flex;
    justify-content: center;
    padding: 2rem;
  }

  .idle-container button {
    padding: 0.75rem 1.5rem;
    background: var(--accent-primary);
    border: none;
    border-radius: 0.5rem;
    color: var(--text-on-accent);
    font-size: 0.9375rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .idle-container button:hover {
    filter: brightness(1.1);
  }

  /* Diagnostics Panel */
  .diagnostics-panel {
    position: fixed;
    bottom: 4rem;
    right: 1rem;
    width: 320px;
    background: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 0.625rem;
    padding: 1rem;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    z-index: 100;
    font-size: 0.8125rem;
  }

  .diagnostics-panel h4 {
    margin: 0 0 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .diagnostics-panel .close-btn {
    margin-left: auto;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0.25rem;
  }

  .diagnostics-panel dl {
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.375rem 0.75rem;
  }

  .diagnostics-panel dt {
    color: var(--text-muted);
  }

  .diagnostics-panel dd {
    margin: 0;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diagnostics-panel code {
    font-size: 0.75rem;
    background: var(--bg-overlay);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
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

    .diagnostics-panel {
      width: calc(100% - 2rem);
      left: 1rem;
      right: 1rem;
    }
  }
</style>
