<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { networkStore } from "$lib/stores/state.svelte";
  import { navigationStore } from "$lib/stores/navigation.svelte";
  import { briefingStore } from "$lib/stores/briefings.svelte";
  import BriefingCard from "./BriefingCard.svelte";

  let briefingContentRef = $state<HTMLDivElement | null>(null);

  onMount(async () => {
    briefingContentRef?.scrollTo({ top: 0 });
    await briefingStore.ensureLoaded();
  });

  function navigateToArticle(fnordId: number) {
    navigationStore.navigateToArticle(fnordId);
  }

  function navigateToKeyword(keywordName: string) {
    const keyword = networkStore.keywords?.find(
      (k: { name: string }) => k.name.toLowerCase() === keywordName.toLowerCase(),
    );
    if (keyword) {
      navigationStore.navigateToNetwork(keyword.id);
    } else {
      navigationStore.navigateTo("network");
    }
  }
</script>

<div class="briefing-view">
  <div class="briefing-header">
    <div class="header-top">
      <h2 class="view-title">
        <i class="fa-solid fa-file-lines nav-icon"></i>
        {$_("briefing.title")}
      </h2>
      <div class="header-actions">
        <button
          class="btn btn-primary"
          onclick={() => briefingStore.generateBriefing("daily")}
          disabled={briefingStore.generating}
        >
          <i class="fa-solid fa-sun"></i>
          {$_("briefing.daily")}
        </button>
        <button
          class="btn btn-primary"
          onclick={() => briefingStore.generateBriefing("weekly")}
          disabled={briefingStore.generating}
        >
          <i class="fa-solid fa-calendar-week"></i>
          {$_("briefing.weekly")}
        </button>
      </div>
    </div>
  </div>

  <div class="briefing-content" bind:this={briefingContentRef}>
    {#if briefingStore.generating}
      <div class="generating-overlay">
        <div class="generating-spinner">
          <i class="fa-solid fa-spinner fa-spin"></i>
          <span>{$_("briefing.generating")}</span>
        </div>
      </div>
    {/if}

    {#if briefingStore.error}
      <div class="error-banner">
        <i class="fa-solid fa-triangle-exclamation"></i>
        {briefingStore.error}
      </div>
    {/if}

    {#if briefingStore.loading}
      <div class="loading-state">
        <i class="fa-solid fa-spinner fa-spin"></i>
      </div>
    {:else if briefingStore.briefings.length === 0}
      <div class="empty-state">
        <i class="fa-solid fa-file-lines empty-icon"></i>
        <p>{$_("briefing.empty")}</p>
      </div>
    {:else}
      <div class="briefing-list">
        {#each briefingStore.briefings as briefing (briefing.id)}
          <BriefingCard
            {briefing}
            ondelete={(id) => briefingStore.deleteBriefing(id)}
            onarticlenavigate={navigateToArticle}
            onkeywordnavigate={navigateToKeyword}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .briefing-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    background-color: var(--bg-surface);
    overflow: hidden;
  }

  .briefing-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .view-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
  }

  .nav-icon {
    color: var(--accent-primary);
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 0.875rem;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: filter 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background-color: var(--accent-primary);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .briefing-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.5rem;
    position: relative;
  }

  .generating-overlay {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    margin-bottom: 1rem;
    background-color: var(--bg-elevated);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
  }

  .generating-spinner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.9375rem;
    color: var(--accent-primary);
  }

  .generating-spinner i {
    font-size: 1.25rem;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
    background-color: color-mix(in srgb, var(--accent-error) 10%, transparent);
    border: 1px solid var(--accent-error);
    border-radius: 0.375rem;
    color: var(--accent-error);
    font-size: 0.875rem;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: var(--text-muted);
    font-size: 1.5rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    color: var(--text-muted);
    gap: 1rem;
  }

  .empty-icon {
    font-size: 3rem;
    opacity: 0.4;
  }

  .empty-state p {
    font-size: 0.9375rem;
    text-align: center;
    max-width: 30rem;
  }

  .briefing-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
</style>
