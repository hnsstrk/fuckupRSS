<script lang="ts">
  import { onMount } from "svelte";
  import { isLoading } from "svelte-i18n";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import ErisianArchives from "./lib/components/ErisianArchives.svelte";
  import KeywordNetwork from "./lib/components/KeywordNetwork.svelte";
  import FnordView from "./lib/components/FnordView.svelte";
  import MindfuckView from "./lib/components/MindfuckView.svelte";
  import BriefingView from "./lib/components/BriefingView.svelte";
  import StoryClusterView from "./lib/components/StoryClusterView.svelte";
  import SettingsView from "./lib/components/SettingsView.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import { settings } from "./lib/stores/settings.svelte";
  import { platformStore } from "./lib/stores/platform.svelte";
  import { initLocaleFromDb } from "./lib/i18n";
  import { navigationStore } from "./lib/stores/navigation.svelte";

  onMount(async () => {
    // Initialize platform detection first (for CSS classes)
    await platformStore.init();
    await settings.init();
    await initLocaleFromDb();
  });
</script>

{#if $isLoading}
  <div class="loading">Loading...</div>
{:else}
  <div class="app-container">
    <!-- Main content row -->
    <div class="main-content">
      <!-- Sidebar: Feed list (Pentacles) -->
      <Sidebar
        onerisianArchives={() => navigationStore.navigateTo("erisianArchives")}
        onfnord={() => navigationStore.navigateTo("fnord")}
        onnetwork={() => navigationStore.navigateTo("network")}
        onmindfuck={() => navigationStore.navigateTo("mindfuck")}
        onbriefings={() => navigationStore.navigateTo("briefings")}
        onstoryClusters={() => navigationStore.navigateTo("storyClusters")}
        onsettings={() => navigationStore.navigateTo("settings")}
        erisianArchivesActive={navigationStore.currentView === "erisianArchives"}
        fnordActive={navigationStore.currentView === "fnord"}
        networkActive={navigationStore.currentView === "network"}
        mindfuckActive={navigationStore.currentView === "mindfuck"}
        briefingsActive={navigationStore.currentView === "briefings"}
        storyClustersActive={navigationStore.currentView === "storyClusters"}
        settingsActive={navigationStore.currentView === "settings"}
      />

      <!-- Main content area -->
      <div class="content-area">
        {#if navigationStore.currentView === "erisianArchives"}
          <!-- Erisian Archives: All articles with tabs and integrated reading pane -->
          <ErisianArchives />
        {:else if navigationStore.currentView === "network"}
          <!-- Keyword Network View -->
          <KeywordNetwork />
        {:else if navigationStore.currentView === "fnord"}
          <!-- Fnord Statistics View -->
          <FnordView />
        {:else if navigationStore.currentView === "mindfuck"}
          <!-- Operation Mindfuck (Bias Mirror) -->
          <MindfuckView />
        {:else if navigationStore.currentView === "briefings"}
          <!-- AI Briefings -->
          <BriefingView />
        {:else if navigationStore.currentView === "storyClusters"}
          <!-- Story Clustering (Perspective Comparison) -->
          <StoryClusterView />
        {:else if navigationStore.currentView === "settings"}
          <!-- Settings View -->
          <SettingsView />
        {/if}
      </div>
    </div>

    <!-- Status bar at bottom -->
    <StatusBar />
  </div>

  <Toast />
{/if}

<style>
  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  .app-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .main-content {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .content-area {
    display: flex;
    flex: 1;
    min-width: 0;
  }
</style>
