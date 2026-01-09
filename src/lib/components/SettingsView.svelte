<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
  import type { BatchProgress, BatchResult } from '../types';
  import { settings, type DarkTheme } from '../stores/settings.svelte';
  import { type LogLevel } from '../logger';
  import { setLocale, locale } from '../i18n';
  import { appState } from '../stores/state.svelte';
  import { onMount, onDestroy } from 'svelte';

  // Local state for form
  let selectedLocale = $state('de');
  let showTooltips = $state(true);
  let selectedDarkTheme = $state<DarkTheme>('mocha');
  let syncInterval = $state(30);
  let syncOnStart = $state(true);
  let selectedLogLevel = $state<LogLevel>('info');

  // Ollama state
  let ollamaStatus = $state<{
    available: boolean;
    models: string[];
    recommended_main: string;
    recommended_embedding: string;
    has_recommended_main: boolean;
    has_recommended_embedding: boolean;
  } | null>(null);
  let loadedModels = $state<{
    name: string;
    size: number;
    size_vram: number;
    parameter_size: string;
  }[]>([]);
  let selectedMainModel = $state('');
  let selectedEmbeddingModel = $state('');
  let downloadingModel = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let loadingModels = $state(false);

  // Prompts state
  let summaryPrompt = $state('');
  let analysisPrompt = $state('');
  let defaultPrompts = $state<{ summary_prompt: string; analysis_prompt: string } | null>(null);
  let promptsModified = $state(false);

  // Dropdown open states
  let langDropdownOpen = $state(false);
  let themeDropdownOpen = $state(false);
  let logLevelDropdownOpen = $state(false);
  let mainModelDropdownOpen = $state(false);
  let embeddingModelDropdownOpen = $state(false);

  // Tab state
  let activeTab = $state<'general' | 'ollama' | 'prompts' | 'maintenance'>('general');

  // Maintenance state
  let maintenanceRunning = $state<string | null>(null);
  let maintenanceResult = $state<string | null>(null);

  // Confirmation dialog state
  let confirmAction = $state<'prune' | 'reset' | null>(null);

  // Synonym candidates state
  let synonymCandidates = $state<{ keyword_a_id: number; keyword_a_name: string; keyword_b_id: number; keyword_b_name: string; similarity: number }[]>([]);

  // Keyword statistics state
  let keywordStats = $state<{ total: number; with_embeddings: number; avg_quality: number; low_quality: number } | null>(null);

  // Reanalyze progress state
  let reanalyzeProgress = $state<BatchProgress | null>(null);
  let reanalyzeRunning = $state(false);
  let reanalyzeResult = $state<BatchResult | null>(null);
  let progressUnlisten: UnlistenFn | null = null;

  const localeOptions = [
    { value: 'de', labelKey: 'settings.languageGerman' },
    { value: 'en', labelKey: 'settings.languageEnglish' },
  ];

  const darkThemeOptions: { value: DarkTheme; labelKey: string }[] = [
    { value: 'mocha', labelKey: 'settings.themeMocha' },
    { value: 'macchiato', labelKey: 'settings.themeMacchiato' },
    { value: 'frappe', labelKey: 'settings.themeFrappe' },
  ];

  const logLevelOptions: { value: LogLevel; label: string }[] = [
    { value: 'error', label: 'Error' },
    { value: 'warn', label: 'Warn' },
    { value: 'info', label: 'Info' },
    { value: 'debug', label: 'Debug' },
    { value: 'trace', label: 'Trace' },
  ];

  onMount(async () => {
    // Initialize from current settings
    selectedLocale = $locale || 'de';
    showTooltips = settings.showTerminologyTooltips;
    selectedDarkTheme = settings.darkTheme;
    syncInterval = settings.syncInterval;
    syncOnStart = settings.syncOnStart;
    selectedLogLevel = settings.logLevel;
    // Load Ollama status and prompts
    await loadOllamaStatus();
    await loadPrompts();
  });

  onDestroy(() => {
    if (progressUnlisten) {
      progressUnlisten();
    }
  });

  async function loadOllamaStatus() {
    try {
      ollamaStatus = await invoke('check_ollama');
      const savedMainModel = await invoke<string | null>('get_setting', { key: 'main_model' });
      const savedEmbeddingModel = await invoke<string | null>('get_setting', { key: 'embedding_model' });

      if (ollamaStatus) {
        selectedMainModel = savedMainModel || ollamaStatus.recommended_main;
        selectedEmbeddingModel = savedEmbeddingModel || ollamaStatus.recommended_embedding;
        appState.ollamaStatus = ollamaStatus;
      }

      await loadLoadedModels();
    } catch (e) {
      console.error('Failed to load Ollama status:', e);
      ollamaStatus = null;
    }
  }

  async function loadLoadedModels() {
    try {
      const response = await invoke<{ models: typeof loadedModels }>('get_loaded_models');
      loadedModels = response.models;
    } catch (e) {
      console.error('Failed to load loaded models:', e);
      loadedModels = [];
    }
  }

  function formatBytes(bytes: number): string {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(1)} GB`;
  }

  function isModelLoaded(modelName: string): boolean {
    return loadedModels.some(m => m.name === modelName);
  }

  async function loadPrompts() {
    try {
      const prompts = await invoke<{ summary_prompt: string; analysis_prompt: string }>('get_prompts');
      summaryPrompt = prompts.summary_prompt;
      analysisPrompt = prompts.analysis_prompt;

      defaultPrompts = await invoke<{ summary_prompt: string; analysis_prompt: string }>('get_default_prompts');

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
    logLevelDropdownOpen = false;
    mainModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
  }

  async function handleSave() {
    // Save general settings
    await setLocale(selectedLocale);
    settings.showTerminologyTooltips = showTooltips;
    settings.darkTheme = selectedDarkTheme;
    settings.syncInterval = syncInterval;
    settings.syncOnStart = syncOnStart;
    settings.logLevel = selectedLogLevel;

    // Save model preferences
    if (selectedMainModel) {
      await invoke('set_setting', { key: 'main_model', value: selectedMainModel });
      appState.selectedModel = selectedMainModel;
    }
    if (selectedEmbeddingModel) {
      await invoke('set_setting', { key: 'embedding_model', value: selectedEmbeddingModel });
    }

    // Ensure only the selected models are loaded
    if (selectedMainModel && selectedEmbeddingModel) {
      loadingModels = true;
      try {
        await invoke('ensure_models_loaded', {
          mainModel: selectedMainModel,
          embeddingModel: selectedEmbeddingModel
        });
        await emit('models-changed');
      } catch (e) {
        console.error('Failed to ensure models loaded:', e);
      }
      loadingModels = false;
    }

    // Save prompts
    await invoke('set_prompts', {
      summaryPrompt: summaryPrompt,
      analysisPrompt: analysisPrompt
    });
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

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeAllDropdowns();
    }
  }

  function selectLocale(value: string) {
    selectedLocale = value;
    langDropdownOpen = false;
  }

  function selectDarkTheme(value: DarkTheme) {
    selectedDarkTheme = value;
    themeDropdownOpen = false;
  }

  function selectLogLevel(value: LogLevel) {
    selectedLogLevel = value;
    logLevelDropdownOpen = false;
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
    logLevelDropdownOpen = false;
  }

  function toggleLogLevelDropdown() {
    logLevelDropdownOpen = !logLevelDropdownOpen;
    langDropdownOpen = false;
    themeDropdownOpen = false;
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

  function getDarkThemeLabelKey(value: DarkTheme): string {
    return darkThemeOptions.find(o => o.value === value)?.labelKey || '';
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

  async function handleCalculateScores() {
    maintenanceRunning = 'scores';
    maintenanceResult = null;
    try {
      const result = await invoke<{ updated_count: number; avg_score: number; low_quality_count: number }>('calculate_keyword_quality_scores', { limit: 1000 });
      maintenanceResult = `${result.updated_count} ${$_('settings.maintenance.updated')} (Ø ${result.avg_score.toFixed(2)})`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  async function handleGenerateEmbeddings() {
    maintenanceRunning = 'embeddings';
    maintenanceResult = null;
    try {
      const result = await invoke<{ generated_count: number; failed_count: number }>('generate_keyword_embeddings', { limit: 50 });
      maintenanceResult = `${result.generated_count} ${$_('settings.maintenance.generated')}`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  function showPruneConfirmation() {
    confirmAction = 'prune';
  }

  function showResetConfirmation() {
    confirmAction = 'reset';
  }

  function cancelConfirmation() {
    confirmAction = null;
  }

  async function handlePruneLowQuality() {
    confirmAction = null;
    maintenanceRunning = 'prune';
    maintenanceResult = null;
    try {
      const result = await invoke<{ pruned_count: number; pruned_keywords: string[] }>('auto_prune_low_quality', {
        quality_threshold: 0.2,
        min_age_days: 7,
        dry_run: false
      });
      maintenanceResult = `${result.pruned_count} ${$_('settings.maintenance.pruned')}`;
      await loadKeywordStats();
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  async function handleResetForReprocessing() {
    confirmAction = null;
    maintenanceRunning = 'reset';
    maintenanceResult = null;
    reanalyzeProgress = null;
    reanalyzeResult = null;

    try {
      const resetResult = await invoke<{ reset_count: number }>('reset_articles_for_reprocessing', {
        only_with_content: true
      });

      if (resetResult.reset_count === 0) {
        maintenanceResult = $_('settings.maintenance.noArticlesToReset');
        maintenanceRunning = null;
        return;
      }

      await emit('articles-reset');
      await appState.loadUnprocessedCount();

      const model = appState.selectedModel || appState.ollamaStatus.models[0];
      if (!model || !appState.ollamaStatus.available) {
        maintenanceResult = `${resetResult.reset_count} ${$_('settings.maintenance.articles')} ${$_('settings.maintenance.reset')}. ${$_('settings.maintenance.ollamaUnavailable')}`;
        maintenanceRunning = null;
        return;
      }

      reanalyzeRunning = true;
      maintenanceRunning = 'reanalyze';
      reanalyzeProgress = {
        current: 0,
        total: resetResult.reset_count,
        fnord_id: 0,
        title: $_('batch.starting'),
        success: true,
        error: null
      };

      progressUnlisten = await listen<BatchProgress>('batch-progress', (event) => {
        reanalyzeProgress = { ...event.payload };
      });

      const batchResult = await invoke<BatchResult>('process_batch', {
        model,
        limit: null
      });

      reanalyzeResult = batchResult;
      maintenanceResult = $_('settings.maintenance.reanalyzeComplete', {
        values: { succeeded: batchResult.succeeded, failed: batchResult.failed }
      });

      await appState.loadFnords();
      await appState.loadPentacles();
      await appState.loadUnprocessedCount();

    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
      reanalyzeRunning = false;
      if (progressUnlisten) {
        progressUnlisten();
        progressUnlisten = null;
      }
    }
  }

  async function handleCancelReanalyze() {
    try {
      await invoke('cancel_batch');
      maintenanceResult = $_('settings.maintenance.reanalyzeCancelled');
    } catch (e) {
      console.error('Failed to cancel reanalyze:', e);
    }
  }

  async function loadKeywordStats() {
    try {
      const [lowQuality, allKeywords] = await Promise.all([
        invoke<{ id: number; name: string; quality_score: number; article_count: number }[]>('get_low_quality_keywords', { threshold: 0.3, limit: 100 }),
        invoke<{ keywords: { id: number; name: string; article_count: number; quality_score: number | null; has_embedding: boolean }[]; total_count: number }>('get_keywords', { limit: 1000, offset: 0 })
      ]);

      const withEmbeddings = allKeywords.keywords.filter(k => k.has_embedding).length;
      const qualityScores = allKeywords.keywords.filter(k => k.quality_score !== null).map(k => k.quality_score!);
      const avgQuality = qualityScores.length > 0 ? qualityScores.reduce((a, b) => a + b, 0) / qualityScores.length : 0;

      keywordStats = {
        total: allKeywords.total_count,
        with_embeddings: withEmbeddings,
        avg_quality: avgQuality,
        low_quality: lowQuality.length
      };
    } catch (e) {
      console.error('Failed to load keyword stats:', e);
    }
  }

  async function handleFindSynonyms() {
    maintenanceRunning = 'synonyms';
    maintenanceResult = null;
    try {
      const result = await invoke<{ keyword_a_id: number; keyword_a_name: string; keyword_b_id: number; keyword_b_name: string; similarity: number }[]>('find_synonym_candidates', { threshold: 0.85, limit: 20 });
      synonymCandidates = result;
      maintenanceResult = `${result.length} ${$_('settings.maintenance.candidates')} ${$_('settings.maintenance.found')}`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  async function handleMergeSynonym(keepId: number, mergeId: number, keepName: string, mergeName: string) {
    try {
      await invoke('merge_keyword_pair', { keepId, mergeId });
      synonymCandidates = synonymCandidates.filter(c =>
        !(c.keyword_a_id === keepId && c.keyword_b_id === mergeId) &&
        !(c.keyword_a_id === mergeId && c.keyword_b_id === keepId)
      );
      maintenanceResult = `"${mergeName}" → "${keepName}" ${$_('settings.maintenance.merged')}`;
      await loadKeywordStats();
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    }
  }

  async function handleDismissSynonym(keywordAId: number, keywordBId: number, nameA: string, nameB: string) {
    try {
      await invoke('dismiss_synonym_pair', { keywordAId, keywordBId });
      synonymCandidates = synonymCandidates.filter(c =>
        !(c.keyword_a_id === keywordAId && c.keyword_b_id === keywordBId) &&
        !(c.keyword_a_id === keywordBId && c.keyword_b_id === keywordAId)
      );
      maintenanceResult = `"${nameA}" ↔ "${nameB}" ${$_('settings.maintenance.dismissed')}`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="settings-view">
  <div class="settings-header">
    <h2>{$_('settings.title')}</h2>
    <button type="button" class="btn-save" onclick={handleSave}>
      {$_('settings.save')}
    </button>
  </div>

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
    <button
      type="button"
      class="tab {activeTab === 'maintenance' ? 'active' : ''}"
      onclick={() => { activeTab = 'maintenance'; maintenanceResult = null; loadKeywordStats(); }}
    >
      {$_('settings.maintenance.title')}
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

      <!-- Dark Theme Dropdown -->
      <div class="setting-group">
        <span class="label">{$_('settings.darkTheme')}</span>
        <div class="custom-select">
          <button type="button" class="select-trigger" aria-label={$_('settings.darkTheme')} onclick={toggleThemeDropdown}>
            <span>{$_(getDarkThemeLabelKey(selectedDarkTheme))}</span>
            <span class="arrow">{themeDropdownOpen ? '▲' : '▼'}</span>
          </button>
          {#if themeDropdownOpen}
            <div class="select-options">
              {#each darkThemeOptions as option (option.value)}
                <button
                  type="button"
                  class="select-option {selectedDarkTheme === option.value ? 'selected' : ''}"
                  onclick={() => selectDarkTheme(option.value)}
                >
                  {$_(option.labelKey)}
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <p class="setting-description">{$_('settings.themeDescription')}</p>
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

      <!-- Log Level (Dev Mode) -->
      {#if import.meta.env.DEV}
        <div class="setting-group">
          <span class="label">{$_('settings.logLevel')}</span>
          <div class="custom-select">
            <button type="button" class="select-trigger" aria-label={$_('settings.logLevel')} onclick={toggleLogLevelDropdown}>
              <span class="log-level-display">
                <span class="log-level-badge {selectedLogLevel}">{selectedLogLevel.toUpperCase()}</span>
              </span>
              <span class="arrow">{logLevelDropdownOpen ? '▲' : '▼'}</span>
            </button>
            {#if logLevelDropdownOpen}
              <div class="select-options">
                {#each logLevelOptions as option (option.value)}
                  <button
                    type="button"
                    class="select-option {selectedLogLevel === option.value ? 'selected' : ''}"
                    onclick={() => selectLogLevel(option.value)}
                  >
                    <span class="log-level-badge {option.value}">{option.label}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          <p class="setting-description">{$_('settings.logLevelDescription')}</p>
        </div>
      {/if}

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
        <!-- Loaded Models Display -->
        <div class="setting-group">
          <span class="label">{$_('settings.ollama.loadedModels') || 'Geladene Modelle (VRAM)'}</span>
          <div class="loaded-models">
            {#if loadedModels.length === 0}
              <div class="no-models">{$_('settings.ollama.noLoadedModels') || 'Keine Modelle geladen'}</div>
            {:else}
              {#each loadedModels as model}
                <div class="loaded-model">
                  <span class="model-name">{model.name}</span>
                  <span class="model-info">{model.parameter_size} · {formatBytes(model.size_vram)}</span>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <!-- Main Model Selection -->
        <div class="setting-group">
          <span class="label">{$_('settings.ollama.mainModel')}</span>
          <div class="model-row">
            <div class="custom-select model-select">
              <button type="button" class="select-trigger" onclick={toggleMainModelDropdown}>
                <span>
                  {selectedMainModel || $_('settings.ollama.noModels')}
                  {#if isModelLoaded(selectedMainModel)}
                    <span class="loaded-badge">●</span>
                  {/if}
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
                      {#if isModelLoaded(model)}
                        <span class="loaded-badge">●</span>
                      {/if}
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

    {:else if activeTab === 'maintenance'}
      <!-- Confirmation Dialog -->
      {#if confirmAction}
        <div class="confirm-overlay">
          <div class="confirm-dialog">
            <p class="confirm-message">
              {#if confirmAction === 'prune'}
                {$_('settings.maintenance.confirmPrune')}
              {:else if confirmAction === 'reset'}
                {$_('settings.maintenance.confirmReset')}
              {/if}
            </p>
            <div class="confirm-actions">
              <button type="button" class="btn-secondary" onclick={cancelConfirmation}>
                {$_('confirm.no')}
              </button>
              <button
                type="button"
                class="btn-danger-solid"
                onclick={confirmAction === 'prune' ? handlePruneLowQuality : handleResetForReprocessing}
              >
                {$_('confirm.yes')}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Keyword Statistics -->
      {#if keywordStats}
        <div class="keyword-stats">
          <h3>{$_('settings.maintenance.stats')}</h3>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="stat-value">{keywordStats.total}</span>
              <span class="stat-label">{$_('settings.maintenance.totalKeywords')}</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{keywordStats.with_embeddings}</span>
              <span class="stat-label">{$_('settings.maintenance.withEmbeddings')}</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{keywordStats.avg_quality.toFixed(2)}</span>
              <span class="stat-label">{$_('settings.maintenance.avgQuality')}</span>
            </div>
            <div class="stat-item">
              <span class="stat-value {keywordStats.low_quality > 0 ? 'warning' : ''}">{keywordStats.low_quality}</span>
              <span class="stat-label">{$_('settings.maintenance.lowQuality')}</span>
            </div>
          </div>
        </div>
      {/if}

      <h3>{$_('settings.maintenance.keywordQuality')}</h3>

      {#if maintenanceResult}
        <div class="maintenance-result">
          {$_('settings.maintenance.result')}: {maintenanceResult}
        </div>
      {/if}

      <div class="maintenance-actions">
        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title">{$_('settings.maintenance.calculateScores')}</span>
            <p class="action-desc">{$_('settings.maintenance.calculateScoresDesc')}</p>
          </div>
          <button
            type="button"
            class="btn-action"
            onclick={handleCalculateScores}
            disabled={maintenanceRunning !== null}
          >
            {maintenanceRunning === 'scores' ? $_('settings.maintenance.running') : $_('settings.maintenance.calculateScores')}
          </button>
        </div>

        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title">{$_('settings.maintenance.generateEmbeddings')}</span>
            <p class="action-desc">{$_('settings.maintenance.generateEmbeddingsDesc')}</p>
          </div>
          <button
            type="button"
            class="btn-action"
            onclick={handleGenerateEmbeddings}
            disabled={maintenanceRunning !== null || !ollamaStatus?.available}
          >
            {maintenanceRunning === 'embeddings' ? $_('settings.maintenance.running') : $_('settings.maintenance.generateEmbeddings')}
          </button>
        </div>

        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title">{$_('settings.maintenance.findSynonyms')}</span>
            <p class="action-desc">{$_('settings.maintenance.findSynonymsDesc')}</p>
          </div>
          <button
            type="button"
            class="btn-action"
            onclick={handleFindSynonyms}
            disabled={maintenanceRunning !== null}
          >
            {maintenanceRunning === 'synonyms' ? $_('settings.maintenance.running') : $_('settings.maintenance.findSynonyms')}
          </button>
        </div>

        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title">{$_('settings.maintenance.pruneLowQuality')}</span>
            <p class="action-desc">{$_('settings.maintenance.pruneLowQualityDesc')}</p>
          </div>
          <button
            type="button"
            class="btn-action btn-danger"
            onclick={showPruneConfirmation}
            disabled={maintenanceRunning !== null}
          >
            {maintenanceRunning === 'prune' ? $_('settings.maintenance.running') : $_('settings.maintenance.pruneLowQuality')}
          </button>
        </div>
      </div>

      <!-- Synonym Candidates -->
      {#if synonymCandidates.length > 0}
        <h3 style="margin-top: 1.5rem;">{$_('settings.maintenance.synonymCandidates')}</h3>
        <div class="synonym-list">
          {#each synonymCandidates as candidate}
            <div class="synonym-item">
              <div class="synonym-pair">
                <span class="synonym-name">{candidate.keyword_a_name}</span>
                <span class="synonym-similarity">≈ {(candidate.similarity * 100).toFixed(0)}%</span>
                <span class="synonym-name">{candidate.keyword_b_name}</span>
              </div>
              <div class="synonym-actions">
                <button
                  type="button"
                  class="btn-merge"
                  onclick={() => handleMergeSynonym(candidate.keyword_a_id, candidate.keyword_b_id, candidate.keyword_a_name, candidate.keyword_b_name)}
                  title="{$_('settings.maintenance.keep')} '{candidate.keyword_a_name}'"
                >
                  ← {$_('settings.maintenance.merge')}
                </button>
                <button
                  type="button"
                  class="btn-merge"
                  onclick={() => handleMergeSynonym(candidate.keyword_b_id, candidate.keyword_a_id, candidate.keyword_b_name, candidate.keyword_a_name)}
                  title="{$_('settings.maintenance.keep')} '{candidate.keyword_b_name}'"
                >
                  {$_('settings.maintenance.merge')} →
                </button>
                <button
                  type="button"
                  class="btn-dismiss"
                  onclick={() => handleDismissSynonym(candidate.keyword_a_id, candidate.keyword_b_id, candidate.keyword_a_name, candidate.keyword_b_name)}
                  title={$_('settings.maintenance.dismiss')}
                >
                  ✕
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

      <h3 style="margin-top: 1.5rem;">{$_('settings.maintenance.reprocessArticles')}</h3>

      <div class="maintenance-actions">
        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title">{$_('settings.maintenance.resetForReprocessing')}</span>
            <p class="action-desc">{$_('settings.maintenance.resetForReprocessingDesc')}</p>
          </div>
          {#if !reanalyzeRunning}
            <button
              type="button"
              class="btn-action btn-danger"
              onclick={showResetConfirmation}
              disabled={maintenanceRunning !== null}
            >
              {maintenanceRunning === 'reset' ? $_('settings.maintenance.running') : $_('settings.maintenance.resetForReprocessing')}
            </button>
          {/if}
        </div>

        {#if reanalyzeRunning && reanalyzeProgress}
          <div class="reanalyze-progress">
            <div class="progress-header">
              <span class="progress-label">{$_('settings.maintenance.reanalyzing')}</span>
              <button
                type="button"
                class="btn-cancel-small"
                onclick={handleCancelReanalyze}
              >
                {$_('batch.cancel')}
              </button>
            </div>
            <div class="progress-bar">
              <div
                class="progress-fill"
                style="width: {reanalyzeProgress.total > 0 ? (reanalyzeProgress.current / reanalyzeProgress.total) * 100 : 0}%"
              ></div>
            </div>
            <div class="progress-details">
              <span class="progress-count">
                {reanalyzeProgress.current} / {reanalyzeProgress.total}
              </span>
              <span class="progress-title" title={reanalyzeProgress.title}>
                {reanalyzeProgress.title.length > 40
                  ? reanalyzeProgress.title.slice(0, 40) + "..."
                  : reanalyzeProgress.title}
              </span>
            </div>
            {#if !reanalyzeProgress.success && reanalyzeProgress.error}
              <div class="progress-error">{reanalyzeProgress.error}</div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    background-color: var(--bg-surface);
    overflow: hidden;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .settings-header h2 {
    margin: 0;
    font-size: 1.25rem;
    color: var(--accent-primary);
  }

  .btn-save {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-save:hover {
    filter: brightness(1.1);
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
    padding: 0 1.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .tab {
    padding: 0.75rem 1rem;
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
    padding: 1.5rem;
  }

  .setting-group {
    margin-bottom: 1.25rem;
    max-width: 600px;
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

  .loaded-badge {
    font-size: 0.625rem;
    color: var(--status-success);
    margin-left: 0.25rem;
  }

  /* Loaded Models Display */
  .loaded-models {
    padding: 0.5rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .no-models {
    color: var(--text-muted);
    font-size: 0.875rem;
    font-style: italic;
  }

  .loaded-model {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.375rem 0;
    border-bottom: 1px solid var(--border-default);
  }

  .loaded-model:last-child {
    border-bottom: none;
  }

  .model-name {
    font-weight: 500;
    color: var(--text-primary);
  }

  .model-info {
    font-size: 0.75rem;
    color: var(--text-muted);
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

  .maintenance-result {
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
    margin-bottom: 1rem;
    font-size: 0.875rem;
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

  /* Reanalyze Progress */
  .reanalyze-progress {
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .progress-label {
    font-weight: 500;
    color: var(--accent-primary);
  }

  .btn-cancel-small {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--status-error);
    border-radius: 0.25rem;
    background: none;
    color: var(--status-error);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-cancel-small:hover {
    background-color: var(--status-error);
    color: var(--text-on-accent);
  }

  .progress-bar {
    height: 8px;
    background-color: var(--bg-surface);
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 0.5rem;
  }

  .progress-fill {
    height: 100%;
    background-color: var(--accent-primary);
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .progress-details {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .progress-count {
    font-weight: 500;
    color: var(--text-primary);
  }

  .progress-title {
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
    text-align: right;
  }

  .progress-error {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.25rem;
    color: var(--status-error);
    font-size: 0.75rem;
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

  .keyword-stats {
    margin-bottom: 1.5rem;
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
  }

  .keyword-stats h3 {
    margin: 0 0 0.75rem 0;
    font-size: 0.875rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
  }

  .stat-item {
    text-align: center;
  }

  .stat-value {
    display: block;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--accent-primary);
  }

  .stat-value.warning {
    color: var(--status-warning);
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .synonym-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 200px;
    overflow-y: auto;
  }

  .synonym-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .synonym-pair {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
  }

  .synonym-name {
    font-weight: 500;
    color: var(--text-primary);
  }

  .synonym-similarity {
    font-size: 0.75rem;
    color: var(--text-muted);
    padding: 0.125rem 0.375rem;
    background-color: var(--bg-muted);
    border-radius: 0.25rem;
  }

  .synonym-actions {
    display: flex;
    gap: 0.25rem;
  }

  .btn-merge {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--accent-secondary);
    border-radius: 0.25rem;
    background: none;
    color: var(--accent-secondary);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-merge:hover {
    background-color: var(--accent-secondary);
    color: var(--text-on-accent);
  }

  .btn-dismiss {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--text-muted);
    border-radius: 0.25rem;
    background: none;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-dismiss:hover {
    border-color: var(--status-error);
    color: var(--status-error);
  }

  /* Log Level Badges */
  .log-level-display {
    display: flex;
    align-items: center;
  }

  .log-level-badge {
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 600;
    font-family: monospace;
  }

  .log-level-badge.error {
    background-color: rgba(243, 139, 168, 0.2);
    color: var(--status-error);
  }

  .log-level-badge.warn {
    background-color: rgba(249, 226, 175, 0.2);
    color: var(--status-warning);
  }

  .log-level-badge.info {
    background-color: rgba(137, 220, 235, 0.2);
    color: #89dceb;
  }

  .log-level-badge.debug {
    background-color: rgba(203, 166, 247, 0.2);
    color: #cba6f7;
  }

  .log-level-badge.trace {
    background-color: rgba(108, 112, 134, 0.2);
    color: var(--text-muted);
  }
</style>
