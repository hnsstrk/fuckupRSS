<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import { appState, toasts } from "../../stores/state.svelte";
  import SettingsOllamaProvider from "./SettingsOllamaProvider.svelte";
  import SettingsOpenAiProvider from "./SettingsOpenAiProvider.svelte";
  import SettingsOllamaEmbedding from "./SettingsOllamaEmbedding.svelte";

  // AI Provider state
  let aiTextProvider = $state("ollama");
  let ollamaUrl = $state("http://localhost:11434");
  let openaiBaseUrl = $state("https://api.openai.com");
  let openaiApiKey = $state("");
  let openaiModel = $state("gpt-5-nano");
  let showApiKey = $state(false);

  // Server history
  let serverHistory = $state<string[]>([]);
  let serverStatusMap = $state<Record<string, boolean | null>>({});

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

  // Proxy state
  let proxyRemoteHost = $state("");
  let proxyRemotePort = $state(11434);
  let proxyRunning = $state(false);
  let proxyStarting = $state(false);

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

  // Concurrency
  let ollamaConcurrency = $state(1);

  // Dropdown states
  let mainModelDropdownOpen = $state(false);
  let embeddingModelDropdownOpen = $state(false);
  let numCtxDropdownOpen = $state(false);
  let embeddingServerDropdownOpen = $state(false);

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

    // Load server history
    const savedHistory = await invoke<string | null>("get_setting", {
      key: "ollama_server_history",
    });
    if (savedHistory) {
      try {
        serverHistory = JSON.parse(savedHistory);
      } catch {
        serverHistory = [];
      }
    }
    // Ensure localhost is always present
    if (!serverHistory.includes("http://localhost:11434")) {
      serverHistory = ["http://localhost:11434", ...serverHistory];
    }
    // Check status of all known servers
    checkAllServerStatus();

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

    const savedNumCtx = await invoke<string | null>("get_setting", {
      key: "ollama_num_ctx",
    });
    if (savedNumCtx) {
      ollamaNumCtx = parseInt(savedNumCtx) || DEFAULT_NUM_CTX;
    }

    const savedOllamaConcurrency = await invoke<string | null>("get_setting", {
      key: "ollama_concurrency",
    });
    if (savedOllamaConcurrency) ollamaConcurrency = parseInt(savedOllamaConcurrency) || 1;

    // Load proxy status
    try {
      const proxyStatus = await invoke<{
        running: boolean;
        remote_host: string | null;
        remote_port: number | null;
        local_url: string | null;
      }>("get_ollama_proxy_status");
      proxyRunning = proxyStatus.running;
      if (proxyStatus.remote_host) proxyRemoteHost = proxyStatus.remote_host;
      if (proxyStatus.remote_port) proxyRemotePort = proxyStatus.remote_port;
    } catch {
      /* proxy commands not yet available */
    }
    const savedProxyHost = await invoke<string | null>("get_setting", {
      key: "ollama_proxy_remote_host",
    });
    if (savedProxyHost) proxyRemoteHost = savedProxyHost;
    const savedProxyPort = await invoke<string | null>("get_setting", {
      key: "ollama_proxy_remote_port",
    });
    if (savedProxyPort) proxyRemotePort = parseInt(savedProxyPort) || 11434;

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

  // --- Proxy handler functions ---

  async function startProxy() {
    proxyStarting = true;
    try {
      const result = await invoke<{ running: boolean; local_url: string | null }>(
        "start_ollama_proxy",
        { remoteHost: proxyRemoteHost, remotePort: proxyRemotePort },
      );
      proxyRunning = result.running;
      toasts.success($_("settings.ollama.proxy.running"));
    } catch (e) {
      toasts.error($_("settings.ollama.proxy.startError") + ": " + String(e));
    }
    proxyStarting = false;
  }

  async function stopProxy() {
    try {
      await invoke("stop_ollama_proxy");
      proxyRunning = false;
    } catch {
      /* ignore */
    }
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
      if (ollamaTestResult?.success) {
        await addToServerHistory(ollamaUrl);
        serverStatusMap = { ...serverStatusMap, [ollamaUrl]: true };
      }
    } catch {
      ollamaTestResult = { success: false, latency_ms: 0, models: [] };
    }
    testingOllama = false;
  }

  async function saveOllamaUrl() {
    await invoke("set_setting", { key: "ollama_url", value: ollamaUrl });
  }

  async function checkAllServerStatus() {
    const checks = serverHistory.map(async (url) => {
      try {
        const result = await invoke<{ success: boolean }>("test_ai_provider", {
          providerType: "ollama",
          baseUrl: url,
          apiKey: null,
        });
        return [url, result.success] as const;
      } catch {
        return [url, false] as const;
      }
    });
    const results = await Promise.all(checks);
    const newMap: Record<string, boolean> = {};
    for (const [url, success] of results) {
      newMap[url] = success;
    }
    serverStatusMap = newMap;
  }

  async function saveServerHistory() {
    await invoke("set_setting", {
      key: "ollama_server_history",
      value: JSON.stringify(serverHistory),
    });
  }

  async function addToServerHistory(url: string) {
    if (!serverHistory.includes(url)) {
      serverHistory = [...serverHistory, url];
      await saveServerHistory();
    }
  }

  async function removeFromServerHistory(url: string) {
    if (url === "http://localhost:11434") return;
    serverHistory = serverHistory.filter((u) => u !== url);
    await saveServerHistory();
  }

  async function selectServerFromHistory(url: string) {
    ollamaUrl = url;
    ollamaTestResult = null;
    saveOllamaUrl();
    await loadOllamaStatus();
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
    } catch {
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

  function isModelInstalled(modelName: string): boolean {
    if (!ollamaStatus || !modelName) return true; // Don't show warning if no data
    return ollamaStatus.models.includes(modelName);
  }

  // Check if main model is missing (selected but not installed)
  let isMainModelMissing = $derived(
    selectedMainModel && ollamaStatus?.available && !isModelInstalled(selectedMainModel),
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

  async function handleConcurrencyChange(value: number) {
    ollamaConcurrency = Math.max(1, value);
    try {
      await invoke("set_setting", {
        key: "ollama_concurrency",
        value: ollamaConcurrency.toString(),
      });
    } catch (e) {
      console.error("Failed to save ollama_concurrency setting:", e);
      toasts.error($_("settings.saveError"));
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
  <SettingsOllamaProvider
    bind:ollamaUrl
    {serverHistory}
    {serverStatusMap}
    onSelectServer={selectServerFromHistory}
    onRemoveServer={removeFromServerHistory}
    {ollamaStatus}
    {loadedModels}
    bind:selectedMainModel
    {selectedEmbeddingModel}
    bind:ollamaNumCtx
    bind:ollamaConcurrency
    {downloadingModel}
    {downloadError}
    {testingOllama}
    {ollamaTestResult}
    bind:mainModelDropdownOpen
    bind:numCtxDropdownOpen
    bind:loadingModels
    isMainModelMissing={!!isMainModelMissing}
    onSaveOllamaUrl={saveOllamaUrl}
    onTestOllamaConnection={testOllamaConnection}
    onSelectMainModel={selectMainModel}
    onDownloadModel={handleDownloadModel}
    onToggleMainModelDropdown={toggleMainModelDropdown}
    onHandleNumCtxChange={handleNumCtxChange}
    onHandleConcurrencyChange={handleConcurrencyChange}
    {proxyRunning}
    {proxyStarting}
    bind:proxyRemoteHost
    bind:proxyRemotePort
    onStartProxy={startProxy}
    onStopProxy={stopProxy}
  />
{/if}

<!-- ============================================ -->
<!-- OPENAI PROVIDER SECTION                      -->
<!-- ============================================ -->
{#if aiTextProvider === "openai_compatible"}
  <SettingsOpenAiProvider
    bind:openaiBaseUrl
    bind:openaiApiKey
    bind:openaiModel
    {openaiModelPreset}
    {openaiModelPresets}
    {openaiModelDropdownOpen}
    bind:openaiTemperature
    bind:openaiConcurrency
    {showApiKey}
    {testingOpenai}
    {openaiTestResult}
    bind:costLimit
    {monthlyCost}
    onSaveOpenaiSettings={saveOpenaiSettings}
    onTestOpenaiConnection={testOpenaiConnection}
    onHandleOpenaiModelPresetChange={handleOpenaiModelPresetChange}
    onToggleOpenaiModelDropdown={toggleOpenaiModelDropdown}
    onSaveCostLimit={saveCostLimit}
    onToggleShowApiKey={() => (showApiKey = !showApiKey)}
  />
{/if}

<!-- ============================================ -->
<!-- EMBEDDING SECTION (always visible)           -->
<!-- ============================================ -->
<SettingsOllamaEmbedding
  bind:embeddingProvider
  bind:openaiEmbeddingModel
  {openaiEmbeddingPresets}
  {ollamaStatus}
  bind:selectedEmbeddingModel
  {downloadingModel}
  bind:embeddingModelDropdownOpen
  bind:embeddingServerDropdownOpen
  bind:ollamaUrl
  {serverHistory}
  {serverStatusMap}
  {testingOllama}
  {ollamaTestResult}
  {aiTextProvider}
  onSaveEmbeddingProvider={saveEmbeddingProvider}
  onSaveOpenaiEmbeddingModel={saveOpenaiEmbeddingModel}
  onSelectEmbeddingModel={selectEmbeddingModel}
  onToggleEmbeddingModelDropdown={toggleEmbeddingModelDropdown}
  onDownloadModel={handleDownloadModel}
  onSaveOllamaUrl={saveOllamaUrl}
  onTestOllamaConnection={testOllamaConnection}
  onSelectServer={selectServerFromHistory}
  onRemoveServer={removeFromServerHistory}
/>

<style>
  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .setting-group {
    margin-bottom: 1.25rem;
    max-width: 900px;
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
</style>
