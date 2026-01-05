<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { settings, type Theme } from '../stores/settings.svelte';
  import { setLocale, locale } from '../i18n';
  import { appState } from '../stores/state.svelte';

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
  let syncInterval = $state(30);
  let syncOnStart = $state(true);

  // Ollama state
  let ollamaStatus = $state<{
    available: boolean;
    models: string[];
    recommended_main: string;
    recommended_embedding: string;
    has_recommended_main: boolean;
    has_recommended_embedding: boolean;
  } | null>(null);
  let selectedMainModel = $state('');
  let selectedEmbeddingModel = $state('');
  let downloadingModel = $state<string | null>(null);
  let downloadError = $state<string | null>(null);

  // Prompts state
  let summaryPrompt = $state('');
  let analysisPrompt = $state('');
  let defaultPrompts = $state<{ summary_prompt: string; analysis_prompt: string } | null>(null);
  let promptsModified = $state(false);

  // Dropdown open states
  let langDropdownOpen = $state(false);
  let themeDropdownOpen = $state(false);
  let mainModelDropdownOpen = $state(false);
  let embeddingModelDropdownOpen = $state(false);

  // Tab state
  let activeTab = $state<'general' | 'ollama' | 'prompts'>('general');

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
      syncInterval = settings.syncInterval;
      syncOnStart = settings.syncOnStart;
      // Close dropdowns
      closeAllDropdowns();
      // Reset tab
      activeTab = 'general';
      // Load Ollama status and prompts
      loadOllamaStatus();
      loadPrompts();
    }
  });

  async function loadOllamaStatus() {
    try {
      ollamaStatus = await invoke('check_ollama');
      // Load saved model preferences from settings
      const savedMainModel = await invoke<string | null>('get_setting', { key: 'main_model' });
      const savedEmbeddingModel = await invoke<string | null>('get_setting', { key: 'embedding_model' });

      if (ollamaStatus) {
        selectedMainModel = savedMainModel || ollamaStatus.recommended_main;
        selectedEmbeddingModel = savedEmbeddingModel || ollamaStatus.recommended_embedding;
        // Update appState
        appState.ollamaStatus = ollamaStatus;
      }
    } catch (e) {
      console.error('Failed to load Ollama status:', e);
      ollamaStatus = null;
    }
  }

  async function loadPrompts() {
    try {
      const prompts = await invoke<{ summary_prompt: string; analysis_prompt: string }>('get_prompts');
      summaryPrompt = prompts.summary_prompt;
      analysisPrompt = prompts.analysis_prompt;

      defaultPrompts = await invoke<{ summary_prompt: string; analysis_prompt: string }>('get_default_prompts');

      // Check if prompts are modified from defaults
      if (defaultPrompts) {
        promptsModified = summaryPrompt !== defaultPrompts.summary_prompt ||
                         analysisPrompt !== defaultPrompts.analysis_prompt;
      }
    } catch (e) {
      console.error('Failed to load prompts:', e);
    }
  }

  function closeAllDropdowns() {
    langDropdownOpen = false;
    themeDropdownOpen = false;
    mainModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
  }

  function handleClose() {
    dialogEl?.close();
    onclose();
  }

  async function handleSave() {
    // Save general settings
    await setLocale(selectedLocale);
    settings.showTerminologyTooltips = showTooltips;
    settings.theme = selectedTheme;
    settings.syncInterval = syncInterval;
    settings.syncOnStart = syncOnStart;

    // Save model preferences
    if (selectedMainModel) {
      await invoke('set_setting', { key: 'main_model', value: selectedMainModel });
      // Update appState so the model is used immediately
      appState.selectedModel = selectedMainModel;
    }
    if (selectedEmbeddingModel) {
      await invoke('set_setting', { key: 'embedding_model', value: selectedEmbeddingModel });
    }

    // Save prompts
    await invoke('set_prompts', {
      summaryPrompt: summaryPrompt,
      analysisPrompt: analysisPrompt
    });

    handleClose();
  }

  async function handleResetPrompts() {
    try {
      const prompts = await invoke<{ summary_prompt: string; analysis_prompt: string }>('reset_prompts');
      summaryPrompt = prompts.summary_prompt;
      analysisPrompt = prompts.analysis_prompt;
      promptsModified = false;
    } catch (e) {
      console.error('Failed to reset prompts:', e);
    }
  }

  async function handleDownloadModel(model: string) {
    if (downloadingModel) return;

    downloadingModel = model;
    downloadError = null;

    try {
      const result = await invoke<{ success: boolean; error: string | null }>('pull_model', { model });
      if (result.success) {
        // Reload status to show new model
        await loadOllamaStatus();
      } else {
        downloadError = result.error || 'Unknown error';
      }
    } catch (e) {
      downloadError = String(e);
    } finally {
      downloadingModel = null;
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === dialogEl) {
      handleClose();
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeAllDropdowns();
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

  function selectMainModel(value: string) {
    selectedMainModel = value;
    mainModelDropdownOpen = false;
  }

  function selectEmbeddingModel(value: string) {
    selectedEmbeddingModel = value;
    embeddingModelDropdownOpen = false;
  }

  function toggleLangDropdown() {
    langDropdownOpen = !langDropdownOpen;
    themeDropdownOpen = false;
  }

  function toggleThemeDropdown() {
    themeDropdownOpen = !themeDropdownOpen;
    langDropdownOpen = false;
  }

  function toggleMainModelDropdown() {
    mainModelDropdownOpen = !mainModelDropdownOpen;
    embeddingModelDropdownOpen = false;
  }

  function toggleEmbeddingModelDropdown() {
    embeddingModelDropdownOpen = !embeddingModelDropdownOpen;
    mainModelDropdownOpen = false;
  }

  function getLocaleLabelKey(value: string): string {
    return localeOptions.find(o => o.value === value)?.labelKey || '';
  }

  function getThemeLabelKey(value: Theme): string {
    return themeOptions.find(o => o.value === value)?.labelKey || '';
  }

  function isRecommendedModel(model: string, recommended: string): boolean {
    return model === recommended || model.startsWith(recommended.split(':')[0] + ':');
  }

  function handlePromptChange() {
    if (defaultPrompts) {
      promptsModified = summaryPrompt !== defaultPrompts.summary_prompt ||
                       analysisPrompt !== defaultPrompts.analysis_prompt;
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

      <!-- Tabs -->
      <div class="tabs">
        <button
          type="button"
          class="tab {activeTab === 'general' ? 'active' : ''}"
          onclick={() => activeTab = 'general'}
        >
          {$_('settings.title')}
        </button>
        <button
          type="button"
          class="tab {activeTab === 'ollama' ? 'active' : ''}"
          onclick={() => activeTab = 'ollama'}
        >
          Ollama
        </button>
        <button
          type="button"
          class="tab {activeTab === 'prompts' ? 'active' : ''}"
          onclick={() => activeTab = 'prompts'}
        >
          Prompts
        </button>
      </div>

      <div class="tab-content">
        {#if activeTab === 'general'}
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

          <!-- Sync Settings -->
          <div class="setting-group">
            <span class="label">{$_('settings.sync.title')}</span>
          </div>

          <div class="setting-group">
            <label class="label" for="sync-interval">{$_('settings.sync.interval')}</label>
            <div class="slider-row">
              <input
                id="sync-interval"
                type="range"
                min="5"
                max="120"
                step="5"
                bind:value={syncInterval}
                class="slider"
              />
              <span class="slider-value">{$_('settings.sync.minutes', { values: { count: syncInterval }})}</span>
            </div>
            <p class="setting-description">{$_('settings.sync.intervalDescription')}</p>
          </div>

          <div class="setting-group checkbox-group">
            <label>
              <input type="checkbox" bind:checked={syncOnStart} />
              <span class="checkbox-label">{$_('settings.sync.onStart')}</span>
            </label>
            <p class="setting-description">{$_('settings.sync.onStartDescription')}</p>
          </div>

        {:else if activeTab === 'ollama'}
          <h3>{$_('settings.ollama.title')}</h3>

          <!-- Ollama Status -->
          <div class="setting-group">
            <span class="label">{$_('settings.ollama.status')}</span>
            {#if ollamaStatus === null}
              <div class="status-loading">...</div>
            {:else if ollamaStatus.available}
              <div class="status-available">
                <span class="status-icon">✓</span>
                {$_('settings.ollama.available')}
              </div>
            {:else}
              <div class="status-unavailable">
                <span class="status-icon">✗</span>
                {$_('settings.ollama.unavailable')}
                <p class="setting-description">{$_('settings.ollama.unavailableDescription')}</p>
              </div>
            {/if}
          </div>

          {#if ollamaStatus?.available}
            <!-- Main Model Selection -->
            <div class="setting-group">
              <span class="label">{$_('settings.ollama.mainModel')}</span>
              <div class="model-row">
                <div class="custom-select model-select">
                  <button type="button" class="select-trigger" onclick={toggleMainModelDropdown}>
                    <span>
                      {selectedMainModel || $_('settings.ollama.noModels')}
                      {#if ollamaStatus && isRecommendedModel(selectedMainModel, ollamaStatus.recommended_main)}
                        <span class="recommended">{$_('settings.ollama.recommended')}</span>
                      {/if}
                    </span>
                    <span class="arrow">{mainModelDropdownOpen ? '▲' : '▼'}</span>
                  </button>
                  {#if mainModelDropdownOpen}
                    <div class="select-options">
                      {#each ollamaStatus.models as model}
                        <button
                          type="button"
                          class="select-option {selectedMainModel === model ? 'selected' : ''}"
                          onclick={() => selectMainModel(model)}
                        >
                          {model}
                          {#if isRecommendedModel(model, ollamaStatus.recommended_main)}
                            <span class="recommended">{$_('settings.ollama.recommended')}</span>
                          {/if}
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
                {#if ollamaStatus && !ollamaStatus.has_recommended_main}
                  <button
                    type="button"
                    class="btn-download"
                    onclick={() => handleDownloadModel(ollamaStatus!.recommended_main)}
                    disabled={downloadingModel !== null}
                  >
                    {#if downloadingModel === ollamaStatus.recommended_main}
                      {$_('settings.ollama.downloading')}
                    {:else}
                      {$_('settings.ollama.downloadModel')} {ollamaStatus.recommended_main}
                    {/if}
                  </button>
                {/if}
              </div>
            </div>

            <!-- Embedding Model Selection -->
            <div class="setting-group">
              <span class="label">{$_('settings.ollama.embeddingModel')}</span>
              <div class="model-row">
                <div class="custom-select model-select">
                  <button type="button" class="select-trigger" onclick={toggleEmbeddingModelDropdown}>
                    <span>
                      {selectedEmbeddingModel || $_('settings.ollama.noModels')}
                      {#if ollamaStatus && isRecommendedModel(selectedEmbeddingModel, ollamaStatus.recommended_embedding)}
                        <span class="recommended">{$_('settings.ollama.recommended')}</span>
                      {/if}
                    </span>
                    <span class="arrow">{embeddingModelDropdownOpen ? '▲' : '▼'}</span>
                  </button>
                  {#if embeddingModelDropdownOpen}
                    <div class="select-options">
                      {#each ollamaStatus.models as model}
                        <button
                          type="button"
                          class="select-option {selectedEmbeddingModel === model ? 'selected' : ''}"
                          onclick={() => selectEmbeddingModel(model)}
                        >
                          {model}
                          {#if isRecommendedModel(model, ollamaStatus.recommended_embedding)}
                            <span class="recommended">{$_('settings.ollama.recommended')}</span>
                          {/if}
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
                {#if ollamaStatus && !ollamaStatus.has_recommended_embedding}
                  <button
                    type="button"
                    class="btn-download"
                    onclick={() => handleDownloadModel(ollamaStatus!.recommended_embedding)}
                    disabled={downloadingModel !== null}
                  >
                    {#if downloadingModel === ollamaStatus.recommended_embedding}
                      {$_('settings.ollama.downloading')}
                    {:else}
                      {$_('settings.ollama.downloadModel')} {ollamaStatus.recommended_embedding}
                    {/if}
                  </button>
                {/if}
              </div>
            </div>

            {#if downloadError}
              <div class="error-message">{$_('settings.ollama.downloadError')}: {downloadError}</div>
            {/if}
          {/if}

        {:else if activeTab === 'prompts'}
          <h3>{$_('settings.prompts.title')}</h3>

          {#if !ollamaStatus?.available}
            <div class="status-unavailable">
              <span class="status-icon">✗</span>
              {$_('settings.ollama.unavailable')}
            </div>
          {:else}
            <div class="setting-group">
              <label class="label" for="summary-prompt">{$_('settings.prompts.summaryPrompt')}</label>
              <textarea
                id="summary-prompt"
                class="prompt-textarea"
                bind:value={summaryPrompt}
                oninput={handlePromptChange}
                rows="6"
              ></textarea>
            </div>

            <div class="setting-group">
              <label class="label" for="analysis-prompt">{$_('settings.prompts.analysisPrompt')}</label>
              <textarea
                id="analysis-prompt"
                class="prompt-textarea"
                bind:value={analysisPrompt}
                oninput={handlePromptChange}
                rows="8"
              ></textarea>
            </div>

            {#if promptsModified}
              <button type="button" class="btn-reset" onclick={handleResetPrompts}>
                {$_('settings.prompts.reset')}
              </button>
            {/if}
          {/if}
        {/if}
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
    max-width: 600px;
    width: 90%;
    max-height: 80vh;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .settings-dialog::backdrop {
    background: rgba(0, 0, 0, 0.6);
  }

  .dialog-content {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    max-height: 80vh;
  }

  h2 {
    margin: 0 0 1rem 0;
    font-size: 1.25rem;
    color: var(--accent-primary);
  }

  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  /* Tabs */
  .tabs {
    display: flex;
    gap: 0.25rem;
    margin-bottom: 1rem;
    border-bottom: 1px solid var(--border-default);
  }

  .tab {
    padding: 0.5rem 1rem;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 0.875rem;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: all 0.2s;
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    min-height: 200px;
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

  .model-select {
    flex: 1;
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
    max-height: 200px;
    overflow-y: auto;
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
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .select-option:hover {
    background-color: var(--bg-muted);
  }

  .select-option.selected {
    background-color: var(--bg-muted);
    color: var(--accent-primary);
  }

  .recommended {
    font-size: 0.75rem;
    color: var(--accent-secondary);
    margin-left: 0.5rem;
  }

  /* Model row */
  .model-row {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .btn-download {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--accent-primary);
    border-radius: 0.375rem;
    background: none;
    color: var(--accent-primary);
    font-size: 0.875rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .btn-download:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-download:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Status */
  .status-available {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--status-success);
    padding: 0.5rem;
    background-color: rgba(166, 227, 161, 0.1);
    border-radius: 0.375rem;
  }

  .status-unavailable {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    color: var(--status-error);
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.375rem;
  }

  .status-unavailable .status-icon {
    display: inline;
  }

  .status-icon {
    font-weight: bold;
  }

  .status-loading {
    color: var(--text-muted);
    padding: 0.5rem;
  }

  .error-message {
    color: var(--status-error);
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.375rem;
    font-size: 0.875rem;
  }

  /* Checkbox */
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
    margin: 0.25rem 0 0 0;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  /* Slider */
  .slider-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .slider {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    appearance: none;
    background: var(--bg-overlay);
    cursor: pointer;
  }

  .slider::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent-primary);
    cursor: pointer;
    transition: transform 0.15s;
  }

  .slider::-webkit-slider-thumb:hover {
    transform: scale(1.1);
  }

  .slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent-primary);
    cursor: pointer;
    border: none;
  }

  .slider-value {
    min-width: 6rem;
    text-align: right;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  /* Prompts */
  .prompt-textarea {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-family: monospace;
    font-size: 0.875rem;
    resize: vertical;
    min-height: 100px;
  }

  .prompt-textarea:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .btn-reset {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: none;
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-reset:hover {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  /* Dialog Actions */
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-default);
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
