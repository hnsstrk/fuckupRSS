<script lang="ts">
  import { appState } from "../stores/state.svelte";
  import { onMount } from "svelte";

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

  function getStatusIcon(status: string): string {
    switch (status) {
      case "fnord":
        return "●";
      case "illuminated":
        return "○";
      case "golden_apple":
        return "🍎";
      default:
        return "○";
    }
  }
</script>

<aside
  class="w-64 bg-zinc-800 border-r border-zinc-700 flex flex-col h-full shrink-0"
>
  <!-- Header -->
  <div class="p-4 border-b border-zinc-700">
    <h1 class="text-lg font-bold text-zinc-100 flex items-center gap-2">
      <span class="text-xl">▲</span>
      fuckupRSS
    </h1>
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
    <span class="font-medium">All Fnords</span>
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
      Pentacles
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
            title="Delete Pentacle"
          >
            ×
          </button>
        </div>
      </div>
    {/each}

    {#if appState.pentacles.length === 0 && !appState.loading}
      <div class="px-4 py-8 text-center text-zinc-500 text-sm">
        No feeds yet.<br />Add your first Pentacle below.
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
          placeholder="Feed URL..."
          class="w-full bg-zinc-700 border border-zinc-600 rounded px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-zinc-500"
          autofocus
        />
        <div class="flex gap-2">
          <button
            type="submit"
            class="flex-1 bg-zinc-600 hover:bg-zinc-500 text-white text-sm py-1.5 rounded transition-colors"
          >
            Add
          </button>
          <button
            type="button"
            onclick={() => (showAddForm = false)}
            class="flex-1 bg-zinc-700 hover:bg-zinc-600 text-zinc-300 text-sm py-1.5 rounded transition-colors"
          >
            Cancel
          </button>
        </div>
      </form>
    {:else}
      <button
        onclick={() => (showAddForm = true)}
        class="w-full bg-zinc-700 hover:bg-zinc-600 text-zinc-300 text-sm py-2 rounded transition-colors"
      >
        + Add Pentacle
      </button>
    {/if}
  </div>

  <!-- Stats -->
  <div class="border-t border-zinc-700 p-4 text-xs text-zinc-500">
    <div class="flex justify-between">
      <span>● Fnords</span>
      <span>{appState.totalUnread}</span>
    </div>
    <div class="flex justify-between">
      <span>○ Illuminated</span>
      <span
        >{appState.fnords.filter((f) => f.status === "illuminated").length}</span
      >
    </div>
    <div class="flex justify-between">
      <span>🍎 Golden Apple</span>
      <span
        >{appState.fnords.filter((f) => f.status === "golden_apple").length}</span
      >
    </div>
  </div>
</aside>
