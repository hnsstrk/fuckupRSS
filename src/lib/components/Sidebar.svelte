<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { appState, toasts, type BatchProgress, type EmbeddingProgress } from "../stores/state.svelte";
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import Tooltip from "./Tooltip.svelte";

  interface LoadedModel {
    name: string;
    size: number;
    size_vram: number;
    parameter_size: string;
  }

  interface Props {
    onsettings?: () => void;
    onnetwork?: () => void;
    onfnord?: () => void;
    onarticles?: () => void;
    settingsActive?: boolean;
    networkActive?: boolean;
    fnordActive?: boolean;
    articlesActive?: boolean;
  }

  let { onsettings, onnetwork, onfnord, onarticles, settingsActive = false, networkActive = false, fnordActive = false, articlesActive = true }: Props = $props();

  let showAddForm = $state(false);
  let newFeedUrl = $state("");
  let sidebarMode = $state<'pentacles' | 'sephiroth'>('pentacles');
  let searchInput = $state("");
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  let unlisten: UnlistenFn | null = null;
  let unlistenModels: UnlistenFn | null = null;
  let unlistenArticlesReset: UnlistenFn | null = null;
  let unlistenEmbedding: UnlistenFn | null = null;
  let loadedModels = $state<LoadedModel[]>([]);
  let maintenanceInterval: ReturnType<typeof setInterval> | null = null;

  async function loadLoadedModels() {
    try {
      const response = await invoke<{ models: LoadedModel[] }>('get_loaded_models');
      loadedModels = response.models;
    } catch (e) {
      console.error('Failed to load loaded models:', e);
      loadedModels = [];
    }
  }

  function formatVram(bytes: number): string {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(1)}G`;
  }

  onMount(async () => {
    await appState.loadPentacles();
    await appState.loadSephiroth();
    await appState.loadFnords();
    // Reset false positive changes from migration bug (one-time fix)
    await appState.resetAllChanges();
    await appState.loadChangedFnords();
    // Check Ollama availability and auto-set models if not configured
    await appState.checkOllama();
    // Auto-sync feeds on startup
    appState.syncAllFeeds();
    // Load unprocessed count
    appState.loadUnprocessedCount();
    // Load currently loaded models
    await loadLoadedModels();

    // Listen for batch progress events
    console.log("Setting up batch-progress event listener...");
    unlisten = await listen<BatchProgress>("batch-progress", (event) => {
      console.log("Batch progress event received:", event.payload);
      console.log("Current batchProgress before update:", appState.batchProgress);
      appState.updateBatchProgress(event.payload);
      console.log("Current batchProgress after update:", appState.batchProgress);
    });
    console.log("Batch-progress listener set up successfully");

    // Listen for model changes from Settings
    unlistenModels = await listen("models-changed", async () => {
      console.log("Models changed event received, refreshing loaded models...");
      await loadLoadedModels();
    });

    // Listen for articles reset from Settings
    unlistenArticlesReset = await listen("articles-reset", async () => {
      console.log("Articles reset event received, refreshing unprocessed count...");
      await appState.loadUnprocessedCount();
    });

    // Listen for embedding progress events
    unlistenEmbedding = await listen<EmbeddingProgress>("embedding-progress", (event) => {
      appState.updateEmbeddingProgress(event.payload);
    });

    // Schedule periodic maintenance (every 60 minutes)
    maintenanceInterval = setInterval(() => {
      runBackgroundMaintenance();
    }, 60 * 60 * 1000);
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (unlistenModels) unlistenModels();
    if (unlistenArticlesReset) unlistenArticlesReset();
    if (unlistenEmbedding) unlistenEmbedding();
    if (maintenanceInterval) clearInterval(maintenanceInterval);
  });

  async function handleSync() {
    const result = await appState.syncAllFeeds();
    if (result) {
      if (result.total_new > 0 || result.total_updated > 0) {
        toasts.success($_('toast.syncSuccess', {
          values: { newCount: result.total_new, updatedCount: result.total_updated }
        }));
        runBackgroundMaintenance();
      } else {
        toasts.info($_('toast.syncSuccessNoNew'));
      }
    } else if (appState.error) {
      toasts.error($_('toast.syncError', { values: { error: appState.error }}));
    }
  }

  async function runBackgroundMaintenance() {
    try {
      await invoke('calculate_keyword_quality_scores', { limit: 500 });
    } catch (e) {
      console.debug('Background maintenance skipped:', e);
    }
  }

  async function handleAddFeed(e: Event) {
    e.preventDefault();
    if (newFeedUrl.trim()) {
      const url = newFeedUrl.trim();
      newFeedUrl = "";
      showAddForm = false;
      await appState.addPentacle(url);
      if (appState.error) {
        toasts.error($_('toast.feedError', { values: { error: appState.error }}));
      } else {
        toasts.success($_('toast.feedAdded'));
      }
    }
  }

  function handleSelectPentacle(id: number) {
    appState.selectedView = "pentacle";
    appState.selectPentacle(id);
  }

  function handleSelectSephiroth(id: number) {
    appState.selectSephiroth(id);
  }

  async function handleDeletePentacle(id: number) {
    await appState.deletePentacle(id);
    if (!appState.error) {
      toasts.success($_('toast.feedDeleted'));
    }
  }

  async function handleBatchProcessing() {
    console.log("=== handleBatchProcessing called ===");
    console.log("batchProcessing:", appState.batchProcessing);
    console.log("ollamaStatus:", appState.ollamaStatus);
    console.log("selectedModel:", appState.selectedModel);
    console.log("unprocessedCount:", appState.unprocessedCount);

    const result = await appState.startBatchProcessing();
    console.log("startBatchProcessing result:", result);
    console.log("appState.error:", appState.error);

    // Refresh loaded models after batch processing (model may have been loaded)
    await loadLoadedModels();

    if (result) {
      toasts.success($_('batch.complete', {
        values: { succeeded: result.succeeded, failed: result.failed }
      }));
    } else if (appState.error) {
      toasts.error($_('toast.analyzeError', { values: { error: appState.error }}));
    }
  }

  function handleCancelBatch() {
    appState.cancelBatch();
  }

  function handleSearchInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    searchInput = value;

    // Clear previous timeout
    if (searchTimeout) {
      clearTimeout(searchTimeout);
    }

    // Debounce search - wait 300ms after user stops typing
    if (value.trim()) {
      searchTimeout = setTimeout(async () => {
        await appState.semanticSearch(value.trim());
      }, 300);
    } else {
      appState.clearSearch();
    }
  }

  function handleClearSearch() {
    searchInput = "";
    appState.clearSearch();
  }

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClearSearch();
    } else if (e.key === 'Enter') {
      // Immediate search on Enter
      if (searchTimeout) {
        clearTimeout(searchTimeout);
      }
      if (searchInput.trim()) {
        appState.semanticSearch(searchInput.trim());
      }
    }
  }
</script>

<aside class="sidebar">
  <!-- Header -->
  <div class="sidebar-header">
    <div class="header-row">
      <h1 class="logo">
        <span class="logo-icon">▲</span>
        {$_('app.title')}
      </h1>
      <!-- Sync -->
      <button
        onclick={handleSync}
        class="icon-btn {appState.syncing ? 'syncing' : ''}"
        title={$_('actions.refresh')}
        aria-label={$_('actions.refresh')}
        disabled={appState.syncing}
      >
        <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
        </svg>
      </button>
    </div>
    <p class="tagline">Immanentize the Eschaton</p>
    <!-- Navigation -->
    <div class="nav-bar">
      <button onclick={onarticles} class="nav-btn {articlesActive ? 'active' : ''}" title={$_('sidebar.allFeeds')} aria-label={$_('sidebar.allFeeds')}>
        <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z"></path>
        </svg>
      </button>
      <button onclick={onfnord} class="nav-btn {fnordActive ? 'active' : ''}" title={$_('fnordView.title')} aria-label={$_('fnordView.title')}>
        <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"></path>
        </svg>
      </button>
      <button onclick={onnetwork} class="nav-btn {networkActive ? 'active' : ''}" title={$_('network.title')} aria-label={$_('network.title')}>
        <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"></path>
        </svg>
      </button>
      <button onclick={onsettings} class="nav-btn {settingsActive ? 'active' : ''}" title={$_('settings.title')} aria-label={$_('settings.title')}>
        <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
        </svg>
      </button>
    </div>
  </div>

  <!-- Mode Toggle -->
  <div class="mode-toggle">
    <button
      class="toggle-btn {sidebarMode === 'pentacles' ? 'active' : ''}"
      onclick={() => sidebarMode = 'pentacles'}
    >
      <Tooltip termKey="pentacle">{$_('sidebar.title')}</Tooltip>
    </button>
    <button
      class="toggle-btn {sidebarMode === 'sephiroth' ? 'active' : ''}"
      onclick={() => sidebarMode = 'sephiroth'}
    >
      <Tooltip termKey="sephiroth">{$_('sidebar.sephiroth')}</Tooltip>
    </button>
  </div>

  <!-- Semantic Search -->
  <div class="search-box">
    <div class="search-input-wrapper">
      <svg class="search-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
      </svg>
      <input
        type="text"
        bind:value={searchInput}
        oninput={handleSearchInput}
        onkeydown={handleSearchKeydown}
        placeholder={$_('search.placeholder')}
        class="search-input"
        disabled={!appState.ollamaStatus.available}
      />
      {#if appState.searching}
        <span class="search-spinner">↻</span>
      {:else if searchInput}
        <button class="search-clear" onclick={handleClearSearch} title={$_('search.clearSearch')}>×</button>
      {/if}
    </div>
    {#if appState.searchResults.length > 0}
      <div class="search-result-count">
        {$_('search.resultsCount', { values: { count: appState.searchResults.length } })}
      </div>
    {/if}
  </div>

  <!-- Feed List -->
  <div class="feed-list">
    {#if sidebarMode === 'pentacles'}
      <!-- Pentacles List -->
      {#each appState.pentacles as pentacle (pentacle.id)}
        <div
          class="feed-item {appState.selectedPentacleId === pentacle.id ? 'active' : ''}"
          onclick={() => handleSelectPentacle(pentacle.id)}
          onkeydown={(e) => e.key === 'Enter' && handleSelectPentacle(pentacle.id)}
          role="button"
          tabindex="0"
        >
          <span class="feed-name">{pentacle.title || pentacle.url}</span>
          <div class="feed-actions">
            <button
              class="delete-btn"
              onclick={(e) => { e.stopPropagation(); handleDeletePentacle(pentacle.id); }}
              title={$_('actions.delete')}
            >×</button>
          </div>
        </div>
      {/each}

      {#if appState.pentacles.length === 0 && !appState.loading}
        <div class="empty-state">
          {$_('articleList.noArticles')}<br />
          <Tooltip termKey="pentacle">{$_('sidebar.addFeed')}</Tooltip>
        </div>
      {/if}
    {:else}
      <!-- Sephiroth (Categories) List -->
      {#each appState.sephiroth as category (category.id)}
        {#if category.article_count > 0}
          <div
            class="feed-item sephiroth-item {appState.selectedSephirothId === category.id ? 'active' : ''}"
            onclick={() => handleSelectSephiroth(category.id)}
            onkeydown={(e) => e.key === 'Enter' && handleSelectSephiroth(category.id)}
            role="button"
            tabindex="0"
          >
            <span class="feed-name">
              {#if category.icon}
                <span class="category-icon">{category.icon}</span>
              {/if}
              {category.name}
            </span>
          </div>
        {/if}
      {/each}
    {/if}
  </div>

  <!-- Add Feed -->
  <div class="add-feed">
    {#if showAddForm}
      <form onsubmit={handleAddFeed} class="add-form">
        <input
          type="url"
          bind:value={newFeedUrl}
          placeholder={$_('sidebar.addFeedPlaceholder')}
          class="add-input"
        />
        <div class="add-buttons">
          <button type="submit" class="btn-primary">{$_('sidebar.addFeed')}</button>
          <button type="button" class="btn-secondary" onclick={() => (showAddForm = false)}>
            {$_('settings.cancel')}
          </button>
        </div>
      </form>
    {:else}
      <button onclick={() => (showAddForm = true)} class="btn-add">
        + <Tooltip termKey="pentacle">{$_('sidebar.addFeed')}</Tooltip>
      </button>
    {/if}
  </div>

  <!-- Batch Processing -->
  <div class="batch-section">
    <div class="batch-header">
      <Tooltip termKey="discordian"><span class="batch-title">{$_('batch.title')}</span></Tooltip>
      {#if appState.unprocessedCount.with_content > 0 && !appState.batchProcessing}
        <span class="unprocessed-badge">{appState.unprocessedCount.with_content}</span>
      {/if}
    </div>

    {#if appState.batchProcessing}
      <div class="batch-progress">
        {#if appState.batchProgress && appState.batchProgress.current > 0}
          <div class="progress-bar">
            <div
              class="progress-fill"
              style="width: {(appState.batchProgress.current / appState.batchProgress.total) * 100}%"
            ></div>
          </div>
          <div class="progress-text">
            {$_('batch.progress', { values: { current: appState.batchProgress.current, total: appState.batchProgress.total }})}
          </div>
          <div class="progress-title" title={appState.batchProgress.title}>
            {appState.batchProgress.title.length > 30
              ? appState.batchProgress.title.slice(0, 30) + "..."
              : appState.batchProgress.title}
          </div>
          {#if !appState.batchProgress.success && appState.batchProgress.error}
            <div class="progress-error">{appState.batchProgress.error}</div>
          {/if}
        {:else if appState.batchProgress}
          <!-- Initial event received (current=0), show total -->
          <div class="progress-bar">
            <div class="progress-fill indeterminate"></div>
          </div>
          <div class="progress-text">{$_('batch.starting')} ({appState.batchProgress.total})</div>
        {:else}
          <!-- No event yet -->
          <div class="progress-bar">
            <div class="progress-fill indeterminate"></div>
          </div>
          <div class="progress-text">{$_('batch.starting')}</div>
        {/if}
        <button onclick={handleCancelBatch} class="btn-cancel" title={$_('batch.cancel')}>
          {$_('batch.cancel')}
        </button>
      </div>
    {:else if appState.ollamaStatus.available}
      <button
        onclick={handleBatchProcessing}
        class="btn-batch"
        disabled={appState.batchProcessing || appState.unprocessedCount.with_content === 0}
      >
        {#if appState.unprocessedCount.with_content > 0}
          {$_('batch.process')} ({appState.unprocessedCount.with_content})
        {:else}
          {$_('batch.process')}
        {/if}
      </button>
    {:else}
      <div class="batch-unavailable">{$_('batch.noOllama')}</div>
    {/if}
  </div>

  <!-- Embedding Progress (shown when generating embeddings) -->
  {#if appState.embeddingProgress}
    {@const progress = appState.embeddingProgress}
    {@const progressPercent = progress.total > 0 ? Math.round(((progress.total - progress.queue_size) / progress.total) * 100) : 0}
    <div class="embedding-progress">
      <div class="embedding-header">
        <span class="embedding-icon">⚡</span>
        <span class="embedding-label">Embeddings</span>
        {#if progress.is_processing && progress.total > 0}
          <span class="embedding-percent">{progressPercent}%</span>
        {/if}
      </div>
      {#if progress.is_processing}
        <div class="progress-bar">
          {#if progress.total > 0}
            <div class="progress-fill" style="width: {progressPercent}%"></div>
          {:else}
            <div class="progress-fill indeterminate"></div>
          {/if}
        </div>
        <div class="embedding-text">
          {progress.total - progress.queue_size} / {progress.total}
        </div>
      {:else if progress.processed > 0 || progress.failed > 0}
        <div class="embedding-text success">
          ✓ {progress.processed} generated
          {#if progress.failed > 0}
            , {progress.failed} failed
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Loaded Models -->
  {#if loadedModels.length > 0}
    <div class="loaded-models-status">
      <span class="loaded-label">VRAM:</span>
      {#each loadedModels as model, i}
        <span class="loaded-model-chip" title="{model.name} ({model.parameter_size})">
          {model.name.split(':')[0]}
          <span class="vram-size">{formatVram(model.size_vram)}</span>
        </span>
        {#if i < loadedModels.length - 1}<span class="model-sep">·</span>{/if}
      {/each}
    </div>
  {/if}

  <!-- Stats -->
  <div class="stats">
    <div class="stat-row">
      <span><span class="stat-icon concealed">●</span> <Tooltip termKey="concealed">{$_('terminology.concealed.term')}</Tooltip></span>
      <span>{appState.totalUnread}</span>
    </div>
    <div class="stat-row">
      <span><span class="stat-icon illuminated">○</span> <Tooltip termKey="illuminated">{$_('terminology.illuminated.term')}</Tooltip></span>
      <span>{appState.fnords.filter((f) => f.status === "illuminated").length}</span>
    </div>
    <div class="stat-row">
      <span><span class="stat-icon golden">✦</span> <Tooltip termKey="golden_apple">{$_('terminology.golden_apple.term')}</Tooltip></span>
      <span>{appState.fnords.filter((f) => f.status === "golden_apple").length}</span>
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: 16rem;
    background-color: var(--bg-surface);
    border-right: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    height: 100%;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: 1rem;
    border-bottom: 1px solid var(--border-default);
  }

  .header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .logo {
    font-size: 1.125rem;
    font-weight: 700;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
  }

  .logo-icon {
    font-size: 1.25rem;
    color: var(--accent-primary);
  }

  .nav-bar {
    display: flex;
    justify-content: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-muted);
  }

  .nav-btn {
    color: var(--text-muted);
    background: none;
    border: none;
    padding: 0.5rem;
    cursor: pointer;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0.375rem;
  }

  .nav-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-overlay);
  }

  .nav-btn.active {
    color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .nav-btn .icon {
    width: 1.25rem;
    height: 1.25rem;
  }

  .icon-btn {
    color: var(--text-muted);
    background: none;
    border: none;
    padding: 0.25rem;
    cursor: pointer;
    transition: color 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon-btn:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .icon-btn.active {
    color: var(--accent-primary);
  }

  .icon-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .icon-btn .icon {
    width: 1.25rem;
    height: 1.25rem;
  }

  .icon-btn.syncing .icon {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .tagline {
    font-size: 0.75rem;
    color: var(--text-faint);
    margin: 0.25rem 0 0 0;
  }

  .feed-item {
    width: 100%;
    padding: 0.75rem 1rem;
    text-align: left;
    background: none;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: space-between;
    transition: background-color 0.2s;
    color: var(--text-primary);
  }

  .feed-item:hover {
    background-color: var(--bg-overlay);
  }

  .feed-item.active {
    background-color: var(--bg-overlay);
  }

  .feed-name {
    font-size: 0.875rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .feed-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .unread-badge {
    background-color: var(--fnord-color);
    color: var(--text-on-accent);
    font-size: 0.75rem;
    padding: 0.125rem 0.5rem;
    border-radius: 9999px;
    font-weight: 500;
  }

  .unread-badge.small {
    padding: 0.125rem 0.375rem;
    min-width: 1.25rem;
    text-align: center;
  }

  .delete-btn {
    opacity: 0;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    transition: opacity 0.2s, color 0.2s;
  }

  .feed-item:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    color: var(--accent-error);
  }

  .feed-list {
    flex: 1;
    overflow-y: auto;
  }

  .section-header {
    padding: 0.5rem 1rem;
    font-size: 0.75rem;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .mode-toggle {
    display: flex;
    margin: 0.5rem 0.5rem 0.25rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    overflow: hidden;
  }

  .toggle-btn {
    flex: 1;
    padding: 0.375rem 0.5rem;
    font-size: 0.6875rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-btn:hover {
    background-color: var(--bg-overlay);
    color: var(--text-primary);
  }

  .toggle-btn.active {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .sephiroth-item {
    font-size: 0.8125rem;
  }

  .category-icon {
    margin-right: 0.375rem;
  }

  .empty-state {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .add-feed {
    border-top: 1px solid var(--border-default);
    padding: 1rem;
  }

  .add-form {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .add-input {
    width: 100%;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .add-input::placeholder {
    color: var(--text-faint);
  }

  .add-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .add-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .btn-primary, .btn-secondary, .btn-add {
    flex: 1;
    padding: 0.375rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    cursor: pointer;
    transition: background-color 0.2s;
    border: none;
  }

  .btn-primary {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-primary:hover {
    filter: brightness(1.1);
  }

  .btn-secondary {
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
  }

  .btn-secondary:hover {
    background-color: var(--bg-muted);
  }

  .btn-add {
    width: 100%;
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
  }

  .btn-add:hover {
    background-color: var(--bg-muted);
  }

  .stats {
    border-top: 1px solid var(--border-default);
    padding: 1rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.25rem;
  }

  .stat-icon {
    display: inline-block;
    width: 1em;
    text-align: center;
  }

  .stat-icon.concealed { color: var(--fnord-color); }
  .stat-icon.illuminated { color: var(--illuminated-color); }
  .stat-icon.golden { color: var(--golden-apple-color); }

  /* Batch Processing */
  .batch-section {
    border-top: 1px solid var(--border-default);
    padding: 0.75rem 1rem;
  }

  .batch-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .batch-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .unprocessed-badge {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-size: 0.625rem;
    font-weight: 600;
  }

  .btn-batch {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid var(--accent-primary);
    background-color: transparent;
    color: var(--accent-primary);
  }

  .btn-batch:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-batch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .batch-progress {
    font-size: 0.75rem;
  }

  .progress-bar {
    height: 4px;
    background-color: var(--bg-overlay);
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 0.375rem;
  }

  .progress-fill {
    height: 100%;
    background-color: var(--accent-primary);
    transition: width 0.3s ease;
  }

  .progress-fill.indeterminate {
    width: 30%;
    animation: indeterminate 1.5s ease-in-out infinite;
  }

  @keyframes indeterminate {
    0% {
      transform: translateX(-100%);
    }
    50% {
      transform: translateX(233%);
    }
    100% {
      transform: translateX(-100%);
    }
  }

  .progress-text {
    color: var(--text-muted);
    text-align: center;
    margin-bottom: 0.25rem;
  }

  .progress-title {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .progress-error {
    color: var(--accent-error);
    font-size: 0.625rem;
    margin-top: 0.25rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .batch-unavailable {
    font-size: 0.75rem;
    color: var(--text-muted);
    text-align: center;
    padding: 0.5rem;
  }

  /* Embedding progress styles */
  .embedding-progress {
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    border-top: 1px solid var(--border-primary);
    background-color: var(--bg-secondary);
  }

  .embedding-header {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin-bottom: 0.375rem;
  }

  .embedding-icon {
    font-size: 0.875rem;
    color: var(--accent-warning);
  }

  .embedding-label {
    color: var(--text-secondary);
    font-weight: 500;
  }

  .embedding-percent {
    margin-left: auto;
    color: var(--accent-warning);
    font-weight: 600;
    font-size: 0.875rem;
  }

  .embedding-text {
    color: var(--text-muted);
    text-align: center;
  }

  .embedding-text.success {
    color: var(--accent-success);
  }

  .btn-cancel {
    width: 100%;
    padding: 0.375rem 0.5rem;
    margin-top: 0.5rem;
    border-radius: 0.375rem;
    font-size: 0.625rem;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid var(--accent-error);
    background-color: transparent;
    color: var(--accent-error);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .btn-cancel:hover {
    background-color: var(--accent-error);
    color: var(--text-on-accent);
  }

  /* Loaded Models Status */
  .loaded-models-status {
    padding: 0.5rem 1rem;
    border-top: 1px solid var(--border-default);
    font-size: 0.625rem;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
  }

  .loaded-label {
    color: var(--text-muted);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .loaded-model-chip {
    background-color: var(--bg-overlay);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }

  .vram-size {
    color: var(--accent-primary);
    font-weight: 600;
  }

  .model-sep {
    color: var(--text-muted);
  }

  /* Semantic Search */
  .search-box {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border-default);
  }

  .search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 0.5rem;
    width: 1rem;
    height: 1rem;
    color: var(--text-muted);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    padding: 0.375rem 2rem 0.375rem 2rem;
    font-size: 0.75rem;
    color: var(--text-primary);
    transition: border-color 0.2s;
  }

  .search-input::placeholder {
    color: var(--text-faint);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .search-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .search-spinner {
    position: absolute;
    right: 0.5rem;
    color: var(--accent-primary);
    animation: spin 1s linear infinite;
    font-size: 0.875rem;
  }

  .search-clear {
    position: absolute;
    right: 0.375rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
    padding: 0.125rem 0.25rem;
    line-height: 1;
    transition: color 0.2s;
  }

  .search-clear:hover {
    color: var(--text-primary);
  }

  .search-result-count {
    margin-top: 0.375rem;
    font-size: 0.6875rem;
    color: var(--accent-primary);
    text-align: center;
  }
</style>
