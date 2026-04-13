<script lang="ts">
  import { _ } from "svelte-i18n";

  type ProviderType = "ollama" | "openai_compatible";

  type OllamaStatus = {
    available: boolean;
    models: string[];
    recommended_main: string;
    recommended_embedding: string;
    has_recommended_main: boolean;
    has_recommended_embedding: boolean;
  };

  type LoadedModel = {
    name: string;
    size: number;
    size_vram: number;
    parameter_size: string;
  };

  let {
    providerOptions,
    fastProvider = $bindable(),
    reasoningProvider = $bindable(),
    selectedMainModel = $bindable(),
    reasoningModel = $bindable(),
    reasoningNumCtx = $bindable(),
    ollamaConcurrency = $bindable(),
    ollamaStatus,
    loadedModels,
    mainModelDropdownOpen = $bindable(),
    reasoningModelDropdownOpen = $bindable(),
    suggestedFastModel,
    suggestedReasoningModel,
    fastModelPriority,
    reasoningModelPriority,
    numCtxOptions,
    onFastProviderChange,
    onReasoningProviderChange,
    onSelectMainModel,
    onSelectReasoningModel,
    onConcurrencyChange,
    onReasoningNumCtxChange,
  }: {
    providerOptions: { value: ProviderType; labelKey: string; icon: string }[];
    fastProvider: ProviderType;
    reasoningProvider: ProviderType;
    selectedMainModel: string;
    reasoningModel: string;
    reasoningNumCtx: number;
    ollamaConcurrency: number;
    ollamaStatus: OllamaStatus | null;
    loadedModels: LoadedModel[];
    mainModelDropdownOpen: boolean;
    reasoningModelDropdownOpen: boolean;
    suggestedFastModel: string | null;
    suggestedReasoningModel: string | null;
    fastModelPriority: string[];
    reasoningModelPriority: string[];
    numCtxOptions: { value: number; label: string }[];
    onFastProviderChange: (provider: ProviderType) => void;
    onReasoningProviderChange: (provider: ProviderType) => void;
    onSelectMainModel: (model: string) => void;
    onSelectReasoningModel: (model: string) => void;
    onConcurrencyChange: (value: number) => void;
    onReasoningNumCtxChange: (value: number) => void;
  } = $props();

  function isModelLoaded(modelName: string): boolean {
    return loadedModels.some((m) => m.name === modelName);
  }

  function isSuggestedModel(model: string, suggestions: string[]): boolean {
    for (const preferred of suggestions) {
      if (model === preferred || model.startsWith(preferred.split(":")[0] + ":")) {
        return true;
      }
    }
    return false;
  }
</script>

