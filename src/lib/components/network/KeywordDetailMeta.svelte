<script lang="ts">
  import { _ } from "svelte-i18n";
  import { formatChangedDate } from "$lib/utils/articleFormat";

  interface Props {
    articleCount: number;
    firstSeen: string | null;
    lastUsed: string | null;
  }

  let { articleCount, firstSeen, lastUsed }: Props = $props();
</script>

<div class="detail-meta">
  <span class="meta-item">
    <span class="meta-label">{$_("network.articleCount")}:</span>
    <span class="meta-value">{articleCount}</span>
  </span>
  <span class="meta-item">
    <span class="meta-label">{$_("network.firstSeen")}:</span>
    <span class="meta-value">{formatChangedDate(firstSeen)}</span>
  </span>
  <span class="meta-item">
    <span class="meta-label">{$_("network.lastUsed")}:</span>
    <span class="meta-value">{formatChangedDate(lastUsed)}</span>
  </span>
</div>

<style>
  .detail-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 1.5rem;
    padding: 0.75rem;
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
  }

  .meta-item {
    font-size: 0.875rem;
  }

  .meta-label {
    color: var(--text-muted);
  }

  .meta-value {
    color: var(--text-primary);
    font-weight: 500;
  }
</style>
