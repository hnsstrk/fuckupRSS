<script lang="ts">
  import Sidebar from "./lib/components/Sidebar.svelte";
  import ArticleList from "./lib/components/ArticleList.svelte";
  import ArticleView from "./lib/components/ArticleView.svelte";
</script>

<div class="flex h-full">
  <!-- Sidebar: Feed list (Pentacles) -->
  <Sidebar />

  <!-- Main content area -->
  <div class="flex flex-1 min-w-0">
    <!-- Article list (Fnords) -->
    <ArticleList />

    <!-- Article view -->
    <ArticleView />
  </div>
</div>
