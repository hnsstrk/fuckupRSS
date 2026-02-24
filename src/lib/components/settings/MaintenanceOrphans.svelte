<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { appState } from "../../stores/state.svelte";

  let {
    maintenanceRunning,
  }: {
    maintenanceRunning: string | null;
  } = $props();

  // Orphaned articles state
  let orphanScanning = $state(false);
  let orphanStats = $state<{ total: number; favorites: number } | null>(null);
  let orphanDeleteResult = $state<number | null>(null);

  // Confirmation state
  let confirmAction = $state<"deleteOrphansAll" | "deleteOrphansKeepFavorites" | null>(null);

  function cancelConfirmation() {
    confirmAction = null;
  }

  async function scanOrphans() {
    orphanScanning = true;
    orphanStats = null;
    orphanDeleteResult = null;
    try {
      orphanStats = await invoke("find_orphaned_articles");
    } catch (e) {
      console.error("Failed to scan orphans:", e);
    } finally {
      orphanScanning = false;
    }
  }

  async function handleDeleteOrphans(includeFavorites: boolean) {
    confirmAction = null;
    try {
      const count: number = await invoke("delete_orphaned_articles", { includeFavorites });
      orphanDeleteResult = count;
      orphanStats = null;
      await appState.loadFnords();
      await appState.loadPentacles();
    } catch (e) {
      console.error("Failed to delete orphans:", e);
    }
  }
</script>

<!-- Confirmation Dialog -->
{#if confirmAction}
  <div class="confirm-overlay">
    <div class="confirm-dialog">
      <p class="confirm-message">
        {#if confirmAction === "deleteOrphansAll"}
          {$_("settings.maintenance.orphanedArticles.confirmDeleteAll", {
            values: { count: orphanStats?.total ?? 0, favorites: orphanStats?.favorites ?? 0 },
          })}
        {:else if confirmAction === "deleteOrphansKeepFavorites"}
          {$_("settings.maintenance.orphanedArticles.confirmDelete", {
            values: { count: (orphanStats?.total ?? 0) - (orphanStats?.favorites ?? 0) },
          })}
        {/if}
      </p>
      <div class="confirm-actions">
        <button type="button" class="btn-secondary" onclick={cancelConfirmation}>
          {$_("confirm.no")}
        </button>
        <button
          type="button"
          class="btn-danger-solid"
          onclick={confirmAction === "deleteOrphansAll"
            ? () => handleDeleteOrphans(true)
            : () => handleDeleteOrphans(false)}
        >
          {$_("confirm.yes")}
        </button>
      </div>
    </div>
  </div>
{/if}

<h3 style="margin-top: 1.5rem;">
  {$_("settings.maintenance.orphanedArticles.title")}
</h3>

<div class="maintenance-actions">
  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.orphanedArticles.title")}</span>
      <p class="action-desc">{$_("settings.maintenance.orphanedArticles.description")}</p>
    </div>
    <button
      type="button"
      class="btn-action"
      onclick={scanOrphans}
      disabled={orphanScanning || maintenanceRunning !== null}
    >
      {#if orphanScanning}
        <i class="fa-solid fa-spinner fa-spin"></i>
        {$_("settings.maintenance.orphanedArticles.scanning")}
      {:else}
        <i class="fa-solid fa-magnifying-glass"></i>
        {$_("settings.maintenance.orphanedArticles.scan")}
      {/if}
    </button>
  </div>

  {#if orphanStats}
    {#if orphanStats.total === 0}
      <div class="orphan-result success">
        <i class="fa-solid fa-check-circle"></i>
        {$_("settings.maintenance.orphanedArticles.noOrphans")}
      </div>
    {:else}
      <div class="orphan-result warning">
        {#if orphanStats.favorites > 0}
          <p class="orphan-message">
            <i class="fa-solid fa-triangle-exclamation"></i>
            {$_("settings.maintenance.orphanedArticles.foundWithFavorites", {
              values: { count: orphanStats.total, favorites: orphanStats.favorites },
            })}
          </p>
          <div class="action-buttons">
            <button
              type="button"
              class="btn-action btn-danger btn-small"
              onclick={() => (confirmAction = "deleteOrphansAll")}
            >
              <i class="fa-solid fa-trash"></i>
              {$_("settings.maintenance.orphanedArticles.deleteAll")}
            </button>
            <button
              type="button"
              class="btn-action btn-small"
              onclick={() => (confirmAction = "deleteOrphansKeepFavorites")}
            >
              <i class="fa-solid fa-trash-can"></i>
              {$_("settings.maintenance.orphanedArticles.deleteExceptFavorites")}
            </button>
          </div>
        {:else}
          <p class="orphan-message">
            <i class="fa-solid fa-triangle-exclamation"></i>
            {$_("settings.maintenance.orphanedArticles.found", {
              values: { count: orphanStats.total },
            })}
          </p>
          <div class="action-buttons">
            <button
              type="button"
              class="btn-action btn-danger btn-small"
              onclick={() => (confirmAction = "deleteOrphansAll")}
            >
              <i class="fa-solid fa-trash"></i>
              {$_("settings.maintenance.orphanedArticles.deleteAll")}
            </button>
          </div>
        {/if}
      </div>
    {/if}
  {/if}

  {#if orphanDeleteResult !== null}
    <div class="orphan-result success">
      <i class="fa-solid fa-check-circle"></i>
      {$_("settings.maintenance.orphanedArticles.deleted", {
        values: { count: orphanDeleteResult },
      })}
    </div>
  {/if}
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

  .btn-action.btn-danger {
    border-color: var(--status-error);
    color: var(--status-error);
  }

  .btn-action.btn-danger:hover:not(:disabled) {
    background-color: var(--status-error);
    color: var(--text-on-accent);
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

  /* Confirmation Dialog */
  .confirm-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .confirm-dialog {
    background: var(--bg-surface);
    padding: 1.5rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
    max-width: 400px;
    text-align: center;
  }

  .confirm-message {
    margin: 0 0 1.5rem 0;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .confirm-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: center;
  }

  .btn-secondary {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary:hover {
    background-color: var(--bg-muted);
  }

  .btn-danger-solid {
    padding: 0.5rem 1.5rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--status-error);
    color: var(--text-on-accent);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-danger-solid:hover {
    filter: brightness(1.1);
  }

  /* Orphaned Articles */
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

  .orphan-result.warning {
    background-color: rgba(250, 179, 135, 0.15);
    border: 1px solid rgba(250, 179, 135, 0.3);
  }

  .orphan-message {
    margin: 0 0 0.75rem 0;
    color: var(--status-warning);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .action-buttons {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
</style>
