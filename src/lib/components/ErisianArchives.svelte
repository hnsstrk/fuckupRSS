<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { appState, type Fnord } from "../stores/state.svelte";
  import Tabs, { type Tab } from "./Tabs.svelte";
  import Tooltip from "./Tooltip.svelte";
  import { ArticleItemCompact } from "./article";
  import ArticleView from "./ArticleView.svelte";

  // Type for analysis status articles from backend
  interface AnalysisStatusArticle {
    id: number;
    title: string;
    pentacle_id: number;
    pentacle_title: string | null;
    summary: string | null;
    published_at: string | null;
    status: string;
    analysis_attempts: number;
    last_error: string | null;
  }

  // Type for count responses from backend
  interface CountResponse {
    count: number;
  }

  // Batch size for lazy loading
  const BATCH_SIZE = 50;

  // State
  let loading = $state(false);
  let activeTab = $state<string>("articles");

  // Special articles for failed/hopeless tabs (loaded separately)
  let specialArticles = $state<Fnord[]>([]);

  // Lazy loading state for special tabs
  let totalSpecialCount = $state(0);
  let loadingMoreSpecial = $state(false);

  // Computed: has more special articles to load
  let hasMoreSpecial = $derived(specialArticles.length < totalSpecialCount);

  // Derived from appState - single source of truth
  let selectedId = $derived(appState.selectedFnordId);

  // Derived: Current source/category name for header display
  let currentSourceName = $derived.by(() => {
    if (appState.selectedPentacleId !== null) {
      return appState.selectedPentacle?.title || "Feed";
    }
    if (appState.selectedSephirothId !== null) {
      return appState.selectedSephirothCategory?.name || "Kategorie";
    }
    return null; // "Alle Artikel"
  });

  // Derived: Filter appState.fnords by active tab status
  let archiveArticles = $derived.by(() => {
    // For failed/hopeless tabs, use specialArticles
    if (activeTab === "failed" || activeTab === "hopeless") {
      return specialArticles;
    }

    // For standard tabs, we now use backend filtering so appState.fnords
    // already contains the correct filtered list
    return appState.fnords;
  });

  // Stats (counts used for potential future badge display)
  let totalCount = $state(0);
  let unreadCount = $state(0);
  let favoritesCount = $state(0);
  // Note: failedCount and hopelessCount are loaded but not displayed (badges removed per user request)

  // Tabs definition (no badges as per user request)
  let tabs = $derived<Tab[]>([
    { id: "articles", label: $_("erisianArchives.tabs.articles") || "Artikel" },
    { id: "unread", label: $_("erisianArchives.tabs.unread") || "Ungelesen" },
    { id: "goldenApple", label: $_("erisianArchives.tabs.goldenApple") || "Golden Apple" },
    { id: "failed", label: $_("erisianArchives.tabs.failed") || "Fehlgeschlagen" },
    { id: "hopeless", label: $_("erisianArchives.tabs.hopeless") || "Hoffnungslos" },
  ]);

  onMount(async () => {
    await loadStats();
    // Load special tabs if needed
    if (activeTab === "failed" || activeTab === "hopeless") {
      await loadSpecialArticles();
    }
  });

  async function loadStats() {
    try {
      // Get total count
      totalCount = await invoke<number>("get_fnords_count", { filter: null });

      // Get unread count
      unreadCount = await invoke<number>("get_fnords_count", { filter: { status: "concealed" } });

      // Get favorites count
      favoritesCount = await invoke<number>("get_fnords_count", {
        filter: { status: "golden_apple" },
      });

      // Note: failed/hopeless counts are available via get_failed_count/get_hopeless_count
      // but not loaded here since badges were removed per user request
    } catch (e) {
      console.error("[ErisianArchives] Error loading stats:", e);
    }
  }

  // Helper to convert AnalysisStatusArticle to Fnord
  function mapToFnord(a: AnalysisStatusArticle): Fnord {
    return {
      id: a.id,
      pentacle_id: a.pentacle_id,
      pentacle_title: a.pentacle_title,
      guid: "",
      url: "",
      title: a.title,
      author: null,
      content_raw: null,
      content_full: null,
      summary: a.summary,
      image_url: null,
      published_at: a.published_at,
      processed_at: null,
      status: a.status,
      political_bias: null,
      sachlichkeit: null,
      quality_score: null,
      has_changes: false,
      changed_at: null,
      revision_count: 0,
      categories: [],
      full_text_fetch_error: null,
      article_type: "unknown",
    } as Fnord;
  }

  // Article type filter state
  let selectedArticleType = $state<string>("");

  async function handleArticleTypeFilter(event: Event) {
    const target = event.target as HTMLSelectElement;
    selectedArticleType = target.value;

    // Rebuild filter and reload
    let filter: Record<string, unknown> = {};

    if (appState.selectedPentacleId !== null) {
      filter.pentacle_id = appState.selectedPentacleId;
    }

    if (activeTab === "unread") {
      filter.status = "concealed";
    } else if (activeTab === "goldenApple") {
      filter.status = "golden_apple";
    }

    if (selectedArticleType) {
      filter.article_type = selectedArticleType;
    }

    await appState.loadFnords(filter);
  }

  // Load special articles for failed/hopeless tabs (not available in appState.fnords)
  async function loadSpecialArticles() {
    if (activeTab !== "failed" && activeTab !== "hopeless") {
      specialArticles = [];
      totalSpecialCount = 0;
      return;
    }

    loading = true;
    try {
      if (activeTab === "failed") {
        // Load count and first batch in parallel
        const [articles, countResult] = await Promise.all([
          invoke<AnalysisStatusArticle[]>("get_failed_articles", { limit: BATCH_SIZE, offset: 0 }),
          invoke<CountResponse>("get_failed_count"),
        ]);
        specialArticles = articles.map(mapToFnord);
        totalSpecialCount = countResult.count;
      } else if (activeTab === "hopeless") {
        const [articles, countResult] = await Promise.all([
          invoke<AnalysisStatusArticle[]>("get_hopeless_articles", {
            limit: BATCH_SIZE,
            offset: 0,
          }),
          invoke<CountResponse>("get_hopeless_count"),
        ]);
        specialArticles = articles.map(mapToFnord);
        totalSpecialCount = countResult.count;
      }
    } catch (e) {
      console.error("[ErisianArchives] Error loading special articles:", e);
      specialArticles = [];
      totalSpecialCount = 0;
    } finally {
      loading = false;
    }
  }

  // Load more special articles (lazy loading)
  async function loadMoreSpecialArticles() {
    if (loadingMoreSpecial || !hasMoreSpecial) return;
    if (activeTab !== "failed" && activeTab !== "hopeless") return;

    loadingMoreSpecial = true;
    try {
      const offset = specialArticles.length;
      let moreArticles: AnalysisStatusArticle[] = [];

      if (activeTab === "failed") {
        moreArticles = await invoke<AnalysisStatusArticle[]>("get_failed_articles", {
          limit: BATCH_SIZE,
          offset,
        });
      } else if (activeTab === "hopeless") {
        moreArticles = await invoke<AnalysisStatusArticle[]>("get_hopeless_articles", {
          limit: BATCH_SIZE,
          offset,
        });
      }

      if (moreArticles.length > 0) {
        specialArticles = [...specialArticles, ...moreArticles.map(mapToFnord)];
      }
    } catch (e) {
      console.error("[ErisianArchives] Error loading more special articles:", e);
    } finally {
      loadingMoreSpecial = false;
    }
  }

  // Sentinel ref for IntersectionObserver-based lazy loading
  let sentinelRef = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (!sentinelRef) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          if (activeTab === "failed" || activeTab === "hopeless") {
            // Special tabs use their own pagination
            if (hasMoreSpecial && !loadingMoreSpecial) {
              loadMoreSpecialArticles();
            }
          } else {
            // Normal tabs use appState's pagination
            if (appState.hasMoreFnords && !appState.loadingMore) {
              appState.loadMoreFnords();
            }
          }
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(sentinelRef);
    return () => observer.disconnect();
  });

  async function handleTabChange(tabId: string) {
    activeTab = tabId;
    // Reset selection when changing tabs to avoid stale state
    appState.selectedFnordId = null;

    // Reset special articles state
    specialArticles = [];
    totalSpecialCount = 0;

    // Load data based on tab
    if (tabId === "failed" || tabId === "hopeless") {
      await loadSpecialArticles();
    } else {
      // For standard tabs, fetch from backend with filter
      let filter = {};

      // If we have a pentacle or category selected, preserve that filter
      if (appState.selectedPentacleId !== null) {
        filter = { ...filter, pentacle_id: appState.selectedPentacleId };
      }
      if (appState.selectedSephirothId !== null) {
        // Need to check if main or subcategory in appState to know which field to set
        // But for now let's just use the current selection logic
        // Ideally we should refactor selectSephiroth/selectPentacle to expose the current filter object
        // For now, simpler approach: just reload with status
      }

      // Add status filter based on tab
      if (tabId === "unread") {
        filter = { ...filter, status: "concealed" };
      } else if (tabId === "goldenApple") {
        filter = { ...filter, status: "golden_apple" };
      }
      // 'articles' tab has no status filter (shows all)

      // Preserve article_type filter
      if (selectedArticleType) {
        filter = { ...filter, article_type: selectedArticleType };
      }

      await appState.loadFnords(filter);
    }
  }

  // Select article with mark-on-switch logic
  // Marks the PREVIOUS article as read when switching to a new one
  async function selectArticle(id: number, markPreviousAsRead: boolean = false) {
    const previousId = appState.selectedFnordId;

    // Mark previous article as read if requested and it was unread
    if (markPreviousAsRead && previousId !== null && previousId !== id) {
      const previousFnord = appState.selectedFnord;
      if (previousFnord && previousFnord.status === "concealed") {
        await appState.updateFnordStatus(previousId, "illuminated");
        // archiveArticles is derived and will auto-update when appState.fnords changes
        // Refresh stats to update badge counts
        loadStats();
      }
    }

    // Ensure article data is loaded in appState
    await appState.ensureFnordLoaded(id);
    // Direct assignment - no auto-mark-as-read
    appState.selectedFnordId = id;
  }

  // Keyboard navigation (j/k/s like in ArticleList)
  function handleKeydown(e: KeyboardEvent) {
    // Ignore if typing in an input
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      return;
    }

    if (e.key === "j") {
      e.preventDefault();
      selectNextArticle();
    } else if (e.key === "k") {
      e.preventDefault();
      selectPrevArticle();
    } else if (e.key === "s" && selectedId) {
      e.preventDefault();
      appState.toggleGoldenApple(selectedId);
    }
  }

  function selectNextArticle() {
    const currentIndex = archiveArticles.findIndex((a) => a.id === selectedId);
    if (currentIndex < archiveArticles.length - 1) {
      // Mark previous as read when navigating to next
      selectArticle(archiveArticles[currentIndex + 1].id, true);
    } else if (currentIndex === -1 && archiveArticles.length > 0) {
      selectArticle(archiveArticles[0].id, false);
    }
  }

  function selectPrevArticle() {
    const currentIndex = archiveArticles.findIndex((a) => a.id === selectedId);
    if (currentIndex > 0) {
      // Mark previous as read when navigating
      selectArticle(archiveArticles[currentIndex - 1].id, true);
    }
  }

  // Note: External navigation (e.g., from Similar Articles) now goes through navigationStore

  // Empty state messages based on active tab
  let emptyMessage = $derived.by(() => {
    switch (activeTab) {
      case "articles":
        return $_("erisianArchives.noArticles") || "Keine Artikel vorhanden";
      case "unread":
        return (
          $_("erisianArchives.noUnread") || "Alle Artikel wurden gelesen - Erleuchtung erreicht!"
        );
      case "goldenApple":
        return (
          $_("erisianArchives.noFavorites") || "Keine Golden Apples - markiere Artikel als Favorit!"
        );
      case "failed":
        return $_("erisianArchives.noFailed") || "Keine fehlgeschlagenen Analysen";
      case "hopeless":
        return (
          $_("erisianArchives.noHopeless") ||
          "Keine hoffnungslosen Faelle - die KI hat alles gemeistert!"
        );
      default:
        return "";
    }
  });
