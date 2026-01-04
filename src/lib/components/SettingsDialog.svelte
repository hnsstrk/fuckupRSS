<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { settings, type Theme } from '../stores/settings.svelte';
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
  let selectedTheme = $state<Theme>('mocha');

  // Dropdown open states
  let langDropdownOpen = $state(false);
  let themeDropdownOpen = $state(false);

  const localeOptions = [
    { value: 'de', labelKey: 'settings.languageGerman' },
    { value: 'en', labelKey: 'settings.languageEnglish' },
  ];

  const themeOptions: { value: Theme; labelKey: string }[] = [
    { value: 'mocha', labelKey: 'settings.themeMocha' },
    { value: 'macchiato', labelKey: 'settings.themeMacchiato' },
    { value: 'frappe', labelKey: 'settings.themeFrappe' },
    { value: 'latte', labelKey: 'settings.themeLatte' },
  ];

  // Sync local state when dialog opens
  $effect(() => {
    if (open && dialogEl) {
      dialogEl.showModal();
      // Initialize from current settings
      selectedLocale = $locale || 'de';
      showTooltips = settings.showTerminologyTooltips;
      selectedTheme = settings.theme;
      // Close dropdowns
      langDropdownOpen = false;
      themeDropdownOpen = false;
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
      langDropdownOpen = false;
      themeDropdownOpen = false;
    }
  }

  function selectLocale(value: string) {
    selectedLocale = value;
    langDropdownOpen = false;
  }

  function selectTheme(value: Theme) {
    selectedTheme = value;
    themeDropdownOpen = false;
  }

  function toggleLangDropdown() {
    langDropdownOpen = !langDropdownOpen;
    themeDropdownOpen = false;
  }

  function toggleThemeDropdown() {
    themeDropdownOpen = !themeDropdownOpen;
    langDropdownOpen = false;
  }

  function getLocaleLabelKey(value: string): string {
    return localeOptions.find(o => o.value === value)?.labelKey || '';
  }

  function getThemeLabelKey(value: Theme): string {
    return themeOptions.find(o => o.value === value)?.labelKey || '';
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

      <!-- Language Dropdown -->
      <div class="setting-group">
        <span class="label">{$_('settings.language')}</span>
        <div class="custom-select">
          <button type="button" class="select-trigger" aria-label={$_('settings.language')} onclick={toggleLangDropdown}>
            <span>{$_(getLocaleLabelKey(selectedLocale))}</span>
            <span class="arrow">{langDropdownOpen ? '▲' : '▼'}</span>
          </button>
          {#if langDropdownOpen}
            <div class="select-options">
              {#each localeOptions as option}
                <button
                  type="button"
                  class="select-option {selectedLocale === option.value ? 'selected' : ''}"
                  onclick={() => selectLocale(option.value)}
                >
                  {$_(option.labelKey)}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <!-- Theme Dropdown -->
      <div class="setting-group">
        <span class="label">{$_('settings.theme')}</span>
        <div class="custom-select">
          <button type="button" class="select-trigger" aria-label={$_('settings.theme')} onclick={toggleThemeDropdown}>
            <span>{$_(getThemeLabelKey(selectedTheme))}</span>
            <span class="arrow">{themeDropdownOpen ? '▲' : '▼'}</span>
          </button>
          {#if themeDropdownOpen}
            <div class="select-options">
              {#each themeOptions as option}
                <button
                  type="button"
                  class="select-option {selectedTheme === option.value ? 'selected' : ''}"
                  onclick={() => selectTheme(option.value)}
                >
                  {$_(option.labelKey)}
                </button>
              {/each}
            </div>
          {/if}
        </div>
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
    border-radius: 0.75rem;
    padding: 0;
    max-width: 400px;
    width: 90%;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .settings-dialog::backdrop {
    background: rgba(0, 0, 0, 0.6);
  }

  .dialog-content {
    padding: 1.5rem;
  }

  h2 {
    margin: 0 0 1.25rem 0;
    font-size: 1.25rem;
    color: var(--accent-primary);
  }

  .setting-group {
    margin-bottom: 1.25rem;
  }

  .setting-group > label,
  .setting-group > .label {
    display: block;
    margin-bottom: 0.375rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  /* Custom Select */
  .custom-select {
    position: relative;
  }

  .select-trigger {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 1rem;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    text-align: left;
  }

  .select-trigger:hover {
    border-color: var(--accent-primary);
  }

  .arrow {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .select-options {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 0.25rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    overflow: hidden;
    z-index: 100;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .select-option {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 1rem;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.15s;
  }

  .select-option:hover {
    background-color: var(--bg-muted);
  }

  .select-option.selected {
    background-color: var(--bg-muted);
    color: var(--accent-primary);
  }

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .checkbox-group input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent-primary);
  }

  .checkbox-label {
    font-weight: 500;
  }

  .setting-description {
    margin: 0.25rem 0 0 1.625rem;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.5rem;
  }

  .btn-primary,
  .btn-secondary {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-primary:hover {
    filter: brightness(1.1);
  }

  .btn-secondary {
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
  }

  .btn-secondary:hover {
    background-color: var(--bg-muted);
  }
</style>
