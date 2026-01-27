<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import { appState, toasts } from "../../stores/state.svelte";

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

  let selectedMainModel = $state("");
  let selectedEmbeddingModel = $state("");
  let downloadingModel = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let loadingModels = $state(false);
  let pullUnlisten: UnlistenFn | null = $state(null);

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
    await loadOllamaStatus();
    await loadHardwareProfiles();

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
    profileDropdownOpen = false;
  }

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

  async function loadHardwareProfiles() {
    try {
      hardwareProfiles = await invoke<HardwareProfile[]>("get_hardware_profiles");
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

  function isRecommendedModel(model: string, recommended: string): boolean {
    return (
      model === recommended || model.startsWith(recommended.split(":")[0] + ":")
    );
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

  function toggleMainModelDropdown() {
    mainModelDropdownOpen = !mainModelDropdownOpen;
    embeddingModelDropdownOpen = false;
    numCtxDropdownOpen = false;
    profileDropdownOpen = false;
  }

  function toggleEmbeddingModelDropdown() {
    embeddingModelDropdownOpen = !embeddingModelDropdownOpen;
    mainModelDropdownOpen = false;
    numCtxDropdownOpen = false;
    profileDropdownOpen = false;
  }

  function toggleProfileDropdown() {
    profileDropdownOpen = !profileDropdownOpen;
    mainModelDropdownOpen = false;
    embeddingModelDropdownOpen = false;
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
    } catch (e) {
      console.error("Failed to save embedding model setting:", e);
      toasts.error($_("settings.saveError"));
    }
  }

  async function handleProfileSelect(profileId: string) {
    selectedProfileId = profileId;
    profileDropdownOpen = false;
    try {
      await invoke("apply_hardware_profile", { profileId });
      toasts.success($_("settings.profileApplied"));
    } catch (e) {
      console.error("Failed to apply profile:", e);
      toasts.error($_("settings.saveError"));
    }
  }
</script>

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
    <span class="label">{$_("settings.ollama.loadedModels") || "Geladene Modelle (VRAM)"}</span>
    <div class="loaded-models">
      {#if loadedModels.length === 0}
        <div class="no-models">
          {$_("settings.ollama.noLoadedModels") || "Keine Modelle geladen"}
        </div>
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
              <span class="recommended">{$_("settings.ollama.recommended")}</span>
            {/if}
          </span>
          <i class="arrow fa-solid {mainModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
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
              <span class="recommended">{$_("settings.ollama.recommended")}</span>
            {/if}
          </span>
          <i class="arrow fa-solid {embeddingModelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
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
      {$_("settings.ollama.loadModelsDescription") || "Ladt die ausgewahlten Modelle in den Grafikspeicher. Die Auswahl wird automatisch gespeichert."}
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
          {hardwareProfiles.find((p) => p.id === selectedProfileId)?.name || "Default"}
          <span class="profile-parallelism">
            ({hardwareProfiles.find((p) => p.id === selectedProfileId)?.ai_parallelism || 1}x Parallel)
          </span>
        </span>
        <i class="arrow fa-solid {profileDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
      </button>
      {#if profileDropdownOpen}
        <div class="select-options">
          {#each hardwareProfiles as profile}
            <button
              type="button"
              class="select-option profile-option {selectedProfileId === profile.id ? 'selected' : ''}"
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
    <span class="label">{$_("settings.ollama.contextLength") || "Kontext-Lange (num_ctx)"}</span>
    <div class="custom-select">
      <button
        type="button"
        class="select-trigger"
        onclick={() => {
          numCtxDropdownOpen = !numCtxDropdownOpen;
          profileDropdownOpen = false;
          mainModelDropdownOpen = false;
          embeddingModelDropdownOpen = false;
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
      {$_("settings.ollama.contextLengthDescription") || "Hohere Werte erlauben langere Artikel, benotigen aber mehr VRAM. 4K ist fur die meisten Artikel ausreichend."}
    </p>
  </div>

  {#if downloadError}
    <div class="error-message">
      {$_("settings.ollama.downloadError")}: {downloadError}
    </div>
  {/if}
{/if}

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
</style>