</script>

<!-- Global keyboard handler for j/k/s navigation -->
<svelte:window onkeydown={handleKeydown} />

<div class="erisian-archives" role="main">
  <!-- Header -->
  <div class="erisian-header">
    <div class="header-top">
      <div class="header-title-group">
        <h2 class="view-title">
          <i class="fa-solid fa-newspaper nav-icon"></i>
          {$_("erisianArchives.title") || "Erisian Archives"}
          <Tooltip termKey="erisian_archives">
            <i class="fa-solid fa-circle-info info-icon"></i>
          </Tooltip>
        </h2>
        {#if currentSourceName}
          <span class="source-filter">
            <i class="fa-solid {appState.selectedPentacleId !== null ? 'fa-rss' : 'fa-folder'}"></i>
            {currentSourceName}
            <button
              type="button"
              class="clear-filter-btn"
              onclick={() => appState.selectView("all")}
              title={$_("erisianArchives.clearFilter") || "Filter entfernen"}
            >
              <i class="fa-solid fa-xmark"></i>
            </button>
          </span>
        {/if}
      </div>
      <div class="erisian-summary">
        <span class="summary-item">
          <span class="summary-value">{totalCount}</span>
          <span class="summary-label">{$_("erisianArchives.stats.total") || "Gesamt"}</span>
        </span>
        <span class="summary-item">
          <span class="summary-value">{unreadCount}</span>
          <span class="summary-label">{$_("erisianArchives.stats.unread") || "Ungelesen"}</span>
        </span>
        <span class="summary-item">
          <span class="summary-value">{favoritesCount}</span>
          <span class="summary-label">{$_("erisianArchives.stats.favorites") || "Favoriten"}</span>
        </span>
      </div>
    </div>

    <!-- Filters -->
    <div class="filter-row">
      <!-- Article Type Filter -->
      <select
        class="article-type-filter"
        value={selectedArticleType}
        onchange={handleArticleTypeFilter}
        title={$_("articleType.filter")}
        disabled={activeTab === "failed" || activeTab === "hopeless"}
      >
        <option value="">{$_("articleType.all")}</option>
        <option value="news">{$_("articleType.news")}</option>
        <option value="analysis">{$_("articleType.analysis")}</option>
        <option value="opinion">{$_("articleType.opinion")}</option>
        <option value="satire">{$_("articleType.satire")}</option>
        <option value="ad">{$_("articleType.ad")}</option>
        <option value="unknown">{$_("articleType.unknown")}</option>
      </select>
    </div>

    <!-- Tabs -->
    <Tabs {tabs} bind:activeTab onchange={handleTabChange} />
  </div>

  <!-- 2-Column Body: Article List + Article View -->
  <div class="erisian-body">
    <!-- Left: Article List -->
    <div class="article-list-column">
      {#if loading}
        <div class="loading-state">
          <div class="spinner"></div>
          <span>{$_("fnordView.loading") || "Laden..."}</span>
        </div>
      {:else if archiveArticles.length === 0}
        <div class="empty-state">
          <i class="empty-icon fa-solid fa-box-open"></i>
          <p>{emptyMessage}</p>
        </div>
      {:else}
        <div class="articles-list">
          {#each archiveArticles as article (article.id)}
            <ArticleItemCompact
              id={article.id}
              title={article.title}
              status={article.status}
              pentacle_title={article.pentacle_title}
              published_at={article.published_at}
              categories={article.categories}
              revision_count={article.revision_count}
              quality_score={article.quality_score}
              political_bias={article.political_bias}
              active={selectedId === article.id}
              onclick={() => selectArticle(article.id, true)}
            />
          {/each}

          <!-- Loading indicator for lazy loading -->
          {#if activeTab === "failed" || activeTab === "hopeless" ? loadingMoreSpecial : appState.loadingMore}
            <div class="loading-more">
              <div class="spinner small"></div>
              <span>{$_("fnordView.loadingMore") || "Lade mehr..."}</span>
            </div>
          {:else if activeTab === "failed" || activeTab === "hopeless" ? hasMoreSpecial : appState.hasMoreFnords}
            <div class="scroll-hint">
              <span class="loaded-count">
                {archiveArticles.length}/{activeTab === "failed" || activeTab === "hopeless"
                  ? totalSpecialCount
                  : appState.totalFnordsCount}
              </span>
            </div>
          {/if}

          <!-- Sentinel for IntersectionObserver lazy loading -->
          <div bind:this={sentinelRef} class="h-1" aria-hidden="true"></div>
        </div>
      {/if}
    </div>

    <!-- Right: Article View -->
    <div class="article-view-column">
      <ArticleView />
    </div>
  </div>
</div>

<style>
  .erisian-archives {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1;
    background-color: var(--bg-base);
    outline: none;
  }

  .erisian-header {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-default);
    background-color: var(--bg-surface);
    flex-shrink: 0;
  }

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .header-title-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .source-filter {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    background-color: var(--accent-primary);
    color: var(--bg-base);
    border-radius: 0.25rem;
    font-size: 0.8125rem;
    font-weight: 500;
  }

  .source-filter i:first-child {
    font-size: 0.75rem;
  }

  .clear-filter-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    padding: 0;
    margin-left: 0.25rem;
    background: transparent;
    border: none;
    border-radius: 50%;
    color: inherit;
    cursor: pointer;
    opacity: 0.7;
    transition: opacity 0.15s ease;
  }

  .clear-filter-btn:hover {
    opacity: 1;
    background-color: rgba(0, 0, 0, 0.1);
  }

  .clear-filter-btn i {
    font-size: 0.625rem;
  }

  .erisian-summary {
    display: flex;
    gap: 1rem;
  }

  .summary-item {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .summary-value {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .summary-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  /* Filter Row */
  .filter-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .article-type-filter {
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    border: 1px solid var(--border-default);
    background-color: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.8125rem;
    cursor: pointer;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .article-type-filter:hover {
    border-color: var(--accent-primary);
  }

  .article-type-filter:focus {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 1px var(--accent-primary);
  }

  .article-type-filter:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* 2-Column Body Layout */
  .erisian-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .article-list-column {
    width: 20rem;
    flex-shrink: 0;
    overflow-y: auto;
    border-right: 1px solid var(--border-default);
    background-color: var(--bg-base);
  }

  .article-view-column {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    display: flex;
  }

  /* Override ArticleView styles to fit properly */
  .article-view-column :global(.article-view) {
    flex: 1;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 200px;
    color: var(--text-muted);
    gap: 0.75rem;
    text-align: center;
    padding: 1rem;
  }

  .empty-icon {
    font-size: 2rem;
    opacity: 0.5;
  }

  .spinner {
    width: 1.5rem;
    height: 1.5rem;
    border: 3px solid var(--border-default);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .articles-list {
    display: flex;
    flex-direction: column;
  }

  .loading-more,
  .scroll-hint {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem;
    color: var(--text-muted);
    font-size: 0.8125rem;
  }

  .spinner.small {
    width: 1rem;
    height: 1rem;
    border-width: 2px;
  }

  .loaded-count {
    color: var(--text-tertiary);
    font-size: 0.75rem;
  }
</style>
