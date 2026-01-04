<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { appState } from "../stores/state.svelte";
  import Tooltip from "./Tooltip.svelte";

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    const currentLocale = $locale || 'de';
    return date.toLocaleDateString(currentLocale.startsWith('de') ? "de-DE" : "en-US", {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function getArticleTypeName(type: string | null): string {
    if (!type) return $locale?.startsWith('de') ? "Unbekannt" : "Unknown";
    return $_(`articleType.${type}`);
  }

  function getBiasLabel(bias: number | null): string {
    if (bias === null) return $locale?.startsWith('de') ? "Nicht bewertet" : "Not rated";
    const isGerman = $locale?.startsWith('de');
    switch (bias) {
      case -2:
        return isGerman ? "Stark links" : "Strong left";
      case -1:
        return isGerman ? "Leicht links" : "Lean left";
      case 0:
        return $_('articleView.greyface.biasCenter');
      case 1:
        return isGerman ? "Leicht rechts" : "Lean right";
      case 2:
        return isGerman ? "Stark rechts" : "Strong right";
      default:
        return isGerman ? "Unbekannt" : "Unknown";
    }
  }

  function getSachlichkeitLabel(s: number | null): string {
    if (s === null) return $locale?.startsWith('de') ? "Nicht bewertet" : "Not rated";
    const isGerman = $locale?.startsWith('de');
    switch (s) {
      case 0:
        return isGerman ? "Stark emotional" : "Highly emotional";
      case 1:
        return isGerman ? "Emotional" : "Emotional";
      case 2:
        return isGerman ? "Gemischt" : "Mixed";
      case 3:
        return isGerman ? "Ueberwiegend sachlich" : "Mostly objective";
      case 4:
        return isGerman ? "Sachlich" : "Objective";
      default:
        return isGerman ? "Unbekannt" : "Unknown";
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
            <span>{$_('articleView.by')} {fnord.author}</span>
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
            <Tooltip termKey="golden_apple">
              <span>{fnord.status === "golden_apple"
                ? $_('terminology.golden_apple.term')
                : $_('actions.favorite')}</span>
            </Tooltip>
          </button>

          <button
            onclick={openInBrowser}
            class="px-3 py-1.5 rounded text-sm bg-zinc-800 text-zinc-400 hover:bg-zinc-700 transition-colors"
          >
            {$_('actions.openInBrowser')}
          </button>
        </div>
      </div>
    </div>

    <!-- Greyface Alert -->
    {#if fnord.political_bias !== null || fnord.sachlichkeit !== null || fnord.article_type}
      <div class="p-4 bg-zinc-800/50 border-b border-zinc-800">
        <div class="max-w-3xl mx-auto">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-zinc-400 text-sm font-medium">
              <Tooltip termKey="greyface">{$_('articleView.greyface.title')}</Tooltip>
            </span>
          </div>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            {#if fnord.article_type}
              <div>
                <div class="text-zinc-500 text-xs mb-1">{$locale?.startsWith('de') ? 'Artikeltyp' : 'Article Type'}</div>
                <div class="text-zinc-300">
                  {getArticleTypeName(fnord.article_type)}
                </div>
              </div>
            {/if}

            {#if fnord.political_bias !== null}
              <div>
                <div class="text-zinc-500 text-xs mb-1">{$_('articleView.greyface.bias')}</div>
                <div class="text-zinc-300">{getBiasLabel(fnord.political_bias)}</div>
              </div>
            {/if}

            {#if fnord.sachlichkeit !== null}
              <div>
                <div class="text-zinc-500 text-xs mb-1">{$_('articleView.greyface.sachlichkeit')}</div>
                <div class="text-zinc-300">
                  {getSachlichkeitLabel(fnord.sachlichkeit)}
                </div>
              </div>
            {/if}

            {#if fnord.quality_score !== null}
              <div>
                <div class="text-zinc-500 text-xs mb-1">{$_('articleView.greyface.quality')}</div>
                <div class="text-golden-500">
                  {"\u2605".repeat(fnord.quality_score)}{"\u2606".repeat(
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
            <span class="text-zinc-400 text-sm font-medium">
              <Tooltip termKey="discordian">{$_('terminology.discordian.term')}</Tooltip>
            </span>
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
              {$locale?.startsWith('de')
                ? 'Kein Inhalt verfuegbar. Klicke auf "Im Browser oeffnen" um den vollstaendigen Artikel zu lesen.'
                : 'No content available. Click "Open in browser" to read the full article.'}
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
      <h2 class="text-xl font-medium mb-2">
        <Tooltip termKey="fnord">{$_('articleView.noSelection')}</Tooltip>
      </h2>
      <p class="text-sm text-center max-w-md">
        {$_('articleView.selectArticle')}<br />
        {$locale?.startsWith('de') ? 'Benutze' : 'Use'} <kbd class="bg-zinc-700 px-1 rounded">j</kbd> {$locale?.startsWith('de') ? 'und' : 'and'}
        <kbd class="bg-zinc-700 px-1 rounded">k</kbd> {$locale?.startsWith('de') ? 'zum Navigieren.' : 'to navigate.'}
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
