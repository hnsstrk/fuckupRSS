<script lang="ts">
  import { _ } from "svelte-i18n";
  import { onMount } from "svelte";
  import Tabs, { type Tab } from "./Tabs.svelte";
  import SettingsGeneral from "./settings/SettingsGeneral.svelte";
  import SettingsOllama from "./settings/SettingsOllama.svelte";
  import SettingsPrompts from "./settings/SettingsPrompts.svelte";
  import SettingsStopwords from "./settings/SettingsStopwords.svelte";
  import SettingsMaintenance from "./settings/SettingsMaintenance.svelte";

  // Tab state
  let activeTab = $state<string>("general");

  // Component references (using $state to satisfy Svelte 5 bind:this requirements)
  let settingsGeneralRef = $state<{ init: () => void; closeAllDropdowns: () => void } | undefined>();
  let settingsOllamaRef = $state<{ init: () => Promise<void>; closeAllDropdowns: () => void; getOllamaStatus: () => { available: boolean } | null } | undefined>();
  let settingsPromptsRef = $state<{ init: () => Promise<void> } | undefined>();
  let settingsStopwordsRef = $state<{ init: () => Promise<void> } | undefined>();
  let settingsMaintenanceRef = $state<{ init: () => Promise<void> } | undefined>();

  // Ollama status for child components
  let ollamaAvailable = $state(false);

  // Tabs definition
  let tabs = $derived<Tab[]>([
    { id: "general", label: $_("settings.title") },
    { id: "ollama", label: "Ollama" },
    { id: "prompts", label: "Prompts" },
    { id: "stopwords", label: $_("settings.stopwords.title") },
    { id: "maintenance", label: $_("settings.maintenance.title") },
  ]);

  function handleTabChange(tabId: string) {
    if (tabId === "maintenance") {
      settingsMaintenanceRef?.init();
    } else if (tabId === "stopwords") {
      settingsStopwordsRef?.init();
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      settingsGeneralRef?.closeAllDropdowns();
      settingsOllamaRef?.closeAllDropdowns();
    }
  }

  onMount(async () => {
    // Initialize general settings
    settingsGeneralRef?.init();

    // Initialize Ollama settings and get status
    await settingsOllamaRef?.init();
    const status = settingsOllamaRef?.getOllamaStatus();
    ollamaAvailable = status?.available ?? false;

    // Initialize prompts
    await settingsPromptsRef?.init();
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="settings-view">
  <div class="settings-header">
    <h2>{$_("settings.title")}</h2>
  </div>

  <!-- Tabs -->
  <div class="tabs-wrapper">
    <Tabs {tabs} bind:activeTab onchange={handleTabChange} />
  </div>

  <div class="tab-content">
    {#if activeTab === "general"}
      <SettingsGeneral bind:this={settingsGeneralRef} />
    {:else if activeTab === "ollama"}
      <SettingsOllama bind:this={settingsOllamaRef} />
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
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-default);
  }

  .settings-header h2 {
    margin: 0;
    font-size: 1.25rem;
    color: var(--accent-primary);
  }

  /* Tabs */
  .tabs-wrapper {
    padding: 0 1.5rem;
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
  }
</style>
