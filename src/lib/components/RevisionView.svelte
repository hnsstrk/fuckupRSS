<script lang="ts">
  import { _ } from 'svelte-i18n';
  import type { Fnord, FnordRevision } from '../stores/state.svelte';
  import { computeWordDiff, diffToHtml, getDiffStats, type DiffSegment } from '../utils/textDiff';

  interface Props {
    fnord: Fnord;
    revisions: FnordRevision[];
  }

  let { fnord, revisions }: Props = $props();

  // Which revision is selected (null = current version)
  let selectedRevisionIndex = $state<number | null>(null);
  let showDiff = $state(true);

  // Get content for the current article
  function getCurrentContent(): string {
    return fnord.content_full || fnord.content_raw || '';
  }

  // Get content for a revision
  function getRevisionContent(rev: FnordRevision): string {
    return rev.content_full || rev.content_raw || '';
  }

  // Get the "newer" content to compare against (what this version changed into)
  function getNewerContent(revIndex: number): string {
    if (revIndex === 0) {
      // Newest revision - compare to current
      return getCurrentContent();
    }
    // Compare to the next newer revision
    return getRevisionContent(revisions[revIndex - 1]);
  }

  // Compute diff segments
  const diffSegments = $derived.by(() => {
    if (!showDiff || selectedRevisionIndex === null) {
      return [] as DiffSegment[];
    }

    const rev = revisions[selectedRevisionIndex];
    const oldContent = getRevisionContent(rev);
    const newContent = getNewerContent(selectedRevisionIndex);

    return computeWordDiff(oldContent, newContent);
  });

  // Get diff HTML
  const diffHtml = $derived(diffToHtml(diffSegments));

  // Get diff stats
  const stats = $derived(getDiffStats(diffSegments));

  // Format date
  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('de-DE', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  // Select a revision
  function selectRevision(index: number | null) {
    selectedRevisionIndex = index;
  }
</script>

<div class="revision-view">
  <!-- Revision Tabs -->
  <div class="revision-tabs">
    <button
      class="revision-tab {selectedRevisionIndex === null ? 'active' : ''}"
      onclick={() => selectRevision(null)}
    >
      {$_('revisions.current') || 'Aktuell'}
    </button>
    {#each revisions as rev, i}
      <button
        class="revision-tab {selectedRevisionIndex === i ? 'active' : ''}"
        onclick={() => selectRevision(i)}
        title={formatDate(rev.revision_at)}
      >
        v{revisions.length - i}
      </button>
    {/each}
  </div>

  <!-- Controls -->
  {#if selectedRevisionIndex !== null}
    <div class="revision-controls">
      <label class="diff-toggle">
        <input type="checkbox" bind:checked={showDiff} />
        <span>{$_('revisions.showChanges') || 'Änderungen anzeigen'}</span>
      </label>

      {#if showDiff && (stats.addedWords > 0 || stats.removedWords > 0)}
        <div class="diff-stats">
          {#if stats.addedWords > 0}
            <span class="stat-added">+{stats.addedWords}</span>
          {/if}
          {#if stats.removedWords > 0}
            <span class="stat-removed">-{stats.removedWords}</span>
          {/if}
        </div>
      {/if}

      <div class="revision-date">
        {formatDate(revisions[selectedRevisionIndex].revision_at)}
      </div>
    </div>
  {/if}

  <!-- Content -->
  <div class="revision-content">
    {#if selectedRevisionIndex === null}
      <!-- Current version -->
      {#if fnord.content_full || fnord.content_raw}
        {@html fnord.content_full || fnord.content_raw}
      {:else}
        <p class="no-content">{$_('revisions.noContent') || 'Kein Inhalt verfügbar'}</p>
      {/if}
    {:else}
      <!-- Revision with optional diff -->
      {#if showDiff && diffSegments.length > 0}
        <div class="diff-content">
          {@html diffHtml}
        </div>
      {:else}
        <!-- Plain revision content -->
        {#if revisions[selectedRevisionIndex].content_full || revisions[selectedRevisionIndex].content_raw}
          {@html revisions[selectedRevisionIndex].content_full || revisions[selectedRevisionIndex].content_raw}
        {:else}
          <p class="no-content">{$_('revisions.noContent') || 'Kein Volltext für diese Revision verfügbar'}</p>
        {/if}
      {/if}
    {/if}
  </div>

  <!-- Title comparison (if changed) -->
  {#if selectedRevisionIndex !== null}
    {@const rev = revisions[selectedRevisionIndex]}
    {@const newerTitle = selectedRevisionIndex === 0 ? fnord.title : revisions[selectedRevisionIndex - 1].title}
    {#if rev.title !== newerTitle}
      <div class="title-change">
        <span class="title-label">{$_('revisions.titleChanged') || 'Titel geändert'}:</span>
        <div class="title-old"><span class="diff-removed">{rev.title}</span></div>
        <div class="title-new"><span class="diff-added">{newerTitle}</span></div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .revision-view {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .revision-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .revision-tab {
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.25rem;
    background-color: var(--bg-surface);
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .revision-tab:hover {
    border-color: var(--accent-primary);
    color: var(--text-primary);
  }

  .revision-tab.active {
    background-color: var(--accent-primary);
    border-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .revision-controls {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
    padding: 0.5rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
  }

  .diff-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .diff-toggle input {
    accent-color: var(--accent-primary);
  }

  .diff-stats {
    display: flex;
    gap: 0.5rem;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .stat-added {
    color: var(--accent-success);
  }

  .stat-removed {
    color: var(--accent-error);
  }

  .revision-date {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .revision-content {
    font-size: 0.9375rem;
    line-height: 1.7;
    color: var(--text-primary);
  }

  .diff-content {
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  /* Diff highlighting - Textmarker style */
  .diff-content :global(.diff-added) {
    background: linear-gradient(180deg, transparent 50%, rgba(166, 227, 161, 0.4) 50%);
    padding: 0 2px;
    border-radius: 2px;
  }

  .diff-content :global(.diff-removed) {
    background-color: rgba(243, 139, 168, 0.25);
    text-decoration: line-through;
    color: var(--text-muted);
    padding: 0 2px;
    border-radius: 2px;
  }

  .no-content {
    color: var(--text-muted);
    font-style: italic;
    text-align: center;
    padding: 2rem;
  }

  .title-change {
    margin-top: 1rem;
    padding: 0.75rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border-left: 3px solid var(--accent-warning);
  }

  .title-label {
    display: block;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    margin-bottom: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .title-old,
  .title-new {
    font-size: 0.875rem;
    padding: 0.25rem 0;
  }

  .title-old :global(.diff-removed) {
    background-color: rgba(243, 139, 168, 0.25);
    text-decoration: line-through;
    padding: 0 4px;
    border-radius: 2px;
  }

  .title-new :global(.diff-added) {
    background: linear-gradient(180deg, transparent 50%, rgba(166, 227, 161, 0.4) 50%);
    padding: 0 4px;
    border-radius: 2px;
  }
</style>
