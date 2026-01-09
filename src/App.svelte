<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { isLoading } from 'svelte-i18n';
  import Sidebar from "./lib/components/Sidebar.svelte";
  import ArticleList from "./lib/components/ArticleList.svelte";
  import ArticleView from "./lib/components/ArticleView.svelte";
  import KeywordNetwork from "./lib/components/KeywordNetwork.svelte";
  import FnordView from "./lib/components/FnordView.svelte";
  import SettingsDialog from "./lib/components/SettingsDialog.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import { settings } from "./lib/stores/settings.svelte";
  import { initLocaleFromDb } from "./lib/i18n";
  import { networkStore, appState } from "./lib/stores/state.svelte";

  let showSettings = $state(false);
  let mainView = $state<'articles' | 'network' | 'fnord'>('articles');

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
    // Reset filter and load all articles so the selected one is found
    appState.selectedPentacleId = null;
    await appState.loadFnords();
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
        onsettings={() => showSettings = true}
        onnetwork={() => mainView = mainView === 'network' ? 'articles' : 'network'}
        onfnord={() => mainView = mainView === 'fnord' ? 'articles' : 'fnord'}
        networkActive={mainView === 'network'}
        fnordActive={mainView === 'fnord'}
      />

      <!-- Main content area -->
      <div class="content-area">
        {#if mainView === 'network'}
          <!-- Keyword Network View -->
          <KeywordNetwork />
        {:else if mainView === 'fnord'}
          <!-- Fnord Statistics View -->
          <FnordView />
        {:else}
          <!-- Article list (Fnords) -->
          <ArticleList />

          <!-- Article view -->
          <ArticleView />
        {/if}
      </div>
    </div>

    <!-- Status bar at bottom -->
    <StatusBar />
  </div>

  <SettingsDialog open={showSettings} onclose={() => showSettings = false} />
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
