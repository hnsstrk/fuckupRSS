<script lang="ts">
  import { _ } from "svelte-i18n";

  let {
    openaiBaseUrl = $bindable(),
    openaiApiKey = $bindable(),
    openaiModel = $bindable(),
    openaiModelPreset,
    openaiModelPresets,
    openaiModelDropdownOpen,
    openaiTemperature = $bindable(),
    openaiConcurrency = $bindable(),
    showApiKey,
    testingOpenai,
    openaiTestResult,
    costLimit = $bindable(),
    monthlyCost,
    onSaveOpenaiSettings,
    onTestOpenaiConnection,
    onHandleOpenaiModelPresetChange,
    onToggleOpenaiModelDropdown,
    onSaveCostLimit,
    onToggleShowApiKey,
  }: {
    openaiBaseUrl: string;
    openaiApiKey: string;
    openaiModel: string;
    openaiModelPreset: string;
    openaiModelPresets: { value: string; label: string; price: string }[];
    openaiModelDropdownOpen: boolean;
    openaiTemperature: string;
    openaiConcurrency: number;
    showApiKey: boolean;
    testingOpenai: boolean;
    openaiTestResult: {
      success: boolean;
      latency_ms: number;
      models: string[];
      error: string | null;
    } | null;
    costLimit: number;
    monthlyCost: {
      spent: number;
      limit: number;
      remaining: number;
      percentage: number;
    } | null;
    onSaveOpenaiSettings: () => void;
    onTestOpenaiConnection: () => void;
    onHandleOpenaiModelPresetChange: (preset: string) => void;
    onToggleOpenaiModelDropdown: () => void;
    onSaveCostLimit: () => void;
    onToggleShowApiKey: () => void;
  } = $props();
</script>

<div class="setting-group">
  <span class="label section-label">{$_("settings.ollama.openaiSection")}</span>

  <!-- API Base URL -->
  <div class="sub-field">
    <span class="sub-label">{$_("settings.ollama.apiBaseUrl")}</span>
    <input
      type="text"
      class="text-input"
      bind:value={openaiBaseUrl}
      onblur={onSaveOpenaiSettings}
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
        onblur={onSaveOpenaiSettings}
        placeholder={$_("settings.ollama.apiKeyPlaceholder")}
      />
      <button
        type="button"
        class="btn-toggle-key"
        onclick={onToggleShowApiKey}
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
      <button type="button" class="select-trigger" onclick={onToggleOpenaiModelDropdown}>
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
              onclick={() => onHandleOpenaiModelPresetChange(preset.value)}
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
            onclick={() => onHandleOpenaiModelPresetChange("custom")}
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
        onblur={onSaveOpenaiSettings}
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
        onchange={onSaveOpenaiSettings}
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
        onchange={onSaveOpenaiSettings}
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
      onclick={onTestOpenaiConnection}
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
      onblur={onSaveCostLimit}
      min="0"
      step="0.5"
    />
    <p class="setting-description">
      {$_("settings.ollama.costLimitDescription")}
    </p>
  </div>
</div>

<style>
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

  /* Connection test */
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
</style>
