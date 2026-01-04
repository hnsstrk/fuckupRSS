<script lang="ts">
  import { appState, type Fnord } from "../stores/state.svelte";

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

  function getStatusClass(status: string): string {
    switch (status) {
      case "fnord":
        return "text-fnord-500";
      case "illuminated":
        return "text-illuminated-500";
      case "golden_apple":
        return "text-golden-500";
      default:
        return "text-zinc-500";
    }
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffMins < 60) {
      return `vor ${diffMins} Min`;
    } else if (diffHours < 24) {
      return `vor ${diffHours} Std`;
    } else if (diffDays < 7) {
      return `vor ${diffDays} Tagen`;
    } else {
      return date.toLocaleDateString("de-DE", {
        day: "numeric",
        month: "short",
      });
    }
  }

  function getArticleTypeIcon(type: string | null): string {
    switch (type) {
      case "news":
        return "📰";
      case "analysis":
        return "🔍";
      case "opinion":
        return "💭";
      case "satire":
        return "🎭";
      case "ad":
        return "📢";
      default:
        return "❓";
    }
  }

  function handleSelectFnord(id: number) {
    appState.selectFnord(id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "j") {
      e.preventDefault();
      appState.selectNextFnord();
    } else if (e.key === "k") {
      e.preventDefault();
      appState.selectPrevFnord();
    } else if (e.key === "s" && appState.selectedFnordId) {
      e.preventDefault();
      appState.toggleGoldenApple(appState.selectedFnordId);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="w-80 bg-zinc-850 border-r border-zinc-700 flex flex-col h-full shrink-0 overflow-hidden"
  style="background-color: rgb(32, 32, 35);"
>
  <!-- Header -->
  <div class="p-4 border-b border-zinc-700">
    <h2 class="font-semibold text-zinc-100">
      {#if appState.selectedPentacle}
        {appState.selectedPentacle.title || "Feed"}
      {:else}
        All Fnords
      {/if}
    </h2>
    <p class="text-xs text-zinc-500 mt-1">
      {appState.fnords.length} articles
    </p>
  </div>

  <!-- Article List -->
  <div class="flex-1 overflow-y-auto">
    {#each appState.fnords as fnord (fnord.id)}
      <button
        class="w-full p-4 text-left border-b border-zinc-700/50 hover:bg-zinc-700/50 transition-colors {appState.selectedFnordId ===
        fnord.id
          ? 'bg-zinc-700'
          : ''}"
        onclick={() => handleSelectFnord(fnord.id)}
      >
        <div class="flex items-start gap-3">
          <!-- Status Icon -->
          <span class="text-lg mt-0.5 {getStatusClass(fnord.status)}">
            {getStatusIcon(fnord.status)}
          </span>

          <div class="flex-1 min-w-0">
            <!-- Title -->
            <h3
              class="font-medium text-sm leading-tight {fnord.status === 'fnord'
                ? 'text-zinc-100'
                : 'text-zinc-400'} line-clamp-2"
            >
              {fnord.title}
            </h3>

            <!-- Meta -->
            <div class="flex items-center gap-2 mt-2 text-xs text-zinc-500">
              <span class="truncate">
                {fnord.pentacle_title || "Unknown"}
              </span>
              <span>·</span>
              <span>{formatDate(fnord.published_at)}</span>
            </div>

            <!-- Greyface Info -->
            {#if fnord.article_type || fnord.quality_score}
              <div class="flex items-center gap-2 mt-1.5 text-xs">
                {#if fnord.article_type}
                  <span title="Article type">
                    {getArticleTypeIcon(fnord.article_type)}
                  </span>
                {/if}
                {#if fnord.quality_score}
                  <span class="text-zinc-500" title="Quality score">
                    {"★".repeat(fnord.quality_score)}{"☆".repeat(
                      5 - fnord.quality_score
                    )}
                  </span>
                {/if}
                {#if fnord.political_bias !== null && fnord.political_bias !== 0}
                  <span
                    class="text-zinc-500"
                    title="Political bias: {fnord.political_bias}"
                  >
                    {fnord.political_bias < 0 ? "←" : "→"}
                  </span>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      </button>
    {/each}

    {#if appState.fnords.length === 0 && !appState.loading}
      <div class="p-8 text-center text-zinc-500 text-sm">
        No articles found.<br />
        {#if appState.pentacles.length === 0}
          Add a Pentacle to get started.
        {:else}
          Select a feed or refresh.
        {/if}
      </div>
    {/if}

    {#if appState.loading}
      <div class="p-8 text-center text-zinc-500 text-sm">Loading...</div>
    {/if}
  </div>

  <!-- Keyboard Hints -->
  <div
    class="border-t border-zinc-700 p-2 text-xs text-zinc-600 flex justify-center gap-4"
  >
    <span><kbd class="bg-zinc-700 px-1 rounded">j</kbd> next</span>
    <span><kbd class="bg-zinc-700 px-1 rounded">k</kbd> prev</span>
    <span><kbd class="bg-zinc-700 px-1 rounded">s</kbd> star</span>
  </div>
</div>

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
