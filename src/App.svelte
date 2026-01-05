<script lang="ts">
  import { onMount } from 'svelte';
  import { isLoading } from 'svelte-i18n';
  import Sidebar from "./lib/components/Sidebar.svelte";
  import ArticleList from "./lib/components/ArticleList.svelte";
  import ArticleView from "./lib/components/ArticleView.svelte";
  import KeywordNetwork from "./lib/components/KeywordNetwork.svelte";
  import SettingsDialog from "./lib/components/SettingsDialog.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import { settings } from "./lib/stores/settings.svelte";
  import { initLocaleFromDb } from "./lib/i18n";

  let showSettings = $state(false);
  let mainView = $state<'articles' | 'network'>('articles');

  onMount(async () => {
    await settings.init();
    await initLocaleFromDb();
  });
</script>

{#if $isLoading}
  <div class="loading">Loading...</div>
{:else}
  <div class="app-container flex h-full">
    <!-- Sidebar: Feed list (Pentacles) -->
    <Sidebar
      onsettings={() => showSettings = true}
      onnetwork={() => mainView = mainView === 'network' ? 'articles' : 'network'}
      networkActive={mainView === 'network'}
    />

    <!-- Main content area -->
    <div class="flex flex-1 min-w-0">
      {#if mainView === 'network'}
        <!-- Keyword Network View -->
        <KeywordNetwork />
      {:else}
        <!-- Article list (Fnords) -->
        <ArticleList />

        <!-- Article view -->
        <ArticleView />
      {/if}
    </div>
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
</style>
