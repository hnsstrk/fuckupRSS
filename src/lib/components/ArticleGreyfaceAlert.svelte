<script lang="ts">
  import { _ } from "svelte-i18n";
  import {
    getBiasColor,
    getBiasLabel,
    getSachlichkeitColor,
    getSachlichkeitIcon,
    getSachlichkeitLabel,
  } from "$lib/utils/articleFormat";

  let {
    politicalBias,
    sachlichkeit,
    qualityScore,
    articleType = "unknown",
  }: {
    politicalBias: number | null;
    sachlichkeit: number | null;
    qualityScore: number | null;
    articleType?: string;
  } = $props();

  function getArticleTypeColor(type_: string): string {
    switch (type_) {
      case "news":
        return "type-news";
      case "analysis":
        return "type-analysis";
      case "opinion":
        return "type-opinion";
      case "satire":
        return "type-satire";
      case "ad":
        return "type-ad";
      default:
        return "type-unknown";
    }
  }

  function getArticleTypeIcon(type_: string): string {
    switch (type_) {
      case "news":
        return "fa-newspaper";
      case "analysis":
        return "fa-magnifying-glass-chart";
      case "opinion":
        return "fa-comment";
      case "satire":
        return "fa-face-grin-squint";
      case "ad":
        return "fa-bullhorn";
      default:
        return "fa-question";
    }
  }

  function getBiasIcon(bias: number | null): string {
    if (bias === null) return "fa-scale-balanced";
    if (bias < 0) return "fa-scale-unbalanced";
    if (bias > 0) return "fa-scale-unbalanced-flip";
    return "fa-scale-balanced";
  }
</script>

{#if politicalBias !== null || sachlichkeit !== null || qualityScore !== null || (articleType && articleType !== "unknown")}
  <div class="greyface-section">
    <div class="section-content">
      <div class="greyface-row">
        <div class="greyface-label">
          {$_("articleView.greyface.title")}
        </div>
        <div class="greyface-indicators">
          {#if articleType && articleType !== "unknown"}
            <span
              class="indicator {getArticleTypeColor(articleType)}"
              title="{$_('articleType.label')}: {$_(`articleType.${articleType}`)}"
            >
              <i class="fa-solid {getArticleTypeIcon(articleType)}"></i>
              <span class="indicator-text">{$_(`articleType.${articleType}`)}</span>
            </span>
          {/if}
          {#if politicalBias !== null}
            <span
              class="indicator bias-{getBiasColor(politicalBias, 'class')}"
              title="{$_('articleView.greyface.bias')}: {getBiasLabel(politicalBias, "de", $_)}"
            >
              <i class="fa-solid {getBiasIcon(politicalBias)}"></i>
              <span class="indicator-text">{getBiasLabel(politicalBias, "de", $_)}</span>
            </span>
          {/if}
          {#if sachlichkeit !== null}
            <span
              class="indicator sach-{getSachlichkeitColor(sachlichkeit)}"
              title="{$_('articleView.greyface.sachlichkeit')}: {getSachlichkeitLabel(
                sachlichkeit, "de", $_,
              )}"
            >
              <i class="fa-solid {getSachlichkeitIcon(sachlichkeit)}"></i>
              <span class="indicator-text">{getSachlichkeitLabel(sachlichkeit, "de", $_)}</span>
            </span>
          {/if}
          {#if qualityScore !== null}
            <span class="indicator quality" title={$_("articleView.greyface.quality")}>
              {#each Array(qualityScore) as _, i (i)}<i class="fa-solid fa-star"
                ></i>{/each}{#each Array(5 - qualityScore) as _, i (i)}<i class="fa-regular fa-star"
                ></i>{/each}
            </span>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .greyface-section {
    padding: 1rem 1.5rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
  }

  .section-content {
    max-width: 48rem;
    margin: 0 auto;
  }

  .greyface-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .greyface-label {
    color: var(--text-muted);
    font-size: 0.75rem;
    min-width: 5rem;
    padding-top: 0.25rem;
    flex-shrink: 0;
  }

  .greyface-indicators {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    flex: 1;
  }

  .greyface-indicators .indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.8125rem;
    background-color: var(--bg-overlay);
    cursor: default;
  }

  .greyface-indicators .indicator i {
    font-size: 0.875rem;
  }

  .indicator-text {
    color: var(--text-secondary);
  }

  /* Bias colors */
  .indicator.bias-strong-left {
    color: var(--bias-strong-left);
  }
  .indicator.bias-lean-left {
    color: var(--bias-lean-left);
  }
  .indicator.bias-center {
    color: var(--bias-center);
  }
  .indicator.bias-lean-right {
    color: var(--bias-lean-right);
  }
  .indicator.bias-strong-right {
    color: var(--bias-strong-right);
  }
  .indicator.bias-neutral {
    color: var(--text-muted);
  }

  /* Sachlichkeit colors */
  .indicator.sach-emotional {
    color: var(--sach-emotional);
  }
  .indicator.sach-mixed {
    color: var(--sach-mixed);
  }
  .indicator.sach-objective {
    color: var(--sach-objective);
  }
  .indicator.sach-neutral {
    color: var(--text-muted);
  }

  /* Quality stars */
  .indicator.quality {
    color: var(--golden-apple-color);
    gap: 0.125rem;
  }

  /* Article type colors */
  .indicator.type-news {
    color: var(--accent-info, #3b82f6);
  }
  .indicator.type-analysis {
    color: #8b5cf6;
  }
  .indicator.type-opinion {
    color: #f97316;
  }
  .indicator.type-satire {
    color: #eab308;
  }
  .indicator.type-ad {
    color: var(--accent-error, #ef4444);
  }
  .indicator.type-unknown {
    color: var(--text-muted);
  }
</style>
