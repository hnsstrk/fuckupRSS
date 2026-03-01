<script lang="ts">
  import { _ } from "svelte-i18n";
  import { getBiasColor, getSachlichkeitColor, getSachlichkeitIcon } from "$lib/utils/articleFormat";

  let {
    politicalBias,
    sachlichkeit,
    qualityScore,
  }: {
    politicalBias: number | null;
    sachlichkeit: number | null;
    qualityScore: number | null;
  } = $props();

  function getBiasLabel(bias: number | null): string {
    if (bias === null) return $_("articleView.notRated");
    switch (bias) {
      case -2:
        return $_("articleView.biasStrongLeft");
      case -1:
        return $_("articleView.biasLeanLeft");
      case 0:
        return $_("articleView.greyface.biasCenter");
      case 1:
        return $_("articleView.biasLeanRight");
      case 2:
        return $_("articleView.biasStrongRight");
      default:
        return $_("articleView.unknown");
    }
  }

  function getSachlichkeitLabel(s: number | null): string {
    if (s === null) return $_("articleView.notRated");
    switch (s) {
      case 0:
        return $_("articleView.sachHighlyEmotional");
      case 1:
        return $_("articleView.sachEmotional");
      case 2:
        return $_("articleView.sachMixed");
      case 3:
        return $_("articleView.sachMostlyObjective");
      case 4:
        return $_("articleView.sachObjective");
      default:
        return $_("articleView.unknown");
    }
  }

  function getBiasIcon(bias: number | null): string {
    if (bias === null) return "fa-scale-balanced";
    if (bias < 0) return "fa-scale-unbalanced";
    if (bias > 0) return "fa-scale-unbalanced-flip";
    return "fa-scale-balanced";
  }
</script>

{#if politicalBias !== null || sachlichkeit !== null || qualityScore !== null}
  <div class="greyface-section">
    <div class="section-content">
      <div class="greyface-row">
        <div class="greyface-label">
          {$_("articleView.greyface.title")}
        </div>
        <div class="greyface-indicators">
          {#if politicalBias !== null}
            <span
              class="indicator bias-{getBiasColor(politicalBias, 'class')}"
              title="{$_('articleView.greyface.bias')}: {getBiasLabel(politicalBias)}"
            >
              <i class="fa-solid {getBiasIcon(politicalBias)}"></i>
              <span class="indicator-text">{getBiasLabel(politicalBias)}</span>
            </span>
          {/if}
          {#if sachlichkeit !== null}
            <span
              class="indicator sach-{getSachlichkeitColor(sachlichkeit)}"
              title="{$_('articleView.greyface.sachlichkeit')}: {getSachlichkeitLabel(sachlichkeit)}"
            >
              <i class="fa-solid {getSachlichkeitIcon(sachlichkeit)}"></i>
              <span class="indicator-text">{getSachlichkeitLabel(sachlichkeit)}</span>
            </span>
          {/if}
          {#if qualityScore !== null}
            <span class="indicator quality" title={$_("articleView.greyface.quality")}>
              {#each Array(qualityScore) as _, i (i)}<i class="fa-solid fa-star"></i>{/each}{#each Array(5 - qualityScore) as _, i (i)}<i
                  class="fa-regular fa-star"
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
</style>