<!-- ============================================ -->
<!-- CARD 1: Fast Analysis                        -->
<!-- ============================================ -->
<div class="settings-card">
  <div class="card-header">
    <i class="fa-solid fa-bolt card-icon"></i>
    <div class="card-header-text">
      <span class="card-title">{$_("settings.ai.fastSection")}</span>
      <span class="card-description">{$_("settings.ai.fastDescription")}</span>
    </div>
  </div>

  <div class="card-body">
    <!-- Provider Select -->
    <div class="field-row">
      <span class="field-label">{$_("settings.ai.provider")}</span>
      <div class="provider-toggle">
        {#each providerOptions as opt (opt.value)}
          <button
            type="button"
            class="toggle-btn {fastProvider === opt.value ? 'active' : ''}"
            onclick={() => onFastProviderChange(opt.value)}
          >
            <i class={opt.icon}></i>
            {$_(opt.labelKey)}
          </button>
        {/each}
      </div>
    </div>

    <!-- Model + Concurrency (Ollama) — side by side -->
    {#if fastProvider === "ollama" && ollamaStatus?.available}
      <div class="field-row-grid">
        <div class="field-row">
          <span class="field-label">{$_("settings.ai.model")}</span>
          <div class="custom-select model-select">
            <button
              type="button"
              class="select-trigger"
              onclick={() => {
                mainModelDropdownOpen = !mainModelDropdownOpen;
              }}
            >
              <span>
                {selectedMainModel || $_("settings.ai.noModelsAvailable")}
                {#if isModelLoaded(selectedMainModel)}
                  <i class="loaded-badge fa-solid fa-circle"></i>
                {/if}
                {#if suggestedFastModel && selectedMainModel === suggestedFastModel}
                  <span class="suggested-badge">{$_("settings.ai.suggested")}</span>
                {/if}
              </span>
              <i class="arrow fa-solid {mainModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"
              ></i>
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
                    {#if isSuggestedModel(model, fastModelPriority)}
                      <span class="suggested-badge">{$_("settings.ai.suggested")}</span>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <div class="field-row">
          <span class="field-label">{$_("settings.ai.concurrency")}</span>
          <input
            type="number"
            class="number-input"
            min="1"
            max="16"
            value={ollamaConcurrency}
            onchange={(e) => onConcurrencyChange(parseInt((e.target as HTMLInputElement).value) || 1)}
          />
        </div>
      </div>
    {:else if fastProvider === "ollama"}
      <!-- Only concurrency when no models available -->
      <div class="field-row">
        <span class="field-label">{$_("settings.ai.concurrency")}</span>
        <input
          type="number"
          class="number-input"
          min="1"
          max="16"
          value={ollamaConcurrency}
          onchange={(e) =>
            onConcurrencyChange(parseInt((e.target as HTMLInputElement).value) || 1)}
        />
      </div>
    {/if}
  </div>
</div>

<!-- ============================================ -->
<!-- CARD 2: Deep Analysis & Briefings            -->
<!-- ============================================ -->
<div class="settings-card">
  <div class="card-header">
    <i class="fa-solid fa-brain card-icon"></i>
    <div class="card-header-text">
      <span class="card-title">{$_("settings.ai.reasoningSection")}</span>
      <span class="card-description">{$_("settings.ai.reasoningDescription")}</span>
    </div>
  </div>

  <div class="card-body">
    <!-- Provider Select -->
    <div class="field-row">
      <span class="field-label">{$_("settings.ai.provider")}</span>
      <div class="provider-toggle">
        {#each providerOptions as opt (opt.value)}
          <button
            type="button"
            class="toggle-btn {reasoningProvider === opt.value ? 'active' : ''}"
            onclick={() => onReasoningProviderChange(opt.value)}
          >
            <i class={opt.icon}></i>
            {$_(opt.labelKey)}
          </button>
        {/each}
      </div>
    </div>

    <!-- Reasoning Model + Context Length (Ollama) — side by side -->
    {#if reasoningProvider === "ollama" && ollamaStatus?.available}
      <div class="field-row-grid">
        <div class="field-row">
          <span class="field-label">{$_("settings.ai.reasoningModel")}</span>
          <div class="custom-select model-select">
            <button
              type="button"
              class="select-trigger"
              onclick={() => {
                reasoningModelDropdownOpen = !reasoningModelDropdownOpen;
              }}
            >
              <span>
                {reasoningModel || $_("settings.ai.noModelsAvailable")}
                {#if isModelLoaded(reasoningModel)}
                  <i class="loaded-badge fa-solid fa-circle"></i>
                {/if}
                {#if suggestedReasoningModel && reasoningModel === suggestedReasoningModel}
                  <span class="suggested-badge">{$_("settings.ai.suggested")}</span>
                {/if}
              </span>
              <i
                class="arrow fa-solid {reasoningModelDropdownOpen
                  ? 'fa-caret-up'
                  : 'fa-caret-down'}"
              ></i>
            </button>
            {#if reasoningModelDropdownOpen}
              <div class="select-options">
                {#each ollamaStatus.models as model (model)}
                  <button
                    type="button"
                    class="select-option {reasoningModel === model ? 'selected' : ''}"
                    onclick={() => onSelectReasoningModel(model)}
                  >
                    {model}
                    {#if isModelLoaded(model)}
                      <i class="loaded-badge fa-solid fa-circle"></i>
                    {/if}
                    {#if isSuggestedModel(model, reasoningModelPriority)}
                      <span class="suggested-badge">{$_("settings.ai.suggested")}</span>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <div class="field-row">
          <span class="field-label">{$_("settings.ai.contextLength")}</span>
          <div class="ctx-buttons">
            {#each numCtxOptions as opt (opt.value)}
              <button
                type="button"
                class="ctx-btn {reasoningNumCtx === opt.value ? 'active' : ''}"
                onclick={() => onReasoningNumCtxChange(opt.value)}
              >
                {opt.label}
              </button>
            {/each}
          </div>
        </div>
      </div>
    {:else if reasoningProvider === "ollama"}
      <!-- Only context length when no models available -->
      <div class="field-row">
        <span class="field-label">{$_("settings.ai.contextLength")}</span>
        <div class="ctx-buttons">
          {#each numCtxOptions as opt (opt.value)}
            <button
              type="button"
              class="ctx-btn {reasoningNumCtx === opt.value ? 'active' : ''}"
              onclick={() => onReasoningNumCtxChange(opt.value)}
            >
              {opt.label}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
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

  .card-body {
    padding: 1rem;
    border-radius: 0 0 0.5rem 0.5rem;
  }

  /* ============================================ */
  /* Provider toggle                              */
  /* ============================================ */
  .provider-toggle {
    display: flex;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }

  .toggle-btn:hover {
    border-color: var(--accent-primary);
    background: var(--bg-overlay);
  }

  .toggle-btn.active {
    border-color: var(--accent-primary);
    background: color-mix(in srgb, var(--accent-primary) 12%, transparent);
    color: var(--accent-primary);
    font-weight: 500;
  }

  .toggle-btn i {
    font-size: 0.75rem;
  }

  /* ============================================ */
  /* Field rows                                   */
  /* ============================================ */
  .field-row {
    margin-bottom: 0.875rem;
  }

  .field-row:last-child {
    margin-bottom: 0;
  }

  /* Two-column grid for model + parameter side by side */
  .field-row-grid {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 1rem;
    align-items: start;
  }

  .field-row-grid > .field-row {
    margin-bottom: 0;
  }

  @media (max-width: 600px) {
    .field-row-grid {
      grid-template-columns: 1fr;
    }
  }

  .field-label {
    display: block;
    margin-bottom: 0.375rem;
    font-weight: 500;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  /* ============================================ */
  /* Custom Select                                */
  /* ============================================ */
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
    font-size: 0.9375rem;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    text-align: left;
    font-family: inherit;
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
    font-size: 0.9375rem;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.15s;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-family: inherit;
  }

  .select-option:hover {
    background-color: var(--bg-muted);
  }

  .select-option.selected {
    background-color: var(--bg-muted);
    color: var(--accent-primary);
  }

  /* ============================================ */
  /* Badges                                       */
  /* ============================================ */
  .suggested-badge {
    font-size: 0.6875rem;
    color: var(--accent-primary);
    background: color-mix(in srgb, var(--accent-primary) 12%, transparent);
    padding: 0.125rem 0.5rem;
    border-radius: 999px;
    margin-left: 0.5rem;
    font-weight: 500;
  }

  .loaded-badge {
    font-size: 0.5rem;
    color: var(--status-success);
    margin-left: 0.25rem;
  }

  /* ============================================ */
  /* Context Length Buttons                       */
  /* ============================================ */
  .ctx-buttons {
    display: flex;
    gap: 0.375rem;
  }

  .ctx-btn {
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    font-family: inherit;
  }

  .ctx-btn:hover {
    border-color: var(--accent-primary);
  }

  .ctx-btn.active {
    border-color: var(--accent-primary);
    background: color-mix(in srgb, var(--accent-primary) 12%, transparent);
    color: var(--accent-primary);
  }

  /* ============================================ */
  /* Number input                                 */
  /* ============================================ */
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
</style>
