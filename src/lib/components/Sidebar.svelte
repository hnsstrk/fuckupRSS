<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { appState, toasts, type BatchProgress, type EmbeddingProgress } from "../stores/state.svelte";
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import Tooltip from "./Tooltip.svelte";

  interface Props {
    onsettings?: () => void;
    onnetwork?: () => void;
    onfnord?: () => void;
    onmindfuck?: () => void;
    onarticles?: () => void;
    settingsActive?: boolean;
    networkActive?: boolean;
    fnordActive?: boolean;
    mindfuckActive?: boolean;
    articlesActive?: boolean;
  }

  let { onsettings, onnetwork, onfnord, onmindfuck, onarticles, settingsActive = false, networkActive = false, fnordActive = false, mindfuckActive = false, articlesActive = true }: Props = $props();

  let showAddForm = $state(false);
  let newFeedUrl = $state("");
  let sidebarMode = $state<'pentacles' | 'sephiroth'>('pentacles');
  let searchInput = $state("");
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  let unlisten: UnlistenFn | null = null;
  let unlistenArticlesReset: UnlistenFn | null = null;
  let unlistenEmbedding: UnlistenFn | null = null;
  let maintenanceInterval: ReturnType<typeof setInterval> | null = null;

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

    // Listen for batch progress events
    console.log("Setting up batch-progress event listener...");
    unlisten = await listen<BatchProgress>("batch-progress", (event) => {
      console.log("Batch progress event received:", event.payload);
      console.log("Current batchProgress before update:", appState.batchProgress);
      appState.updateBatchProgress(event.payload);
      console.log("Current batchProgress after update:", appState.batchProgress);
    });
    console.log("Batch-progress listener set up successfully");

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

  function handleSelectAll() {
    appState.selectedView = "all";
    appState.selectedPentacleId = null;
    appState.selectedSephirothId = null;
    appState.loadFnords();
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
        <i class="logo-icon fa-solid fa-eye"></i>
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
        <i class="icon fa-solid fa-rotate"></i>
      </button>
    </div>
    <p class="tagline">Immanentize the Eschaton</p>
    <!-- Navigation -->
    <div class="nav-bar">
      <button onclick={onarticles} class="nav-btn {articlesActive ? 'active' : ''}" title={$_('sidebar.allFeeds')} aria-label={$_('sidebar.allFeeds')}>
        <i class="icon fa-solid fa-newspaper"></i>
      </button>
      <button onclick={onfnord} class="nav-btn {fnordActive ? 'active' : ''}" title={$_('fnordView.title')} aria-label={$_('fnordView.title')}>
        <i class="icon fa-solid fa-clipboard-list"></i>
      </button>
      <button onclick={onnetwork} class="nav-btn {networkActive ? 'active' : ''}" title={$_('network.title')} aria-label={$_('network.title')}>
        <i class="icon fa-solid fa-circle-nodes"></i>
      </button>
      <button onclick={onmindfuck} class="nav-btn {mindfuckActive ? 'active' : ''}" title={$_('mindfuck.title')} aria-label={$_('mindfuck.title')}>
        <i class="icon fa-solid fa-brain"></i>
      </button>
      <button onclick={onsettings} class="nav-btn {settingsActive ? 'active' : ''}" title={$_('settings.title')} aria-label={$_('settings.title')}>
        <i class="icon fa-solid fa-gear"></i>
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
      <i class="search-icon fa-solid fa-magnifying-glass"></i>
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
        <i class="search-spinner fa-solid fa-rotate fa-spin"></i>
      {:else if searchInput}
        <button class="search-clear" onclick={handleClearSearch} title={$_('search.clearSearch')}><i class="fa-solid fa-xmark"></i></button>
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
      <!-- All Articles -->
      <div
        class="feed-item all-feeds {appState.selectedPentacleId === null && appState.selectedSephirothId === null ? 'active' : ''}"
        onclick={handleSelectAll}
        onkeydown={(e) => e.key === 'Enter' && handleSelectAll()}
        role="button"
        tabindex="0"
      >
        <span class="feed-name">{$_('sidebar.allFeeds')}</span>
      </div>

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
            ><i class="fa-solid fa-xmark"></i></button>
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
                <i class="{category.icon} category-icon"></i>
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
    {#if appState.batchProcessing}
      <button onclick={handleCancelBatch} class="btn-batch processing" title={$_('batch.cancel')}>
        <Tooltip termKey="discordian">DISCORDIAN ANALYSIS</Tooltip>
        <i class="cancel-icon fa-solid fa-xmark"></i>
      </button>
    {:else if appState.ollamaStatus.available}
      <button
        onclick={handleBatchProcessing}
        class="btn-batch"
        disabled={appState.batchProcessing || appState.unprocessedCount.with_content === 0}
      >
        <Tooltip termKey="discordian">DISCORDIAN ANALYSIS</Tooltip>
        {#if appState.unprocessedCount.with_content > 0}
          <span class="unprocessed-badge">{appState.unprocessedCount.with_content}</span>
        {/if}
      </button>
    {:else}
      <button class="btn-batch" disabled title={$_('batch.noOllama')}>
        <Tooltip termKey="discordian">DISCORDIAN ANALYSIS</Tooltip>
      </button>
    {/if}
  </div>

  <!-- Stats -->
  <div class="stats">
    <div class="stat-row">
      <span><i class="stat-icon fa-solid fa-eye-slash concealed"></i> <Tooltip termKey="concealed">{$_('terminology.concealed.term')}</Tooltip></span>
      <span>{appState.totalUnread}</span>
    </div>
    <div class="stat-row">
      <span><i class="stat-icon fa-solid fa-check illuminated"></i> <Tooltip termKey="illuminated">{$_('terminology.illuminated.term')}</Tooltip></span>
      <span>{appState.fnords.filter((f) => f.status === "illuminated").length}</span>
    </div>
    <div class="stat-row">
      <span><i class="stat-icon fa-solid fa-apple-whole golden"></i> <Tooltip termKey="golden_apple">{$_('terminology.golden_apple.term')}</Tooltip></span>
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

  .all-feeds {
    border-bottom: 1px solid var(--border-default);
    margin-bottom: 0.25rem;
  }

  .all-feeds.active {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .feed-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
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
    width: 1.25rem;
    margin-right: 0.375rem;
    text-align: center;
    display: inline-block;
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

  .btn-batch {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid var(--accent-primary);
    background-color: transparent;
    color: var(--accent-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .btn-batch:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-batch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-batch.processing {
    border-color: var(--accent-warning);
    color: var(--accent-warning);
    animation: pulse-border 1.5s ease-in-out infinite;
  }

  .btn-batch.processing:hover {
    background-color: var(--accent-error);
    border-color: var(--accent-error);
    color: var(--text-on-accent);
  }

  @keyframes pulse-border {
    0%, 100% { opacity: 0.7; }
    50% { opacity: 1; }
  }

  .unprocessed-badge {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-size: 0.625rem;
    font-weight: 600;
  }

  .btn-batch:hover .unprocessed-badge {
    background-color: var(--text-on-accent);
    color: var(--accent-primary);
  }

  .cancel-icon {
    font-size: 1rem;
    font-weight: 400;
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
