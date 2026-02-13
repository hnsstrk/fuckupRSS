<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import { appState, toasts } from "../../stores/state.svelte";

  // AI Provider state
  let aiTextProvider = $state("ollama");
  let ollamaUrl = $state("http://localhost:11434");
  let openaiBaseUrl = $state("https://api.openai.com");
  let openaiApiKey = $state("");
  let openaiModel = $state("gpt-5-nano");
  let showApiKey = $state(false);

  // OpenAI model presets
  const openaiModelPresets = [
    {
      value: "gpt-5-nano",
      label: "GPT-5 nano",
      price: "$0.05/$0.40 per 1M tokens",
    },
    {
      value: "gpt-5-mini",
      label: "GPT-5 mini",
      price: "$0.25/$2.00 per 1M tokens",
    },
    {
      value: "gpt-4.1-mini",
      label: "GPT-4.1 mini",
      price: "$0.40/$1.60 per 1M tokens",
    },
    {
      value: "gpt-4.1-nano",
      label: "GPT-4.1 nano",
      price: "$0.10/$0.40 per 1M tokens",
    },
  ];
  let openaiModelPreset = $state<string>("gpt-5-nano");
  let openaiModelDropdownOpen = $state(false);
  let openaiTemperature = $state("auto");
  let openaiConcurrency = $state(20);
  let costLimit = $state(5.0);
  let monthlyCost = $state<{
    spent: number;
    limit: number;
    remaining: number;
    percentage: number;
  } | null>(null);

  // Connection test state
  let testingOllama = $state(false);
  let ollamaTestResult = $state<{
    success: boolean;
    latency_ms: number;
    models: string[];
  } | null>(null);
  let testingOpenai = $state(false);
  let openaiTestResult = $state<{
    success: boolean;
    latency_ms: number;
    models: string[];
    error: string | null;
  } | null>(null);

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
  let pullUnlisten: UnlistenFn | null = $state(null);

  // Embedding provider state
  let embeddingProvider = $state("ollama");
  let openaiEmbeddingModel = $state("text-embedding-3-small");

  const openaiEmbeddingPresets = [
    {
      value: "text-embedding-3-small",
      label: "text-embedding-3-small",
      price: "$0.02 per 1M tokens",
    },
    {
      value: "text-embedding-3-large",
      label: "text-embedding-3-large",
      price: "$0.13 per 1M tokens",
    },
  ];

  // Context length (num_ctx)
  const DEFAULT_NUM_CTX = 4096;
  let ollamaNumCtx = $state(DEFAULT_NUM_CTX);
  const numCtxOptions = [
    { value: 2048, label: "2K", desc: "Minimal - sehr schnell" },
    { value: 4096, label: "4K", desc: "Standard - empfohlen" },
    { value: 8192, label: "8K", desc: "Erweitert - mehr VRAM" },
    { value: 16384, label: "16K", desc: "Gro\u00DF - hoher VRAM-Bedarf" },
    { value: 32768, label: "32K", desc: "Maximum - sehr hoher VRAM-Bedarf" },
  ];

  // Dropdown states
  let mainModelDropdownOpen = $state(false);
  let embeddingModelDropdownOpen = $state(false);
  let numCtxDropdownOpen = $state(false);

  export function getOllamaStatus() {
    return ollamaStatus;
  }

  export async function init() {
    // Load AI provider settings
    const savedProvider = await invoke<string | null>("get_setting", {
      key: "ai_text_provider",
    });
    if (savedProvider) aiTextProvider = savedProvider;

    const savedOllamaUrl = await invoke<string | null>("get_setting", {
      key: "ollama_url",
    });
    if (savedOllamaUrl) ollamaUrl = savedOllamaUrl;

    const savedOpenaiBaseUrl = await invoke<string | null>("get_setting", {
      key: "openai_base_url",
    });
    if (savedOpenaiBaseUrl) openaiBaseUrl = savedOpenaiBaseUrl;

    const savedOpenaiApiKey = await invoke<string | null>("get_setting", {
      key: "openai_api_key",
    });
    if (savedOpenaiApiKey) openaiApiKey = savedOpenaiApiKey;

    const savedOpenaiModel = await invoke<string | null>("get_setting", {
      key: "openai_model",
    });
    if (savedOpenaiModel) {
      openaiModel = savedOpenaiModel;
      // Determine if the saved model matches a preset
      const matchingPreset = openaiModelPresets.find((p) => p.value === savedOpenaiModel);
      openaiModelPreset = matchingPreset ? savedOpenaiModel : "custom";
    }

    const savedTemperature = await invoke<string | null>("get_setting", {
      key: "openai_temperature",
    });
    if (savedTemperature) openaiTemperature = savedTemperature;

    const savedConcurrency = await invoke<string | null>("get_setting", {
      key: "openai_concurrency",
    });
    if (savedConcurrency) openaiConcurrency = parseInt(savedConcurrency) || 20;

    const savedCostLimit = await invoke<string | null>("get_setting", {
      key: "cost_limit_monthly",
    });
    if (savedCostLimit) costLimit = parseFloat(savedCostLimit) || 5.0;

    // Load embedding provider settings
    const savedEmbeddingProvider = await invoke<string | null>("get_setting", {
      key: "embedding_provider",
    });
    if (savedEmbeddingProvider) embeddingProvider = savedEmbeddingProvider;

    const savedOpenaiEmbeddingModel = await invoke<string | null>("get_setting", {
      key: "openai_embedding_model",
    });
    if (savedOpenaiEmbeddingModel) openaiEmbeddingModel = savedOpenaiEmbeddingModel;

    // Load cost data
    await loadMonthlyCost();

    // Load existing Ollama settings
    await loadOllamaStatus();
    await loadOllamaStatus();

    const savedNumCtx = await invoke<string | null>("get_setting", {
      key: "ollama_num_ctx",
    });
    if (savedNumCtx) {
      ollamaNumCtx = parseInt(savedNumCtx) || DEFAULT_NUM_CTX;
    }

    // Listen for model pull completion events
    pullUnlisten = await listen<string>("model-pull-complete", async () => {
      await loadOllamaStatus();
    });
  }

  onDestroy(() => {
    if (pullUnlisten) {
      pullUnlisten();
    }
  });

  export function closeAllDropdowns() {
    mainModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
    numCtxDropdownOpen = false;
    openaiModelDropdownOpen = false;
  }

  // --- Provider handler functions ---

  async function handleProviderChange(provider: string) {
    aiTextProvider = provider;
    await invoke("set_setting", { key: "ai_text_provider", value: provider });
  }

  async function testOllamaConnection() {
    testingOllama = true;
    ollamaTestResult = null;
    try {
      ollamaTestResult = await invoke("test_ai_provider", {
        providerType: "ollama",
        baseUrl: ollamaUrl,
        apiKey: null,
      });
    } catch (e) {
      ollamaTestResult = { success: false, latency_ms: 0, models: [] };
    }
    testingOllama = false;
  }

  async function saveOllamaUrl() {
    await invoke("set_setting", { key: "ollama_url", value: ollamaUrl });
  }

  async function testOpenaiConnection() {
    testingOpenai = true;
    openaiTestResult = null;
    try {
      openaiTestResult = await invoke("test_ai_provider", {
        providerType: "openai_compatible",
        baseUrl: openaiBaseUrl,
        apiKey: openaiApiKey || null,
      });
    } catch (e) {
      openaiTestResult = {
        success: false,
        latency_ms: 0,
        models: [],
        error: String(e),
      };
    }
    testingOpenai = false;
  }

  async function saveOpenaiSettings() {
    await invoke("set_setting", {
      key: "openai_base_url",
      value: openaiBaseUrl,
    });
    await invoke("set_setting", { key: "openai_api_key", value: openaiApiKey });
    await invoke("set_setting", { key: "openai_model", value: openaiModel });
    await invoke("set_setting", {
      key: "openai_temperature",
      value: openaiTemperature,
    });
    await invoke("set_setting", {
      key: "openai_concurrency",
      value: openaiConcurrency.toString(),
    });
  }

  async function handleOpenaiModelPresetChange(preset: string) {
    openaiModelPreset = preset;
    openaiModelDropdownOpen = false;
    if (preset !== "custom") {
      openaiModel = preset;
      await saveOpenaiSettings();
    }
  }

  function toggleOpenaiModelDropdown() {
    openaiModelDropdownOpen = !openaiModelDropdownOpen;
    mainModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
    numCtxDropdownOpen = false;
  }

  async function saveCostLimit() {
    await invoke("set_setting", {
      key: "cost_limit_monthly",
      value: costLimit.toString(),
    });
  }

  async function loadMonthlyCost() {
    try {
      monthlyCost = await invoke("get_monthly_cost");
    } catch (e) {
      monthlyCost = null;
    }
  }

  // --- Existing handler functions ---

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
        selectedEmbeddingModel = savedEmbeddingModel || ollamaStatus.recommended_embedding;
        appState.ollamaStatus = ollamaStatus;
        // Sync selectedModel to appState for batch processing if not already set
        if (selectedMainModel && !appState.selectedModel) {
          appState.selectedModel = selectedMainModel;
        }
      }

      await loadLoadedModels();
    } catch (e) {
      console.error("Failed to load Ollama status:", e);
      ollamaStatus = null;
    }
  }

  async function loadLoadedModels() {
    try {
      const response = await invoke<{ models: typeof loadedModels }>("get_loaded_models");
      loadedModels = response.models;
    } catch (e) {
      console.error("Failed to load loaded models:", e);
      loadedModels = [];
    }
  }

  function formatBytes(bytes: number): string {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(1)} GB`;
  }

  function isModelLoaded(modelName: string): boolean {
    return loadedModels.some((m) => m.name === modelName);
  }

  function isRecommendedModel(model: string, recommended: string): boolean {
    return model === recommended || model.startsWith(recommended.split(":")[0] + ":");
  }

  function isModelInstalled(modelName: string): boolean {
    if (!ollamaStatus || !modelName) return true; // Don't show warning if no data
    return ollamaStatus.models.includes(modelName);
  }

  // Check if main model is missing (selected but not installed)
  let isMainModelMissing = $derived(
    selectedMainModel && ollamaStatus?.available && !isModelInstalled(selectedMainModel),
  );

  // Check if embedding model is missing (selected but not installed)
  let isEmbeddingModelMissing = $derived(
    selectedEmbeddingModel && ollamaStatus?.available && !isModelInstalled(selectedEmbeddingModel),
  );

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
      toasts.error($_("settings.saveError"));
    }
  }

  async function handleLoadModels() {
    if (!selectedMainModel || !selectedEmbeddingModel) return;

    loadingModels = true;
    try {
      await invoke("ensure_models_loaded", {
        mainModel: selectedMainModel,
        embeddingModel: selectedEmbeddingModel,
      });
      await emit("models-changed");
      toasts.success($_("settings.modelsLoaded"));
    } catch (e) {
      console.error("Failed to load models:", e);
      toasts.error($_("settings.modelsLoadError"));
    } finally {
      loadingModels = false;
    }
  }

  async function handleDownloadModel(model: string) {
    if (downloadingModel) return;

    downloadingModel = model;
    downloadError = null;

    try {
      const result = await invoke<{ success: boolean; error: string | null }>("pull_model", {
        model,
      });
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

  function toggleMainModelDropdown() {
    mainModelDropdownOpen = !mainModelDropdownOpen;
    embeddingModelDropdownOpen = false;
    numCtxDropdownOpen = false;
  }

  function toggleEmbeddingModelDropdown() {
    embeddingModelDropdownOpen = !embeddingModelDropdownOpen;
    mainModelDropdownOpen = false;
    numCtxDropdownOpen = false;
  }

  async function selectMainModel(value: string) {
    selectedMainModel = value;
    mainModelDropdownOpen = false;
    try {
      await invoke("set_setting", { key: "main_model", value });
      appState.selectedModel = value;
    } catch (e) {
      console.error("Failed to save main model setting:", e);
      toasts.error($_("settings.saveError"));
    }
  }

  async function selectEmbeddingModel(value: string) {
    selectedEmbeddingModel = value;
    embeddingModelDropdownOpen = false;
    try {
      await invoke("set_setting", { key: "embedding_model", value });
      appState.selectedEmbeddingModel = value;
    } catch (e) {
      console.error("Failed to save embedding model setting:", e);
      toasts.error($_("settings.saveError"));
    }
  }

  async function saveEmbeddingProvider(value: string) {
    embeddingProvider = value;
    try {
      await invoke("set_setting", { key: "embedding_provider", value });
    } catch (e) {
      console.error("Failed to save embedding provider setting:", e);
      toasts.error($_("settings.saveError"));
    }
  }

  async function saveOpenaiEmbeddingModel(value: string) {
    openaiEmbeddingModel = value;
    try {
      await invoke("set_setting", { key: "openai_embedding_model", value });
    } catch (e) {
      console.error("Failed to save OpenAI embedding model setting:", e);
      toasts.error($_("settings.saveError"));
    }
  }
</script>

<h3>{$_("settings.ollama.title")}</h3>

<!-- 1. Provider Radio Selection -->
<div class="setting-group">
  <span class="label">{$_("settings.ollama.textProvider")}</span>
  <div class="provider-radios">
    <label class="radio-label">
      <input
        type="radio"
        name="ai_text_provider"
        value="ollama"
        checked={aiTextProvider === "ollama"}
        onchange={() => handleProviderChange("ollama")}
      />
      <span class="radio-text">{$_("settings.ollama.providerOllama")}</span>
    </label>
    <label class="radio-label">
      <input
        type="radio"
        name="ai_text_provider"
        value="openai_compatible"
        checked={aiTextProvider === "openai_compatible"}
        onchange={() => handleProviderChange("openai_compatible")}
      />
      <span class="radio-text">{$_("settings.ollama.providerOpenAi")}</span>
    </label>
  </div>
  <p class="setting-description">
    {$_("settings.ollama.providerDescription")}
  </p>
</div>

<!-- ============================================ -->
<!-- OLLAMA PROVIDER SECTION                      -->
<!-- ============================================ -->
{#if aiTextProvider === "ollama"}
  <!-- Ollama Server URL + Connection Test -->
  <div class="setting-group">
    <span class="label">{$_("settings.ollama.ollamaSection")}</span>
    <div class="url-row">
      <input
        type="text"
        class="text-input"
        bind:value={ollamaUrl}
        onblur={saveOllamaUrl}
        placeholder="http://localhost:11434"
      />
      <button
        type="button"
        class="btn-test"
        onclick={testOllamaConnection}
        disabled={testingOllama}
      >
        {#if testingOllama}
          {$_("settings.ollama.testing")}
        {:else}
          {$_("settings.ollama.testConnection")}
        {/if}
      </button>
    </div>
    <p class="setting-description">
      {$_("settings.ollama.ollamaUrlDescription")}
    </p>
    {#if ollamaTestResult}
      <div class="connection-result {ollamaTestResult.success ? 'success' : 'error'}">
        {#if ollamaTestResult.success}
          <i class="fa-solid fa-check"></i>
          {$_("settings.ollama.connected")}
          <span class="latency"
            >{$_("settings.ollama.latency", {
              values: { ms: ollamaTestResult.latency_ms },
            })}</span
          >
        {:else}
          <i class="fa-solid fa-xmark"></i>
          {$_("settings.ollama.disconnected")}
        {/if}
      </div>
    {/if}
  </div>

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
      <span class="label">{$_("settings.ollama.loadedModels") || "Geladene Modelle (VRAM)"}</span>
      <div class="loaded-models">
        {#if loadedModels.length === 0}
          <div class="no-models">
            {$_("settings.ollama.noLoadedModels") || "Keine Modelle geladen"}
          </div>
        {:else}
          {#each loadedModels as model (model.name)}
            <div class="loaded-model">
              <span class="model-name">{model.name}</span>
              <span class="model-info">{model.parameter_size} · {formatBytes(model.size_vram)}</span
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
          <button type="button" class="select-trigger" onclick={toggleMainModelDropdown}>
            <span>
              {selectedMainModel || $_("settings.ollama.noModels")}
              {#if isModelLoaded(selectedMainModel)}
                <i class="loaded-badge fa-solid fa-circle"></i>
              {/if}
              {#if ollamaStatus && isRecommendedModel(selectedMainModel, ollamaStatus.recommended_main)}
                <span class="recommended">{$_("settings.ollama.recommended")}</span>
              {/if}
            </span>
            <i class="arrow fa-solid {mainModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
          </button>
          {#if mainModelDropdownOpen}
            <div class="select-options">
              {#each ollamaStatus.models as model (model)}
                <button
                  type="button"
                  class="select-option {selectedMainModel === model ? 'selected' : ''}"
                  onclick={() => selectMainModel(model)}
                >
                  {model}
                  {#if isModelLoaded(model)}
                    <i class="loaded-badge fa-solid fa-circle"></i>
                  {/if}
                  {#if isRecommendedModel(model, ollamaStatus.recommended_main)}
                    <span class="recommended">{$_("settings.ollama.recommended")}</span>
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
              {$_("settings.ollama.downloading")}
            {:else}
              {$_("settings.ollama.downloadModel")}
              {ollamaStatus.recommended_main}
            {/if}
          </button>
        {/if}
      </div>
      {#if isMainModelMissing}
        <div class="model-warning">
          <i class="warning-icon fa-solid fa-triangle-exclamation"></i>
          <div class="warning-content">
            <span class="warning-text"
              >{$_("settings.ollama.modelMissing", {
                values: { model: selectedMainModel },
              })}</span
            >
            <span class="warning-hint">{$_("settings.ollama.modelMissingHint")}</span>
          </div>
          <button
            type="button"
            class="btn-download-inline"
            onclick={() => handleDownloadModel(selectedMainModel)}
            disabled={downloadingModel !== null}
          >
            {#if downloadingModel === selectedMainModel}
              {$_("settings.ollama.downloading")}
            {:else}
              {$_("settings.ollama.downloadNow")}
            {/if}
          </button>
        </div>
      {/if}
    </div>

    <!-- Context Length (num_ctx) -->
    <div class="setting-group">
      <span class="label">{$_("settings.ollama.contextLength") || "Kontext-Lange (num_ctx)"}</span>
      <div class="custom-select">
        <button
          type="button"
          class="select-trigger"
          onclick={() => {
            numCtxDropdownOpen = !numCtxDropdownOpen;
            mainModelDropdownOpen = false;
            embeddingModelDropdownOpen = false;
          }}
        >
          <span>
            {numCtxOptions.find((o) => o.value === ollamaNumCtx)?.label || ollamaNumCtx}
            <span class="ctx-desc">
              ({numCtxOptions.find((o) => o.value === ollamaNumCtx)?.desc || ""})
            </span>
          </span>
          <i class="arrow fa-solid {numCtxDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
        </button>
        {#if numCtxDropdownOpen}
          <div class="select-options">
            {#each numCtxOptions as option (option.value)}
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
        {$_("settings.ollama.contextLengthDescription") ||
          "Hohere Werte erlauben langere Artikel, benotigen aber mehr VRAM. 4K ist fur die meisten Artikel ausreichend."}
      </p>
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
        {$_("settings.ollama.loadModelsDescription") ||
          "Ladt die ausgewahlten Modelle in den Grafikspeicher. Die Auswahl wird automatisch gespeichert."}
      </p>
    </div>

    {#if downloadError}
      <div class="error-message">
        {$_("settings.ollama.downloadError")}: {downloadError}
      </div>
    {/if}
  {/if}
{/if}

<!-- ============================================ -->
<!-- OPENAI PROVIDER SECTION                      -->
<!-- ============================================ -->
{#if aiTextProvider === "openai_compatible"}
  <div class="setting-group">
    <span class="label section-label">{$_("settings.ollama.openaiSection")}</span>

    <!-- API Base URL -->
    <div class="sub-field">
      <span class="sub-label">{$_("settings.ollama.apiBaseUrl")}</span>
      <input
        type="text"
        class="text-input"
        bind:value={openaiBaseUrl}
        onblur={saveOpenaiSettings}
        placeholder="https://api.openai.com"
      />
      <p class="setting-description">
        {$_("settings.ollama.apiBaseUrlDescription")}
      </p>
    </div>

    <!-- API Key -->
    <div class="sub-field">
      <span class="sub-label">{$_("settings.ollama.apiKey")}</span>
      <div class="api-key-row">
        <input
          type={showApiKey ? "text" : "password"}
          class="text-input api-key-input"
          bind:value={openaiApiKey}
          onblur={saveOpenaiSettings}
          placeholder={$_("settings.ollama.apiKeyPlaceholder")}
        />
        <button
          type="button"
          class="btn-toggle-key"
          onclick={() => (showApiKey = !showApiKey)}
          aria-label={showApiKey ? "Hide API key" : "Show API key"}
        >
          <i class="fa-solid {showApiKey ? 'fa-eye-slash' : 'fa-eye'}"></i>
        </button>
      </div>
      <p class="setting-description">
        {$_("settings.ollama.apiKeyDescription")}
      </p>
    </div>

    <!-- Model Selection Dropdown -->
    <div class="sub-field">
      <span class="sub-label">{$_("settings.ollama.openaiModelSelect")}</span>
      <div class="custom-select">
        <button type="button" class="select-trigger" onclick={toggleOpenaiModelDropdown}>
          <span>
            {#if openaiModelPreset === "custom"}
              {$_("settings.ollama.openaiModelCustom")}: {openaiModel}
            {:else}
              {openaiModelPresets.find((p) => p.value === openaiModelPreset)?.label || openaiModel}
              {#if openaiModelPreset === "gpt-5-nano"}
                <span class="recommended">{$_("settings.ollama.openaiModelRecommended")}</span>
              {/if}
            {/if}
          </span>
          <i class="arrow fa-solid {openaiModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
        </button>
        {#if openaiModelDropdownOpen}
          <div class="select-options">
            {#each openaiModelPresets as preset (preset.value)}
              <button
                type="button"
                class="select-option openai-model-option {openaiModelPreset === preset.value
                  ? 'selected'
                  : ''}"
                onclick={() => handleOpenaiModelPresetChange(preset.value)}
              >
                <div class="openai-model-info">
                  <span class="openai-model-name">{preset.label}</span>
                  <span class="openai-model-price">{preset.price}</span>
                </div>
                {#if preset.value === "gpt-5-nano"}
                  <span class="recommended">{$_("settings.ollama.recommended")}</span>
                {/if}
              </button>
            {/each}
            <button
              type="button"
              class="select-option openai-model-option {openaiModelPreset === 'custom'
                ? 'selected'
                : ''}"
              onclick={() => handleOpenaiModelPresetChange("custom")}
            >
              <span class="openai-model-name">{$_("settings.ollama.openaiModelCustom")}...</span>
            </button>
          </div>
        {/if}
      </div>
      {#if openaiModelPreset === "custom"}
        <input
          type="text"
          class="text-input custom-model-input"
          bind:value={openaiModel}
          onblur={saveOpenaiSettings}
          placeholder={$_("settings.ollama.openaiModelCustomPlaceholder")}
        />
      {/if}
      <p class="setting-description">
        {$_("settings.ollama.openaiModelDescription")}
      </p>
    </div>

    <!-- Temperature Setting -->
    <div class="sub-field">
      <span class="sub-label">{$_("settings.ollama.openaiTemperature")}</span>
      <div class="temperature-row">
        <select
          class="text-input temperature-select"
          bind:value={openaiTemperature}
          onchange={saveOpenaiSettings}
        >
          <option value="auto">{$_("settings.ollama.openaiTemperatureAuto")}</option>
          <option value="0">0 ({$_("settings.ollama.openaiTemperatureDeterministic")})</option>
          <option value="0.3">0.3</option>
          <option value="0.5">0.5</option>
          <option value="0.7">0.7</option>
          <option value="1.0">1.0 ({$_("settings.ollama.openaiTemperatureDefault")})</option>
        </select>
      </div>
      <p class="setting-description">
        {$_("settings.ollama.openaiTemperatureDescription")}
      </p>
    </div>

    <!-- Concurrency Setting -->
    <div class="sub-field">
      <span class="sub-label">{$_("settings.ollama.openaiConcurrency")}</span>
      <div class="range-container">
        <input
          type="range"
          class="range-input"
          min="1"
          max="50"
          bind:value={openaiConcurrency}
          onchange={saveOpenaiSettings}
        />
        <span class="range-value">{openaiConcurrency}</span>
      </div>
      <p class="setting-description">
        {$_("settings.ollama.openaiConcurrencyDescription")}
      </p>
    </div>

    <!-- Test Connection -->
    <div class="sub-field">
      <button
        type="button"
        class="btn-test"
        onclick={testOpenaiConnection}
        disabled={testingOpenai}
      >
        {#if testingOpenai}
          {$_("settings.ollama.testing")}
        {:else}
          {$_("settings.ollama.testConnection")}
        {/if}
      </button>
      {#if openaiTestResult}
        <div class="connection-result {openaiTestResult.success ? 'success' : 'error'}">
          {#if openaiTestResult.success}
            <i class="fa-solid fa-check"></i>
            {$_("settings.ollama.connected")}
            <span class="latency"
              >{$_("settings.ollama.latency", {
                values: { ms: openaiTestResult.latency_ms },
              })}</span
            >
          {:else}
            <i class="fa-solid fa-xmark"></i>
            {$_("settings.ollama.disconnected")}
            {#if openaiTestResult.error}
              <span class="error-detail">{openaiTestResult.error}</span>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Cost Tracking Section -->
  <div class="setting-group">
    <span class="label">{$_("settings.ollama.costSection")}</span>

    {#if monthlyCost}
      <div class="cost-display">
        <div class="cost-header">
          <span class="cost-spent"
            >{$_("settings.ollama.costSpent")}: ${monthlyCost.spent.toFixed(2)}</span
          >
          <span class="cost-limit">/ ${monthlyCost.limit.toFixed(2)}</span>
        </div>
        <div class="cost-bar-container">
          <div
            class="cost-bar"
            style="width: {Math.min(monthlyCost.percentage, 100)}%"
            class:cost-bar-warning={monthlyCost.percentage > 80}
            class:cost-bar-danger={monthlyCost.percentage > 95}
          ></div>
        </div>
        <div class="cost-footer">
          <span class="cost-remaining"
            >{$_("settings.ollama.costRemaining")}: ${monthlyCost.remaining.toFixed(2)}</span
          >
          <span class="cost-percentage">{monthlyCost.percentage.toFixed(0)}%</span>
        </div>
      </div>
    {/if}

    <div class="sub-field">
      <span class="sub-label">{$_("settings.ollama.costLimit")}</span>
      <input
        type="number"
        class="text-input cost-input"
        bind:value={costLimit}
        onblur={saveCostLimit}
        min="0"
        step="0.5"
      />
      <p class="setting-description">
        {$_("settings.ollama.costLimitDescription")}
      </p>
    </div>
  </div>
{/if}

<!-- ============================================ -->
<!-- EMBEDDING SECTION (always visible)           -->
<!-- ============================================ -->
<div class="setting-group">
  <span class="label">{$_("settings.ollama.embeddingSection")}</span>
  <p class="setting-description embedding-note">
    <i class="fa-light fa-circle-info"></i>
    {$_("settings.ollama.embeddingNote")}
  </p>

  <!-- Embedding Provider Selection -->
  <div class="provider-toggle">
    <button
      type="button"
      class="toggle-btn {embeddingProvider === 'ollama' ? 'active' : ''}"
      onclick={() => saveEmbeddingProvider("ollama")}
    >
      <i class="fa-solid fa-server"></i>
      Ollama
    </button>
    <button
      type="button"
      class="toggle-btn {embeddingProvider === 'openai_compatible' ? 'active' : ''}"
      onclick={() => saveEmbeddingProvider("openai_compatible")}
    >
      <i class="fa-solid fa-cloud"></i>
      OpenAI
    </button>
  </div>

  {#if embeddingProvider === "ollama"}
    <!-- Ollama Embedding Configuration -->
    {#if aiTextProvider === "openai_compatible"}
      <!-- Show Ollama URL for embeddings when OpenAI is the text provider -->
      <div class="sub-field">
        <span class="sub-label">{$_("settings.ollama.ollamaUrlForEmbeddings")}</span>
        <div class="url-row">
          <input
            type="text"
            class="text-input"
            bind:value={ollamaUrl}
            onblur={saveOllamaUrl}
            placeholder="http://localhost:11434"
          />
          <button
            type="button"
            class="btn-test"
            onclick={testOllamaConnection}
            disabled={testingOllama}
          >
            {#if testingOllama}
              {$_("settings.ollama.testing")}
            {:else}
              {$_("settings.ollama.testConnection")}
            {/if}
          </button>
        </div>
        {#if ollamaTestResult}
          <div class="connection-result {ollamaTestResult.success ? 'success' : 'error'}">
            {#if ollamaTestResult.success}
              <i class="fa-solid fa-check"></i>
              {$_("settings.ollama.connected")}
              <span class="latency"
                >{$_("settings.ollama.latency", {
                  values: { ms: ollamaTestResult.latency_ms },
                })}</span
              >
            {:else}
              <i class="fa-solid fa-xmark"></i>
              {$_("settings.ollama.disconnected")}
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    {#if ollamaStatus?.available}
      <div class="model-row">
        <div class="custom-select model-select">
          <button type="button" class="select-trigger" onclick={toggleEmbeddingModelDropdown}>
            <span>
              {selectedEmbeddingModel || $_("settings.ollama.noModels")}
              {#if ollamaStatus && isRecommendedModel(selectedEmbeddingModel, ollamaStatus.recommended_embedding)}
                <span class="recommended">{$_("settings.ollama.recommended")}</span>
              {/if}
            </span>
            <i class="arrow fa-solid {embeddingModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"
            ></i>
          </button>
          {#if embeddingModelDropdownOpen}
            <div class="select-options">
              {#each ollamaStatus.models as model (model)}
                <button
                  type="button"
                  class="select-option {selectedEmbeddingModel === model ? 'selected' : ''}"
                  onclick={() => selectEmbeddingModel(model)}
                >
                  {model}
                  {#if isRecommendedModel(model, ollamaStatus.recommended_embedding)}
                    <span class="recommended">{$_("settings.ollama.recommended")}</span>
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
              {$_("settings.ollama.downloading")}
            {:else}
              {$_("settings.ollama.downloadModel")}
              {ollamaStatus.recommended_embedding}
            {/if}
          </button>
        {/if}
      </div>
      {#if isEmbeddingModelMissing}
        <div class="model-warning">
          <i class="warning-icon fa-solid fa-triangle-exclamation"></i>
          <div class="warning-content">
            <span class="warning-text"
              >{$_("settings.ollama.modelMissing", {
                values: { model: selectedEmbeddingModel },
              })}</span
            >
            <span class="warning-hint">{$_("settings.ollama.modelMissingHint")}</span>
          </div>
          <button
            type="button"
            class="btn-download-inline"
            onclick={() => handleDownloadModel(selectedEmbeddingModel)}
            disabled={downloadingModel !== null}
          >
            {#if downloadingModel === selectedEmbeddingModel}
              {$_("settings.ollama.downloading")}
            {:else}
              {$_("settings.ollama.downloadNow")}
            {/if}
          </button>
        </div>
      {/if}
    {/if}
  {:else}
    <!-- OpenAI Embedding Configuration -->
    <div class="sub-field">
      <span class="sub-label">{$_("settings.ollama.openaiEmbeddingModel")}</span>
      <div class="openai-embedding-presets">
        {#each openaiEmbeddingPresets as preset (preset.value)}
          <button
            type="button"
            class="preset-btn {openaiEmbeddingModel === preset.value ? 'active' : ''}"
            onclick={() => saveOpenaiEmbeddingModel(preset.value)}
          >
            <span class="preset-name">{preset.label}</span>
            <span class="preset-price">{preset.price}</span>
          </button>
        {/each}
      </div>
      <p class="setting-description">
        <i class="fa-light fa-circle-info"></i>
        {$_("settings.ollama.openaiEmbeddingNote")}
      </p>
    </div>
  {/if}

  <p class="setting-description embedding-warning">
    <i class="fa-solid fa-triangle-exclamation"></i>
    {$_("settings.ollama.embeddingProviderWarning")}
  </p>
</div>

<style>
  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .setting-group {
    margin-bottom: 1.25rem;
    max-width: 600px;
  }

  .setting-group > .label {
    display: block;
    margin-bottom: 0.375rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .setting-description {
    margin: 0.25rem 0 0 0;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  /* Provider Radio Selection */
  .provider-radios {
    display: flex;
    gap: 1.5rem;
    margin-bottom: 0.25rem;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    color: var(--text-primary);
    font-size: 0.9375rem;
  }

  .radio-label input[type="radio"] {
    accent-color: var(--accent-primary);
    width: 1rem;
    height: 1rem;
    cursor: pointer;
  }

  .radio-text {
    user-select: none;
  }

  /* URL row / connection test */
  .url-row {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .text-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.9375rem;
    font-family: inherit;
  }

  .text-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .text-input::placeholder {
    color: var(--text-muted);
  }

  .btn-test {
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

  .btn-test:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-test:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .connection-result {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.5rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    margin-top: 0.375rem;
  }

  .connection-result.success {
    color: var(--status-success);
    background-color: rgba(166, 227, 161, 0.1);
  }

  .connection-result.error {
    color: var(--status-error);
    background-color: rgba(243, 139, 168, 0.1);
  }

  .latency {
    color: var(--text-muted);
    font-size: 0.8125rem;
    margin-left: 0.25rem;
  }

  .error-detail {
    color: var(--text-muted);
    font-size: 0.8125rem;
    margin-left: 0.25rem;
    word-break: break-all;
  }

  /* OpenAI section */
  .section-label {
    font-size: 0.9375rem;
    font-weight: 600;
    margin-bottom: 0.75rem !important;
  }

  .sub-field {
    margin-bottom: 0.75rem;
  }

  .sub-label {
    display: block;
    margin-bottom: 0.25rem;
    font-weight: 500;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .sub-field-group {
    display: flex;
    gap: 1.5rem;
    margin-bottom: 0.75rem;
  }

  .sub-field-half {
    flex: 1;
  }

  .api-key-row {
    display: flex;
    gap: 0.375rem;
    align-items: flex-start;
  }

  .api-key-input {
    flex: 1;
  }

  .btn-toggle-key {
    padding: 0.5rem 0.625rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-toggle-key:hover {
    color: var(--text-primary);
    border-color: var(--accent-primary);
  }

  /* Concurrency Setting */
  .range-container {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .range-input {
    flex: 1;
    accent-color: var(--accent-primary);
  }

  .range-value {
    font-variant-numeric: tabular-nums;
    font-weight: 500;
    min-width: 2ch;
    text-align: right;
  }

  /* Cost tracking */
  .cost-display {
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    margin-bottom: 0.75rem;
  }

  .cost-header {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
    margin-bottom: 0.5rem;
  }

  .cost-spent {
    font-weight: 600;
    color: var(--text-primary);
    font-size: 0.9375rem;
  }

  .cost-limit {
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .cost-bar-container {
    width: 100%;
    height: 6px;
    background-color: var(--bg-muted, rgba(255, 255, 255, 0.1));
    border-radius: 3px;
    overflow: hidden;
  }

  .cost-bar {
    height: 100%;
    background-color: var(--accent-primary);
    border-radius: 3px;
    transition: width 0.3s;
  }

  .cost-bar-warning {
    background-color: var(--accent-warning, #fab387);
  }

  .cost-bar-danger {
    background-color: var(--status-error);
  }

  .cost-footer {
    display: flex;
    justify-content: space-between;
    margin-top: 0.375rem;
    font-size: 0.8125rem;
  }

  .cost-remaining {
    color: var(--text-muted);
  }

  .cost-percentage {
    color: var(--text-muted);
    font-weight: 500;
  }

  .cost-input {
    max-width: 120px;
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

  /* Model Missing Warning */
  .model-warning {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.75rem;
    margin-top: 0.5rem;
    background-color: rgba(250, 179, 135, 0.15);
    border: 1px solid var(--accent-warning);
    border-radius: 0.375rem;
  }

  .warning-icon {
    color: var(--accent-warning);
    font-size: 1rem;
    flex-shrink: 0;
    margin-top: 0.125rem;
  }

  .warning-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .warning-text {
    color: var(--accent-warning);
    font-weight: 500;
    font-size: 0.875rem;
  }

  .warning-hint {
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .btn-download-inline {
    padding: 0.375rem 0.625rem;
    border: 1px solid var(--accent-warning);
    border-radius: 0.375rem;
    background: none;
    color: var(--accent-warning);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.2s;
    flex-shrink: 0;
  }

  .btn-download-inline:hover:not(:disabled) {
    background-color: var(--accent-warning);
    color: var(--bg-surface);
  }

  .btn-download-inline:disabled {
    opacity: 0.6;
    cursor: not-allowed;
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

  /* OpenAI Model Dropdown */
  .openai-model-option {
    flex-direction: column;
    align-items: flex-start !important;
    gap: 0.125rem;
  }

  .openai-model-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }

  .openai-model-name {
    font-weight: 500;
  }

  .openai-model-price {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .custom-model-input {
    margin-top: 0.5rem;
  }

  .temperature-select {
    max-width: 280px;
  }

  /* Embedding Note */
  .embedding-note {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    margin-bottom: 0.75rem !important;
  }

  .embedding-note i {
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  /* Embedding Provider Warning */
  .embedding-warning {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--warning) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--warning) 30%, transparent);
    border-radius: 0.375rem;
    margin-top: 0.75rem !important;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .embedding-warning i {
    color: var(--warning);
    flex-shrink: 0;
  }

  /* OpenAI Embedding Presets */
  .openai-embedding-presets {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .openai-embedding-presets .preset-btn {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.25rem;
    padding: 0.75rem 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    background: var(--bg-surface);
    cursor: pointer;
    flex: 1;
    transition: all 0.15s ease;
  }

  .openai-embedding-presets .preset-btn:hover {
    border-color: var(--accent-primary);
    background: var(--bg-overlay);
  }

  .openai-embedding-presets .preset-btn.active {
    border-color: var(--accent-primary);
    background: color-mix(in srgb, var(--accent-primary) 10%, transparent);
  }

  .openai-embedding-presets .preset-name {
    font-weight: 600;
    font-size: 0.85rem;
  }

  .openai-embedding-presets .preset-price {
    color: var(--text-muted);
    font-size: 0.75rem;
  }
</style>
