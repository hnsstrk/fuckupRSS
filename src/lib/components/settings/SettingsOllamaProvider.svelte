<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";
  import { toasts } from "../../stores/state.svelte";

  let {
    ollamaUrl = $bindable(),
    ollamaStatus,
    loadedModels,
    selectedMainModel = $bindable(),
    selectedEmbeddingModel,
    ollamaNumCtx = $bindable(),
    ollamaConcurrency = $bindable(),
    downloadingModel,
    downloadError,
    testingOllama,
    ollamaTestResult,
    mainModelDropdownOpen = $bindable(),
    numCtxDropdownOpen = $bindable(),
    loadingModels = $bindable(),
    isMainModelMissing,
    onSaveOllamaUrl,
    onTestOllamaConnection,
    onSelectMainModel,
    onDownloadModel,
    onToggleMainModelDropdown,
    onHandleNumCtxChange,
    onHandleConcurrencyChange,
    serverHistory,
    onSelectServer,
    onRemoveServer,
    serverStatusMap,
    proxyRunning,
    proxyStarting,
    proxyRemoteHost = $bindable(),
    proxyRemotePort = $bindable(),
    onStartProxy,
    onStopProxy,
  }: {
    ollamaUrl: string;
    ollamaStatus: {
      available: boolean;
      models: string[];
      recommended_main: string;
      recommended_embedding: string;
      has_recommended_main: boolean;
      has_recommended_embedding: boolean;
    } | null;
    loadedModels: {
      name: string;
      size: number;
      size_vram: number;
      parameter_size: string;
    }[];
    selectedMainModel: string;
    selectedEmbeddingModel: string;
    ollamaNumCtx: number;
    ollamaConcurrency: number;
    downloadingModel: string | null;
    downloadError: string | null;
    testingOllama: boolean;
    ollamaTestResult: {
      success: boolean;
      latency_ms: number;
      models: string[];
    } | null;
    mainModelDropdownOpen: boolean;
    numCtxDropdownOpen: boolean;
    loadingModels: boolean;
    isMainModelMissing: boolean;
    onSaveOllamaUrl: () => void;
    onTestOllamaConnection: () => void;
    onSelectMainModel: (value: string) => void;
    onDownloadModel: (model: string) => void;
    onToggleMainModelDropdown: () => void;
    onHandleNumCtxChange: (value: number) => void;
    onHandleConcurrencyChange: (value: number) => void;
    serverHistory: string[];
    serverStatusMap: Record<string, boolean | null>;
    onSelectServer: (url: string) => void;
    onRemoveServer: (url: string) => void;
    proxyRunning: boolean;
    proxyStarting: boolean;
    proxyRemoteHost: string;
    proxyRemotePort: number;
    onStartProxy: () => void;
    onStopProxy: () => void;
  } = $props();

  let serverDropdownOpen = $state(false);

  const numCtxOptions = [
    { value: 2048, label: "2K", desc: "Minimal - sehr schnell" },
    { value: 4096, label: "4K", desc: "Standard - empfohlen" },
    { value: 8192, label: "8K", desc: "Erweitert - mehr VRAM" },
    { value: 16384, label: "16K", desc: "Gro\u00DF - hoher VRAM-Bedarf" },
    { value: 32768, label: "32K", desc: "Maximum - sehr hoher VRAM-Bedarf" },
  ];

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
</script>

