<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { appState } from "../stores/state.svelte";
  import { onMount } from "svelte";
  import Tooltip from "./Tooltip.svelte";

  interface Props {
    onsettings?: () => void;
  }

  let { onsettings }: Props = $props();

  let showAddForm = $state(false);
  let newFeedUrl = $state("");

  onMount(async () => {
    await appState.loadPentacles();
    await appState.loadFnords();
    // Auto-sync feeds on startup
    appState.syncAllFeeds();
    // Check Ollama availability
    appState.checkOllama();
  });

  function handleSync() {
    appState.syncAllFeeds();
  }

  function handleAddFeed(e: Event) {
    e.preventDefault();
    if (newFeedUrl.trim()) {
      appState.addPentacle(newFeedUrl.trim());
      newFeedUrl = "";
      showAddForm = false;
    }
  }

  function handleSelectAll() {
    appState.selectPentacle(null);
  }

  function handleSelectPentacle(id: number) {
    appState.selectPentacle(id);
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
      <div class="header-actions">
        <button
          onclick={handleSync}
          class="icon-btn {appState.syncing ? 'syncing' : ''}"
          title={$_('actions.refresh')}
          disabled={appState.syncing}
        >
          <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
        </button>
        {#if onsettings}
          <button onclick={onsettings} class="icon-btn" title={$_('settings.title')}>
            <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
            </svg>
          </button>
        {/if}
      </div>
    </div>
    <p class="tagline">Immanentize the Eschaton</p>
  </div>

  <!-- All Feeds -->
  <button
    class="feed-item {appState.selectedPentacleId === null ? 'active' : ''}"
    onclick={handleSelectAll}
  >
    <span class="feed-name">
      {$_('sidebar.allFeeds')} (<Tooltip termKey="fnord"><span class="text-fnord">{$_('terminology.fnord.term')}</span></Tooltip>)
    </span>
    {#if appState.totalUnread > 0}
      <span class="unread-badge">{appState.totalUnread}</span>
    {/if}
  </button>

  <!-- Pentacles List -->
  <div class="feed-list">
    <div class="section-header">
      <Tooltip termKey="pentacle">{$_('sidebar.title')}</Tooltip>
    </div>

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
          {#if pentacle.unread_count > 0}
            <span class="unread-badge small">{pentacle.unread_count}</span>
          {/if}
          <button
            class="delete-btn"
            onclick={(e) => { e.stopPropagation(); appState.deletePentacle(pentacle.id); }}
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

  <!-- Stats -->
  <div class="stats">
    <div class="stat-row">
      <span>● <Tooltip termKey="fnord">{$_('terminology.fnord.term')}</Tooltip></span>
      <span>{appState.totalUnread}</span>
    </div>
    <div class="stat-row">
      <span>○ <Tooltip termKey="illuminated">{$_('terminology.illuminated.term')}</Tooltip></span>
      <span>{appState.fnords.filter((f) => f.status === "illuminated").length}</span>
    </div>
    <div class="stat-row">
      <span><Tooltip termKey="golden_apple">{$_('terminology.golden_apple.term')}</Tooltip></span>
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

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
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

  .text-fnord {
    color: var(--fnord-color);
  }
</style>
