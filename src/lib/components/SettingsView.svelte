<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type { BatchProgress, BatchResult } from "../types";
  import {
    settings,
    type DarkTheme,
    type LightTheme,
    type ThemeMode,
  } from "../stores/settings.svelte";
  import { type LogLevel } from "../logger";
  import { setLocale, locale } from "../i18n";
  import { appState, toasts } from "../stores/state.svelte";
  import { onMount, onDestroy } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
  import type { OpmlFeedPreview, OpmlImportResult } from "../types";
  import Tabs, { type Tab } from "./Tabs.svelte";

  // Local state for form
  let selectedLocale = $state("de");
  let showTooltips = $state(true);
  let selectedThemeMode = $state<ThemeMode>("system");
  let selectedDarkTheme = $state<DarkTheme>("mocha");
  let selectedLightTheme = $state<LightTheme>("latte");
  let syncInterval = $state(30);
  let syncOnStart = $state(true);
  let selectedLogLevel = $state<LogLevel>("info");

  // Ollama state
  let ollamaStatus = $state<{
    available: boolean;
    models: string[];
    recommended_main: string;
    recommended_embedding: string;
    has_recommended_main: boolean;
    has_recommended_embedding: boolean;
  } | null>(null);
  let loadedModels = $state<
    {
      name: string;
      size: number;
      size_vram: number;
      parameter_size: string;
    }[]
  >([]);
  let selectedMainModel = $state("");
  let selectedEmbeddingModel = $state("");
  let downloadingModel = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let loadingModels = $state(false);
  // Hardware Profiles
  interface HardwareProfile {
    id: string;
    name: string;
    description: string;
    ai_parallelism: number;
  }

  let hardwareProfiles = $state<HardwareProfile[]>([]);
  let selectedProfileId = $state("default");
  let profileDropdownOpen = $state(false);

  // Context length (num_ctx)
  const DEFAULT_NUM_CTX = 4096;
  let ollamaNumCtx = $state(DEFAULT_NUM_CTX);
  const numCtxOptions = [
    { value: 2048, label: "2K", desc: "Minimal - sehr schnell" },
    { value: 4096, label: "4K", desc: "Standard - empfohlen" },
    { value: 8192, label: "8K", desc: "Erweitert - mehr VRAM" },
    { value: 16384, label: "16K", desc: "Groß - hoher VRAM-Bedarf" },
    { value: 32768, label: "32K", desc: "Maximum - sehr hoher VRAM-Bedarf" },
  ];

  // Prompts state
  // Prompts state
  let summaryPrompt = $state("");
  let analysisPrompt = $state("");
  let defaultPrompts = $state<{
    summary_prompt: string;
    analysis_prompt: string;
  } | null>(null);
  let promptsModified = $state(false);

  // Dropdown open states
  let langDropdownOpen = $state(false);
  let themeDropdownOpen = $state(false);
  let lightThemeDropdownOpen = $state(false);
  let logLevelDropdownOpen = $state(false);
  let mainModelDropdownOpen = $state(false);
  let embeddingModelDropdownOpen = $state(false);
  let numCtxDropdownOpen = $state(false);

  // Tab state
  let activeTab = $state<string>("general");

  // Tabs definition
  let tabs = $derived<Tab[]>([
    { id: "general", label: $_("settings.title") },
    { id: "ollama", label: "Ollama" },
    { id: "prompts", label: "Prompts" },
    { id: "stopwords", label: $_("settings.stopwords.title") },
    { id: "maintenance", label: $_("settings.maintenance.title") },
  ]);

  function handleTabChange(tabId: string) {
    if (tabId === "maintenance") {
      maintenanceResult = null;
      loadKeywordStats();
    } else if (tabId === "stopwords") {
      loadStopwordStats();
      loadUserStopwords();
    }
  }

  // Stopword state
  interface UserStopword {
    word: string;
    added_at: string | null;
  }

  interface StopwordSearchResult {
    word: string;
    is_builtin: boolean;
  }

  interface StopwordStats {
    builtin_count: number;
    user_count: number;
    total_count: number;
  }

  let stopwordStats = $state<StopwordStats | null>(null);
  let userStopwords = $state<UserStopword[]>([]);
  let stopwordSearchQuery = $state("");
  let stopwordSearchResults = $state<StopwordSearchResult[]>([]);
  let newStopword = $state("");
  let confirmClearStopwords = $state(false);
  let stopwordLoading = $state(false);

  // Maintenance state
  let maintenanceRunning = $state<string | null>(null);
  let maintenanceResult = $state<string | null>(null);

  // Confirmation dialog state
  let confirmAction = $state<"prune" | "reset" | null>(null);

  // Keyword statistics state
  let keywordStats = $state<{
    total: number;
    with_embeddings: number;
    avg_quality: number;
    low_quality: number;
  } | null>(null);

  // Reanalyze progress state
  let reanalyzeProgress = $state<BatchProgress | null>(null);
  let reanalyzeRunning = $state(false);
  let reanalyzeResult = $state<BatchResult | null>(null);
  let progressUnlisten: UnlistenFn | null = null;

  // Statistical analysis progress state
  let statisticalProgress = $state<BatchProgress | null>(null);
  let statisticalRunning = $state(false);
  let statisticalUnlisten: UnlistenFn | null = null;

  // OPML Import state
  let opmlPreview = $state<OpmlFeedPreview[]>([]);
  let opmlContent = $state<string | null>(null);
  let opmlImporting = $state(false);
  let opmlResult = $state<OpmlImportResult | null>(null);
  let opmlError = $state<string | null>(null);

  // OPML Export state
  let opmlExporting = $state(false);
  let opmlExportResult = $state<string | null>(null);
  let opmlExportError = $state<string | null>(null);

  const localeOptions = [
    { value: "de", labelKey: "settings.languageGerman" },
    { value: "en", labelKey: "settings.languageEnglish" },
  ];

  const themeModeOptions: { value: ThemeMode; labelKey: string }[] = [
    { value: "light", labelKey: "settings.themeModeLight" },
    { value: "dark", labelKey: "settings.themeModeDark" },
    { value: "system", labelKey: "settings.themeModeSystem" },
  ];

  // Dark theme options grouped by family
  interface ThemeOption<T> {
    value: T;
    labelKey: string;
    family: string;
  }

  const darkThemeOptions: ThemeOption<DarkTheme>[] = [
    // Catppuccin
    { value: "mocha", labelKey: "settings.themes.mocha", family: "catppuccin" },
    {
      value: "macchiato",
      labelKey: "settings.themes.macchiato",
      family: "catppuccin",
    },
    {
      value: "frappe",
      labelKey: "settings.themes.frappe",
      family: "catppuccin",
    },
    // Ayu
    { value: "ayu-dark", labelKey: "settings.themes.ayu-dark", family: "ayu" },
    {
      value: "ayu-mirage",
      labelKey: "settings.themes.ayu-mirage",
      family: "ayu",
    },
    // Gruvbox
    {
      value: "gruvbox-dark",
      labelKey: "settings.themes.gruvbox-dark",
      family: "gruvbox",
    },
    // Tokyo Night
    {
      value: "tokyo-night",
      labelKey: "settings.themes.tokyo-night",
      family: "tokyoNight",
    },
    {
      value: "tokyo-storm",
      labelKey: "settings.themes.tokyo-storm",
      family: "tokyoNight",
    },
    // Solarized
    {
      value: "solarized-dark",
      labelKey: "settings.themes.solarized-dark",
      family: "solarized",
    },
  ];

  const lightThemeOptions: ThemeOption<LightTheme>[] = [
    // Catppuccin
    { value: "latte", labelKey: "settings.themes.latte", family: "catppuccin" },
    // Ayu
    { value: "ayu-light", labelKey: "settings.themes.ayu-light", family: "ayu" },
    // Gruvbox
    {
      value: "gruvbox-light",
      labelKey: "settings.themes.gruvbox-light",
      family: "gruvbox",
    },
    // Tokyo Night
    {
      value: "tokyo-day",
      labelKey: "settings.themes.tokyo-day",
      family: "tokyoNight",
    },
    // Solarized
    {
      value: "solarized-light",
      labelKey: "settings.themes.solarized-light",
      family: "solarized",
    },
  ];

  const themeFamilies = [
    { id: "catppuccin", labelKey: "settings.themeFamily.catppuccin" },
    { id: "ayu", labelKey: "settings.themeFamily.ayu" },
    { id: "gruvbox", labelKey: "settings.themeFamily.gruvbox" },
    { id: "tokyoNight", labelKey: "settings.themeFamily.tokyoNight" },
    { id: "solarized", labelKey: "settings.themeFamily.solarized" },
  ];

  const logLevelOptions: { value: LogLevel; label: string }[] = [
    { value: "error", label: "Error" },
    { value: "warn", label: "Warn" },
    { value: "info", label: "Info" },
    { value: "debug", label: "Debug" },
    { value: "trace", label: "Trace" },
  ];

  onMount(async () => {
    // Initialize from current settings
    selectedLocale = $locale || "de";
    showTooltips = settings.showTerminologyTooltips;
    selectedThemeMode = settings.themeMode;
    selectedDarkTheme = settings.darkTheme;
    selectedLightTheme = settings.lightTheme;
    syncInterval = settings.syncInterval;
    syncOnStart = settings.syncOnStart;
    selectedLogLevel = settings.logLevel;
    // Load Ollama status and prompts
    await loadOllamaStatus();
    await loadHardwareProfiles();
    await loadPrompts();
    // Load num_ctx setting
    const savedNumCtx = await invoke<string | null>("get_setting", {
      key: "ollama_num_ctx",
    });
    if (savedNumCtx) {
      ollamaNumCtx = parseInt(savedNumCtx) || DEFAULT_NUM_CTX;
    }
  });

  onDestroy(() => {
    if (progressUnlisten) {
      progressUnlisten();
    }
  });

  async function loadOllamaStatus() {
    try {
      ollamaStatus = await invoke("check_ollama");
      const savedMainModel = await invoke<string | null>("get_setting", {
        key: "main_model",
      });
      const savedEmbeddingModel = await invoke<string | null>("get_setting", {
        key: "embedding_model",
      });

      if (ollamaStatus) {
        selectedMainModel = savedMainModel || ollamaStatus.recommended_main;
        selectedEmbeddingModel =
          savedEmbeddingModel || ollamaStatus.recommended_embedding;
        appState.ollamaStatus = ollamaStatus;
      }

      await loadLoadedModels();
    } catch (e) {
      console.error("Failed to load Ollama status:", e);
      ollamaStatus = null;
    }
  }

  async function loadLoadedModels() {
    try {
      const response = await invoke<{ models: typeof loadedModels }>(
        "get_loaded_models",
      );
      loadedModels = response.models;
    } catch (e) {
      console.error("Failed to load loaded models:", e);
      loadedModels = [];
    }
  }

  async function loadHardwareProfiles() {
    try {
      hardwareProfiles = await invoke<HardwareProfile[]>(
        "get_hardware_profiles",
      );
      const active = await invoke<string | null>("get_setting", {
        key: "active_hardware_profile",
      });
      selectedProfileId = active || "default";
    } catch (e) {
      console.error("Failed to load hardware profiles:", e);
    }
  }

  function formatBytes(bytes: number): string {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(1)} GB`;
  }

  function isModelLoaded(modelName: string): boolean {
    return loadedModels.some((m) => m.name === modelName);
  }

  async function loadPrompts() {
    try {
      const prompts = await invoke<{
        summary_prompt: string;
        analysis_prompt: string;
      }>("get_prompts");
      summaryPrompt = prompts.summary_prompt;
      analysisPrompt = prompts.analysis_prompt;

      defaultPrompts = await invoke<{
        summary_prompt: string;
        analysis_prompt: string;
      }>("get_default_prompts");

      if (defaultPrompts) {
        promptsModified =
          summaryPrompt !== defaultPrompts.summary_prompt ||
          analysisPrompt !== defaultPrompts.analysis_prompt;
      }
    } catch (e) {
      console.error("Failed to load prompts:", e);
    }
  }

  function closeAllDropdowns() {
    langDropdownOpen = false;
    themeDropdownOpen = false;
    logLevelDropdownOpen = false;
    mainModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
    numCtxDropdownOpen = false;
  }

  // Auto-save handlers for individual settings
  function handleTooltipsChange(checked: boolean) {
    showTooltips = checked;
    settings.showTerminologyTooltips = checked;
  }

  function handleSyncIntervalChange(value: number) {
    syncInterval = value;
    settings.syncInterval = value;
  }

  function handleSyncOnStartChange(checked: boolean) {
    syncOnStart = checked;
    settings.syncOnStart = checked;
  }

  async function handleNumCtxChange(value: number) {
    ollamaNumCtx = value;
    numCtxDropdownOpen = false;
    try {
      await invoke("set_setting", {
        key: "ollama_num_ctx",
        value: value.toString(),
      });
    } catch (e) {
      console.error("Failed to save num_ctx setting:", e);
      toasts.error($_('settings.saveError'));
    }
  }

  // Load models into VRAM (separate from saving preferences)
  async function handleLoadModels() {
    if (!selectedMainModel || !selectedEmbeddingModel) return;

    loadingModels = true;
    try {
      await invoke("ensure_models_loaded", {
        mainModel: selectedMainModel,
        embeddingModel: selectedEmbeddingModel,
      });
      await emit("models-changed");
      toasts.success($_('settings.modelsLoaded'));
    } catch (e) {
      console.error("Failed to load models:", e);
      toasts.error($_('settings.modelsLoadError'));
    } finally {
      loadingModels = false;
    }
  }

  // Save prompts (called by individual OK buttons)
  async function handleSavePrompts() {
    try {
      await invoke("set_prompts", {
        summaryPrompt: summaryPrompt,
        analysisPrompt: analysisPrompt,
      });
      promptsModified = false;
      toasts.success($_('settings.promptsSaved'));
    } catch (e) {
      console.error("Failed to save prompts:", e);
      toasts.error($_('settings.saveError'));
    }
  }

  async function handleResetPrompts() {
    try {
      const prompts = await invoke<{
        summary_prompt: string;
        analysis_prompt: string;
      }>("reset_prompts");
      summaryPrompt = prompts.summary_prompt;
      analysisPrompt = prompts.analysis_prompt;
      promptsModified = false;
    } catch (e) {
      console.error("Failed to reset prompts:", e);
    }
  }

  async function handleDownloadModel(model: string) {
    if (downloadingModel) return;

    downloadingModel = model;
    downloadError = null;

    try {
      const result = await invoke<{ success: boolean; error: string | null }>(
        "pull_model",
        { model },
      );
      if (result.success) {
        await loadOllamaStatus();
      } else {
        downloadError = result.error || "Unknown error";
      }
    } catch (e) {
      downloadError = String(e);
    } finally {
      downloadingModel = null;
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeAllDropdowns();
    }
  }

  async function selectLocale(value: string) {
    selectedLocale = value;
    langDropdownOpen = false;
    // Auto-save
    await setLocale(value);
  }

  function selectThemeMode(value: ThemeMode) {
    selectedThemeMode = value;
    // Auto-save
    settings.themeMode = value;
  }

  function selectDarkTheme(value: DarkTheme) {
    selectedDarkTheme = value;
    themeDropdownOpen = false;
    // Auto-save
    settings.darkTheme = value;
  }

  function selectLightTheme(value: LightTheme) {
    selectedLightTheme = value;
    lightThemeDropdownOpen = false;
    // Auto-save
    settings.lightTheme = value;
  }

  function selectLogLevel(value: LogLevel) {
    selectedLogLevel = value;
    logLevelDropdownOpen = false;
    // Auto-save
    settings.logLevel = value;
  }

  async function selectMainModel(value: string) {
    selectedMainModel = value;
    mainModelDropdownOpen = false;
    // Auto-save model preference
    try {
      await invoke("set_setting", { key: "main_model", value });
      appState.selectedModel = value;
    } catch (e) {
      console.error("Failed to save main model setting:", e);
      toasts.error($_('settings.saveError'));
    }
  }

  async function selectEmbeddingModel(value: string) {
    selectedEmbeddingModel = value;
    embeddingModelDropdownOpen = false;
    // Auto-save model preference
    try {
      await invoke("set_setting", { key: "embedding_model", value });
    } catch (e) {
      console.error("Failed to save embedding model setting:", e);
      toasts.error($_('settings.saveError'));
    }
  }

  function toggleLangDropdown() {
    langDropdownOpen = !langDropdownOpen;
    themeDropdownOpen = false;
  }

  function toggleThemeDropdown() {
    themeDropdownOpen = !themeDropdownOpen;
    langDropdownOpen = false;
    lightThemeDropdownOpen = false;
    logLevelDropdownOpen = false;
  }

  function toggleLightThemeDropdown() {
    lightThemeDropdownOpen = !lightThemeDropdownOpen;
    langDropdownOpen = false;
    themeDropdownOpen = false;
    logLevelDropdownOpen = false;
  }

  function toggleLogLevelDropdown() {
    logLevelDropdownOpen = !logLevelDropdownOpen;
    langDropdownOpen = false;
    themeDropdownOpen = false;
    lightThemeDropdownOpen = false;
  }

  function toggleMainModelDropdown() {
    mainModelDropdownOpen = !mainModelDropdownOpen;
    embeddingModelDropdownOpen = false;
  }

  function toggleEmbeddingModelDropdown() {
    embeddingModelDropdownOpen = !embeddingModelDropdownOpen;
    mainModelDropdownOpen = false;
  }

  function toggleProfileDropdown() {
    profileDropdownOpen = !profileDropdownOpen;
    mainModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
  }

  async function handleProfileSelect(profileId: string) {
    selectedProfileId = profileId;
    profileDropdownOpen = false;
    try {
      await invoke("apply_hardware_profile", { profileId });
      toasts.success($_('settings.profileApplied'));
    } catch (e) {
      console.error("Failed to apply profile:", e);
      toasts.error($_('settings.saveError'));
    }
  }

  function getLocaleLabelKey(value: string): string {
    return localeOptions.find((o) => o.value === value)?.labelKey || "";
  }

  function getDarkThemeLabelKey(value: DarkTheme): string {
    return darkThemeOptions.find((o) => o.value === value)?.labelKey || "";
  }

  function getThemeDisplayName<T extends string>(
    value: T,
    options: ThemeOption<T>[]
  ): string {
    const option = options.find((o) => o.value === value);
    if (!option) return value;

    const family = themeFamilies.find((f) => f.id === option.family);
    const familyName = family ? $_(family.labelKey) : "";
    const themeName = $_(option.labelKey);

    return `${familyName} ${themeName}`;
  }

  function isRecommendedModel(model: string, recommended: string): boolean {
    return (
      model === recommended || model.startsWith(recommended.split(":")[0] + ":")
    );
  }

  function handlePromptChange() {
    if (defaultPrompts) {
      promptsModified =
        summaryPrompt !== defaultPrompts.summary_prompt ||
        analysisPrompt !== defaultPrompts.analysis_prompt;
    }
  }

  async function handleCalculateScores() {
    maintenanceRunning = "scores";
    maintenanceResult = null;
    try {
      const result = await invoke<{
        updated_count: number;
        avg_score: number;
        low_quality_count: number;
      }>("calculate_keyword_quality_scores", { limit: 1000 });
      maintenanceResult = `${result.updated_count} ${$_("settings.maintenance.updated")} (Ø ${result.avg_score.toFixed(2)})`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  async function handleGenerateEmbeddings() {
    maintenanceRunning = "embeddings";
    maintenanceResult = null;
    try {
      // Queue keywords for embedding generation - the background worker handles the rest
      const queuedCount = await invoke<number>("queue_missing_embeddings");
      maintenanceResult = `${queuedCount} ${$_("settings.maintenance.queued")}`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  async function handleStatisticalAnalysis() {
    maintenanceRunning = "statistical";
    maintenanceResult = null;
    statisticalProgress = null;

    try {
      // Get count first
      const count = await invoke<number>("get_unprocessed_statistical_count");
      if (count === 0) {
        maintenanceResult = $_("settings.maintenance.noUnprocessedArticles");
        maintenanceRunning = null;
        return;
      }

      // Set up progress tracking
      statisticalRunning = true;
      statisticalProgress = {
        current: 0,
        total: count,
        fnord_id: 0,
        title: $_("batch.starting"),
        success: true,
        error: null,
      };

      // Listen for progress events
      statisticalUnlisten = await listen<BatchProgress>(
        "statistical-progress",
        (event) => {
          statisticalProgress = { ...event.payload };
        },
      );

      // Run statistical analysis
      const result = await invoke<{
        processed: number;
        total: number;
        errors: string[];
      }>("process_statistical_batch", { limit: 10000 });

      maintenanceResult = `${result.processed} ${$_("settings.maintenance.articlesAnalyzed")}`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
      statisticalRunning = false;
      if (statisticalUnlisten) {
        statisticalUnlisten();
        statisticalUnlisten = null;
      }
    }
  }

  function showPruneConfirmation() {
    confirmAction = "prune";
  }

  function showResetConfirmation() {
    confirmAction = "reset";
  }

  function cancelConfirmation() {
    confirmAction = null;
  }

  async function handlePruneLowQuality() {
    confirmAction = null;
    maintenanceRunning = "prune";
    maintenanceResult = null;
    try {
      const result = await invoke<{
        pruned_count: number;
        pruned_keywords: string[];
      }>("auto_prune_low_quality", {
        quality_threshold: 0.2,
        min_age_days: 7,
        dry_run: false,
      });
      maintenanceResult = `${result.pruned_count} ${$_("settings.maintenance.pruned")}`;
      await loadKeywordStats();
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  async function handleResetForReprocessing() {
    confirmAction = null;
    maintenanceRunning = "reset";
    maintenanceResult = null;
    reanalyzeProgress = null;
    reanalyzeResult = null;

    try {
      const resetResult = await invoke<{ reset_count: number }>(
        "reset_articles_for_reprocessing",
        {
          only_with_content: true,
        },
      );

      if (resetResult.reset_count === 0) {
        maintenanceResult = $_("settings.maintenance.noArticlesToReset");
        maintenanceRunning = null;
        return;
      }

      await emit("articles-reset");
      await appState.loadUnprocessedCount();

      const model = appState.selectedModel || appState.ollamaStatus.models[0];
      if (!model || !appState.ollamaStatus.available) {
        maintenanceResult = `${resetResult.reset_count} ${$_("settings.maintenance.articles")} ${$_("settings.maintenance.reset")}. ${$_("settings.maintenance.ollamaUnavailable")}`;
        maintenanceRunning = null;
        return;
      }

      reanalyzeRunning = true;
      maintenanceRunning = "reanalyze";
      // Also update appState so Sidebar shows progress
      appState.batchProcessing = true;
      reanalyzeProgress = {
        current: 0,
        total: resetResult.reset_count,
        fnord_id: 0,
        title: $_("batch.starting"),
        success: true,
        error: null,
      };

      progressUnlisten = await listen<BatchProgress>(
        "batch-progress",
        (event) => {
          reanalyzeProgress = { ...event.payload };
        },
      );

      const batchResult = await invoke<BatchResult>("process_batch", {
        model,
        limit: null,
      });

      reanalyzeResult = batchResult;
      maintenanceResult = $_("settings.maintenance.reanalyzeComplete", {
        values: {
          succeeded: batchResult.succeeded,
          failed: batchResult.failed,
        },
      });

      await appState.loadFnords();
      await appState.loadPentacles();
      await appState.loadUnprocessedCount();

      // Notify components that batch processing is complete (for refreshing similar articles etc.)
      window.dispatchEvent(new CustomEvent('batch-complete'));
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
      reanalyzeRunning = false;
      appState.batchProcessing = false;
      if (progressUnlisten) {
        progressUnlisten();
        progressUnlisten = null;
      }
    }
  }

  async function handleCancelReanalyze() {
    try {
      await invoke("cancel_batch");
      maintenanceResult = $_("settings.maintenance.reanalyzeCancelled");
    } catch (e) {
      console.error("Failed to cancel reanalyze:", e);
    }
  }

  // Stopword functions
  async function loadStopwordStats() {
    try {
      stopwordStats = await invoke<StopwordStats>("get_stopwords_stats");
    } catch (e) {
      console.error("Failed to load stopword stats:", e);
    }
  }

  async function loadUserStopwords() {
    try {
      userStopwords = await invoke<UserStopword[]>("get_user_stopwords");
    } catch (e) {
      console.error("Failed to load user stopwords:", e);
    }
  }

  async function searchStopwords(query: string) {
    if (query.length < 2) {
      stopwordSearchResults = [];
      return;
    }
    try {
      stopwordSearchResults = await invoke<StopwordSearchResult[]>(
        "search_stopwords",
        { query, limit: 50 }
      );
    } catch (e) {
      console.error("Failed to search stopwords:", e);
    }
  }

  async function addStopword() {
    const word = newStopword.trim().toLowerCase();
    if (word.length < 2) {
      toasts.add($_("settings.stopwords.minLength"), "error");
      return;
    }

    stopwordLoading = true;
    try {
      const added = await invoke<boolean>("add_stopword", { word });
      if (added) {
        toasts.add($_("settings.stopwords.added"), "success");
        newStopword = "";
        await Promise.all([loadStopwordStats(), loadUserStopwords()]);
      } else {
        toasts.add($_("settings.stopwords.alreadyExists"), "warning");
      }
    } catch (e) {
      toasts.add(`Error: ${e}`, "error");
    } finally {
      stopwordLoading = false;
    }
  }

  async function removeStopword(word: string) {
    stopwordLoading = true;
    try {
      await invoke<boolean>("remove_stopword", { word });
      toasts.add($_("settings.stopwords.removed"), "success");
      await Promise.all([loadStopwordStats(), loadUserStopwords()]);
    } catch (e) {
      toasts.add(`Error: ${e}`, "error");
    } finally {
      stopwordLoading = false;
    }
  }

  async function clearAllUserStopwords() {
    stopwordLoading = true;
    try {
      const deleted = await invoke<number>("clear_user_stopwords");
      toasts.add(`${$_("settings.stopwords.cleared")} (${deleted})`, "success");
      confirmClearStopwords = false;
      await Promise.all([loadStopwordStats(), loadUserStopwords()]);
    } catch (e) {
      toasts.add(`Error: ${e}`, "error");
    } finally {
      stopwordLoading = false;
    }
  }

  async function loadKeywordStats() {
    try {
      const [lowQuality, allKeywords] = await Promise.all([
        invoke<
          {
            id: number;
            name: string;
            quality_score: number;
            article_count: number;
          }[]
        >("get_low_quality_keywords", { threshold: 0.3, limit: 100 }),
        invoke<{
          keywords: {
            id: number;
            name: string;
            article_count: number;
            quality_score: number | null;
            has_embedding: boolean;
          }[];
          total_count: number;
        }>("get_keywords", { limit: 1000, offset: 0 }),
      ]);

      const withEmbeddings = allKeywords.keywords.filter(
        (k) => k.has_embedding,
      ).length;
      const qualityScores = allKeywords.keywords
        .filter((k) => k.quality_score !== null)
        .map((k) => k.quality_score!);
      const avgQuality =
        qualityScores.length > 0
          ? qualityScores.reduce((a, b) => a + b, 0) / qualityScores.length
          : 0;

      keywordStats = {
        total: allKeywords.total_count,
        with_embeddings: withEmbeddings,
        avg_quality: avgQuality,
        low_quality: lowQuality.length,
      };
    } catch (e) {
      console.error("Failed to load keyword stats:", e);
    }
  }

  // OPML Import handlers
  async function handleSelectOpmlFile() {
    opmlError = null;
    opmlResult = null;
    opmlPreview = [];
    opmlContent = null;

    try {
      const filePath = await open({
        multiple: false,
        filters: [
          {
            name: "OPML",
            extensions: ["opml", "xml"],
          },
        ],
      });

      if (!filePath) return;

      const content = await readTextFile(filePath as string);
      opmlContent = content;
      const preview = await invoke<OpmlFeedPreview[]>("parse_opml_preview", {
        content,
      });
      opmlPreview = preview;
    } catch (e) {
      opmlError = String(e);
    }
  }

  async function handleImportOpml() {
    if (opmlPreview.length === 0 || !opmlContent) return;

    opmlImporting = true;
    opmlError = null;

    try {
      const result = await invoke<OpmlImportResult>("import_opml", {
        content: opmlContent,
        skipExisting: true,
      });

      opmlResult = result;
      opmlPreview = [];
      opmlContent = null;

      // Refresh pentacles list
      await appState.loadPentacles();
    } catch (e) {
      opmlError = String(e);
    } finally {
      opmlImporting = false;
    }
  }

  function handleClearOpmlPreview() {
    opmlPreview = [];
    opmlContent = null;
    opmlResult = null;
    opmlError = null;
  }

  // OPML Export handler
  async function handleExportOpml() {
    opmlExporting = true;
    opmlExportResult = null;
    opmlExportError = null;

    try {
      // Check if there are feeds to export
      if (appState.pentacles.length === 0) {
        opmlExportError = $_("settings.opml.noFeedsToExport");
        opmlExporting = false;
        return;
      }

      // Get OPML content from backend
      const opmlContent = await invoke<string>("export_opml");

      // Open save dialog
      const filePath = await save({
        filters: [
          {
            name: "OPML",
            extensions: ["opml"],
          },
        ],
        defaultPath: "fuckupRSS-feeds.opml",
      });

      if (!filePath) {
        opmlExporting = false;
        return;
      }

      // Write to file
      await writeTextFile(filePath, opmlContent);

      opmlExportResult = $_("settings.opml.exportSuccess", {
        values: { count: appState.pentacles.length },
      });
    } catch (e) {
      opmlExportError = String(e);
    } finally {
      opmlExporting = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="settings-view">
  <div class="settings-header">
    <h2>{$_("settings.title")}</h2>
  </div>

  <!-- Tabs -->
  <div class="tabs-wrapper">
    <Tabs {tabs} bind:activeTab onchange={handleTabChange} />
  </div>

  <div class="tab-content">
    {#if activeTab === "general"}
      <!-- Language Dropdown -->
      <div class="setting-group">
        <span class="label">{$_("settings.language")}</span>
        <div class="custom-select">
          <button
            type="button"
            class="select-trigger"
            aria-label={$_("settings.language")}
            onclick={toggleLangDropdown}
          >
            <span>{$_(getLocaleLabelKey(selectedLocale))}</span>
            <i class="arrow fa-solid {langDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
          </button>
          {#if langDropdownOpen}
            <div class="select-options">
              {#each localeOptions as option}
                <button
                  type="button"
                  class="select-option {selectedLocale === option.value
                    ? 'selected'
                    : ''}"
                  onclick={() => selectLocale(option.value)}
                >
                  {$_(option.labelKey)}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <!-- Theme Mode Selection -->
      <div class="setting-group">
        <span class="label">{$_("settings.themeMode")}</span>
        <div class="theme-mode-buttons">
          {#each themeModeOptions as option (option.value)}
            <button
              type="button"
              class="theme-mode-btn {selectedThemeMode === option.value
                ? 'active'
                : ''}"
              onclick={() => selectThemeMode(option.value)}
            >
              {$_(option.labelKey)}
            </button>
          {/each}
        </div>
        <p class="setting-description">{$_("settings.themeModeDescription")}</p>
      </div>

      <!-- Dark Theme Dropdown -->
      <div class="setting-group">
        <span class="label">{$_("settings.darkTheme")}</span>
        <div class="custom-select">
          <button
            type="button"
            class="select-trigger"
            aria-label={$_("settings.darkTheme")}
            onclick={toggleThemeDropdown}
          >
            <span>{getThemeDisplayName(selectedDarkTheme, darkThemeOptions)}</span>
            <i class="arrow fa-solid {themeDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
          </button>
          {#if themeDropdownOpen}
            <div class="select-options theme-options">
              {#each themeFamilies as family (family.id)}
                {@const familyThemes = darkThemeOptions.filter(
                  (t) => t.family === family.id
                )}
                {#if familyThemes.length > 0}
                  <div class="theme-family-group">
                    <span class="theme-family-label"
                      >{$_(family.labelKey)}</span
                    >
                    {#each familyThemes as option (option.value)}
                      <button
                        type="button"
                        class="select-option {selectedDarkTheme === option.value
                          ? 'selected'
                          : ''}"
                        onclick={() => selectDarkTheme(option.value)}
                      >
                        {$_(option.labelKey)}
                      </button>
                    {/each}
                  </div>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
        <p class="setting-description">{$_("settings.darkThemeDescription")}</p>
      </div>

      <!-- Light Theme Dropdown -->
      <div class="setting-group">
        <span class="label">{$_("settings.lightTheme")}</span>
        <div class="custom-select">
          <button
            type="button"
            class="select-trigger"
            aria-label={$_("settings.lightTheme")}
            onclick={toggleLightThemeDropdown}
          >
            <span
              >{getThemeDisplayName(
                selectedLightTheme,
                lightThemeOptions
              )}</span
            >
            <i class="arrow fa-solid {lightThemeDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
          </button>
          {#if lightThemeDropdownOpen}
            <div class="select-options theme-options">
              {#each themeFamilies as family (family.id)}
                {@const familyThemes = lightThemeOptions.filter(
                  (t) => t.family === family.id
                )}
                {#if familyThemes.length > 0}
                  <div class="theme-family-group">
                    <span class="theme-family-label"
                      >{$_(family.labelKey)}</span
                    >
                    {#each familyThemes as option (option.value)}
                      <button
                        type="button"
                        class="select-option {selectedLightTheme === option.value
                          ? 'selected'
                          : ''}"
                        onclick={() => selectLightTheme(option.value)}
                      >
                        {$_(option.labelKey)}
                      </button>
                    {/each}
                  </div>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
        <p class="setting-description">{$_("settings.lightThemeDescription")}</p>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input
            type="checkbox"
            checked={showTooltips}
            onchange={(e) => handleTooltipsChange(e.currentTarget.checked)}
          />
          <span class="checkbox-label">{$_("settings.tooltips")}</span>
        </label>
        <p class="setting-description">{$_("settings.tooltipsDescription")}</p>
      </div>

      <!-- Sync Settings -->
      <div class="setting-group">
        <span class="label">{$_("settings.sync.title")}</span>
      </div>

      <div class="setting-group">
        <label class="label" for="sync-interval"
          >{$_("settings.sync.interval")}</label
        >
        <div class="slider-row">
          <input
            id="sync-interval"
            type="range"
            min="5"
            max="120"
            step="5"
            value={syncInterval}
            onchange={(e) => handleSyncIntervalChange(parseInt(e.currentTarget.value))}
            class="slider"
          />
          <span class="slider-value"
            >{$_("settings.sync.minutes", {
              values: { count: syncInterval },
            })}</span
          >
        </div>
        <p class="setting-description">
          {$_("settings.sync.intervalDescription")}
        </p>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input
            type="checkbox"
            checked={syncOnStart}
            onchange={(e) => handleSyncOnStartChange(e.currentTarget.checked)}
          />
          <span class="checkbox-label">{$_("settings.sync.onStart")}</span>
        </label>
        <p class="setting-description">
          {$_("settings.sync.onStartDescription")}
        </p>
      </div>

      <!-- OPML Import -->
      <div class="setting-group">
        <span class="label">{$_("settings.opml.title")}</span>
        <p class="setting-description">{$_("settings.opml.description")}</p>
      </div>

      <div class="opml-section">
        <button
          type="button"
          class="btn-action"
          onclick={handleSelectOpmlFile}
          disabled={opmlImporting}
        >
          {$_("settings.opml.selectFile")}
        </button>

        {#if opmlError}
          <div class="opml-error">{opmlError}</div>
        {/if}

        {#if opmlResult}
          <div class="opml-result">
            <p>
              {$_("settings.opml.importResult", {
                values: {
                  imported: opmlResult.imported,
                  skipped: opmlResult.skipped,
                  total: opmlResult.total_feeds,
                },
              })}
            </p>
            {#if opmlResult.errors.length > 0}
              <div class="opml-errors">
                {#each opmlResult.errors as error}
                  <div class="opml-error-item">{error}</div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        {#if opmlPreview.length > 0}
          <div class="opml-preview">
            <div class="opml-preview-header">
              <span>
                {$_("settings.opml.feedsFound", {
                  values: { count: opmlPreview.length },
                })}
              </span>
              <button
                type="button"
                class="btn-small"
                onclick={handleClearOpmlPreview}
              >
                {$_("settings.opml.clear")}
              </button>
            </div>
            <div class="opml-feed-list">
              {#each opmlPreview as feed}
                <div class="opml-feed-item" class:exists={feed.already_exists}>
                  <div class="opml-feed-info">
                    <span class="opml-feed-title">
                      {feed.title || feed.url}
                    </span>
                    {#if feed.category}
                      <span class="opml-feed-category">{feed.category}</span>
                    {/if}
                  </div>
                  {#if feed.already_exists}
                    <span class="opml-feed-exists">
                      {$_("settings.opml.alreadyExists")}
                    </span>
                  {/if}
                </div>
              {/each}
            </div>
            <div class="opml-preview-actions">
              <button
                type="button"
                class="btn-action"
                onclick={handleImportOpml}
                disabled={opmlImporting ||
                  opmlPreview.every((f) => f.already_exists)}
              >
                {#if opmlImporting}
                  {$_("settings.opml.importing")}
                {:else}
                  {$_("settings.opml.import")}
                {/if}
              </button>
              <span class="opml-preview-info">
                {$_("settings.opml.newFeeds", {
                  values: {
                    count: opmlPreview.filter((f) => !f.already_exists).length,
                  },
                })}
              </span>
            </div>
          </div>
        {/if}
      </div>

      <!-- OPML Export -->
      <div class="opml-section opml-export">
        <div class="export-row">
          <div class="export-info">
            <span class="export-label">{$_("settings.opml.export")}</span>
            <p class="export-desc">{$_("settings.opml.exportDescription")}</p>
          </div>
          <button
            type="button"
            class="btn-action"
            onclick={handleExportOpml}
            disabled={opmlExporting || appState.pentacles.length === 0}
          >
            {#if opmlExporting}
              {$_("settings.opml.exporting")}
            {:else}
              {$_("settings.opml.exportButton")}
            {/if}
          </button>
        </div>

        {#if opmlExportError}
          <div class="opml-error">{opmlExportError}</div>
        {/if}

        {#if opmlExportResult}
          <div class="opml-result">{opmlExportResult}</div>
        {/if}
      </div>

      <!-- Log Level (Dev Mode) -->
      {#if import.meta.env.DEV}
        <div class="setting-group">
          <span class="label">{$_("settings.logLevel")}</span>
          <div class="custom-select">
            <button
              type="button"
              class="select-trigger"
              aria-label={$_("settings.logLevel")}
              onclick={toggleLogLevelDropdown}
            >
              <span class="log-level-display">
                <span class="log-level-badge {selectedLogLevel}"
                  >{selectedLogLevel.toUpperCase()}</span
                >
              </span>
              <i class="arrow fa-solid {logLevelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
            </button>
            {#if logLevelDropdownOpen}
              <div class="select-options">
                {#each logLevelOptions as option (option.value)}
                  <button
                    type="button"
                    class="select-option {selectedLogLevel === option.value
                      ? 'selected'
                      : ''}"
                    onclick={() => selectLogLevel(option.value)}
                  >
                    <span class="log-level-badge {option.value}"
                      >{option.label}</span
                    >
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          <p class="setting-description">
            {$_("settings.logLevelDescription")}
          </p>
        </div>
      {/if}
    {:else if activeTab === "ollama"}
      <h3>{$_("settings.ollama.title")}</h3>

      <!-- Ollama Status -->
      <div class="setting-group">
        <span class="label">{$_("settings.ollama.status")}</span>
        {#if ollamaStatus === null}
          <div class="status-loading">...</div>
        {:else if ollamaStatus.available}
          <div class="status-available">
            <i class="status-icon fa-solid fa-check"></i>
            {$_("settings.ollama.available")}
          </div>
        {:else}
          <div class="status-unavailable">
            <i class="status-icon fa-solid fa-xmark"></i>
            {$_("settings.ollama.unavailable")}
            <p class="setting-description">
              {$_("settings.ollama.unavailableDescription")}
            </p>
          </div>
        {/if}
      </div>

      {#if ollamaStatus?.available}
        <!-- Loaded Models Display -->
        <div class="setting-group">
          <span class="label"
            >{$_("settings.ollama.loadedModels") ||
              "Geladene Modelle (VRAM)"}</span
          >
          <div class="loaded-models">
            {#if loadedModels.length === 0}
              <div class="no-models">
                {$_("settings.ollama.noLoadedModels") ||
                  "Keine Modelle geladen"}
              </div>
            {:else}
              {#each loadedModels as model}
                <div class="loaded-model">
                  <span class="model-name">{model.name}</span>
                  <span class="model-info"
                    >{model.parameter_size} · {formatBytes(
                      model.size_vram,
                    )}</span
                  >
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <!-- Main Model Selection -->
        <div class="setting-group">
          <span class="label">{$_("settings.ollama.mainModel")}</span>
          <div class="model-row">
            <div class="custom-select model-select">
              <button
                type="button"
                class="select-trigger"
                onclick={toggleMainModelDropdown}
              >
                <span>
                  {selectedMainModel || $_("settings.ollama.noModels")}
                  {#if isModelLoaded(selectedMainModel)}
                    <i class="loaded-badge fa-solid fa-circle"></i>
                  {/if}
                  {#if ollamaStatus && isRecommendedModel(selectedMainModel, ollamaStatus.recommended_main)}
                    <span class="recommended"
                      >{$_("settings.ollama.recommended")}</span
                    >
                  {/if}
                </span>
                <i class="arrow fa-solid {mainModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
              </button>
              {#if mainModelDropdownOpen}
                <div class="select-options">
                  {#each ollamaStatus.models as model}
                    <button
                      type="button"
                      class="select-option {selectedMainModel === model
                        ? 'selected'
                        : ''}"
                      onclick={() => selectMainModel(model)}
                    >
                      {model}
                      {#if isModelLoaded(model)}
                        <i class="loaded-badge fa-solid fa-circle"></i>
                      {/if}
                      {#if isRecommendedModel(model, ollamaStatus.recommended_main)}
                        <span class="recommended"
                          >{$_("settings.ollama.recommended")}</span
                        >
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
                onclick={() =>
                  handleDownloadModel(ollamaStatus!.recommended_main)}
                disabled={downloadingModel !== null}
              >
                {#if downloadingModel === ollamaStatus.recommended_main}
                  {$_("settings.ollama.downloading")}
                {:else}
                  {$_("settings.ollama.downloadModel")}
                  {ollamaStatus.recommended_main}
                {/if}
              </button>
            {/if}
          </div>
        </div>

        <!-- Embedding Model Selection -->
        <div class="setting-group">
          <span class="label">{$_("settings.ollama.embeddingModel")}</span>
          <div class="model-row">
            <div class="custom-select model-select">
              <button
                type="button"
                class="select-trigger"
                onclick={toggleEmbeddingModelDropdown}
              >
                <span>
                  {selectedEmbeddingModel || $_("settings.ollama.noModels")}
                  {#if ollamaStatus && isRecommendedModel(selectedEmbeddingModel, ollamaStatus.recommended_embedding)}
                    <span class="recommended"
                      >{$_("settings.ollama.recommended")}</span
                    >
                  {/if}
                </span>
                <i class="arrow fa-solid {embeddingModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
              </button>
              {#if embeddingModelDropdownOpen}
                <div class="select-options">
                  {#each ollamaStatus.models as model}
                    <button
                      type="button"
                      class="select-option {selectedEmbeddingModel === model
                        ? 'selected'
                        : ''}"
                      onclick={() => selectEmbeddingModel(model)}
                    >
                      {model}
                      {#if isRecommendedModel(model, ollamaStatus.recommended_embedding)}
                        <span class="recommended"
                          >{$_("settings.ollama.recommended")}</span
                        >
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
                onclick={() =>
                  handleDownloadModel(ollamaStatus!.recommended_embedding)}
                disabled={downloadingModel !== null}
              >
                {#if downloadingModel === ollamaStatus.recommended_embedding}
                  {$_("settings.ollama.downloading")}
                {:else}
                  {$_("settings.ollama.downloadModel")}
                  {ollamaStatus.recommended_embedding}
                {/if}
              </button>
            {/if}
          </div>
        </div>

        <!-- Load Models Button -->
        <div class="setting-group">
          <button
            type="button"
            class="btn-load-models"
            onclick={handleLoadModels}
            disabled={loadingModels || !selectedMainModel || !selectedEmbeddingModel}
          >
            {#if loadingModels}
              {$_("settings.ollama.loadingModels") || "Lade Modelle..."}
            {:else}
              {$_("settings.ollama.loadModels") || "Modelle in VRAM laden"}
            {/if}
          </button>
          <p class="setting-description">
            {$_("settings.ollama.loadModelsDescription") || "Lädt die ausgewählten Modelle in den Grafikspeicher. Die Auswahl wird automatisch gespeichert."}
          </p>
        </div>

        <!-- Hardware Profile Selection -->
        <div class="setting-group">
          <span class="label">{$_("settings.ollama.hardwareProfile")}</span>
          <div class="custom-select">
            <button
              type="button"
              class="select-trigger"
              onclick={toggleProfileDropdown}
            >
              <span>
                {hardwareProfiles.find((p) => p.id === selectedProfileId)
                  ?.name || "Default"}
                <span class="profile-parallelism">
                  ({hardwareProfiles.find((p) => p.id === selectedProfileId)
                    ?.ai_parallelism || 1}x Parallel)
                </span>
              </span>
              <i class="arrow fa-solid {profileDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
            </button>
            {#if profileDropdownOpen}
              <div class="select-options">
                {#each hardwareProfiles as profile}
                  <button
                    type="button"
                    class="select-option profile-option {selectedProfileId ===
                    profile.id
                      ? 'selected'
                      : ''}"
                    onclick={() => handleProfileSelect(profile.id)}
                  >
                    <div class="profile-info">
                      <span class="profile-name">{profile.name}</span>
                      <span class="profile-desc">{profile.description}</span>
                    </div>
                    <span class="profile-badge">{profile.ai_parallelism}x</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          <p class="setting-description">
            {$_("settings.ollama.profileDescription")}
          </p>
        </div>

        <!-- Context Length (num_ctx) -->
        <div class="setting-group">
          <span class="label">{$_("settings.ollama.contextLength") || "Kontext-Länge (num_ctx)"}</span>
          <div class="custom-select">
            <button
              type="button"
              class="select-trigger"
              onclick={() => {
                numCtxDropdownOpen = !numCtxDropdownOpen;
                profileDropdownOpen = false;
              }}
            >
              <span>
                {numCtxOptions.find(o => o.value === ollamaNumCtx)?.label || ollamaNumCtx}
                <span class="ctx-desc">
                  ({numCtxOptions.find(o => o.value === ollamaNumCtx)?.desc || ""})
                </span>
              </span>
              <i class="arrow fa-solid {numCtxDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
            </button>
            {#if numCtxDropdownOpen}
              <div class="select-options">
                {#each numCtxOptions as option}
                  <button
                    type="button"
                    class="select-option ctx-option {ollamaNumCtx === option.value ? 'selected' : ''}"
                    onclick={() => handleNumCtxChange(option.value)}
                  >
                    <span class="ctx-label">{option.label}</span>
                    <span class="ctx-option-desc">{option.desc}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          <p class="setting-description">
            {$_("settings.ollama.contextLengthDescription") || "Höhere Werte erlauben längere Artikel, benötigen aber mehr VRAM. 4K ist für die meisten Artikel ausreichend."}
          </p>
        </div>

        {#if downloadError}
          <div class="error-message">
            {$_("settings.ollama.downloadError")}: {downloadError}
          </div>
        {/if}
      {/if}
    {:else if activeTab === "prompts"}
      <h3>{$_("settings.prompts.title")}</h3>

      {#if !ollamaStatus?.available}
        <div class="status-unavailable">
          <i class="status-icon fa-solid fa-xmark"></i>
          {$_("settings.ollama.unavailable")}
        </div>
      {:else}
        <div class="setting-group">
          <label class="label" for="summary-prompt"
            >{$_("settings.prompts.summaryPrompt")}</label
          >
          <textarea
            id="summary-prompt"
            class="prompt-textarea"
            bind:value={summaryPrompt}
            oninput={handlePromptChange}
            rows="6"
          ></textarea>
        </div>

        <div class="setting-group">
          <label class="label" for="analysis-prompt"
            >{$_("settings.prompts.analysisPrompt")}</label
          >
          <textarea
            id="analysis-prompt"
            class="prompt-textarea"
            bind:value={analysisPrompt}
            oninput={handlePromptChange}
            rows="8"
          ></textarea>
        </div>

        {#if promptsModified}
          <div class="prompt-actions">
            <button type="button" class="btn-save-prompts" onclick={handleSavePrompts}>
              {$_("settings.prompts.save") || "Prompts speichern"}
            </button>
            <button type="button" class="btn-reset" onclick={handleResetPrompts}>
              {$_("settings.prompts.reset")}
            </button>
          </div>
        {/if}
      {/if}
    {:else if activeTab === "stopwords"}
      <!-- Stopwords Editor -->
      <p class="settings-description">{$_("settings.stopwords.description")}</p>

      <!-- Confirmation Dialog for Clear -->
      {#if confirmClearStopwords}
        <div class="confirm-overlay">
          <div class="confirm-dialog">
            <p class="confirm-message">{$_("settings.stopwords.confirmClear")}</p>
            <div class="confirm-actions">
              <button
                type="button"
                class="btn-secondary"
                onclick={() => (confirmClearStopwords = false)}
              >
                {$_("confirm.no")}
              </button>
              <button
                type="button"
                class="btn-danger-solid"
                onclick={clearAllUserStopwords}
              >
                {$_("confirm.yes")}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Stopword Statistics -->
      {#if stopwordStats}
        <div class="keyword-stats">
          <h3>{$_("settings.maintenance.stats")}</h3>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="stat-value">{stopwordStats.builtin_count}</span>
              <span class="stat-label">{$_("settings.stopwords.builtinCount")}</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{stopwordStats.user_count}</span>
              <span class="stat-label">{$_("settings.stopwords.userCount")}</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{stopwordStats.total_count}</span>
              <span class="stat-label">{$_("settings.stopwords.totalCount")}</span>
            </div>
          </div>
        </div>
      {/if}

      <!-- Add Stopword -->
      <div class="stopword-add-section">
        <h3>{$_("settings.stopwords.add")}</h3>
        <form
          class="stopword-add-form"
          onsubmit={(e) => {
            e.preventDefault();
            addStopword();
          }}
        >
          <input
            type="text"
            bind:value={newStopword}
            placeholder={$_("settings.stopwords.addStopword")}
            class="stopword-input"
            disabled={stopwordLoading}
          />
          <button type="submit" class="btn-action" disabled={stopwordLoading || newStopword.trim().length < 2}>
            {$_("settings.stopwords.add")}
          </button>
        </form>
      </div>

      <!-- User Stopwords List -->
      <div class="stopword-list-section">
        <div class="stopword-list-header">
          <h3>{$_("settings.stopwords.userCount")} ({userStopwords.length})</h3>
          {#if userStopwords.length > 0}
            <button
              type="button"
              class="btn-danger"
              onclick={() => (confirmClearStopwords = true)}
              disabled={stopwordLoading}
            >
              {$_("settings.stopwords.clear")}
            </button>
          {/if}
        </div>

        {#if userStopwords.length === 0}
          <p class="stopword-empty">{$_("settings.stopwords.noResults")}</p>
        {:else}
          <div class="stopword-chips">
            {#each userStopwords as sw}
              <span class="stopword-chip user">
                <span class="stopword-word">{sw.word}</span>
                <button
                  type="button"
                  class="stopword-remove"
                  onclick={() => removeStopword(sw.word)}
                  disabled={stopwordLoading}
                  title={$_("settings.stopwords.remove")}
                >
                  <i class="fa-solid fa-xmark"></i>
                </button>
              </span>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Search Stopwords -->
      <div class="stopword-search-section">
        <h3>{$_("settings.stopwords.search")}</h3>
        <input
          type="text"
          bind:value={stopwordSearchQuery}
          oninput={() => searchStopwords(stopwordSearchQuery)}
          placeholder={$_("settings.stopwords.search")}
          class="stopword-input"
        />

        {#if stopwordSearchResults.length > 0}
          <div class="stopword-search-results">
            {#each stopwordSearchResults as result}
              <span class="stopword-chip {result.is_builtin ? 'builtin' : 'user'}">
                <span class="stopword-word">{result.word}</span>
                <span class="stopword-type">
                  {result.is_builtin
                    ? $_("settings.stopwords.isBuiltin")
                    : $_("settings.stopwords.isUser")}
                </span>
              </span>
            {/each}
          </div>
        {:else if stopwordSearchQuery.length >= 2}
          <p class="stopword-empty">{$_("settings.stopwords.noResults")}</p>
        {/if}
      </div>
    {:else if activeTab === "maintenance"}
      <!-- Confirmation Dialog -->
      {#if confirmAction}
        <div class="confirm-overlay">
          <div class="confirm-dialog">
            <p class="confirm-message">
              {#if confirmAction === "prune"}
                {$_("settings.maintenance.confirmPrune")}
              {:else if confirmAction === "reset"}
                {$_("settings.maintenance.confirmReset")}
              {/if}
            </p>
            <div class="confirm-actions">
              <button
                type="button"
                class="btn-secondary"
                onclick={cancelConfirmation}
              >
                {$_("confirm.no")}
              </button>
              <button
                type="button"
                class="btn-danger-solid"
                onclick={confirmAction === "prune"
                  ? handlePruneLowQuality
                  : handleResetForReprocessing}
              >
                {$_("confirm.yes")}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Keyword Statistics -->
      {#if keywordStats}
        <div class="keyword-stats">
          <h3>{$_("settings.maintenance.stats")}</h3>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="stat-value">{keywordStats.total}</span>
              <span class="stat-label"
                >{$_("settings.maintenance.totalKeywords")}</span
              >
            </div>
            <div class="stat-item">
              <span class="stat-value">{keywordStats.with_embeddings}</span>
              <span class="stat-label"
                >{$_("settings.maintenance.withEmbeddings")}</span
              >
            </div>
            <div class="stat-item">
              <span class="stat-value"
                >{keywordStats.avg_quality.toFixed(2)}</span
              >
              <span class="stat-label"
                >{$_("settings.maintenance.avgQuality")}</span
              >
            </div>
            <div class="stat-item">
              <span
                class="stat-value {keywordStats.low_quality > 0
                  ? 'warning'
                  : ''}">{keywordStats.low_quality}</span
              >
              <span class="stat-label"
                >{$_("settings.maintenance.lowQuality")}</span
              >
            </div>
          </div>
        </div>
      {/if}

      <h3>{$_("settings.maintenance.keywordQuality")}</h3>

      {#if maintenanceResult}
        <div class="maintenance-result">
          {$_("settings.maintenance.result")}: {maintenanceResult}
        </div>
      {/if}

      <div class="maintenance-actions">
        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title"
              >{$_("settings.maintenance.calculateScores")}</span
            >
            <p class="action-desc">
              {$_("settings.maintenance.calculateScoresDesc")}
            </p>
          </div>
          <button
            type="button"
            class="btn-action"
            onclick={handleCalculateScores}
            disabled={maintenanceRunning !== null}
          >
            {maintenanceRunning === "scores"
              ? $_("settings.maintenance.running")
              : $_("settings.maintenance.calculateScores")}
          </button>
        </div>

        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title"
              >{$_("settings.maintenance.generateEmbeddings")}</span
            >
            <p class="action-desc">
              {$_("settings.maintenance.generateEmbeddingsDesc")}
            </p>
          </div>
          <button
            type="button"
            class="btn-action"
            onclick={handleGenerateEmbeddings}
            disabled={maintenanceRunning !== null || !ollamaStatus?.available}
          >
            {maintenanceRunning === "embeddings"
              ? $_("settings.maintenance.running")
              : $_("settings.maintenance.generateEmbeddings")}
          </button>
        </div>

        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title"
              >{$_("settings.maintenance.statisticalAnalysis")}</span
            >
            <p class="action-desc">
              {$_("settings.maintenance.statisticalAnalysisDesc")}
            </p>
          </div>
          {#if !statisticalRunning}
            <button
              type="button"
              class="btn-action"
              onclick={handleStatisticalAnalysis}
              disabled={maintenanceRunning !== null}
            >
              {$_("settings.maintenance.statisticalAnalysis")}
            </button>
          {/if}
        </div>

        {#if statisticalRunning && statisticalProgress}
          <div class="reanalyze-progress">
            <div class="progress-header">
              <span class="progress-label"
                >{$_("settings.maintenance.analyzing")}</span
              >
            </div>
            <div class="progress-bar">
              <div
                class="progress-fill"
                style="width: {statisticalProgress.total > 0
                  ? (statisticalProgress.current / statisticalProgress.total) * 100
                  : 0}%"
              ></div>
            </div>
            <div class="progress-details">
              <span class="progress-count">
                {statisticalProgress.current} / {statisticalProgress.total}
              </span>
              <span class="progress-title" title={statisticalProgress.title}>
                {statisticalProgress.title.length > 40
                  ? statisticalProgress.title.slice(0, 40) + "..."
                  : statisticalProgress.title}
              </span>
            </div>
            {#if !statisticalProgress.success && statisticalProgress.error}
              <div class="progress-error">{statisticalProgress.error}</div>
            {/if}
          </div>
        {/if}

        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title"
              >{$_("settings.maintenance.pruneLowQuality")}</span
            >
            <p class="action-desc">
              {$_("settings.maintenance.pruneLowQualityDesc")}
            </p>
          </div>
          <button
            type="button"
            class="btn-action btn-danger"
            onclick={showPruneConfirmation}
            disabled={maintenanceRunning !== null}
          >
            {maintenanceRunning === "prune"
              ? $_("settings.maintenance.running")
              : $_("settings.maintenance.pruneLowQuality")}
          </button>
        </div>
      </div>

      <h3 style="margin-top: 1.5rem;">
        {$_("settings.maintenance.reprocessArticles")}
      </h3>

      <div class="maintenance-actions">
        <div class="maintenance-action">
          <div class="action-info">
            <span class="action-title"
              >{$_("settings.maintenance.resetForReprocessing")}</span
            >
            <p class="action-desc">
              {$_("settings.maintenance.resetForReprocessingDesc")}
            </p>
          </div>
          {#if !reanalyzeRunning}
            <button
              type="button"
              class="btn-action btn-danger"
              onclick={showResetConfirmation}
              disabled={maintenanceRunning !== null}
            >
              {maintenanceRunning === "reset"
                ? $_("settings.maintenance.running")
                : $_("settings.maintenance.resetForReprocessing")}
            </button>
          {/if}
        </div>

        {#if reanalyzeRunning && reanalyzeProgress}
          <div class="reanalyze-progress">
            <div class="progress-header">
              <span class="progress-label"
                >{$_("settings.maintenance.reanalyzing")}</span
              >
              <button
                type="button"
                class="btn-cancel-small"
                onclick={handleCancelReanalyze}
              >
                {$_("batch.cancel")}
              </button>
            </div>
            <div class="progress-bar">
              <div
                class="progress-fill"
                style="width: {reanalyzeProgress.total > 0
                  ? (reanalyzeProgress.current / reanalyzeProgress.total) * 100
                  : 0}%"
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
  .tabs-wrapper {
    padding: 0 1.5rem;
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

  /* Theme Mode Buttons */
  .theme-mode-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .theme-mode-btn {
    flex: 1;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .theme-mode-btn:hover {
    border-color: var(--accent-primary);
  }

  .theme-mode-btn.active {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    border-color: var(--accent-primary);
  }

  /* Theme Family Groups */
  .theme-options {
    max-height: 350px;
  }

  .theme-family-group {
    border-bottom: 1px solid var(--border-muted);
  }

  .theme-family-group:last-child {
    border-bottom: none;
  }

  .theme-family-label {
    display: block;
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background-color: var(--bg-surface);
  }

  .theme-family-group .select-option {
    padding-left: 1.25rem;
    font-size: 0.875rem;
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

  /* Hardware Profiles */
  .profile-parallelism {
    color: var(--text-muted);
    font-size: 0.85em;
    margin-left: 0.5rem;
  }

  .profile-option {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem !important;
  }

  .profile-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .profile-name {
    font-weight: 500;
    color: var(--text-primary);
  }

  .profile-desc {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Context Length (num_ctx) */
  .ctx-desc {
    color: var(--text-muted);
    font-size: 0.85em;
    margin-left: 0.5rem;
  }

  .ctx-option {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem !important;
  }

  .ctx-label {
    font-weight: 600;
    color: var(--accent-primary);
    min-width: 3rem;
  }

  .ctx-option-desc {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .ctx-option.selected .ctx-label {
    color: var(--accent-secondary);
  }

  .profile-badge {
    background-color: var(--bg-surface);
    color: var(--accent-secondary);
    padding: 0.125rem 0.375rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .profile-option.selected .profile-badge {
    background-color: var(--bg-surface);
    color: var(--accent-primary);
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

  /* Load Models Button */
  .btn-load-models {
    width: 100%;
    padding: 0.75rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-load-models:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-load-models:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Prompt Actions */
  .prompt-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .btn-save-prompts {
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

  .btn-save-prompts:hover {
    filter: brightness(1.1);
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

  /* OPML Import */
  .opml-section {
    max-width: 600px;
    margin-bottom: 1.5rem;
  }

  .opml-error {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.375rem;
    color: var(--status-error);
    font-size: 0.875rem;
  }

  .opml-result {
    margin-top: 0.5rem;
    padding: 0.75rem;
    background-color: rgba(166, 227, 161, 0.1);
    border-radius: 0.375rem;
    color: var(--status-success);
    font-size: 0.875rem;
  }

  .opml-result p {
    margin: 0;
  }

  .opml-errors {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid rgba(243, 139, 168, 0.2);
  }

  .opml-error-item {
    font-size: 0.75rem;
    color: var(--status-error);
    margin-top: 0.25rem;
  }

  .opml-preview {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .opml-preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .btn-small {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--text-muted);
    border-radius: 0.25rem;
    background: none;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-small:hover {
    border-color: var(--text-primary);
    color: var(--text-primary);
  }

  .opml-feed-list {
    max-height: 200px;
    overflow-y: auto;
    margin-bottom: 0.75rem;
  }

  .opml-feed-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.375rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }

  .opml-feed-item:hover {
    background-color: var(--bg-muted);
  }

  .opml-feed-item.exists {
    opacity: 0.6;
  }

  .opml-feed-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    flex: 1;
    min-width: 0;
  }

  .opml-feed-title {
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .opml-feed-category {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .opml-feed-exists {
    font-size: 0.75rem;
    color: var(--status-warning);
    white-space: nowrap;
    margin-left: 0.5rem;
  }

  .opml-preview-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-default);
  }

  .opml-preview-info {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* OPML Export */
  .opml-export {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-default);
  }

  .export-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .export-info {
    flex: 1;
  }

  .export-label {
    font-weight: 500;
    color: var(--text-primary);
  }

  .export-desc {
    margin: 0.25rem 0 0 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Stopword Editor */
  .stopword-add-section,
  .stopword-list-section,
  .stopword-search-section {
    margin-top: 1.5rem;
  }

  .stopword-add-form {
    display: flex;
    gap: 0.5rem;
  }

  .stopword-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .stopword-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .stopword-list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .stopword-list-header h3 {
    margin: 0;
  }

  .stopword-chips,
  .stopword-search-results {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    max-height: 300px;
    overflow-y: auto;
    padding: 0.5rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .stopword-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border-radius: 1rem;
    font-size: 0.75rem;
  }

  .stopword-chip.user {
    background-color: var(--accent-primary-alpha);
    border: 1px solid var(--accent-primary);
    color: var(--accent-primary);
  }

  .stopword-chip.builtin {
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    color: var(--text-muted);
  }

  .stopword-word {
    font-family: monospace;
  }

  .stopword-type {
    font-size: 0.625rem;
    opacity: 0.7;
    text-transform: uppercase;
  }

  .stopword-remove {
    background: none;
    border: none;
    padding: 0 0.125rem;
    cursor: pointer;
    color: inherit;
    opacity: 0.7;
    transition: opacity 0.2s;
  }

  .stopword-remove:hover {
    opacity: 1;
  }

  .stopword-remove:disabled {
    cursor: not-allowed;
    opacity: 0.3;
  }

  .stopword-empty {
    color: var(--text-muted);
    font-style: italic;
    padding: 0.5rem;
  }

  .btn-danger {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--status-error);
    border-radius: 0.25rem;
    background: none;
    color: var(--status-error);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-danger:hover {
    background-color: var(--status-error);
    color: var(--text-on-accent);
  }

  .btn-danger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
