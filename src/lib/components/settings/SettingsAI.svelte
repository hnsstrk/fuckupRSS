<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import { appState, toasts } from "../../stores/state.svelte";
  import SettingsOllamaProvider from "./SettingsOllamaProvider.svelte";
  import SettingsOpenAiProvider from "./SettingsOpenAiProvider.svelte";
  import SettingsAIAnalysis from "./SettingsAIAnalysis.svelte";
  import SettingsAIEmbeddings from "./SettingsAIEmbeddings.svelte";

  // ============================================
  // Provider types
  // ============================================
  type ProviderType = "ollama" | "openai_compatible" | "gemini_cli" | "claude_code_cli";

  const providerOptions: { value: ProviderType; labelKey: string; icon: string }[] = [
    { value: "ollama", labelKey: "settings.ai.providerOllama", icon: "fa-solid fa-server" },
    {
      value: "openai_compatible",
      labelKey: "settings.ai.providerOpenAi",
      icon: "fa-solid fa-cloud",
    },
    {
      value: "gemini_cli",
      labelKey: "settings.ai.providerGeminiCli",
      icon: "fa-solid fa-terminal",
    },
    {
      value: "claude_code_cli",
      labelKey: "settings.ai.providerClaudeCli",
      icon: "fa-solid fa-terminal",
    },
  ];

  const embeddingProviderOptions: { value: string; labelKey: string; icon: string }[] = [
    { value: "ollama", labelKey: "settings.ai.providerOllama", icon: "fa-solid fa-server" },
    {
      value: "openai_compatible",
      labelKey: "settings.ai.providerOpenAi",
      icon: "fa-solid fa-cloud",
    },
  ];

  // ============================================
  // Fast Analysis state
  // ============================================
  let fastProvider = $state<ProviderType>("ollama");
  let selectedMainModel = $state("");

  // ============================================
  // Reasoning / Deep Analysis state
  // ============================================
  let reasoningProvider = $state<ProviderType>("ollama");
  let reasoningModel = $state("");
  let reasoningNumCtx = $state(4096);

  // ============================================
  // Embedding state
  // ============================================
  let embeddingProvider = $state("ollama");
  let selectedEmbeddingModel = $state("");
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

  // ============================================
  // Ollama Server state
  // ============================================
  let ollamaUrl = $state("http://localhost:11434");
  let serverHistory = $state<string[]>([]);
  let serverStatusMap = $state<Record<string, boolean | null>>({});
  let testingOllama = $state(false);
  let ollamaTestResult = $state<{
    success: boolean;
    latency_ms: number;
    models: string[];
  } | null>(null);
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
  let downloadingModel = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let loadingModels = $state(false);
  let pullUnlisten: UnlistenFn | null = $state(null);

  // Ollama concurrency
  let ollamaConcurrency = $state(1);

  // Context length (num_ctx)
  const DEFAULT_NUM_CTX = 4096;
  let ollamaNumCtx = $state(DEFAULT_NUM_CTX);

  // Proxy state
  let proxyRemoteHost = $state("");
  let proxyRemotePort = $state(11434);
  let proxyRunning = $state(false);
  let proxyStarting = $state(false);

  // ============================================
  // OpenAI API state
  // ============================================
  let openaiBaseUrl = $state("https://api.openai.com");
  let openaiApiKey = $state("");
  let openaiModel = $state("gpt-5-nano");
  let showApiKey = $state(false);

  const openaiModelPresets = [
    { value: "gpt-5-nano", label: "GPT-5 nano", price: "$0.05/$0.40 per 1M tokens" },
    { value: "gpt-5-mini", label: "GPT-5 mini", price: "$0.25/$2.00 per 1M tokens" },
    { value: "gpt-4.1-mini", label: "GPT-4.1 mini", price: "$0.40/$1.60 per 1M tokens" },
    { value: "gpt-4.1-nano", label: "GPT-4.1 nano", price: "$0.10/$0.40 per 1M tokens" },
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
  let testingOpenai = $state(false);
  let openaiTestResult = $state<{
    success: boolean;
    latency_ms: number;
    models: string[];
    error: string | null;
  } | null>(null);

  // ============================================
  // Dropdown states
  // ============================================
  let mainModelDropdownOpen = $state(false);
  let reasoningModelDropdownOpen = $state(false);
  let embeddingModelDropdownOpen = $state(false);
  let numCtxDropdownOpen = $state(false);

  // ============================================
  // Section collapse state
  // ============================================
  let ollamaServerExpanded = $state(true);
  let openaiSectionExpanded = $state(true);

  // ============================================
  // Auto-detect model suggestions (Task 11)
  // ============================================
  const fastModelPriority = ["qwen3:8b", "qwen3:4b", "ministral-3:latest"];

  const reasoningModelPriority = ["deepseek-r1:32b", "deepseek-r1:14b", "qwen3:14b"];

  let suggestedFastModel = $derived.by(() => {
    if (!ollamaStatus?.available || !ollamaStatus.models.length) return null;
    for (const preferred of fastModelPriority) {
      const found = ollamaStatus.models.find(
        (m) => m === preferred || m.startsWith(preferred.split(":")[0] + ":"),
      );
      if (found) return found;
    }
    return ollamaStatus.models[0] || null;
  });

  let suggestedReasoningModel = $derived.by(() => {
    if (!ollamaStatus?.available || !ollamaStatus.models.length) return null;
    for (const preferred of reasoningModelPriority) {
      const found = ollamaStatus.models.find(
        (m) => m === preferred || m.startsWith(preferred.split(":")[0] + ":"),
      );
      if (found) return found;
    }
    return ollamaStatus.models[0] || null;
  });

  // ============================================
  // Derived: needs ollama / openai sections
  // ============================================
  let needsOllamaServer = $derived(
    fastProvider === "ollama" || reasoningProvider === "ollama" || embeddingProvider === "ollama",
  );

  let needsOpenAiSection = $derived(
    fastProvider === "openai_compatible" || reasoningProvider === "openai_compatible",
  );

  // Model missing checks
  let isMainModelMissing = $derived(
    selectedMainModel && ollamaStatus?.available && !isModelInstalled(selectedMainModel),
  );

  // ============================================
  // Context length options
  // ============================================
  const numCtxOptions = [
    { value: 2048, label: "2K" },
    { value: 4096, label: "4K" },
    { value: 8192, label: "8K" },
    { value: 16384, label: "16K" },
    { value: 32768, label: "32K" },
  ];

  // ============================================
  // Exports for parent component
  // ============================================
  export function getOllamaStatus() {
    return ollamaStatus;
  }

  export async function init() {
    // Load AI provider settings (fast = ai_text_provider for backward compat)
    const savedProvider = await invoke<string | null>("get_setting", {
      key: "ai_text_provider",
    });
    if (savedProvider) fastProvider = savedProvider as ProviderType;

    // Load reasoning provider (new setting, defaults to same as fast)
    const savedReasoningProvider = await invoke<string | null>("get_setting", {
      key: "ai_reasoning_provider",
    });
    if (savedReasoningProvider) {
      reasoningProvider = savedReasoningProvider as ProviderType;
    } else {
      reasoningProvider = fastProvider;
    }

    // Load reasoning model
    const savedReasoningModel = await invoke<string | null>("get_setting", {
      key: "reasoning_model",
    });
    if (savedReasoningModel) reasoningModel = savedReasoningModel;

    // Load reasoning num_ctx
    const savedReasoningNumCtx = await invoke<string | null>("get_setting", {
      key: "reasoning_num_ctx",
    });
    if (savedReasoningNumCtx) {
      reasoningNumCtx = parseInt(savedReasoningNumCtx) || 4096;
    }

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
    if (!serverHistory.includes("http://localhost:11434")) {
      serverHistory = ["http://localhost:11434", ...serverHistory];
    }
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

    // Auto-expand sections based on provider selection
    ollamaServerExpanded = needsOllamaServer;
    openaiSectionExpanded = needsOpenAiSection;
  }

  onDestroy(() => {
    if (pullUnlisten) {
      pullUnlisten();
    }
  });

  export function closeAllDropdowns() {
    mainModelDropdownOpen = false;
    reasoningModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
    numCtxDropdownOpen = false;
    openaiModelDropdownOpen = false;
  }

  // ============================================
  // Ollama handler functions
  // ============================================
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
        if (selectedMainModel && !appState.selectedModel) {
          appState.selectedModel = selectedMainModel;
        }

        // Auto-suggest reasoning model if not set
        if (!reasoningModel && suggestedReasoningModel) {
          reasoningModel = suggestedReasoningModel;
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
    if (!ollamaStatus || !modelName) return true;
    return ollamaStatus.models.includes(modelName);
  }

  // ============================================
  // Provider change handlers
  // ============================================
  async function handleFastProviderChange(provider: ProviderType) {
    fastProvider = provider;
    // Save as ai_text_provider for backward compat with batch processor
    await invoke("set_setting", { key: "ai_text_provider", value: provider });
    ollamaServerExpanded = needsOllamaServer;
    openaiSectionExpanded = needsOpenAiSection;
  }

  async function handleReasoningProviderChange(provider: ProviderType) {
    reasoningProvider = provider;
    await invoke("set_setting", { key: "ai_reasoning_provider", value: provider });
    ollamaServerExpanded = needsOllamaServer;
    openaiSectionExpanded = needsOpenAiSection;
  }

  async function handleEmbeddingProviderChange(value: string) {
    embeddingProvider = value;
    try {
      await invoke("set_setting", { key: "embedding_provider", value });
    } catch (e) {
      console.error("Failed to save embedding provider setting:", e);
      toasts.error($_("settings.saveError"));
    }
    ollamaServerExpanded = needsOllamaServer;
  }

  // ============================================
  // Model selection handlers
  // ============================================
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

  async function selectReasoningModel(value: string) {
    reasoningModel = value;
    reasoningModelDropdownOpen = false;
    try {
      await invoke("set_setting", { key: "reasoning_model", value });
    } catch (e) {
      console.error("Failed to save reasoning model setting:", e);
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

  // ============================================
  // Settings save handlers
  // ============================================
  async function handleNumCtxChange(value: number) {
    ollamaNumCtx = value;
    numCtxDropdownOpen = false;
    try {
      await invoke("set_setting", { key: "ollama_num_ctx", value: value.toString() });
    } catch (e) {
      console.error("Failed to save num_ctx setting:", e);
      toasts.error($_("settings.saveError"));
    }
  }

  async function handleReasoningNumCtxChange(value: number) {
    reasoningNumCtx = value;
    try {
      await invoke("set_setting", { key: "reasoning_num_ctx", value: value.toString() });
    } catch (e) {
      console.error("Failed to save reasoning_num_ctx setting:", e);
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

  async function saveOllamaUrl() {
    await invoke("set_setting", { key: "ollama_url", value: ollamaUrl });
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

  // ============================================
  // Proxy handlers
  // ============================================
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

  // ============================================
  // OpenAI handler functions
  // ============================================
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
    await invoke("set_setting", { key: "openai_base_url", value: openaiBaseUrl });
    await invoke("set_setting", { key: "openai_api_key", value: openaiApiKey });
    await invoke("set_setting", { key: "openai_model", value: openaiModel });
    await invoke("set_setting", { key: "openai_temperature", value: openaiTemperature });
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
  }

  async function saveCostLimit() {
    await invoke("set_setting", { key: "cost_limit_monthly", value: costLimit.toString() });
  }

  async function loadMonthlyCost() {
    try {
      monthlyCost = await invoke("get_monthly_cost");
    } catch {
      monthlyCost = null;
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

  // ============================================
  // Dropdown toggle helpers
  // ============================================
  function toggleMainModelDropdown() {
    mainModelDropdownOpen = !mainModelDropdownOpen;
    reasoningModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
    numCtxDropdownOpen = false;
  }

</script>

<h3>{$_("settings.ai.title")}</h3>

<SettingsAIAnalysis
  {providerOptions}
  bind:fastProvider
  bind:reasoningProvider
  bind:selectedMainModel
  bind:reasoningModel
  bind:reasoningNumCtx
  bind:ollamaConcurrency
  {ollamaStatus}
  {loadedModels}
  bind:mainModelDropdownOpen
  bind:reasoningModelDropdownOpen
  {suggestedFastModel}
  {suggestedReasoningModel}
  {fastModelPriority}
  {reasoningModelPriority}
  {numCtxOptions}
  onFastProviderChange={handleFastProviderChange}
  onReasoningProviderChange={handleReasoningProviderChange}
  onSelectMainModel={selectMainModel}
  onSelectReasoningModel={selectReasoningModel}
  onConcurrencyChange={handleConcurrencyChange}
  onReasoningNumCtxChange={handleReasoningNumCtxChange}
/>

<SettingsAIEmbeddings
  {embeddingProviderOptions}
  bind:embeddingProvider
  bind:selectedEmbeddingModel
  {openaiEmbeddingModel}
  {openaiEmbeddingPresets}
  {ollamaStatus}
  bind:embeddingModelDropdownOpen
  {downloadingModel}
  onEmbeddingProviderChange={handleEmbeddingProviderChange}
  onSelectEmbeddingModel={selectEmbeddingModel}
  onSaveOpenaiEmbeddingModel={saveOpenaiEmbeddingModel}
  onDownloadModel={handleDownloadModel}
/>

<!-- ============================================ -->
<!-- CARD 4: Ollama Server (collapsible)          -->
<!-- ============================================ -->
{#if needsOllamaServer}
  <div class="settings-card">
    <button
      type="button"
      class="card-header card-header-toggle"
      onclick={() => (ollamaServerExpanded = !ollamaServerExpanded)}
    >
      <i class="fa-solid fa-server card-icon"></i>
      <div class="card-header-text">
        <span class="card-title">{$_("settings.ai.ollamaServer")}</span>
      </div>
      <div class="card-header-status">
        {#if ollamaStatus?.available}
          <span class="status-pill online">
            <i class="fa-solid fa-check"></i>
            {$_("settings.ollama.connected")}
          </span>
        {:else}
          <span class="status-pill offline">
            <i class="fa-solid fa-xmark"></i>
            {$_("settings.ollama.disconnected")}
          </span>
        {/if}
        <i
          class="fa-solid {ollamaServerExpanded
            ? 'fa-chevron-up'
            : 'fa-chevron-down'} toggle-chevron"
        ></i>
      </div>
    </button>

    {#if ollamaServerExpanded}
      <div class="card-body">
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
      </div>
    {/if}
  </div>
{/if}

<!-- ============================================ -->
<!-- CARD 5: OpenAI API (collapsible)             -->
<!-- ============================================ -->
{#if needsOpenAiSection}
  <div class="settings-card">
    <button
      type="button"
      class="card-header card-header-toggle"
      onclick={() => (openaiSectionExpanded = !openaiSectionExpanded)}
    >
      <i class="fa-solid fa-cloud card-icon"></i>
      <div class="card-header-text">
        <span class="card-title">{$_("settings.ollama.openaiSection")}</span>
      </div>
      <div class="card-header-status">
        {#if openaiTestResult?.success}
          <span class="status-pill online">
            <i class="fa-solid fa-check"></i>
            {$_("settings.ollama.connected")}
          </span>
        {/if}
        <i
          class="fa-solid {openaiSectionExpanded
            ? 'fa-chevron-up'
            : 'fa-chevron-down'} toggle-chevron"
        ></i>
      </div>
    </button>

    {#if openaiSectionExpanded}
      <div class="card-body">
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
      </div>
    {/if}
  </div>
{/if}

{#if downloadError}
  <div class="error-message">
    {$_("settings.ollama.downloadError")}: {downloadError}
  </div>
{/if}

<style>
  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  /* ============================================ */
  /* Card layout                                  */
  /* ============================================ */
  .settings-card {
    max-width: 900px;
    margin-bottom: 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    background-color: var(--bg-surface);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.875rem 1rem;
    background-color: var(--bg-overlay);
    border-bottom: 1px solid var(--border-default);
    border-radius: 0.5rem 0.5rem 0 0;
  }

  .card-header:last-child {
    border-bottom: none;
    border-radius: 0.5rem;
  }

  .card-header-toggle {
    width: 100%;
    border: none;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    transition: background-color 0.15s;
  }

  .card-header-toggle:hover {
    background-color: var(--bg-muted);
  }

  .card-icon {
    font-size: 1rem;
    color: var(--accent-primary);
    flex-shrink: 0;
    width: 1.25rem;
    text-align: center;
  }

  .card-header-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .card-title {
    font-weight: 600;
    font-size: 0.9375rem;
    color: var(--text-primary);
  }

  .card-description {
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .card-header-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .toggle-chevron {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .card-body {
    padding: 1rem;
    border-radius: 0 0 0.5rem 0.5rem;
  }

  /* ============================================ */
  /* Status pills                                 */
  /* ============================================ */
  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.625rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .status-pill.online {
    background-color: rgba(166, 227, 161, 0.15);
    color: var(--status-success);
  }

  .status-pill.offline {
    background-color: rgba(243, 139, 168, 0.15);
    color: var(--status-error);
  }

  .error-message {
    max-width: 900px;
    color: var(--status-error);
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.375rem;
    font-size: 0.875rem;
  }
</style>
