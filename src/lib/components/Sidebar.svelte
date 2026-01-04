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

  onMount(() => {
    appState.loadPentacles();
    appState.loadFnords();
  });

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

<aside
  class="w-64 bg-zinc-800 border-r border-zinc-700 flex flex-col h-full shrink-0"
>
  <!-- Header -->
  <div class="p-4 border-b border-zinc-700">
    <div class="flex items-center justify-between">
      <h1 class="text-lg font-bold text-zinc-100 flex items-center gap-2">
        <span class="text-xl">▲</span>
        {$_('app.title')}
      </h1>
      {#if onsettings}
        <button
          onclick={onsettings}
          class="text-zinc-400 hover:text-zinc-100 transition-colors p-1"
          title={$_('settings.title')}
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
          </svg>
        </button>
      {/if}
    </div>
    <p class="text-xs text-zinc-500 mt-1">Immanentize the Eschaton</p>
  </div>

  <!-- All Feeds -->
  <button
    class="px-4 py-3 text-left hover:bg-zinc-700 transition-colors flex items-center justify-between {appState.selectedPentacleId ===
    null
      ? 'bg-zinc-700'
      : ''}"
    onclick={handleSelectAll}
  >
    <span class="font-medium">
      {$_('sidebar.allFeeds')} (<Tooltip termKey="fnord"><span class="text-fnord-400">{$_('terminology.fnord.term')}</span></Tooltip>)
    </span>
    {#if appState.totalUnread > 0}
      <span
        class="bg-fnord-600 text-white text-xs px-2 py-0.5 rounded-full font-medium"
      >
        {appState.totalUnread}
      </span>
    {/if}
  </button>

  <!-- Pentacles List -->
  <div class="flex-1 overflow-y-auto">
    <div class="px-4 py-2 text-xs text-zinc-500 uppercase tracking-wide">
      <Tooltip termKey="pentacle">{$_('sidebar.title')}</Tooltip>
    </div>

    {#each appState.pentacles as pentacle (pentacle.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="w-full px-4 py-2 text-left hover:bg-zinc-700 transition-colors flex items-center justify-between group cursor-pointer {appState.selectedPentacleId ===
        pentacle.id
          ? 'bg-zinc-700'
          : ''}"
        onclick={() => handleSelectPentacle(pentacle.id)}
        onkeydown={(e) => e.key === 'Enter' && handleSelectPentacle(pentacle.id)}
        role="button"
        tabindex="0"
      >
        <span class="truncate text-sm">
          {pentacle.title || pentacle.url}
        </span>
        <div class="flex items-center gap-2">
          {#if pentacle.unread_count > 0}
            <span
              class="bg-fnord-600 text-white text-xs px-1.5 py-0.5 rounded-full font-medium min-w-[20px] text-center"
            >
              {pentacle.unread_count}
            </span>
          {/if}
          <button
            class="opacity-0 group-hover:opacity-100 text-zinc-500 hover:text-red-400 transition-opacity"
            onclick={(e) => {
              e.stopPropagation();
              appState.deletePentacle(pentacle.id);
            }}
            title={$_('actions.delete')}
          >
            ×
          </button>
        </div>
      </div>
    {/each}

    {#if appState.pentacles.length === 0 && !appState.loading}
      <div class="px-4 py-8 text-center text-zinc-500 text-sm">
        {$_('articleList.noArticles')}<br />
        <Tooltip termKey="pentacle">{$_('sidebar.addFeed')}</Tooltip>
      </div>
    {/if}
  </div>

  <!-- Add Feed -->
  <div class="border-t border-zinc-700 p-4">
    {#if showAddForm}
      <form onsubmit={handleAddFeed} class="space-y-2">
        <input
          type="url"
          bind:value={newFeedUrl}
          placeholder={$_('sidebar.addFeedPlaceholder')}
          class="w-full bg-zinc-700 border border-zinc-600 rounded px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-zinc-500"
          autofocus
        />
        <div class="flex gap-2">
          <button
            type="submit"
            class="flex-1 bg-zinc-600 hover:bg-zinc-500 text-white text-sm py-1.5 rounded transition-colors"
          >
            {$_('sidebar.addFeed')}
          </button>
          <button
            type="button"
            onclick={() => (showAddForm = false)}
            class="flex-1 bg-zinc-700 hover:bg-zinc-600 text-zinc-300 text-sm py-1.5 rounded transition-colors"
          >
            {$_('settings.cancel')}
          </button>
        </div>
      </form>
    {:else}
      <button
        onclick={() => (showAddForm = true)}
        class="w-full bg-zinc-700 hover:bg-zinc-600 text-zinc-300 text-sm py-2 rounded transition-colors"
      >
        + <Tooltip termKey="pentacle">{$_('sidebar.addFeed')}</Tooltip>
      </button>
    {/if}
  </div>

  <!-- Stats -->
  <div class="border-t border-zinc-700 p-4 text-xs text-zinc-500">
    <div class="flex justify-between">
      <span>● <Tooltip termKey="fnord">{$_('terminology.fnord.term')}</Tooltip></span>
      <span>{appState.totalUnread}</span>
    </div>
    <div class="flex justify-between">
      <span>○ <Tooltip termKey="illuminated">{$_('terminology.illuminated.term')}</Tooltip></span>
      <span
        >{appState.fnords.filter((f) => f.status === "illuminated").length}</span
      >
    </div>
    <div class="flex justify-between">
      <span><Tooltip termKey="golden_apple">{$_('terminology.golden_apple.term')}</Tooltip></span>
      <span
        >{appState.fnords.filter((f) => f.status === "golden_apple").length}</span
      >
    </div>
  </div>
</aside>
