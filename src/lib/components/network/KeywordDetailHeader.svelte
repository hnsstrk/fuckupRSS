<script lang="ts">
  import { _ } from "svelte-i18n";

  interface Props {
    name: string;
    isRenaming: boolean;
    renameInput: string;
    renameLoading: boolean;
    renameError: string | null;
    onStartRename: () => void;
    onCancelRename: () => void;
    onHandleRename: () => void;
    onRenameInputChange: (value: string) => void;
  }

  let {
    name,
    isRenaming,
    renameInput,
    renameLoading,
    renameError,
    onStartRename,
    onCancelRename,
    onHandleRename,
    onRenameInputChange,
  }: Props = $props();

  function handleRenameKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      onHandleRename();
    } else if (e.key === "Escape") {
      onCancelRename();
    }
  }
</script>

<div class="detail-title-row">
  {#if isRenaming}
    <div class="rename-form">
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        class="rename-input"
        value={renameInput}
        oninput={(e) => onRenameInputChange(e.currentTarget.value)}
        onkeydown={handleRenameKeydown}
        disabled={renameLoading}
        autofocus
      />
      <button
        class="rename-btn save"
        onclick={onHandleRename}
        disabled={renameLoading || !renameInput.trim()}
        title={$_("common.save") || "Speichern"}
        aria-label={$_("common.save") || "Speichern"}
      >
        {#if renameLoading}
          <i class="fa-solid fa-spinner fa-spin"></i>
        {:else}
          <i class="fa-solid fa-check"></i>
        {/if}
      </button>
      <button
        class="rename-btn cancel"
        onclick={onCancelRename}
        disabled={renameLoading}
        title={$_("common.cancel") || "Abbrechen"}
        aria-label={$_("common.cancel") || "Abbrechen"}
      >
        <i class="fa-solid fa-times" aria-hidden="true"></i>
      </button>
    </div>
    {#if renameError}
      <div class="rename-error">{renameError}</div>
    {/if}
  {:else}
    <h3 class="detail-title">{name}</h3>
    <button
      class="edit-btn"
      onclick={onStartRename}
      title={$_("network.renameKeyword") || "Umbenennen"}
      aria-label={$_("network.renameKeyword") || "Umbenennen"}
    >
      <i class="fa-solid fa-pen" aria-hidden="true"></i>
    </button>
  {/if}
</div>

<style>
  .detail-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .detail-title {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .edit-btn {
    padding: 0.375rem 0.5rem;
    background: none;
    border: 1px solid transparent;
    border-radius: 0.25rem;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .edit-btn:hover {
    color: var(--accent-primary);
    border-color: var(--accent-primary);
    background-color: var(--bg-overlay);
  }

  .rename-form {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
  }

  .rename-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    font-size: 1.25rem;
    font-weight: 600;
    border: 2px solid var(--accent-primary);
    border-radius: 0.375rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    outline: none;
  }

  .rename-input:focus {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-primary) 25%, transparent);
  }

  .rename-btn {
    padding: 0.5rem 0.625rem;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.875rem;
  }

  .rename-btn.save {
    background-color: var(--accent-success);
    color: var(--text-on-accent);
  }

  .rename-btn.save:hover:not(:disabled) {
    background-color: color-mix(in srgb, var(--accent-success) 80%, black);
  }

  .rename-btn.cancel {
    background-color: var(--bg-overlay);
    color: var(--text-muted);
  }

  .rename-btn.cancel:hover:not(:disabled) {
    background-color: var(--bg-muted);
    color: var(--text-primary);
  }

  .rename-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .rename-error {
    margin-top: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--accent-error) 15%, transparent);
    border-radius: 0.375rem;
    color: var(--accent-error);
    font-size: 0.8125rem;
  }
</style>
