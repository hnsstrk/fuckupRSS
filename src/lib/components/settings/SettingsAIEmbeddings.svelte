<script lang="ts">
  import { _ } from "svelte-i18n";

  type OllamaStatus = {
    available: boolean;
    models: string[];
    recommended_main: string;
    recommended_embedding: string;
    has_recommended_main: boolean;
    has_recommended_embedding: boolean;
  };

  let {
    embeddingProviderOptions,
    embeddingProvider = $bindable(),
    selectedEmbeddingModel = $bindable(),
    openaiEmbeddingModel,
    openaiEmbeddingPresets,
    ollamaStatus,
    embeddingModelDropdownOpen = $bindable(),
    downloadingModel,
    onEmbeddingProviderChange,
    onSelectEmbeddingModel,
    onSaveOpenaiEmbeddingModel,
    onDownloadModel,
  }: {
    embeddingProviderOptions: { value: string; labelKey: string; icon: string }[];
    embeddingProvider: string;
    selectedEmbeddingModel: string;
    openaiEmbeddingModel: string;
    openaiEmbeddingPresets: { value: string; label: string; price: string }[];
    ollamaStatus: OllamaStatus | null;
    embeddingModelDropdownOpen: boolean;
    downloadingModel: string | null;
    onEmbeddingProviderChange: (value: string) => void;
    onSelectEmbeddingModel: (model: string) => void;
    onSaveOpenaiEmbeddingModel: (value: string) => void;
    onDownloadModel: (model: string) => void;
  } = $props();

  let isEmbeddingModelMissing = $derived(
    selectedEmbeddingModel &&
      ollamaStatus?.available &&
      !ollamaStatus.models.includes(selectedEmbeddingModel),
  );

  function isRecommendedModel(model: string, recommended: string): boolean {
    return model === recommended || model.startsWith(recommended.split(":")[0] + ":");
  }
</script>

<!-- ============================================ -->
<!-- CARD 3: Embeddings                           -->
<!-- ============================================ -->
<div class="settings-card">
  <div class="card-header">
    <i class="fa-solid fa-vector-square card-icon"></i>
    <div class="card-header-text">
      <span class="card-title">{$_("settings.ai.embeddingSection")}</span>
      <span class="card-description">{$_("settings.ai.embeddingDescription")}</span>
    </div>
  </div>

  <div class="card-body">
    <!-- Provider Select -->
    <div class="field-row">
      <span class="field-label">{$_("settings.ai.provider")}</span>
      <div class="provider-toggle">
        {#each embeddingProviderOptions as opt (opt.value)}
          <button
            type="button"
            class="toggle-btn {embeddingProvider === opt.value ? 'active' : ''}"
            onclick={() => onEmbeddingProviderChange(opt.value)}
          >
            <i class={opt.icon}></i>
            {$_(opt.labelKey)}
          </button>
        {/each}
      </div>
    </div>

    {#if embeddingProvider === "ollama" && ollamaStatus?.available}
      <!-- Ollama Embedding Model -->
      <div class="field-row">
        <span class="field-label">{$_("settings.ollama.embeddingModel")}</span>
        <div class="custom-select model-select">
          <button
            type="button"
            class="select-trigger"
            onclick={() => {
              embeddingModelDropdownOpen = !embeddingModelDropdownOpen;
            }}
          >
            <span>
              {selectedEmbeddingModel || $_("settings.ai.noModelsAvailable")}
              {#if ollamaStatus && isRecommendedModel(selectedEmbeddingModel, ollamaStatus.recommended_embedding)}
                <span class="recommended">{$_("settings.ollama.recommended")}</span>
              {/if}
            </span>
            <i
              class="arrow fa-solid {embeddingModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"
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
      </div>
      {#if isEmbeddingModelMissing}
        <div class="model-warning">
          <i class="warning-icon fa-solid fa-triangle-exclamation"></i>
          <span class="warning-text"
            >{$_("settings.ollama.modelMissing", {
              values: { model: selectedEmbeddingModel },
            })}</span
          >
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
    {:else if embeddingProvider === "openai_compatible"}
      <!-- OpenAI Embedding Model -->
      <div class="field-row">
        <span class="field-label">{$_("settings.ollama.openaiEmbeddingModel")}</span>
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
      </div>
    {/if}

    <!-- Warning about switching providers -->
    <p class="embedding-warning">
      <i class="fa-solid fa-triangle-exclamation"></i>
      {$_("settings.ai.embeddingWarning")}
    </p>
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
  .recommended {
    font-size: 0.75rem;
    color: var(--accent-secondary);
    margin-left: 0.5rem;
  }

  /* ============================================ */
  /* OpenAI Embedding Presets                     */
  /* ============================================ */
  .openai-embedding-presets {
    display: flex;
    gap: 0.5rem;
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
    font-family: inherit;
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

  /* ============================================ */
  /* Warnings                                     */
  /* ============================================ */
  .embedding-warning {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--accent-warning, #fab387) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent-warning, #fab387) 30%, transparent);
    border-radius: 0.375rem;
    margin-top: 0.75rem;
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }

  .embedding-warning i {
    color: var(--accent-warning, #fab387);
    flex-shrink: 0;
  }

  .model-warning {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.625rem 0.75rem;
    margin-top: 0.5rem;
    background-color: rgba(250, 179, 135, 0.15);
    border: 1px solid var(--accent-warning);
    border-radius: 0.375rem;
  }

  .warning-icon {
    color: var(--accent-warning);
    font-size: 0.875rem;
    flex-shrink: 0;
  }

  .warning-text {
    flex: 1;
    color: var(--accent-warning);
    font-weight: 500;
    font-size: 0.8125rem;
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
    font-family: inherit;
  }

  .btn-download-inline:hover:not(:disabled) {
    background-color: var(--accent-warning);
    color: var(--bg-surface);
  }

  .btn-download-inline:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
