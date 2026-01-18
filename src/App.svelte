<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { isLoading } from 'svelte-i18n';
  import Sidebar from "./lib/components/Sidebar.svelte";
  import ArticleList from "./lib/components/ArticleList.svelte";
  import ArticleView from "./lib/components/ArticleView.svelte";
  import ErisianArchives from "./lib/components/ErisianArchives.svelte";
  import KeywordNetwork from "./lib/components/KeywordNetwork.svelte";
  import FnordView from "./lib/components/FnordView.svelte";
  import MindfuckView from "./lib/components/MindfuckView.svelte";
  import SettingsView from "./lib/components/SettingsView.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import { settings } from "./lib/stores/settings.svelte";
  import { initLocaleFromDb } from "./lib/i18n";
  import { networkStore, appState } from "./lib/stores/state.svelte";

  let mainView = $state<'articles' | 'erisianArchives' | 'network' | 'fnord' | 'mindfuck' | 'settings'>('erisianArchives');

  // Listen for navigation events from other components
  function handleNavigateToNetwork(event: CustomEvent<{ keywordId?: number }>) {
    mainView = 'network';
    if (event.detail?.keywordId !== undefined) {
      networkStore.selectKeyword(event.detail.keywordId);
    }
  }

  // Listen for article navigation events
  async function handleNavigateToArticle(event: CustomEvent<{ articleId: number }>) {
    mainView = 'articles';
    // Reset filter
    appState.selectedPentacleId = null;
    appState.selectedSephirothId = null;

    // Ensure the article is loaded (fetch if not in current list)
    await appState.ensureFnordLoaded(event.detail.articleId);
    appState.selectFnord(event.detail.articleId);
  }

  onMount(async () => {
    await settings.init();
    await initLocaleFromDb();
    window.addEventListener('navigate-to-network', handleNavigateToNetwork as EventListener);
    window.addEventListener('navigate-to-article', handleNavigateToArticle as EventListener);
  });

  onDestroy(() => {
    window.removeEventListener('navigate-to-network', handleNavigateToNetwork as EventListener);
    window.removeEventListener('navigate-to-article', handleNavigateToArticle as EventListener);
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
        onerisianArchives={() => mainView = 'erisianArchives'}
        onarticles={() => mainView = 'articles'}
        onfnord={() => mainView = 'fnord'}
        onnetwork={() => mainView = 'network'}
        onmindfuck={() => mainView = 'mindfuck'}
        onsettings={() => mainView = 'settings'}
        erisianArchivesActive={mainView === 'erisianArchives'}
        articlesActive={mainView === 'articles'}
        fnordActive={mainView === 'fnord'}
        networkActive={mainView === 'network'}
        mindfuckActive={mainView === 'mindfuck'}
        settingsActive={mainView === 'settings'}
      />

      <!-- Main content area -->
      <div class="content-area">
        {#if mainView === 'erisianArchives'}
          <!-- Erisian Archives: All articles with tabs -->
          <ErisianArchives />
        {:else if mainView === 'articles'}
          <!-- Article list + Article view -->
          <ArticleList />
          <ArticleView />
        {:else if mainView === 'network'}
          <!-- Keyword Network View -->
          <KeywordNetwork />
        {:else if mainView === 'fnord'}
          <!-- Fnord Statistics View -->
          <FnordView />
        {:else if mainView === 'mindfuck'}
          <!-- Operation Mindfuck (Bias Mirror) -->
          <MindfuckView />
        {:else if mainView === 'settings'}
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
