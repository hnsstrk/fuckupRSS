<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { appState } from "../stores/state.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  interface LoadedModel {
    name: string;
    size: number;
    size_vram: number;
    parameter_size: string;
  }

  interface KeywordStats {
    total: number;
    with_embeddings: number;
    queue_size: number;
  }

  let loadedModels = $state<LoadedModel[]>([]);
  let keywordStats = $state<KeywordStats>({ total: 0, with_embeddings: 0, queue_size: 0 });
  let hopelessCount = $state(0);
  let failedCount = $state(0);
  let refreshInterval: ReturnType<typeof setInterval> | null = null;
  let unlistenModels: UnlistenFn | null = null;

  function formatVram(bytes: number): string {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(1)}G`;
  }

  async function loadStats() {
    try {
      // Load models
      const response = await invoke<{ models: LoadedModel[] }>('get_loaded_models');
      loadedModels = response.models;
    } catch (e) {
      loadedModels = [];
    }

    try {
      // Load keyword stats
      const status = await invoke<{ queue_size: number; worker_running: boolean }>('get_embedding_queue_status');
      const stats = await invoke<{ total_keywords: number; total_connections: number }>('get_network_stats');

      // Get embedding count
      const keywords = await invoke<Array<{ id: number; embedding?: boolean }>>('get_keywords', {
        limit: 1,
        offset: 0
      });

      keywordStats = {
        total: stats.total_keywords,
        with_embeddings: stats.total_keywords - status.queue_size,
        queue_size: status.queue_size
      };
    } catch (e) {
      // Ignore errors
    }

    try {
      // Load hopeless article count
      const hopeless = await invoke<{ count: number }>('get_hopeless_count');
      hopelessCount = hopeless.count;
    } catch (e) {
      hopelessCount = 0;
    }

    try {
      // Load failed article count (articles with errors but not yet hopeless)
      const failed = await invoke<{ count: number }>('get_failed_count');
      failedCount = failed.count;
    } catch (e) {
      failedCount = 0;
    }
  }

  onMount(async () => {
    loadStats();
    // Refresh every 10 seconds
    refreshInterval = setInterval(() => {
      loadStats();
    }, 10000);

    unlistenModels = await listen("models-changed", () => {
      loadStats();
    });
  });

  onDestroy(() => {
    if (refreshInterval) clearInterval(refreshInterval);
    if (unlistenModels) unlistenModels();
  });

  // Derived states
  let syncStatus = $derived(
    appState.syncing ? 'syncing' :
    appState.lastSyncResult ? 'done' : 'idle'
  );

  let batchStatus = $derived(
    appState.batchProcessing ? 'processing' : 'idle'
  );

  let embeddingStatus = $derived(
    appState.embeddingProgress?.is_processing ? 'processing' :
    keywordStats.queue_size > 0 ? 'queued' : 'idle'
  );

  let totalVram = $derived(
    loadedModels.reduce((sum, m) => sum + m.size_vram, 0)
  );

  let totalArticles = $derived(
    appState.pentacles.reduce((sum, p) => sum + p.article_count, 0)
  );
</script>

<footer class="status-bar">
  <!-- Sync Status -->
  <div class="status-section" title="Feed Sync">
    <i class="status-icon {syncStatus === 'syncing' ? 'fa-solid fa-rotate fa-spin' : 'fa-solid fa-circle'}" class:active={syncStatus === 'syncing'}></i>
    <span class="status-label">Sync</span>
    {#if syncStatus === 'syncing'}
      <span class="status-value spinning">...</span>
    {:else if appState.lastSyncResult}
      <span class="status-value">{appState.lastSyncResult.total_new} new</span>
    {/if}
  </div>

  <!-- Batch Analysis Status -->
  <div class="status-section" title="Discordian Analysis">
    <i class="status-icon fa-solid fa-bolt" class:active={batchStatus === 'processing'}></i>
    <span class="status-label">Analysis</span>
    {#if batchStatus === 'processing' && appState.batchProgress}
      <span class="status-value">{appState.batchProgress.current}/{appState.batchProgress.total}</span>
    {:else if appState.unprocessedCount.with_content > 0}
      <span class="status-value pending">{appState.unprocessedCount.with_content} pending</span>
    {:else}
      <span class="status-value done">done</span>
    {/if}
  </div>

  <!-- Failed Articles Counter (only show if > 0) -->
  {#if failedCount > 0}
    <div class="status-section" title={$_('statusBar.failedTooltip')}>
      <i class="status-icon fa-solid fa-triangle-exclamation failed"></i>
      <span class="status-label">{$_('statusBar.failed')}</span>
      <span class="status-value failed">{failedCount}</span>
    </div>
  {/if}

  <!-- Hopeless Articles Counter (only show if > 0) -->
  {#if hopelessCount > 0}
    <div class="status-section" title={$_('statusBar.hopelessTooltip')}>
      <i class="status-icon fa-solid fa-skull-crossbones hopeless"></i>
      <span class="status-label">{$_('statusBar.hopeless')}</span>
      <span class="status-value hopeless">{hopelessCount}</span>
    </div>
  {/if}

  <!-- Embedding Status -->
  <div class="status-section" title="Keyword Embeddings">
    <i class="status-icon fa-solid fa-gem" class:active={embeddingStatus === 'processing'}></i>
    <span class="status-label">Embeddings</span>
    {#if embeddingStatus === 'processing'}
      <span class="status-value spinning">generating...</span>
    {:else if keywordStats.queue_size > 0}
      <span class="status-value pending">{keywordStats.queue_size} queued</span>
    {:else}
      <span class="status-value done">{keywordStats.with_embeddings} ready</span>
    {/if}
  </div>

  <!-- Keyword Stats -->
  <div class="status-section" title="Immanentize Network">
    <i class="status-icon fa-solid fa-circle-nodes"></i>
    <span class="status-label">Keywords</span>
    <span class="status-value">{keywordStats.total}</span>
  </div>

  <!-- Spacer -->
  <div class="spacer"></div>

  <!-- Ollama Status -->
  <div class="status-section ollama-section" title={$_('statusBar.ollamaTooltip')}>
    {#if loadedModels.length > 0}
      <i class="status-icon fa-solid fa-microchip active"></i>
      <span class="status-label">Ollama</span>
      <span class="status-value models">
        {#each loadedModels as model, i}
          <span class="model-name" title="{model.name} ({model.parameter_size})">{model.name.split(':')[0]}</span>
          {#if i < loadedModels.length - 1}<span class="sep">·</span>{/if}
        {/each}
        <span class="vram">({formatVram(totalVram)})</span>
      </span>
    {:else if appState.ollamaStatus.available}
      <i class="status-icon fa-solid fa-microchip active"></i>
      <span class="status-label">Ollama</span>
      <span class="status-value done">{$_('statusBar.ollamaReady')}</span>
    {:else}
      <i class="status-icon fa-solid fa-microchip"></i>
      <span class="status-label">Ollama</span>
      <span class="status-value offline">{$_('statusBar.ollamaStopped')}</span>
    {/if}
  </div>

  <!-- Article Stats -->
  <div class="status-section" title="{$_('statusBar.articlesTooltip')}">
    <i class="status-icon fa-solid fa-newspaper"></i>
    <span class="status-label">{$_('statusBar.articles')}</span>
    <span class="status-value">{totalArticles}</span>
  </div>
</footer>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.25rem 0.75rem;
    background-color: var(--bg-tertiary);
    border-top: 1px solid var(--border-primary);
    font-size: 0.7rem;
    color: var(--text-muted);
    height: 1.75rem;
    flex-shrink: 0;
  }

  .status-section {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .status-icon {
    font-size: 0.625rem;
    opacity: 0.5;
    transition: opacity 0.2s, color 0.2s;
  }

  .status-icon.active {
    opacity: 1;
    color: var(--accent-primary);
  }

  .status-label {
    color: var(--text-secondary);
    font-weight: 500;
  }

  .status-value {
    color: var(--text-muted);
  }

  .status-value.done {
    color: var(--accent-success);
  }

  .status-value.pending {
    color: var(--accent-warning);
  }

  .status-value.offline {
    color: var(--accent-error);
  }

  .status-value.hopeless {
    color: var(--accent-error);
  }

  .status-icon.hopeless {
    opacity: 0.7;
    color: var(--accent-error);
  }

  .status-value.failed {
    color: var(--accent-warning);
  }

  .status-icon.failed {
    opacity: 0.7;
    color: var(--accent-warning);
  }

  .status-value.spinning {
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.5; }
    50% { opacity: 1; }
  }

  .spacer {
    flex: 1;
  }

  .ollama-section .status-value.models {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .model-name {
    color: var(--accent-primary);
  }

  .sep {
    opacity: 0.3;
  }

  .vram {
    color: var(--text-muted);
    font-size: 0.625rem;
  }
</style>
