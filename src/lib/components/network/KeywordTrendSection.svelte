<script lang="ts">
  import { _ } from "svelte-i18n";
  import KeywordTrendChart from "../KeywordTrendChart.svelte";

  interface CooccurringKeyword {
    id: number;
    name: string;
    cooccurrence_count: number;
  }

  interface Props {
    keywordId: number;
    keywordName: string;
    cooccurringKeywords: CooccurringKeyword[];
    onKeywordSelect: (id: number) => void;
    onDaysChange: (days: number) => void;
  }

  let { keywordId, keywordName, cooccurringKeywords, onKeywordSelect, onDaysChange }: Props =
    $props();
</script>

<div class="detail-section">
  <h4 class="section-title">{$_("network.trendComparison")}</h4>
  <KeywordTrendChart
    {keywordId}
    {keywordName}
    neighborIds={cooccurringKeywords.slice(0, 4).map((k) => k.id)}
    ondayschange={onDaysChange}
  />
  {#if cooccurringKeywords.length > 0}
    <div class="neighbor-legend">
      <span class="legend-label">{$_("network.comparedWith")}:</span>
      <!-- Top 4 with colors matching the chart -->
      <div class="colored-neighbors">
        {#each cooccurringKeywords.slice(0, 4) as coKw, idx (coKw.id)}
          <button
            class="neighbor-tag neighbor-tag-colored neighbor-color-{idx + 1}"
            onclick={() => onKeywordSelect(coKw.id)}
            title="{coKw.cooccurrence_count} {$_('network.articleCount')}"
          >
            {coKw.name}
          </button>
        {/each}
      </div>
      <!-- Remaining keywords in neutral style -->
      {#if cooccurringKeywords.length > 4}
        <div class="neutral-neighbors">
          {#each cooccurringKeywords.slice(4) as coKw (coKw.id)}
            <button
              class="neighbor-tag neighbor-tag-neutral"
              onclick={() => onKeywordSelect(coKw.id)}
              title="{coKw.cooccurrence_count} {$_('network.articleCount')}"
            >
              {coKw.name}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .detail-section {
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  /* Neighbor Legend */
  .neighbor-legend {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding: 0.5rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }

  .legend-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .colored-neighbors {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .neutral-neighbors {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    padding-top: 0.375rem;
    border-top: 1px solid var(--border-muted);
  }

  .neighbor-tag {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    border-radius: 0.25rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  /* Colored neighbor tags using theme category colors */
  .neighbor-tag-colored {
    font-weight: 500;
  }

  .neighbor-color-1 {
    background-color: var(--category-1-bg);
    border: 1px solid var(--category-1-border);
    color: var(--category-1);
  }

  .neighbor-color-1:hover {
    background-color: var(--category-1);
    color: var(--text-on-accent);
  }

  .neighbor-color-2 {
    background-color: var(--category-2-bg);
    border: 1px solid var(--category-2-border);
    color: var(--category-2);
  }

  .neighbor-color-2:hover {
    background-color: var(--category-2);
    color: var(--text-on-accent);
  }

  .neighbor-color-3 {
    background-color: var(--category-3-bg);
    border: 1px solid var(--category-3-border);
    color: var(--category-3);
  }

  .neighbor-color-3:hover {
    background-color: var(--category-3);
    color: var(--text-on-accent);
  }

  .neighbor-color-4 {
    background-color: var(--category-4-bg);
    border: 1px solid var(--category-4-border);
    color: var(--category-4);
  }

  .neighbor-color-4:hover {
    background-color: var(--category-4);
    color: var(--text-on-accent);
  }

  /* Neutral neighbor tags */
  .neighbor-tag-neutral {
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    color: var(--text-secondary);
    font-size: 0.6875rem;
  }

  .neighbor-tag-neutral:hover {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }
</style>
