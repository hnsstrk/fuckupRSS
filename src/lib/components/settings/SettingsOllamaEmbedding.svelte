<script lang="ts">
  import { _ } from "svelte-i18n";

  let {
    embeddingProvider = $bindable(),
    openaiEmbeddingModel = $bindable(),
    openaiEmbeddingPresets,
    ollamaStatus,
    selectedEmbeddingModel = $bindable(),
    downloadingModel,
    embeddingModelDropdownOpen = $bindable(),
    embeddingServerDropdownOpen = $bindable(),
    ollamaUrl = $bindable(),
    serverHistory,
    serverStatusMap,
    testingOllama,
    ollamaTestResult,
    aiTextProvider,
    onSaveEmbeddingProvider,
    onSaveOpenaiEmbeddingModel,
    onSelectEmbeddingModel,
    onToggleEmbeddingModelDropdown,
    onDownloadModel,
    onSaveOllamaUrl,
    onTestOllamaConnection,
    onSelectServer,
    onRemoveServer,
  }: {
    embeddingProvider: string;
    openaiEmbeddingModel: string;
    openaiEmbeddingPresets: { value: string; label: string; price: string }[];
    ollamaStatus: {
      available: boolean;
      models: string[];
      recommended_main: string;
      recommended_embedding: string;
      has_recommended_main: boolean;
      has_recommended_embedding: boolean;
    } | null;
    selectedEmbeddingModel: string;
    downloadingModel: string | null;
    embeddingModelDropdownOpen: boolean;
    embeddingServerDropdownOpen: boolean;
    ollamaUrl: string;
    serverHistory: string[];
    serverStatusMap: Record<string, boolean | null>;
    testingOllama: boolean;
    ollamaTestResult: {
      success: boolean;
      latency_ms: number;
      models: string[];
    } | null;
    aiTextProvider: string;
    onSaveEmbeddingProvider: (value: string) => void;
    onSaveOpenaiEmbeddingModel: (value: string) => void;
    onSelectEmbeddingModel: (model: string) => void;
    onToggleEmbeddingModelDropdown: () => void;
    onDownloadModel: (model: string) => void;
    onSaveOllamaUrl: () => void;
    onTestOllamaConnection: () => void;
    onSelectServer: (url: string) => void;
    onRemoveServer: (url: string) => void;
  } = $props();

  function isRecommendedModel(model: string, recommended: string): boolean {
    return model === recommended || model.startsWith(recommended.split(":")[0] + ":");
  }

  let isEmbeddingModelMissing = $derived(
    selectedEmbeddingModel && ollamaStatus?.available && !ollamaStatus?.models.includes(selectedEmbeddingModel),
  );
</script>

<!-- ============================================ -->
<!-- EMBEDDING SECTION (always visible)           -->
<!-- ============================================ -->
<div class="setting-group">
  <span class="label">{$_("settings.ollama.embeddingSection")}</span>
  <p class="setting-description embedding-note">
    <i class="fa-solid fa-circle-info"></i>
    {$_("settings.ollama.embeddingNote")}
  </p>

  <!-- Embedding Provider Selection -->
  <div class="provider-toggle">
    <button
      type="button"
      class="toggle-btn {embeddingProvider === 'ollama' ? 'active' : ''}"
      onclick={() => onSaveEmbeddingProvider("ollama")}
    >
      <i class="fa-solid fa-server"></i>
      Ollama
    </button>
    <button
      type="button"
      class="toggle-btn {embeddingProvider === 'openai_compatible' ? 'active' : ''}"
      onclick={() => onSaveEmbeddingProvider("openai_compatible")}
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
          <div class="server-combobox">
            <div class="combobox-input-row">
              <input
                type="text"
                class="text-input combobox-text"
                bind:value={ollamaUrl}
                onblur={() => {
                  setTimeout(() => {
                    embeddingServerDropdownOpen = false;
                  }, 200);
                  onSaveOllamaUrl();
                }}
                onfocus={() => {
                  embeddingServerDropdownOpen = true;
                }}
                placeholder="http://localhost:11434"
              />
              <button
                type="button"
                class="combobox-toggle"
                aria-label={$_("settings.ollama.toggleServerList")}
                onclick={() => {
                  embeddingServerDropdownOpen = !embeddingServerDropdownOpen;
                }}
              >
                <i class="fa-solid {embeddingServerDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"
                ></i>
              </button>
            </div>
            {#if embeddingServerDropdownOpen && serverHistory.length > 0}
              <div class="server-dropdown">
                {#each serverHistory as serverUrl (serverUrl)}
                  <div class="server-option">
                    <button
                      type="button"
                      class="server-select-btn {serverUrl === ollamaUrl ? 'selected' : ''}"
                      onmousedown={(e) => {
                        e.preventDefault();
                        onSelectServer(serverUrl);
                        embeddingServerDropdownOpen = false;
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
          <button type="button" class="select-trigger" onclick={onToggleEmbeddingModelDropdown}>
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
                  onclick={() => onSelectEmbeddingModel(model)}
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
            onclick={() => onDownloadModel(ollamaStatus!.recommended_embedding)}
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
            onclick={() => onDownloadModel(selectedEmbeddingModel)}
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
            onclick={() => onSaveOpenaiEmbeddingModel(preset.value)}
          >
            <span class="preset-name">{preset.label}</span>
            <span class="preset-price">{preset.price}</span>
          </button>
        {/each}
      </div>
      <p class="setting-description">
        <i class="fa-solid fa-circle-info"></i>
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

  /* Embedding section styles */
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

  /* Provider Toggle */
  .provider-toggle {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .toggle-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: none;
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-btn:hover {
    border-color: var(--accent-primary);
    color: var(--text-primary);
  }

  .toggle-btn.active {
    border-color: var(--accent-primary);
    background-color: color-mix(in srgb, var(--accent-primary) 15%, transparent);
    color: var(--accent-primary);
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

  /* Server Combobox (Embedding URL) */
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
</style>
