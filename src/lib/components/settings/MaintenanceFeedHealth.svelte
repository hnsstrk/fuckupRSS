<script lang="ts">
  import { _, locale } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { formatDateTimeShort } from "$lib/utils/articleFormat";
  import { createLogger } from "$lib/logger";

  const log = createLogger("MaintenanceFeedHealth");

  let {
    maintenanceRunning,
  }: {
    maintenanceRunning: string | null;
  } = $props();

  interface FeedHealthEntry {
    pentacle_id: number;
    title: string | null;
    url: string;
    health_status: "ok" | "stale" | "broken";
    last_successful_fetch: string | null;
    last_new_article: string | null;
    consecutive_empty_syncs: number;
    error_count: number;
    last_error: string | null;
  }

  let feeds = $state<FeedHealthEntry[]>([]);
  let loading = $state(false);
  let loadError = $state<string | null>(null);

  const allHealthy = $derived(feeds.length > 0 && feeds.every((f) => f.health_status === "ok"));

  async function loadFeedHealth() {
    loading = true;
    loadError = null;
    try {
      feeds = await invoke<FeedHealthEntry[]>("get_feed_health");
    } catch (e) {
      log.error("Failed to load feed health:", e);
      loadError = `${e}`;
    } finally {
      loading = false;
    }
  }

  async function resetHealth(pentacleId: number) {
    try {
      await invoke("reset_feed_health", { pentacleId });
      await loadFeedHealth();
    } catch (e) {
      log.error("Failed to reset feed health:", e);
      loadError = `${e}`;
    }
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return $_("settings.maintenance.feedHealth.never");
    return formatDateTimeShort(dateStr, $locale ?? "de");
  }

  function statusIcon(status: FeedHealthEntry["health_status"]): string {
    switch (status) {
      case "stale":
        return "fa-solid fa-clock";
      case "broken":
        return "fa-solid fa-triangle-exclamation";
      default:
        return "fa-solid fa-check-circle";
    }
  }

  function statusLabel(status: FeedHealthEntry["health_status"]): string {
    switch (status) {
      case "stale":
        return $_("settings.maintenance.feedHealth.statusStale");
      case "broken":
        return $_("settings.maintenance.feedHealth.statusBroken");
      default:
        return $_("settings.maintenance.feedHealth.statusOk");
    }
  }

  onMount(loadFeedHealth);
</script>

<h3 style="margin-top: 1.5rem;">
  {$_("settings.maintenance.feedHealth.title")}
</h3>

<div class="maintenance-actions">
  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.feedHealth.title")}</span>
      <p class="action-desc">{$_("settings.maintenance.feedHealth.description")}</p>
    </div>
    <button
      type="button"
      class="btn-action"
      onclick={loadFeedHealth}
      disabled={loading || maintenanceRunning !== null}
    >
      {#if loading}
        <i class="fa-solid fa-spinner fa-spin"></i>
        {$_("settings.maintenance.feedHealth.loading")}
      {:else}
        <i class="fa-solid fa-rotate"></i>
        {$_("settings.maintenance.feedHealth.refresh")}
      {/if}
    </button>
  </div>

  {#if loadError}
    <div class="feed-health-error">
      <i class="fa-solid fa-triangle-exclamation"></i>
      {loadError}
    </div>
  {/if}

  {#if allHealthy}
    <div class="orphan-result success">
      <i class="fa-solid fa-check-circle"></i>
      {$_("settings.maintenance.feedHealth.allHealthy")}
    </div>
  {/if}

  {#each feeds as feed (feed.pentacle_id)}
    <div class="feed-health-item status-{feed.health_status}">
      <div class="feed-health-main">
        <span class="feed-status-badge status-{feed.health_status}">
          <i class={statusIcon(feed.health_status)}></i>
          {statusLabel(feed.health_status)}
        </span>
        <span class="feed-title">{feed.title ?? feed.url}</span>
        {#if feed.health_status !== "ok"}
          <button
            type="button"
            class="btn-action btn-small"
            onclick={() => resetHealth(feed.pentacle_id)}
            disabled={maintenanceRunning !== null}
          >
            <i class="fa-solid fa-rotate-left"></i>
            {$_("settings.maintenance.feedHealth.reset")}
          </button>
        {/if}
      </div>
      <div class="feed-health-meta">
        <span>
          {$_("settings.maintenance.feedHealth.lastSuccess")}: {formatDate(
            feed.last_successful_fetch,
          )}
        </span>
        <span>
          {$_("settings.maintenance.feedHealth.lastNewArticle")}: {formatDate(
            feed.last_new_article,
          )}
        </span>
        <span>
          {$_("settings.maintenance.feedHealth.errorCount")}: {feed.error_count}
        </span>
      </div>
      {#if feed.last_error}
        <p class="feed-last-error">{feed.last_error}</p>
      {/if}
    </div>
  {/each}
</div>

<style>
  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .maintenance-actions {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .maintenance-action {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .action-info {
    flex: 1;
  }

  .action-title {
    font-weight: 500;
    color: var(--text-primary);
  }

  .action-desc {
    margin: 0.25rem 0 0 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .btn-action {
    padding: 0.5rem 1rem;
    border: 1px solid var(--accent-primary);
    border-radius: 0.375rem;
    background: none;
    color: var(--accent-primary);
    font-size: 0.875rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .btn-action:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-action:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-action.btn-small {
    padding: 0.375rem 0.75rem;
    font-size: 0.75rem;
  }

  .btn-action.btn-small i {
    margin-right: 0.375rem;
  }

  .btn-action i {
    margin-right: 0.375rem;
  }

  /* Success message (matches orphan-result.success) */
  .orphan-result {
    padding: 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
  }

  .orphan-result.success {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background-color: rgba(166, 227, 161, 0.15);
    color: var(--status-success);
  }

  .feed-health-error {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    background-color: rgba(243, 139, 168, 0.15);
    color: var(--status-error);
  }

  /* Feed health list item */
  .feed-health-item {
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .feed-health-item.status-stale {
    border-color: rgba(249, 226, 175, 0.4);
  }

  .feed-health-item.status-broken {
    border-color: rgba(243, 139, 168, 0.4);
  }

  .feed-health-main {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .feed-status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 500;
    white-space: nowrap;
  }

  .feed-status-badge.status-ok {
    background-color: rgba(166, 227, 161, 0.2);
    color: var(--status-success);
  }

  .feed-status-badge.status-stale {
    background-color: rgba(249, 226, 175, 0.2);
    color: var(--status-warning);
  }

  .feed-status-badge.status-broken {
    background-color: rgba(243, 139, 168, 0.2);
    color: var(--status-error);
  }

  .feed-title {
    flex: 1;
    min-width: 0;
    font-weight: 500;
    color: var(--text-primary);
    overflow-wrap: anywhere;
  }

  .feed-health-meta {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    margin-top: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .feed-last-error {
    margin: 0.5rem 0 0 0;
    font-size: 0.75rem;
    color: var(--status-error);
    overflow-wrap: anywhere;
  }
</style>