<!-- Ollama Server URL + Connection Test -->
<div class="setting-group">
  <span class="label">{$_("settings.ollama.ollamaSection")}</span>
  <div class="url-row">
    <div class="server-combobox">
      <div class="combobox-input-row">
        <input
          type="text"
          class="text-input combobox-text"
          bind:value={ollamaUrl}
          onblur={() => {
            setTimeout(() => {
              serverDropdownOpen = false;
            }, 200);
            onSaveOllamaUrl();
          }}
          onfocus={() => {
            serverDropdownOpen = true;
          }}
          placeholder="http://localhost:11434"
        />
        <button
          type="button"
          class="combobox-toggle"
          aria-label={$_("settings.ollama.toggleServerList")}
          onclick={() => {
            serverDropdownOpen = !serverDropdownOpen;
          }}
        >
          <i class="fa-solid {serverDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
        </button>
      </div>
      {#if serverDropdownOpen && serverHistory.length > 0}
        <div class="server-dropdown">
          {#each serverHistory as serverUrl (serverUrl)}
            <div class="server-option">
              <button
                type="button"
                class="server-select-btn {serverUrl === ollamaUrl ? 'selected' : ''}"
                onmousedown={(e) => {
                  e.preventDefault();
                  onSelectServer(serverUrl);
                  serverDropdownOpen = false;
                }}
              >
                <span
                  class="status-dot {serverStatusMap[serverUrl] === true
                    ? 'online'
                    : serverStatusMap[serverUrl] === false
                      ? 'offline'
                      : 'unknown'}"
                ></span>
                <span class="server-url-text">{serverUrl}</span>
                {#if serverUrl === "http://localhost:11434"}
                  <span class="default-badge">{$_("settings.ollama.defaultServer")}</span>
                {/if}
              </button>
              {#if serverUrl !== "http://localhost:11434"}
                <button
                  type="button"
                  class="server-remove-btn"
                  title={$_("settings.ollama.removeServer")}
                  onmousedown={(e) => {
                    e.preventDefault();
                    onRemoveServer(serverUrl);
                  }}
                >
                  <i class="fa-solid fa-xmark"></i>
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="btn-test"
      onclick={onTestOllamaConnection}
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

  <!-- LAN-Proxy Section -->
  <div class="proxy-section">
    <div class="proxy-header">
      <i class="fa-light fa-network-wired"></i>
      <span class="proxy-title">{$_("settings.ollama.proxy.title")}</span>
      <span class="status-dot {proxyRunning ? 'online' : 'offline'}"></span>
      <span class="proxy-status-text">
        {proxyRunning ? $_("settings.ollama.proxy.running") : $_("settings.ollama.proxy.stopped")}
      </span>
    </div>
    <p class="setting-description proxy-description">
      {$_("settings.ollama.proxy.description")}
    </p>
    <div class="proxy-fields">
      <div class="proxy-field">
        <label class="sub-label">
          {$_("settings.ollama.proxy.remoteHost")}
          <input
            type="text"
            class="text-input"
            bind:value={proxyRemoteHost}
            placeholder="192.168.1.100"
            disabled={proxyRunning}
          />
        </label>
      </div>
      <div class="proxy-field proxy-port-field">
        <label class="sub-label">
          {$_("settings.ollama.proxy.remotePort")}
          <input
            type="number"
            class="text-input"
            bind:value={proxyRemotePort}
            disabled={proxyRunning}
          />
        </label>
      </div>
    </div>
    <div class="proxy-actions">
      {#if proxyRunning}
        <button type="button" class="btn-test proxy-stop" onclick={onStopProxy}>
          <i class="fa-solid fa-stop"></i>
          {$_("settings.ollama.proxy.stop")}
        </button>
        <span class="proxy-routing">
          <i class="fa-light fa-route"></i>
          {$_("settings.ollama.proxy.routingVia", { values: { port: 11435 } })}
        </span>
      {:else}
        <button
          type="button"
          class="btn-test"
          onclick={onStartProxy}
          disabled={proxyStarting || !proxyRemoteHost}
        >
          {#if proxyStarting}
            <i class="fa-solid fa-spinner fa-spin"></i>
          {:else}
            <i class="fa-solid fa-play"></i>
          {/if}
          {$_("settings.ollama.proxy.start")}
        </button>
      {/if}
    </div>
  </div>
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
            <span class="model-info">{model.parameter_size} · {formatBytes(model.size_vram)}</span>
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
        <button type="button" class="select-trigger" onclick={onToggleMainModelDropdown}>
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
                onclick={() => onSelectMainModel(model)}
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
          onclick={() => onDownloadModel(ollamaStatus.recommended_main)}
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
          onclick={() => onDownloadModel(selectedMainModel)}
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
    <span class="label">{$_("settings.ollama.contextLength") || "Kontext-Länge (num_ctx)"}</span>
    <div class="custom-select">
      <button
        type="button"
        class="select-trigger"
        onclick={() => {
          numCtxDropdownOpen = !numCtxDropdownOpen;
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
              onclick={() => onHandleNumCtxChange(option.value)}
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
        "Höhere Werte erlauben längere Artikel, benötigen aber mehr VRAM. 4K ist für die meisten Artikel ausreichend."}
    </p>
  </div>

  <!-- Ollama Concurrency -->
  <div class="setting-group">
    <span class="label"
      >{$_("settings.ollama.ollamaConcurrency") || "Parallelität (Concurrency)"}</span
    >
    <input
      type="number"
      class="number-input"
      min="1"
      max="16"
      value={ollamaConcurrency}
      onchange={(e) =>
        onHandleConcurrencyChange(parseInt((e.target as HTMLInputElement).value) || 1)}
    />
    <p class="setting-description">
      {$_("settings.ollama.ollamaConcurrencyDescription") ||
        "Anzahl paralleler Requests an Ollama. Bei lokalem Ollama auf 1 lassen."}
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
        "Lädt die ausgewählten Modelle in den Grafikspeicher. Die Auswahl wird automatisch gespeichert."}
    </p>
  </div>

  {#if downloadError}
    <div class="error-message">
      {$_("settings.ollama.downloadError")}: {downloadError}
    </div>
  {/if}
{/if}

<style>
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

  .number-input {
    width: 80px;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.9375rem;
    font-family: inherit;
  }

  .number-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

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

  /* Loaded Models */
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

  /* Context Length */
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

  .error-message {
    color: var(--status-error);
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.375rem;
    font-size: 0.875rem;
  }

  /* Server Combobox */
  .server-combobox {
    flex: 1;
    position: relative;
  }

  .combobox-input-row {
    display: flex;
  }

  .combobox-text {
    border-top-right-radius: 0 !important;
    border-bottom-right-radius: 0 !important;
    border-right: none !important;
  }

  .combobox-toggle {
    padding: 0.5rem 0.625rem;
    border: 1px solid var(--border-default);
    border-top-right-radius: 0.375rem;
    border-bottom-right-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    transition: all 0.15s;
  }

  .combobox-toggle:hover {
    border-color: var(--accent-primary);
    color: var(--text-primary);
  }

  .server-dropdown {
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

  .server-option {
    display: flex;
    align-items: center;
  }

  .server-select-btn {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 0.9375rem;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.15s;
    font-family: inherit;
  }

  .server-select-btn:hover {
    background-color: var(--bg-muted);
  }

  .server-select-btn.selected {
    background-color: var(--bg-muted);
    color: var(--accent-primary);
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-dot.online {
    background-color: var(--status-success);
  }

  .status-dot.offline {
    background-color: var(--status-error);
  }

  .status-dot.unknown {
    background-color: var(--text-muted);
  }

  .server-url-text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .default-badge {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-left: 0.25rem;
  }

  .server-remove-btn {
    padding: 0.5rem;
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: color 0.15s;
    flex-shrink: 0;
  }

  .server-remove-btn:hover {
    color: var(--status-error);
  }

  .sub-label {
    display: block;
    margin-bottom: 0.25rem;
    font-weight: 500;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  /* LAN-Proxy Section */
  .proxy-section {
    margin-top: 1rem;
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
  }

  .proxy-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.375rem;
  }

  .proxy-header i {
    color: var(--accent-primary);
  }

  .proxy-title {
    font-weight: 500;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .proxy-status-text {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .proxy-description {
    margin-bottom: 0.75rem !important;
  }

  .proxy-fields {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .proxy-field {
    flex: 1;
  }

  .proxy-port-field {
    max-width: 120px;
  }

  .proxy-field .text-input {
    width: 100%;
  }

  .proxy-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .proxy-stop {
    border-color: var(--status-error) !important;
    color: var(--status-error) !important;
  }

  .proxy-stop:hover {
    background-color: var(--status-error) !important;
    color: var(--text-on-accent) !important;
  }

  .proxy-routing {
    font-size: 0.8125rem;
    color: var(--status-success);
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }
</style>
