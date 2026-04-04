<script lang="ts">
  import { _ } from "svelte-i18n";
  import ActionButton from "$lib/components/ui/ActionButton.svelte";

  interface Props {
    newKeywordInput: string;
    createKeywordLoading: boolean;
    createKeywordSuccess: string | null;
    createKeywordError: string | null;
    onNewKeywordInput: (value: string) => void;
    onCreateNewKeyword: () => void;
  }

  let {
    newKeywordInput,
    createKeywordLoading,
    createKeywordSuccess,
    createKeywordError,
    onNewKeywordInput,
    onCreateNewKeyword,
  }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      onCreateNewKeyword();
    }
  }
</script>

<div class="synonyms-section">
  <h3 class="section-heading">{$_("network.createKeyword") || "Neues Keyword erstellen"}</h3>
  <div class="create-keyword-form">
    <input
      type="text"
      value={newKeywordInput}
      oninput={(e) => onNewKeywordInput(e.currentTarget.value)}
      placeholder={$_("network.newKeywordPlaceholder") || "Keyword eingeben..."}
      class="create-keyword-input"
      onkeydown={handleKeydown}
    />
    <ActionButton
      variant="primary"
      onclick={onCreateNewKeyword}
      disabled={createKeywordLoading || !newKeywordInput.trim()}
      loading={createKeywordLoading}
    >
      {#if createKeywordLoading}
        {$_("network.loading") || "Lade..."}
      {:else}
        {$_("network.create") || "Erstellen"}
      {/if}
    </ActionButton>
  </div>
  {#if createKeywordError}
    <div class="feedback-message error">{createKeywordError}</div>
  {/if}
  {#if createKeywordSuccess}
    <div class="feedback-message success">{createKeywordSuccess}</div>
  {/if}
</div>

<style>
  .synonyms-section {
    background-color: var(--bg-surface);
    border-radius: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--border-default);
    flex: 1;
  }

  .section-heading {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .feedback-message {
    margin-top: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.75rem;
  }

  .feedback-message.error {
    background-color: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--accent-error);
    color: var(--accent-error);
  }

  .feedback-message.success {
    background-color: rgba(34, 197, 94, 0.1);
    border: 1px solid var(--accent-success);
    color: var(--accent-success);
  }

  /* Create Keyword Form */
  .create-keyword-form {
    display: flex;
    gap: 0.5rem;
  }

  .create-keyword-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .create-keyword-input::placeholder {
    color: var(--text-faint);
  }

  .create-keyword-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }
</style>
