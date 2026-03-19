<script lang="ts">
  import { _ } from "svelte-i18n";
  import { onMount, tick } from "svelte";
  import { appState } from "../stores/state.svelte";
  import { settings } from "../stores/settings.svelte";
  import Tabs, { type Tab } from "./Tabs.svelte";
  import Tooltip from "./Tooltip.svelte";
  import SettingsGeneral from "./settings/SettingsGeneral.svelte";
  import SettingsAI from "./settings/SettingsAI.svelte";
  import SettingsPrompts from "./settings/SettingsPrompts.svelte";
  import SettingsStopwords from "./settings/SettingsStopwords.svelte";
  import SettingsMaintenance from "./settings/SettingsMaintenance.svelte";

  // Tab state
  let activeTab = $state<string>("general");

  // Component references (using $state to satisfy Svelte 5 bind:this requirements)
  let settingsGeneralRef = $state<
    { init: () => void; closeAllDropdowns: () => void } | undefined
  >();
  let settingsAIRef = $state<
    | {
        init: () => Promise<void>;
        closeAllDropdowns: () => void;
        getOllamaStatus: () => { available: boolean } | null;
      }
    | undefined
  >();
  let settingsPromptsRef = $state<{ init: () => Promise<void> } | undefined>();
  let settingsStopwordsRef = $state<{ init: () => Promise<void> } | undefined>();
  let settingsMaintenanceRef = $state<{ init: () => Promise<void> } | undefined>();

  // Ollama status for child components
  let ollamaAvailable = $derived(appState.ollamaStatus.available);

  async function refreshOllamaStatus() {
    await appState.checkOllama();
  }

  // Tabs definition
  let tabs = $derived<Tab[]>([
    { id: "general", label: $_("settings.title") },
    { id: "ai", label: $_("settings.ai.title") },
    { id: "prompts", label: "Prompts" },
    { id: "stopwords", label: $_("settings.stopwords.title") },
    { id: "maintenance", label: $_("settings.maintenance.title") },
  ]);

  async function handleTabChange(tabId: string) {
    if (tabId === "maintenance") {
      await tick();
      await settingsMaintenanceRef?.init();
    } else if (tabId === "stopwords") {
      await tick();
      await settingsStopwordsRef?.init();
    } else if (tabId === "prompts") {
      await tick();
      await settingsPromptsRef?.init();
    } else if (tabId === "ai") {
      await tick();
      await settingsAIRef?.init();
      await refreshOllamaStatus();
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      settingsGeneralRef?.closeAllDropdowns();
      settingsAIRef?.closeAllDropdowns();
    }
  }

  onMount(async () => {
    // Initialize general settings
    settingsGeneralRef?.init();

    // Initialize Ollama settings and get status
    await settingsAIRef?.init();
    await refreshOllamaStatus();

    // Initialize prompts
    await settingsPromptsRef?.init();
    await refreshOllamaStatus();
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="settings-view">
  <div class="settings-header">
    <div class="header-top">
      <h2 class="view-title">
        <i class="fa-solid fa-gear nav-icon"></i>
        {$_("settings.title")}
        <Tooltip termKey="settings_view">
          <i class="fa-solid fa-circle-info info-icon"></i>
        </Tooltip>
      </h2>
      <div class="settings-stats">
        <span class="stat-item">
          <span class="stat-value">{appState.pentacles.length}</span>
          <span class="stat-label">{$_("settings.stats.feeds")}</span>
        </span>
        <span class="stat-item">
          <span class="stat-value">{settings.syncInterval}</span>
          <span class="stat-label">{$_("settings.stats.syncInterval")}</span>
        </span>
        <span class="stat-item ollama-status" class:available={ollamaAvailable}>
          <span class="stat-value">
            <i class="fa-solid {ollamaAvailable ? 'fa-check-circle' : 'fa-times-circle'}"></i>
          </span>
          <span class="stat-label">AI</span>
        </span>
      </div>
    </div>
    <Tabs {tabs} bind:activeTab onchange={handleTabChange} />
  </div>

  <div class="tab-content">
    {#if activeTab === "general"}
      <SettingsGeneral bind:this={settingsGeneralRef} />
    {:else if activeTab === "ai"}
      <SettingsAI bind:this={settingsAIRef} />
    {:else if activeTab === "prompts"}
      <SettingsPrompts bind:this={settingsPromptsRef} {ollamaAvailable} />
    {:else if activeTab === "stopwords"}
      <SettingsStopwords bind:this={settingsStopwordsRef} />
    {:else if activeTab === "maintenance"}
      <SettingsMaintenance bind:this={settingsMaintenanceRef} {ollamaAvailable} />
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
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .settings-stats {
    display: flex;
    gap: 1.5rem;
    align-items: flex-end;
  }

  .settings-stats .stat-item {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    min-width: 4rem;
  }

  .settings-stats .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--accent-primary);
    line-height: 1;
  }

  .settings-stats .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .settings-stats .ollama-status .stat-value {
    font-size: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    height: 1.5rem;
  }

  .settings-stats .ollama-status.available .stat-value {
    color: var(--accent-success, #10b981);
  }

  .settings-stats .ollama-status:not(.available) .stat-value {
    color: var(--accent-error, #ef4444);
  }

  /* .settings-header h2 removed - now uses global .view-title class */

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
  }
</style>
