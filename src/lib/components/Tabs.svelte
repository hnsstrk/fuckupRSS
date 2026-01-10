<script lang="ts" module>
  export interface Tab {
    id: string;
    label: string;
    badge?: number;
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Array of tab definitions */
    tabs: Tab[];
    /** Currently active tab ID (bindable) */
    activeTab?: string;
    /** Callback when tab changes */
    onchange?: (tabId: string) => void;
    /** Optional snippet for custom tab rendering */
    tab?: Snippet<[Tab, boolean]>;
  }

  let {
    tabs,
    activeTab = $bindable(tabs[0]?.id ?? ''),
    onchange,
    tab: customTab
  }: Props = $props();

  function handleTabClick(tabId: string) {
    if (tabId !== activeTab) {
      activeTab = tabId;
      onchange?.(tabId);
    }
  }

  function handleKeyDown(event: KeyboardEvent, tabId: string) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleTabClick(tabId);
    }
  }
</script>

<div class="tabs" role="tablist">
  {#each tabs as tabItem (tabItem.id)}
    {#if customTab}
      {@render customTab(tabItem, activeTab === tabItem.id)}
    {:else}
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === tabItem.id}
        class="tab {activeTab === tabItem.id ? 'active' : ''}"
        onclick={() => handleTabClick(tabItem.id)}
        onkeydown={(e) => handleKeyDown(e, tabItem.id)}
      >
        <span class="tab-label">{tabItem.label}</span>
        {#if tabItem.badge !== undefined && tabItem.badge > 0}
          <span class="tab-badge">{tabItem.badge}</span>
        {/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: 0.25rem;
    border-bottom: 1px solid var(--border-default);
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 0.875rem;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: all 0.2s;
    border-radius: 0.25rem 0.25rem 0 0;
  }

  .tab:hover {
    color: var(--text-primary);
    background-color: var(--bg-overlay);
  }

  .tab:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .tab.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .tab-label {
    white-space: nowrap;
  }

  .tab-badge {
    padding: 0.125rem 0.375rem;
    font-size: 0.6875rem;
    font-weight: 600;
    background-color: var(--accent-error);
    color: white;
    border-radius: 0.75rem;
    min-width: 1.25rem;
    text-align: center;
  }
</style>
