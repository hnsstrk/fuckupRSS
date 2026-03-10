<script lang="ts">
  import { onMount, onDestroy } from "svelte";
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
  import { networkStore, appState } from "./lib/stores/state.svelte";

  let mainView = $state<
    "erisianArchives" | "network" | "fnord" | "mindfuck" | "briefings" | "storyClusters" | "settings"
  >("erisianArchives");

  // Listen for navigation events from other components
  function handleNavigateToNetwork(event: CustomEvent<{ keywordId?: number }>) {
    mainView = "network";
    if (event.detail?.keywordId !== undefined) {
      networkStore.selectKeyword(event.detail.keywordId);
    }
  }

  // Listen for article navigation events (e.g., from Similar Articles in ArticleView)
  async function handleNavigateToArticle(event: CustomEvent<{ articleId: number }>) {
    // Navigate to ErisianArchives if not already there
    if (mainView !== "erisianArchives") {
      mainView = "erisianArchives";
    }

    // Ensure the article is loaded (fetch if not in current list)
    await appState.ensureFnordLoaded(event.detail.articleId);
    appState.selectFnord(event.detail.articleId);
  }

  onMount(async () => {
    // Initialize platform detection first (for CSS classes)
    await platformStore.init();
    await settings.init();
    await initLocaleFromDb();
    window.addEventListener(
      "navigate-to-network",
      handleNavigateToNetwork as unknown as EventListener,
    );
    window.addEventListener(
      "navigate-to-article",
      handleNavigateToArticle as unknown as EventListener,
    );
  });

  onDestroy(() => {
    window.removeEventListener(
      "navigate-to-network",
      handleNavigateToNetwork as unknown as EventListener,
    );
    window.removeEventListener(
      "navigate-to-article",
      handleNavigateToArticle as unknown as EventListener,
    );
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
        onerisianArchives={() => (mainView = "erisianArchives")}
        onfnord={() => (mainView = "fnord")}
        onnetwork={() => (mainView = "network")}
        onmindfuck={() => (mainView = "mindfuck")}
        onbriefings={() => (mainView = "briefings")}
        onstoryClusters={() => (mainView = "storyClusters")}
        onsettings={() => (mainView = "settings")}
        erisianArchivesActive={mainView === "erisianArchives"}
        fnordActive={mainView === "fnord"}
        networkActive={mainView === "network"}
        mindfuckActive={mainView === "mindfuck"}
        briefingsActive={mainView === "briefings"}
        storyClustersActive={mainView === "storyClusters"}
        settingsActive={mainView === "settings"}
      />

      <!-- Main content area -->
      <div class="content-area">
        {#if mainView === "erisianArchives"}
          <!-- Erisian Archives: All articles with tabs and integrated reading pane -->
          <ErisianArchives />
        {:else if mainView === "network"}
          <!-- Keyword Network View -->
          <KeywordNetwork />
        {:else if mainView === "fnord"}
          <!-- Fnord Statistics View -->
          <FnordView />
        {:else if mainView === "mindfuck"}
          <!-- Operation Mindfuck (Bias Mirror) -->
          <MindfuckView />
        {:else if mainView === "briefings"}
          <!-- AI Briefings -->
          <BriefingView />
        {:else if mainView === "storyClusters"}
          <!-- Story Clustering (Perspective Comparison) -->
          <StoryClusterView />
        {:else if mainView === "settings"}
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
