<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type { BatchProgress, BatchResult } from "../../types";
  import { appState } from "../../stores/state.svelte";
  import { onDestroy } from "svelte";
  import MaintenanceProgress from "./MaintenanceProgress.svelte";
  import ActionButton from "$lib/components/ui/ActionButton.svelte";
  import MaintenanceOrphans from "./MaintenanceOrphans.svelte";
  import MaintenanceShortContent from "./MaintenanceShortContent.svelte";

  interface Props {
    ollamaAvailable: boolean;
  }

  let { ollamaAvailable }: Props = $props();

  // Maintenance state
  let maintenanceRunning = $state<string | null>(null);
  let maintenanceResult = $state<string | null>(null);

  // Confirmation dialog state
  let confirmAction = $state<"prune" | "reset" | null>(null);

  // Keyword statistics state
  let keywordStats = $state<{
    total: number;
    with_embeddings: number;
    avg_quality: number;
    low_quality: number;
  } | null>(null);

  // Reanalyze progress state
  let reanalyzeProgress = $state<BatchProgress | null>(null);
  let reanalyzeRunning = $state(false);
  // Result stored for potential future display (assigned in handleResetForReprocessing)
  // @ts-expect-error - stored for future display, currently write-only
  let _reanalyzeResult = $state<BatchResult | null>(null);
  let progressUnlisten: UnlistenFn | null = null;

  // Statistical analysis progress state
  let statisticalProgress = $state<BatchProgress | null>(null);
  let statisticalRunning = $state(false);
  let statisticalUnlisten: UnlistenFn | null = null;

  // Quality score calculation progress state
  interface QualityScoreProgress {
    current: number;
    total: number;
    keyword_name: string;
    score: number | null;
  }
  let qualityProgress = $state<QualityScoreProgress | null>(null);
  let qualityRunning = $state(false);
  let qualityUnlisten: UnlistenFn | null = null;

  // Prototype status for semantic keyword type detection
  let prototypeStatus = $state<{
    total: number;
    expected: number;
    complete: boolean;
    by_type: Record<string, number>;
  } | null>(null);
  let generatingPrototypes = $state(false);

  export async function init() {
    // Don't reset maintenanceResult - keep showing the last result
    await Promise.all([loadKeywordStats(), loadPrototypeStatus()]);
  }

  async function loadPrototypeStatus() {
    try {
      prototypeStatus = await invoke("get_prototype_stats");
    } catch (e) {
      console.error("Failed to load prototype status:", e);
    }
  }

  onDestroy(() => {
    if (progressUnlisten) {
      progressUnlisten();
    }
    if (statisticalUnlisten) {
      statisticalUnlisten();
    }
    if (qualityUnlisten) {
      qualityUnlisten();
    }
  });

  async function loadKeywordStats() {
    try {
      const [lowQuality, allKeywords] = await Promise.all([
        invoke<
          {
            id: number;
            name: string;
            quality_score: number;
            article_count: number;
          }[]
        >("get_low_quality_keywords", { threshold: 0.3, limit: 100 }),
        invoke<{
          keywords: {
            id: number;
            name: string;
            article_count: number;
            quality_score: number | null;
            has_embedding: boolean;
          }[];
          total_count: number;
        }>("get_keywords", { limit: 1000, offset: 0 }),
      ]);

      const withEmbeddings = allKeywords.keywords.filter((k) => k.has_embedding).length;
      const qualityScores = allKeywords.keywords
        .filter((k) => k.quality_score !== null)
        .map((k) => k.quality_score!);
      const avgQuality =
        qualityScores.length > 0
          ? qualityScores.reduce((a, b) => a + b, 0) / qualityScores.length
          : 0;

      keywordStats = {
        total: allKeywords.total_count,
        with_embeddings: withEmbeddings,
        avg_quality: avgQuality,
        low_quality: lowQuality.length,
      };
    } catch (e) {
      console.error("Failed to load keyword stats:", e);
    }
  }

  async function handleCalculateScores() {
    maintenanceRunning = "scores";
    maintenanceResult = null;
    qualityProgress = null;
    qualityRunning = true;

    try {
      // Set up progress listener
      qualityUnlisten = await listen<QualityScoreProgress>("quality-score-progress", (event) => {
        qualityProgress = { ...event.payload };
      });

      const result = await invoke<{
        updated_count: number;
        avg_score: number;
        low_quality_count: number;
      }>("calculate_keyword_quality_scores", {});

      if (result.updated_count === 0) {
        maintenanceResult = $_("settings.maintenance.noKeywordsToUpdate");
      } else {
        maintenanceResult = `${result.updated_count} ${$_("settings.maintenance.updated")} (Ø ${result.avg_score.toFixed(2)})`;
      }

      // Refresh stats after calculation
      await loadKeywordStats();
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
      qualityRunning = false;
      qualityProgress = null;
      if (qualityUnlisten) {
        qualityUnlisten();
        qualityUnlisten = null;
      }
    }
  }

  async function handleGenerateEmbeddings() {
    maintenanceRunning = "embeddings";
    maintenanceResult = null;
    try {
      const queuedCount = await invoke<number>("queue_missing_embeddings");
      maintenanceResult = `${queuedCount} ${$_("settings.maintenance.queued")}`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  async function handleStatisticalAnalysis() {
    maintenanceRunning = "statistical";
    maintenanceResult = null;
    statisticalProgress = null;

    try {
      const count = await invoke<number>("get_unprocessed_statistical_count");
      if (count === 0) {
        maintenanceResult = $_("settings.maintenance.noUnprocessedArticles");
        maintenanceRunning = null;
        return;
      }

      statisticalRunning = true;
      statisticalProgress = {
        current: 0,
        total: count,
        fnord_id: 0,
        title: $_("batch.starting"),
        success: true,
        error: null,
      };

      statisticalUnlisten = await listen<BatchProgress>("statistical-progress", (event) => {
        statisticalProgress = { ...event.payload };
      });

      const result = await invoke<{
        processed: number;
        total: number;
        errors: string[];
      }>("process_statistical_batch", { limit: 10000 });

      maintenanceResult = `${result.processed} ${$_("settings.maintenance.articlesAnalyzed")}`;
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
      statisticalRunning = false;
      if (statisticalUnlisten) {
        statisticalUnlisten();
        statisticalUnlisten = null;
      }
    }
  }

  function showPruneConfirmation() {
    confirmAction = "prune";
  }

  function showResetConfirmation() {
    confirmAction = "reset";
  }

  function cancelConfirmation() {
    confirmAction = null;
  }

  async function handlePruneLowQuality() {
    confirmAction = null;
    maintenanceRunning = "prune";
    maintenanceResult = null;
    try {
      const result = await invoke<{
        pruned_count: number;
        pruned_keywords: string[];
      }>("auto_prune_low_quality", {
        quality_threshold: 0.2,
        min_age_days: 7,
        dry_run: false,
      });
      if (result.pruned_count === 0) {
        maintenanceResult = $_("settings.maintenance.noPruneCandidates");
      } else {
        maintenanceResult = `${result.pruned_count} ${$_("settings.maintenance.pruned")}`;
      }
      await loadKeywordStats();
    } catch (e) {
      console.error("Prune error:", e);
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  async function handleResetForReprocessing() {
    confirmAction = null;
    maintenanceRunning = "reset";
    maintenanceResult = null;
    reanalyzeProgress = null;
    _reanalyzeResult = null;

    try {
      const resetResult = await invoke<{ reset_count: number }>("reset_articles_for_reprocessing", {
        only_with_content: true,
      });

      if (resetResult.reset_count === 0) {
        maintenanceResult = $_("settings.maintenance.noArticlesToReset");
        maintenanceRunning = null;
        return;
      }

      await emit("articles-reset");
      await appState.loadUnprocessedCount();

      const model = appState.selectedModel || appState.ollamaStatus.models[0];
      if (!model || !appState.ollamaStatus.available) {
        maintenanceResult = `${resetResult.reset_count} ${$_("settings.maintenance.articles")} ${$_("settings.maintenance.reset")}. ${$_("settings.maintenance.ollamaUnavailable")}`;
        maintenanceRunning = null;
        return;
      }

      reanalyzeRunning = true;
      maintenanceRunning = "reanalyze";
      appState.batchProcessing = true;
      reanalyzeProgress = {
        current: 0,
        total: resetResult.reset_count,
        fnord_id: 0,
        title: $_("batch.starting"),
        success: true,
        error: null,
      };

      progressUnlisten = await listen<BatchProgress>("batch-progress", (event) => {
        reanalyzeProgress = { ...event.payload };
      });

      const batchResult = await invoke<BatchResult>("process_batch", {
        model,
        limit: null,
      });

      _reanalyzeResult = batchResult;
      maintenanceResult = $_("settings.maintenance.reanalyzeComplete", {
        values: {
          succeeded: batchResult.succeeded,
          failed: batchResult.failed,
        },
      });

      await appState.loadFnords();
      await appState.loadPentacles();
      await appState.loadUnprocessedCount();

      window.dispatchEvent(new CustomEvent("batch-complete"));
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
      reanalyzeRunning = false;
      appState.batchProcessing = false;
      if (progressUnlisten) {
        progressUnlisten();
        progressUnlisten = null;
      }
    }
  }

  async function handleCancelReanalyze() {
    try {
      await invoke("cancel_batch");
      maintenanceResult = $_("settings.maintenance.reanalyzeCancelled");
    } catch (e) {
      console.error("Failed to cancel reanalyze:", e);
    }
  }

  async function handleGeneratePrototypes() {
    generatingPrototypes = true;
    maintenanceResult = null;
    try {
      const result = await invoke<{
        total: number;
        generated: number;
        errors: number;
      }>("generate_keyword_type_prototypes");

      if (result.errors > 0) {
        maintenanceResult = $_("settings.maintenance.prototypesGeneratedWithErrors", {
          values: {
            count: result.generated,
            errors: result.errors,
          },
        });
      } else {
        maintenanceResult = $_("settings.maintenance.prototypesGenerated", {
          values: { count: result.generated },
        });
      }

      await loadPrototypeStatus();
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      generatingPrototypes = false;
    }
  }

  async function handleUpdateKeywordTypes() {
    maintenanceRunning = "keywordTypes";
    maintenanceResult = null;
    try {
      // Use hybrid detection (heuristic + semantic)
      const result = await invoke<{
        total: number;
        processed: number;
        updated: number;
        errors: number;
        by_type: {
          person: number;
          organization: number;
          location: number;
          acronym: number;
          concept: number;
        };
        by_method: {
          heuristic: number;
          semantic: number;
          llm: number;
        };
      }>("update_keyword_types_hybrid");

      maintenanceResult = $_("settings.maintenance.keywordTypesUpdatedSemantic", {
        values: {
          total: result.processed,
          concept: result.by_type.concept,
          person: result.by_type.person,
          organization: result.by_type.organization,
          location: result.by_type.location,
          acronym: result.by_type.acronym,
          lowConfidence: result.errors,
        },
      });
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }

  function handleShortContentResult(msg: string) {
    maintenanceResult = msg;
  }

  // Category Fix handler
  interface CategoryFixResult {
    fixed_count: number;
    categories_added: Record<string, number>;
    total_scanned: number;
  }

  async function handleFixCategories() {
    maintenanceRunning = "fixCategories";
    maintenanceResult = null;
    try {
      const result = await invoke<CategoryFixResult>("fix_category_assignments");
      if (result.fixed_count > 0) {
        const categories = Object.keys(result.categories_added).length;
        maintenanceResult = $_("settings.maintenance.fixCategoriesResult", {
          values: {
            fixed: result.fixed_count,
            categories: categories,
          },
        });
        // Refresh article data
        await appState.loadFnords();
      } else {
        maintenanceResult = $_("settings.maintenance.fixCategoriesNone");
      }
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceRunning = null;
    }
  }
</script>

<!-- Confirmation Dialog -->
{#if confirmAction}
  <div class="confirm-overlay">
    <div class="confirm-dialog">
      <p class="confirm-message">
        {#if confirmAction === "prune"}
          {$_("settings.maintenance.confirmPrune")}
        {:else if confirmAction === "reset"}
          {$_("settings.maintenance.confirmReset")}
        {/if}
      </p>
      <div class="confirm-actions">
        <button type="button" class="btn-secondary" onclick={cancelConfirmation}>
          {$_("confirm.no")}
        </button>
        <button
          type="button"
          class="btn-danger-solid"
          onclick={confirmAction === "prune"
            ? handlePruneLowQuality
            : handleResetForReprocessing}
        >
          {$_("confirm.yes")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Keyword Statistics -->
{#if keywordStats}
  <div class="keyword-stats">
    <h3>{$_("settings.maintenance.stats")}</h3>
    <div class="stats-grid">
      <div class="stat-item">
        <span class="stat-value">{keywordStats.total}</span>
        <span class="stat-label">{$_("settings.maintenance.totalKeywords")}</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{keywordStats.with_embeddings}</span>
        <span class="stat-label">{$_("settings.maintenance.withEmbeddings")}</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{keywordStats.avg_quality.toFixed(2)}</span>
        <span class="stat-label">{$_("settings.maintenance.avgQuality")}</span>
      </div>
      <div class="stat-item">
        <span class="stat-value {keywordStats.low_quality > 0 ? 'warning' : ''}"
          >{keywordStats.low_quality}</span
        >
        <span class="stat-label">{$_("settings.maintenance.lowQuality")}</span>
      </div>
    </div>
  </div>
{/if}

<h3>{$_("settings.maintenance.keywordQuality")}</h3>

{#if maintenanceResult}
  <div class="maintenance-result">
    {$_("settings.maintenance.result")}: {maintenanceResult}
  </div>
{/if}

<div class="maintenance-actions">
  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.calculateScores")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.calculateScoresDesc")}
      </p>
    </div>
    {#if maintenanceRunning !== "scores"}
      <ActionButton onclick={handleCalculateScores} disabled={maintenanceRunning !== null}>
        {$_("settings.maintenance.calculateScores")}
      </ActionButton>
    {/if}
  </div>

  {#if qualityRunning && qualityProgress}
    <MaintenanceProgress
      mode="determinate"
      current={qualityProgress.current}
      total={qualityProgress.total}
      label={$_("settings.maintenance.calculatingScores")}
      message={qualityProgress.keyword_name}
      status="running"
    />
  {:else if maintenanceRunning === "scores"}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.calculateScores")}
      message={$_("settings.maintenance.running")}
    />
  {/if}

  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.generateEmbeddings")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.generateEmbeddingsDesc")}
      </p>
    </div>
    {#if maintenanceRunning !== "embeddings"}
      <ActionButton
        onclick={handleGenerateEmbeddings}
        disabled={maintenanceRunning !== null || !ollamaAvailable}
      >
        {$_("settings.maintenance.generateEmbeddings")}
      </ActionButton>
    {/if}
  </div>

  {#if maintenanceRunning === "embeddings"}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.generateEmbeddings")}
      message={$_("settings.maintenance.running")}
    />
  {/if}

  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.statisticalAnalysis")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.statisticalAnalysisDesc")}
      </p>
    </div>
    {#if !statisticalRunning}
      <button
        type="button"
        class="btn-action"
        onclick={handleStatisticalAnalysis}
        disabled={maintenanceRunning !== null}
      >
        {$_("settings.maintenance.statisticalAnalysis")}
      </button>
    {/if}
  </div>

  {#if statisticalRunning && statisticalProgress}
    <MaintenanceProgress
      mode="determinate"
      current={statisticalProgress.current}
      total={statisticalProgress.total}
      label={$_("settings.maintenance.analyzing")}
      message={statisticalProgress.title}
      status={!statisticalProgress.success ? "error" : "running"}
      error={statisticalProgress.error}
    />
  {/if}

  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.fixCategories")}</span>
      <p class="action-desc">{$_("settings.maintenance.fixCategoriesDesc")}</p>
    </div>
    {#if maintenanceRunning !== "fixCategories"}
      <button
        type="button"
        class="btn-action"
        onclick={handleFixCategories}
        disabled={maintenanceRunning !== null}
      >
        {$_("settings.maintenance.fixCategories")}
      </button>
    {/if}
  </div>

  {#if maintenanceRunning === "fixCategories"}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.fixCategories")}
      message={$_("settings.maintenance.running")}
    />
  {/if}

  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.pruneLowQuality")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.pruneLowQualityDesc")}
      </p>
    </div>
    {#if maintenanceRunning !== "prune"}
      <ActionButton
        variant="danger"
        onclick={showPruneConfirmation}
        disabled={maintenanceRunning !== null}
      >
        {$_("settings.maintenance.pruneLowQuality")}
      </ActionButton>
    {/if}
  </div>

  {#if maintenanceRunning === "prune"}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.pruneLowQuality")}
      message={$_("settings.maintenance.running")}
    />
  {/if}

  <!-- Compound Keywords - Link to Network Tab -->
  <div class="maintenance-action compound-link">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.compoundKeywords")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.compoundKeywordsLinkDesc")}
      </p>
    </div>
    <span class="link-hint">
      <i class="fa-solid fa-arrow-right"></i>
      {$_("settings.maintenance.compoundKeywordsLocation")}
    </span>
  </div>

  <!-- Prototype Status Card -->
  {#if prototypeStatus}
    <div class="prototype-status" class:incomplete={!prototypeStatus.complete}>
      <div class="prototype-header">
        <span class="prototype-title">{$_("settings.maintenance.prototypeStatus")}</span>
        {#if prototypeStatus.complete}
          <span class="prototype-badge complete">
            <i class="fa-solid fa-check"></i>
            {$_("settings.maintenance.prototypeComplete")}
          </span>
        {:else}
          <span class="prototype-badge incomplete">
            <i class="fa-solid fa-exclamation-triangle"></i>
            {prototypeStatus.total}/{prototypeStatus.expected}
          </span>
        {/if}
      </div>
      <div class="prototype-info">
        <span
          >{$_("settings.maintenance.typesConfigured")}: {Object.keys(prototypeStatus.by_type)
            .length}</span
        >
      </div>
      {#if !prototypeStatus.complete || !generatingPrototypes}
        <button
          type="button"
          class="btn-action btn-small"
          onclick={handleGeneratePrototypes}
          disabled={generatingPrototypes || maintenanceRunning !== null || !ollamaAvailable}
        >
          {#if generatingPrototypes}
            <i class="fa-solid fa-spinner fa-spin"></i>
          {:else}
            <i class="fa-solid fa-wand-magic-sparkles"></i>
          {/if}
          {prototypeStatus.complete
            ? $_("settings.maintenance.regeneratePrototypes")
            : $_("settings.maintenance.generatePrototypes")}
        </button>
      {/if}
    </div>
  {/if}

  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.updateKeywordTypes")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.updateKeywordTypesDescSemantic")}
      </p>
    </div>
    {#if maintenanceRunning !== "keywordTypes"}
      <button
        type="button"
        class="btn-action"
        onclick={handleUpdateKeywordTypes}
        disabled={maintenanceRunning !== null}
      >
        {$_("settings.maintenance.updateKeywordTypes")}
      </button>
    {/if}
  </div>

  {#if maintenanceRunning === "keywordTypes"}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.updateKeywordTypes")}
      message={$_("settings.maintenance.running")}
    />
  {/if}
</div>

<h3 class="maintenance-section-heading">
  {$_("settings.maintenance.reprocessArticles")}
</h3>

<div class="maintenance-actions">
  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.resetForReprocessing")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.resetForReprocessingDesc")}
      </p>
    </div>
    {#if !reanalyzeRunning && maintenanceRunning !== "reset"}
      <ActionButton
        variant="danger"
        onclick={showResetConfirmation}
        disabled={maintenanceRunning !== null}
      >
        {$_("settings.maintenance.resetForReprocessing")}
      </ActionButton>
    {/if}
  </div>

  {#if maintenanceRunning === "reset" && !reanalyzeRunning}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.resetForReprocessing")}
      message={$_("settings.maintenance.running")}
    />
  {/if}

  {#if reanalyzeRunning && reanalyzeProgress}
    <MaintenanceProgress
      mode="determinate"
      current={reanalyzeProgress.current}
      total={reanalyzeProgress.total}
      label={$_("settings.maintenance.reanalyzing")}
      message={reanalyzeProgress.title}
      status={!reanalyzeProgress.success ? "error" : "running"}
      error={reanalyzeProgress.error}
      showCancel={true}
      onCancel={handleCancelReanalyze}
    />
  {/if}
</div>

<!-- Orphaned Articles Section -->
<MaintenanceOrphans {maintenanceRunning} />

<!-- Short Content Analysis Section -->
<MaintenanceShortContent {maintenanceRunning} onResult={handleShortContentResult} />

<style>
  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .keyword-stats {
    margin-bottom: 1.5rem;
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
  }

  .keyword-stats h3 {
    margin: 0 0 0.75rem 0;
    font-size: 0.875rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
  }

  .stat-item {
    text-align: center;
  }

  .stat-value {
    display: block;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--accent-primary);
  }

  .stat-value.warning {
    color: var(--status-warning);
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .maintenance-result {
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
    margin-bottom: 1rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .maintenance-actions {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .maintenance-action {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .action-info {
    flex: 1;
  }

  .action-title {
    font-weight: 500;
    color: var(--text-primary);
  }

  .action-desc {
    margin: 0.25rem 0 0 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .btn-action {
    padding: 0.5rem 1rem;
    border: 1px solid var(--accent-primary);
    border-radius: 0.375rem;
    background: none;
    color: var(--accent-primary);
    font-size: 0.875rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .btn-action:hover:not(:disabled) {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
  }

  .btn-action:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-action.btn-small {
    padding: 0.375rem 0.75rem;
    font-size: 0.75rem;
  }

  .btn-action.btn-small i {
    margin-right: 0.375rem;
  }

  /* Compound Keywords Link */
  .compound-link {
    border-color: var(--accent-primary);
    background: linear-gradient(90deg, var(--bg-overlay), rgba(137, 180, 250, 0.05));
  }

  .link-hint {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: var(--accent-primary);
    font-weight: 500;
  }

  .link-hint i {
    font-size: 0.75rem;
  }

  /* Prototype Status Card */
  .prototype-status {
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--status-success);
    margin-bottom: 0.5rem;
  }

  .prototype-status.incomplete {
    border-color: var(--status-warning);
  }

  .prototype-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .prototype-title {
    font-weight: 500;
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .prototype-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
  }

  .prototype-badge.complete {
    background-color: rgba(166, 227, 161, 0.2);
    color: var(--status-success);
  }

  .prototype-badge.incomplete {
    background-color: rgba(249, 226, 175, 0.2);
    color: var(--status-warning);
  }

  .prototype-info {
    display: flex;
    gap: 1rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-bottom: 0.5rem;
  }

  /* Confirmation Dialog */
  .confirm-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .confirm-dialog {
    background: var(--bg-surface);
    padding: 1.5rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
    max-width: 400px;
    text-align: center;
  }

  .confirm-message {
    margin: 0 0 1.5rem 0;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .confirm-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: center;
  }

  .btn-secondary {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-secondary);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary:hover {
    background-color: var(--bg-muted);
  }

  .btn-danger-solid {
    padding: 0.5rem 1.5rem;
    border: none;
    border-radius: 0.375rem;
    background-color: var(--status-error);
    color: var(--text-on-accent);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-danger-solid:hover {
    filter: brightness(1.1);
  }

  .btn-action i {
    margin-right: 0.375rem;
  }

  :global(.maintenance-section-heading) {
    margin-top: 1.5rem;
  }
</style>
