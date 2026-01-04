<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { settings } from '../stores/settings.svelte';
  import { setLocale, locale } from '../i18n';

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open, onclose }: Props = $props();

  let dialogEl: HTMLDialogElement | null = $state(null);

  // Local state for form
  let selectedLocale = $state('de');
  let showTooltips = $state(true);
  let selectedTheme = $state<'dark' | 'light'>('dark');

  // Sync local state when dialog opens
  $effect(() => {
    if (open && dialogEl) {
      dialogEl.showModal();
      // Initialize from current settings
      selectedLocale = $locale || 'de';
      showTooltips = settings.showTerminologyTooltips;
      selectedTheme = settings.theme;
    }
  });

  function handleClose() {
    dialogEl?.close();
    onclose();
  }

  function handleSave() {
    setLocale(selectedLocale);
    settings.showTerminologyTooltips = showTooltips;
    settings.theme = selectedTheme;
    handleClose();
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === dialogEl) {
      handleClose();
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose();
    }
  }
</script>

{#if open}
  <dialog
    bind:this={dialogEl}
    class="settings-dialog"
    onclick={handleBackdropClick}
    onkeydown={handleKeyDown}
  >
    <div class="dialog-content">
      <h2>{$_('settings.title')}</h2>

      <div class="setting-group">
        <label for="language">{$_('settings.language')}</label>
        <select id="language" bind:value={selectedLocale}>
          <option value="de">{$_('settings.languageGerman')}</option>
          <option value="en">{$_('settings.languageEnglish')}</option>
        </select>
      </div>

      <div class="setting-group">
        <label for="theme">{$_('settings.theme')}</label>
        <select id="theme" bind:value={selectedTheme}>
          <option value="dark">{$_('settings.themeDark')}</option>
          <option value="light">{$_('settings.themeLight')}</option>
        </select>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input type="checkbox" bind:checked={showTooltips} />
          <span class="checkbox-label">{$_('settings.tooltips')}</span>
        </label>
        <p class="setting-description">{$_('settings.tooltipsDescription')}</p>
      </div>

      <div class="dialog-actions">
        <button type="button" class="btn-secondary" onclick={handleClose}>
          {$_('settings.cancel')}
        </button>
        <button type="button" class="btn-primary" onclick={handleSave}>
          {$_('settings.save')}
        </button>
      </div>
    </div>
  </dialog>
{/if}

<style>
  .settings-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    border: none;
    border-radius: 12px;
    padding: 0;
    max-width: 400px;
    width: 90%;
    background: #1a1a2e;
    color: #e0e0e0;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .settings-dialog::backdrop {
    background: rgba(0, 0, 0, 0.6);
  }

  .dialog-content {
    padding: 24px;
  }

  h2 {
    margin: 0 0 20px 0;
    font-size: 1.25rem;
    color: #ffd700;
  }

  .setting-group {
    margin-bottom: 20px;
  }

  .setting-group label {
    display: block;
    margin-bottom: 6px;
    font-weight: 500;
  }

  select {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #4a4a6a;
    border-radius: 6px;
    background: #0f0f1a;
    color: #e0e0e0;
    font-size: 1rem;
  }

  select:focus {
    outline: none;
    border-color: #ffd700;
  }

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .checkbox-group input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: #ffd700;
  }

  .checkbox-label {
    font-weight: 500;
  }

  .setting-description {
    margin: 4px 0 0 26px;
    font-size: 0.875rem;
    color: #888;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 24px;
  }

  .btn-primary,
  .btn-secondary {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .btn-primary {
    background: #ffd700;
    color: #0f0f1a;
  }

  .btn-primary:hover {
    background: #ffed4a;
  }

  .btn-secondary {
    background: #4a4a6a;
    color: #e0e0e0;
  }

  .btn-secondary:hover {
    background: #5a5a7a;
  }

  /* Light theme */
  :global(.light) .settings-dialog {
    background: #ffffff;
    color: #333333;
  }

  :global(.light) h2 {
    color: #b8860b;
  }

  :global(.light) select {
    background: #f5f5f5;
    border-color: #d0d0d0;
    color: #333333;
  }

  :global(.light) select:focus {
    border-color: #b8860b;
  }

  :global(.light) .setting-description {
    color: #666666;
  }

  :global(.light) .btn-secondary {
    background: #e0e0e0;
    color: #333333;
  }

  :global(.light) .btn-secondary:hover {
    background: #d0d0d0;
  }

  :global(.light) .btn-primary {
    background: #b8860b;
    color: #ffffff;
  }

  :global(.light) .btn-primary:hover {
    background: #d4a017;
  }
</style>
