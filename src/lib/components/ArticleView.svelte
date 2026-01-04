<script lang="ts">
  import { appState } from "../stores/state.svelte";

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    return date.toLocaleDateString("de-DE", {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function getArticleTypeName(type: string | null): string {
    switch (type) {
      case "news":
        return "Nachricht";
      case "analysis":
        return "Analyse";
      case "opinion":
        return "Meinung";
      case "satire":
        return "Satire";
      case "ad":
        return "Werbung";
      default:
        return "Unbekannt";
    }
  }

  function getBiasLabel(bias: number | null): string {
    if (bias === null) return "Nicht bewertet";
    switch (bias) {
      case -2:
        return "Stark links";
      case -1:
        return "Leicht links";
      case 0:
        return "Neutral";
      case 1:
        return "Leicht rechts";
      case 2:
        return "Stark rechts";
      default:
        return "Unbekannt";
    }
  }

  function getSachlichkeitLabel(s: number | null): string {
    if (s === null) return "Nicht bewertet";
    switch (s) {
      case 0:
        return "Stark emotional";
      case 1:
        return "Emotional";
      case 2:
        return "Gemischt";
      case 3:
        return "Überwiegend sachlich";
      case 4:
        return "Sachlich";
      default:
        return "Unbekannt";
    }
  }

  function openInBrowser() {
    if (appState.selectedFnord) {
      window.open(appState.selectedFnord.url, "_blank");
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "v" && appState.selectedFnord) {
      e.preventDefault();
      openInBrowser();
    } else if (e.key === "o" || e.key === "Enter") {
      // Already handled in ArticleList, but we could expand content here
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex-1 bg-zinc-900 overflow-y-auto">
  {#if appState.selectedFnord}
    {@const fnord = appState.selectedFnord}

    <!-- Article Header -->
    <div class="p-6 border-b border-zinc-800">
      <div class="max-w-3xl mx-auto">
        <!-- Source & Date -->
        <div class="flex items-center gap-2 text-sm text-zinc-500 mb-3">
          <span class="font-medium text-zinc-400">
            {fnord.pentacle_title || "Unknown Source"}
          </span>
          <span>·</span>
          <span>{formatDate(fnord.published_at)}</span>
          {#if fnord.author}
            <span>·</span>
            <span>{fnord.author}</span>
          {/if}
        </div>

        <!-- Title -->
        <h1 class="text-2xl font-bold text-zinc-100 leading-tight mb-4">
          {fnord.title}
        </h1>

        <!-- Actions -->
        <div class="flex items-center gap-3">
          <button
            onclick={() => appState.toggleGoldenApple(fnord.id)}
            class="px-3 py-1.5 rounded text-sm transition-colors {fnord.status ===
            'golden_apple'
              ? 'bg-golden-600 text-white'
              : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700'}"
          >
            🍎 {fnord.status === "golden_apple"
              ? "Golden Apple"
              : "Als Favorit markieren"}
          </button>

          <button
            onclick={openInBrowser}
            class="px-3 py-1.5 rounded text-sm bg-zinc-800 text-zinc-400 hover:bg-zinc-700 transition-colors"
          >
            🔗 Im Browser öffnen
          </button>
        </div>
      </div>
    </div>

    <!-- Greyface Alert -->
    {#if fnord.political_bias !== null || fnord.sachlichkeit !== null || fnord.article_type}
      <div class="p-4 bg-zinc-800/50 border-b border-zinc-800">
        <div class="max-w-3xl mx-auto">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-zinc-400 text-sm font-medium">Greyface Alert</span>
          </div>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            {#if fnord.article_type}
              <div>
                <div class="text-zinc-500 text-xs mb-1">Artikeltyp</div>
                <div class="text-zinc-300">
                  {getArticleTypeName(fnord.article_type)}
                </div>
              </div>
            {/if}

            {#if fnord.political_bias !== null}
              <div>
                <div class="text-zinc-500 text-xs mb-1">Politische Tendenz</div>
                <div class="text-zinc-300">{getBiasLabel(fnord.political_bias)}</div>
              </div>
            {/if}

            {#if fnord.sachlichkeit !== null}
              <div>
                <div class="text-zinc-500 text-xs mb-1">Sachlichkeit</div>
                <div class="text-zinc-300">
                  {getSachlichkeitLabel(fnord.sachlichkeit)}
                </div>
              </div>
            {/if}

            {#if fnord.quality_score !== null}
              <div>
                <div class="text-zinc-500 text-xs mb-1">Quellenqualität</div>
                <div class="text-golden-500">
                  {"★".repeat(fnord.quality_score)}{"☆".repeat(
                    5 - fnord.quality_score
                  )}
                </div>
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/if}

    <!-- Summary (Discordian Analysis) -->
    {#if fnord.summary}
      <div class="p-4 bg-zinc-800/30 border-b border-zinc-800">
        <div class="max-w-3xl mx-auto">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-zinc-400 text-sm font-medium"
              >Discordian Analysis</span
            >
          </div>
          <p class="text-zinc-300 text-sm leading-relaxed">{fnord.summary}</p>
        </div>
      </div>
    {/if}

    <!-- Content -->
    <div class="p-6">
      <div class="max-w-3xl mx-auto">
        <article class="prose prose-invert prose-zinc max-w-none">
          {#if fnord.content_full}
            {@html fnord.content_full}
          {:else if fnord.content_raw}
            <p class="text-zinc-300 leading-relaxed whitespace-pre-wrap">
              {fnord.content_raw}
            </p>
          {:else}
            <p class="text-zinc-500 italic">
              Kein Inhalt verfügbar. Klicke auf "Im Browser öffnen" um den
              vollständigen Artikel zu lesen.
            </p>
          {/if}
        </article>
      </div>
    </div>
  {:else}
    <!-- Empty State -->
    <div
      class="h-full flex flex-col items-center justify-center text-zinc-500 p-8"
    >
      <div class="text-6xl mb-4">▲</div>
      <h2 class="text-xl font-medium mb-2">Wähle einen Fnord</h2>
      <p class="text-sm text-center max-w-md">
        Wähle einen Artikel aus der Liste, um ihn hier anzuzeigen.<br />
        Benutze <kbd class="bg-zinc-700 px-1 rounded">j</kbd> und
        <kbd class="bg-zinc-700 px-1 rounded">k</kbd> zum Navigieren.
      </p>
    </div>
  {/if}
</div>

<style>
  /* Prose styles for article content */
  :global(.prose) {
    color: theme("colors.zinc.300");
  }

  :global(.prose h1, .prose h2, .prose h3, .prose h4) {
    color: theme("colors.zinc.100");
  }

  :global(.prose a) {
    color: theme("colors.blue.400");
  }

  :global(.prose a:hover) {
    color: theme("colors.blue.300");
  }

  :global(.prose strong) {
    color: theme("colors.zinc.100");
  }

  :global(.prose code) {
    background: theme("colors.zinc.800");
    padding: 0.125rem 0.25rem;
    border-radius: 0.25rem;
  }

  :global(.prose pre) {
    background: theme("colors.zinc.800");
  }

  :global(.prose blockquote) {
    border-left-color: theme("colors.zinc.600");
    color: theme("colors.zinc.400");
  }
</style>
