<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { appState } from "../../stores/state.svelte";
  import { maintenanceStore } from "../../stores/maintenance.svelte";
  import { onDestroy } from "svelte";
  import MaintenanceProgress from "./MaintenanceProgress.svelte";
  import ActionButton from "$lib/components/ui/ActionButton.svelte";
  import MaintenanceOrphans from "./MaintenanceOrphans.svelte";
  import MaintenanceShortContent from "./MaintenanceShortContent.svelte";
  import { createLogger } from "$lib/logger";

  const log = createLogger("SettingsMaintenance");
  interface Props {
    ollamaAvailable: boolean;
  }

  let { ollamaAvailable }: Props = $props();

  // Maintenance state
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

  // Prototype status for semantic keyword type detection
  let prototypeStatus = $state<{
    total: number;
    expected: number;
    complete: boolean;
    by_type: Record<string, number>;
  } | null>(null);
  let destroyed = false;

  // NER Backfill state
  interface NerProgress {
    processed: number;
    total: number;
    entities_found: number;
    errors: number;
    current_fnord_id: number | null;
  }

  interface NerPendingCount {
    pending: number;
    failed: number;
  }

  let nerProgress = $state<NerProgress | null>(null);
  let nerPendingCount = $state<NerPendingCount | null>(null);
  let nerBackfillUnlisten: UnlistenFn | null = null;

  // DB Reset state
  interface DbResetPreview {
    articles: number;
    keywords: number;
    entities: number;
    theme_reports: number;
    briefings: number;
    analysis_cache_entries: number;
    db_size_bytes: number;
    tables_to_clear: string[];
    tables_preserved: string[];
  }

  interface DbResetTableInfo {
    name: string;
    rows_before: number;
  }

  interface DbResetResult {
    backup_path: string;
    size_before_bytes: number;
    size_after_bytes: number;
    bytes_freed: number;
    tables_cleared: DbResetTableInfo[];
  }

  interface DbResetProgress {
    phase: "backup" | "clearing" | "vacuum" | "done";
    current_table: string | null;
    tables_done: number;
    tables_total: number;
  }

  let dbResetPreview = $state<DbResetPreview | null>(null);
  let dbResetPreviewLoading = $state(false);
  let dbResetConfirmChecked = $state(false);
  let dbResetConfirmInput = $state("");
  let dbResetProgress = $state<DbResetProgress | null>(null);
  let dbResetTablesClearedExpanded = $state(false);
  let dbResetTablesPreservedExpanded = $state(false);
  let dbResetUnlisten: UnlistenFn | null = null;

  const dbResetCanRun = $derived(
    dbResetConfirmChecked &&
      dbResetConfirmInput === "RESET" &&
      maintenanceStore.maintenanceRunning === null,
  );

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const kb = bytes / 1024;
    if (kb < 1024) return `${kb.toFixed(1)} KB`;
    const mb = kb / 1024;
    if (mb < 1024) return `${mb.toFixed(1)} MB`;
    const gb = mb / 1024;
    return `${gb.toFixed(2)} GB`;
  }

  function bytesToMb(bytes: number): string {
    return (bytes / (1024 * 1024)).toFixed(1);
  }

  async function loadDbResetPreview() {
    dbResetPreviewLoading = true;
    try {
      dbResetPreview = await invoke<DbResetPreview>("get_db_reset_preview");
    } catch (e) {
      log.error("Failed to load db reset preview:", e);
      maintenanceResult = `Error: ${e}`;
    } finally {
      dbResetPreviewLoading = false;
    }
  }

  async function handleDbReset() {
    if (!dbResetCanRun) return;

    maintenanceStore.maintenanceRunning = "dbReset";
    clearResultMessages();
    dbResetProgress = null;

    try {
      if (dbResetUnlisten) {
        dbResetUnlisten();
        dbResetUnlisten = null;
      }
      dbResetUnlisten = await listen<DbResetProgress>("db-reset-progress", (event) => {
        dbResetProgress = event.payload;
      });

      const result = await invoke<DbResetResult>("reset_articles_data", {
        confirmToken: "RESET",
      });

      const freedMb = bytesToMb(result.bytes_freed);
      const beforeMb = bytesToMb(result.size_before_bytes);
      const afterMb = bytesToMb(result.size_after_bytes);

      maintenanceResult = $_("settings.maintenance.dbResetResult", {
        values: {
          backup: result.backup_path,
          before: beforeMb,
          after: afterMb,
          freed: freedMb,
        },
      });

      // Reset UI state
      dbResetConfirmChecked = false;
      dbResetConfirmInput = "";

      // Refresh data
      await loadDbResetPreview();
      await appState.loadFnords();
      await appState.loadPentacles();
      await appState.loadUnprocessedCount();
      window.dispatchEvent(new CustomEvent("batch-complete"));
    } catch (e) {
      log.error("DB Reset error:", e);
      maintenanceResult = `Error: ${e}`;
    } finally {
      if (dbResetUnlisten) {
        dbResetUnlisten();
        dbResetUnlisten = null;
      }
      dbResetProgress = null;
      maintenanceStore.maintenanceRunning = null;
    }
  }

  export async function init() {
    // Don't reset maintenanceResult - keep showing the last result
    await Promise.all([
      loadKeywordStats(),
      loadPrototypeStatus(),
      loadNerPendingCount(),
      loadDbResetPreview(),
    ]);
  }

  async function loadPrototypeStatus() {
    try {
      prototypeStatus = await invoke("get_prototype_stats");
    } catch (e) {
      log.error("Failed to load prototype status:", e);
    }
  }

  async function loadNerPendingCount() {
    try {
      nerPendingCount = await invoke<NerPendingCount>("get_ner_pending_count");
    } catch (e) {
      log.error("Failed to load NER pending count:", e);
    }
  }

  onDestroy(() => {
    destroyed = true;
    if (nerBackfillUnlisten) {
      nerBackfillUnlisten();
      nerBackfillUnlisten = null;
    }
    if (dbResetUnlisten) {
      dbResetUnlisten();
      dbResetUnlisten = null;
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
      log.error("Failed to load keyword stats:", e);
    }
  }

  function clearResultMessages() {
    maintenanceResult = null;
    maintenanceStore.clearResult();
  }

  async function handleCalculateScores() {
    clearResultMessages();
    await maintenanceStore.calculateQualityScores();
    if (!destroyed) {
      await loadKeywordStats();
    }
  }

  async function handleGenerateEmbeddings() {
    clearResultMessages();
    await maintenanceStore.queueEmbeddings();
  }

  async function handleStatisticalAnalysis() {
    clearResultMessages();
    await maintenanceStore.processStatisticalAnalysis();
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
    maintenanceStore.maintenanceRunning = "prune";
    clearResultMessages();
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
      log.error("Prune error:", e);
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceStore.maintenanceRunning = null;
    }
  }

  async function handleResetForReprocessing() {
    confirmAction = null;
    maintenanceStore.maintenanceRunning = "reset";
    clearResultMessages();
    maintenanceStore.reanalyzeProgress = null;

    try {
      const resetResult = await invoke<{ reset_count: number }>("reset_articles_for_reprocessing", {
        only_with_content: true,
      });

      if (resetResult.reset_count === 0) {
        maintenanceResult = $_("settings.maintenance.noArticlesToReset");
        maintenanceStore.maintenanceRunning = null;
        return;
      }

      await emit("articles-reset");
      await appState.loadUnprocessedCount();

      const model = appState.selectedModel || appState.ollamaStatus.models[0];
      if (!model || !appState.ollamaStatus.available) {
        maintenanceResult = `${resetResult.reset_count} ${$_("settings.maintenance.articles")} ${$_("settings.maintenance.reset")}. ${$_("settings.maintenance.ollamaUnavailable")}`;
        maintenanceStore.maintenanceRunning = null;
        return;
      }

      await maintenanceStore.startReanalyze(resetResult.reset_count, model);
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceStore.maintenanceRunning = null;
    }
  }

  async function handleCancelReanalyze() {
    await maintenanceStore.cancelReanalyze();
  }

  async function handleGeneratePrototypes() {
    clearResultMessages();
    await maintenanceStore.generatePrototypes();
    if (!destroyed) {
      await loadPrototypeStatus();
    }
  }

  async function handleUpdateKeywordTypes() {
    maintenanceStore.maintenanceRunning = "keywordTypes";
    clearResultMessages();
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
      maintenanceStore.maintenanceRunning = null;
    }
  }

  async function handleExtractEntities() {
    maintenanceStore.maintenanceRunning = "entities";
    clearResultMessages();
    try {
      const result = await invoke<{
        processed: number;
        total_entities: number;
        errors: number;
      }>("extract_entities_batch", { limit: 50 });

      if (result.processed === 0) {
        maintenanceResult = $_("settings.maintenance.extractEntitiesNone");
      } else if (result.errors > 0) {
        maintenanceResult = $_("settings.maintenance.extractEntitiesResultWithErrors", {
          values: {
            processed: result.processed,
            entities: result.total_entities,
            errors: result.errors,
          },
        });
      } else {
        maintenanceResult = $_("settings.maintenance.extractEntitiesResult", {
          values: {
            processed: result.processed,
            entities: result.total_entities,
          },
        });
      }
    } catch (e) {
      maintenanceResult = `Error: ${e}`;
    } finally {
      maintenanceStore.maintenanceRunning = null;
    }
  }

  async function handleNerBackfill() {
    maintenanceStore.maintenanceRunning = "nerBackfill";
    clearResultMessages();
    nerProgress = null;

    // Register progress listener (once-off for this run)
    try {
      if (nerBackfillUnlisten) {
        nerBackfillUnlisten();
        nerBackfillUnlisten = null;
      }
      nerBackfillUnlisten = await listen<NerProgress>("ner-backfill-progress", (event) => {
        nerProgress = event.payload;
      });
    } catch (e) {
      log.error("Failed to register ner-backfill-progress listener:", e);
    }

    try {
      const result = await invoke<{
        processed: number;
        total_entities: number;
        errors: number;
      }>("extract_entities_backfill_all");

      if (result.errors > 0) {
        maintenanceResult = $_("settings.maintenance.nerBackfillResultWithErrors", {
          values: {
            processed: result.processed,
            entities: result.total_entities,
            errors: result.errors,
          },
        });
      } else {
        maintenanceResult = $_("settings.maintenance.nerBackfillResult", {
          values: {
            processed: result.processed,
            entities: result.total_entities,
          },
        });
      }

      if (!destroyed) {
        await loadNerPendingCount();
      }
    } catch (e) {
      log.error("NER Backfill error:", e);
      maintenanceResult = `Error: ${e}`;
    } finally {
      if (nerBackfillUnlisten) {
        nerBackfillUnlisten();
        nerBackfillUnlisten = null;
      }
      nerProgress = null;
      maintenanceStore.maintenanceRunning = null;
    }
  }

  // Category Fix handler
  interface CategoryFixResult {
    fixed_count: number;
    categories_added: Record<string, number>;
    total_scanned: number;
  }

  async function handleFixCategories() {
    maintenanceStore.maintenanceRunning = "fixCategories";
    clearResultMessages();
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
      maintenanceStore.maintenanceRunning = null;
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
          onclick={confirmAction === "prune" ? handlePruneLowQuality : handleResetForReprocessing}
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

{#if maintenanceStore.resultMessage || maintenanceResult}
  <div class="maintenance-result">
    {$_("settings.maintenance.result")}: {maintenanceStore.resultMessage ?? maintenanceResult}
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
    {#if maintenanceStore.maintenanceRunning !== "scores"}
      <ActionButton
        onclick={handleCalculateScores}
        disabled={maintenanceStore.maintenanceRunning !== null}
      >
        {$_("settings.maintenance.calculateScores")}
      </ActionButton>
    {/if}
  </div>

  {#if maintenanceStore.qualityRunning && maintenanceStore.qualityProgress}
    <MaintenanceProgress
      mode="determinate"
      current={maintenanceStore.qualityProgress.current}
      total={maintenanceStore.qualityProgress.total}
      label={$_("settings.maintenance.calculatingScores")}
      message={maintenanceStore.qualityProgress.keyword_name}
      status="running"
    />
  {:else if maintenanceStore.maintenanceRunning === "scores"}
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
    {#if maintenanceStore.maintenanceRunning !== "embeddings"}
      <ActionButton
        onclick={handleGenerateEmbeddings}
        disabled={maintenanceStore.maintenanceRunning !== null || !ollamaAvailable}
      >
        {$_("settings.maintenance.generateEmbeddings")}
      </ActionButton>
    {/if}
  </div>

  {#if maintenanceStore.maintenanceRunning === "embeddings"}
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
    {#if !maintenanceStore.statisticalRunning}
      <button
        type="button"
        class="btn-action"
        onclick={handleStatisticalAnalysis}
        disabled={maintenanceStore.maintenanceRunning !== null}
      >
        {$_("settings.maintenance.statisticalAnalysis")}
      </button>
    {/if}
  </div>

  {#if maintenanceStore.statisticalRunning && maintenanceStore.statisticalProgress}
    <MaintenanceProgress
      mode="determinate"
      current={maintenanceStore.statisticalProgress.current}
      total={maintenanceStore.statisticalProgress.total}
      label={$_("settings.maintenance.analyzing")}
      message={maintenanceStore.statisticalProgress.title}
      status={!maintenanceStore.statisticalProgress.success ? "error" : "running"}
      error={maintenanceStore.statisticalProgress.error}
    />
  {/if}

  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.fixCategories")}</span>
      <p class="action-desc">{$_("settings.maintenance.fixCategoriesDesc")}</p>
    </div>
    {#if maintenanceStore.maintenanceRunning !== "fixCategories"}
      <button
        type="button"
        class="btn-action"
        onclick={handleFixCategories}
        disabled={maintenanceStore.maintenanceRunning !== null}
      >
        {$_("settings.maintenance.fixCategories")}
      </button>
    {/if}
  </div>

  {#if maintenanceStore.maintenanceRunning === "fixCategories"}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.fixCategories")}
      message={$_("settings.maintenance.running")}
    />
  {/if}

  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.extractEntities")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.extractEntitiesDesc")}
      </p>
    </div>
    {#if maintenanceStore.maintenanceRunning !== "entities"}
      <button
        type="button"
        class="btn-action"
        onclick={handleExtractEntities}
        disabled={maintenanceStore.maintenanceRunning !== null}
      >
        {$_("settings.maintenance.extractEntities")}
      </button>
    {/if}
  </div>

  {#if maintenanceStore.maintenanceRunning === "entities"}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.extractEntities")}
      message={$_("settings.maintenance.extractEntitiesRunning")}
    />
  {/if}

  <!-- NER Backfill (all missing) -->
  <div class="maintenance-action">
    <div class="action-info">
      <div class="action-title-row">
        <span class="action-title">{$_("settings.maintenance.nerBackfillTitle")}</span>
        {#if nerPendingCount && nerPendingCount.pending + nerPendingCount.failed > 0}
          <span class="ner-badge">
            {$_("settings.maintenance.nerBackfillPendingBadge", {
              values: {
                pending: nerPendingCount.pending,
                failed: nerPendingCount.failed,
              },
            })}
          </span>
        {/if}
      </div>
      <p class="action-desc">
        {$_("settings.maintenance.nerBackfillDescription")}
      </p>
    </div>
    {#if nerPendingCount && nerPendingCount.pending + nerPendingCount.failed === 0}
      <span class="ner-all-done">
        {$_("settings.maintenance.nerBackfillAllDone")}
      </span>
    {:else if maintenanceStore.maintenanceRunning !== "nerBackfill"}
      <button
        type="button"
        class="btn-action"
        onclick={handleNerBackfill}
        disabled={maintenanceStore.maintenanceRunning !== null}
      >
        {$_("settings.maintenance.nerBackfillButton")}
      </button>
    {/if}
  </div>

  {#if maintenanceStore.maintenanceRunning === "nerBackfill"}
    {#if nerProgress && nerProgress.total > 0}
      <MaintenanceProgress
        mode="determinate"
        current={nerProgress.processed}
        total={nerProgress.total}
        label={$_("settings.maintenance.nerBackfillTitle")}
        message={$_("settings.maintenance.nerBackfillProgress", {
          values: {
            processed: nerProgress.processed,
            total: nerProgress.total,
            entities: nerProgress.entities_found,
          },
        })}
        status="running"
      />
    {:else}
      <MaintenanceProgress
        mode="indeterminate"
        label={$_("settings.maintenance.nerBackfillTitle")}
        message={$_("settings.maintenance.running")}
      />
    {/if}
  {/if}

  <div class="maintenance-action">
    <div class="action-info">
      <span class="action-title">{$_("settings.maintenance.pruneLowQuality")}</span>
      <p class="action-desc">
        {$_("settings.maintenance.pruneLowQualityDesc")}
      </p>
    </div>
    {#if maintenanceStore.maintenanceRunning !== "prune"}
      <ActionButton
        variant="danger"
        onclick={showPruneConfirmation}
        disabled={maintenanceStore.maintenanceRunning !== null}
      >
        {$_("settings.maintenance.pruneLowQuality")}
      </ActionButton>
    {/if}
  </div>

  {#if maintenanceStore.maintenanceRunning === "prune"}
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
      {#if !prototypeStatus.complete || !maintenanceStore.generatingPrototypes}
        <button
          type="button"
          class="btn-action btn-small"
          onclick={handleGeneratePrototypes}
          disabled={maintenanceStore.generatingPrototypes ||
            maintenanceStore.maintenanceRunning !== null ||
            !ollamaAvailable}
        >
          {#if maintenanceStore.generatingPrototypes}
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
    {#if maintenanceStore.maintenanceRunning !== "keywordTypes"}
      <button
        type="button"
        class="btn-action"
        onclick={handleUpdateKeywordTypes}
        disabled={maintenanceStore.maintenanceRunning !== null}
      >
        {$_("settings.maintenance.updateKeywordTypes")}
      </button>
    {/if}
  </div>

  {#if maintenanceStore.maintenanceRunning === "keywordTypes"}
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
    {#if !maintenanceStore.reanalyzeRunning && maintenanceStore.maintenanceRunning !== "reset"}
      <ActionButton
        variant="danger"
        onclick={showResetConfirmation}
        disabled={maintenanceStore.maintenanceRunning !== null}
      >
        {$_("settings.maintenance.resetForReprocessing")}
      </ActionButton>
    {/if}
  </div>

  {#if maintenanceStore.maintenanceRunning === "reset" && !maintenanceStore.reanalyzeRunning}
    <MaintenanceProgress
      mode="indeterminate"
      label={$_("settings.maintenance.resetForReprocessing")}
      message={$_("settings.maintenance.running")}
    />
  {/if}

  {#if maintenanceStore.reanalyzeRunning && maintenanceStore.reanalyzeProgress}
    <MaintenanceProgress
      mode="determinate"
      current={maintenanceStore.reanalyzeProgress.current}
      total={maintenanceStore.reanalyzeProgress.total}
      label={$_("settings.maintenance.reanalyzing")}
      message={maintenanceStore.reanalyzeProgress.title}
      status={!maintenanceStore.reanalyzeProgress.success ? "error" : "running"}
      error={maintenanceStore.reanalyzeProgress.error}
      showCancel={true}
      onCancel={handleCancelReanalyze}
    />
  {/if}
</div>

<!-- Orphaned Articles Section -->
<MaintenanceOrphans maintenanceRunning={maintenanceStore.maintenanceRunning} />

<!-- Short Content Analysis Section -->
<MaintenanceShortContent />

<!-- Danger Zone: Database Reset -->
<div class="db-reset-zone">
  <div class="db-reset-header">
    <i class="fa-solid fa-triangle-exclamation"></i>
    <h3 class="db-reset-title">{$_("settings.maintenance.dbResetTitle")}</h3>
  </div>
  <p class="db-reset-description">{$_("settings.maintenance.dbResetDescription")}</p>

  <div class="db-reset-stats-row">
    <button
      type="button"
      class="btn-preview"
      onclick={loadDbResetPreview}
      disabled={dbResetPreviewLoading || maintenanceStore.maintenanceRunning !== null}
    >
      {#if dbResetPreviewLoading}
        <i class="fa-solid fa-spinner fa-spin"></i>
      {:else}
        <i class="fa-solid fa-rotate"></i>
      {/if}
      {$_("settings.maintenance.dbResetPreviewButton")}
    </button>
  </div>

  {#if dbResetPreview}
    <div class="db-reset-stats">
      <div class="db-reset-stats-label">
        {$_("settings.maintenance.dbResetStatsLabel")}
      </div>
      <div class="db-reset-stats-grid">
        <div class="db-reset-stat">
          <span class="db-reset-stat-value">{formatBytes(dbResetPreview.db_size_bytes)}</span>
          <span class="db-reset-stat-label">DB-Size</span>
        </div>
        <div class="db-reset-stat">
          <span class="db-reset-stat-value">{dbResetPreview.articles}</span>
          <span class="db-reset-stat-label"
            >{$_("settings.maintenance.dbResetStatsArticles", {
              values: { n: dbResetPreview.articles },
            })}</span
          >
        </div>
        <div class="db-reset-stat">
          <span class="db-reset-stat-value">{dbResetPreview.keywords}</span>
          <span class="db-reset-stat-label"
            >{$_("settings.maintenance.dbResetStatsKeywords", {
              values: { n: dbResetPreview.keywords },
            })}</span
          >
        </div>
        <div class="db-reset-stat">
          <span class="db-reset-stat-value">{dbResetPreview.entities}</span>
          <span class="db-reset-stat-label"
            >{$_("settings.maintenance.dbResetStatsEntities", {
              values: { n: dbResetPreview.entities },
            })}</span
          >
        </div>
        <div class="db-reset-stat">
          <span class="db-reset-stat-value">{dbResetPreview.theme_reports}</span>
          <span class="db-reset-stat-label"
            >{$_("settings.maintenance.dbResetStatsReports", {
              values: { n: dbResetPreview.theme_reports },
            })}</span
          >
        </div>
        <div class="db-reset-stat">
          <span class="db-reset-stat-value">{dbResetPreview.briefings}</span>
          <span class="db-reset-stat-label"
            >{$_("settings.maintenance.dbResetStatsBriefings", {
              values: { n: dbResetPreview.briefings },
            })}</span
          >
        </div>
      </div>

      <details class="db-reset-tables" bind:open={dbResetTablesClearedExpanded}>
        <summary class="db-reset-tables-summary cleared">
          <i class="fa-solid fa-trash"></i>
          {$_("settings.maintenance.dbResetTablesCleared", {
            values: { count: dbResetPreview.tables_to_clear.length },
          })}
        </summary>
        <ul class="db-reset-tables-list">
          {#each dbResetPreview.tables_to_clear as table (table)}
            <li>{table}</li>
          {/each}
        </ul>
      </details>

      <details class="db-reset-tables" bind:open={dbResetTablesPreservedExpanded}>
        <summary class="db-reset-tables-summary preserved">
          <i class="fa-solid fa-shield-halved"></i>
          {$_("settings.maintenance.dbResetTablesPreserved", {
            values: { count: dbResetPreview.tables_preserved.length },
          })}
        </summary>
        <ul class="db-reset-tables-list">
          {#each dbResetPreview.tables_preserved as table (table)}
            <li>{table}</li>
          {/each}
        </ul>
      </details>
    </div>
  {/if}

  <div class="db-reset-confirm">
    <label class="db-reset-checkbox-label">
      <input
        type="checkbox"
        bind:checked={dbResetConfirmChecked}
        disabled={maintenanceStore.maintenanceRunning !== null}
      />
      <span>{$_("settings.maintenance.dbResetConfirmCheckbox")}</span>
    </label>

    <div class="db-reset-input-group">
      <input
        type="text"
        class="db-reset-input"
        bind:value={dbResetConfirmInput}
        placeholder={$_("settings.maintenance.dbResetConfirmPlaceholder")}
        disabled={maintenanceStore.maintenanceRunning !== null}
        autocomplete="off"
        spellcheck="false"
      />
      <small class="db-reset-input-hint">
        {$_("settings.maintenance.dbResetConfirmHint")}
      </small>
    </div>

    <button type="button" class="btn-db-reset" onclick={handleDbReset} disabled={!dbResetCanRun}>
      <i class="fa-solid fa-triangle-exclamation"></i>
      {$_("settings.maintenance.dbResetButton")}
    </button>
  </div>

  {#if maintenanceStore.maintenanceRunning === "dbReset"}
    {#if dbResetProgress}
      {#if dbResetProgress.phase === "backup"}
        <MaintenanceProgress
          mode="indeterminate"
          label={$_("settings.maintenance.dbResetTitle")}
          message={$_("settings.maintenance.dbResetProgressBackup")}
        />
      {:else if dbResetProgress.phase === "clearing"}
        <MaintenanceProgress
          mode="determinate"
          current={dbResetProgress.tables_done}
          total={dbResetProgress.tables_total}
          label={$_("settings.maintenance.dbResetTitle")}
          message={$_("settings.maintenance.dbResetProgressClearing", {
            values: {
              table: dbResetProgress.current_table ?? "",
              done: dbResetProgress.tables_done,
              total: dbResetProgress.tables_total,
            },
          })}
          status="running"
        />
      {:else if dbResetProgress.phase === "vacuum"}
        <MaintenanceProgress
          mode="indeterminate"
          label={$_("settings.maintenance.dbResetTitle")}
          message={$_("settings.maintenance.dbResetProgressVacuum")}
        />
      {:else}
        <MaintenanceProgress
          mode="indeterminate"
          label={$_("settings.maintenance.dbResetTitle")}
          message={$_("settings.maintenance.running")}
        />
      {/if}
    {:else}
      <MaintenanceProgress
        mode="indeterminate"
        label={$_("settings.maintenance.dbResetTitle")}
        message={$_("settings.maintenance.running")}
      />
    {/if}
  {/if}
</div>

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

  .action-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .ner-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
    background-color: rgba(249, 226, 175, 0.2);
    color: var(--status-warning);
    font-size: 0.75rem;
    font-weight: 500;
  }

  .ner-all-done {
    font-size: 0.8125rem;
    color: var(--status-success);
    font-weight: 500;
    white-space: nowrap;
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

  /* Danger Zone: DB Reset */
  .db-reset-zone {
    margin-top: 2.5rem;
    padding: 1.25rem;
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-left: 4px solid rgb(239, 68, 68);
    border-radius: 0.5rem;
    background-color: rgba(239, 68, 68, 0.05);
  }

  .db-reset-header {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    margin-bottom: 0.5rem;
  }

  .db-reset-header i {
    color: rgb(239, 68, 68);
    font-size: 1.125rem;
  }

  .db-reset-title {
    margin: 0;
    font-size: 1rem;
    color: rgb(239, 68, 68);
    font-weight: 600;
  }

  .db-reset-description {
    margin: 0 0 1rem 0;
    font-size: 0.8125rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .db-reset-stats-row {
    display: flex;
    justify-content: flex-start;
    margin-bottom: 1rem;
  }

  .btn-preview {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.85rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-preview:hover:not(:disabled) {
    background: var(--bg-overlay);
    color: var(--text-primary);
  }

  .btn-preview:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .db-reset-stats {
    margin-bottom: 1rem;
    padding: 0.875rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .db-reset-stats-label {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 0.75rem;
  }

  .db-reset-stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .db-reset-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 0.5rem;
    background: var(--bg-overlay);
    border-radius: 0.25rem;
  }

  .db-reset-stat-value {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .db-reset-stat-label {
    font-size: 0.7rem;
    color: var(--text-muted);
    margin-top: 0.125rem;
  }

  .db-reset-tables {
    margin-top: 0.5rem;
    border-top: 1px solid var(--border-default);
    padding-top: 0.5rem;
  }

  .db-reset-tables-summary {
    cursor: pointer;
    font-size: 0.8125rem;
    padding: 0.25rem 0;
    user-select: none;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .db-reset-tables-summary.cleared {
    color: rgb(239, 68, 68);
  }

  .db-reset-tables-summary.preserved {
    color: var(--status-success);
  }

  .db-reset-tables-summary:hover {
    opacity: 0.8;
  }

  .db-reset-tables-list {
    list-style: none;
    padding: 0.5rem 0 0.25rem 1.5rem;
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.75rem;
    color: var(--text-secondary);
    max-height: 160px;
    overflow-y: auto;
  }

  .db-reset-tables-list li {
    padding: 0.125rem 0;
  }

  .db-reset-confirm {
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
    padding: 0.875rem;
    background-color: var(--bg-surface);
    border-radius: 0.375rem;
    border: 1px solid rgba(239, 68, 68, 0.25);
  }

  .db-reset-checkbox-label {
    display: flex;
    align-items: flex-start;
    gap: 0.625rem;
    font-size: 0.8125rem;
    color: var(--text-primary);
    cursor: pointer;
    line-height: 1.4;
  }

  .db-reset-checkbox-label input[type="checkbox"] {
    margin-top: 0.15rem;
    flex-shrink: 0;
    accent-color: rgb(239, 68, 68);
    cursor: pointer;
  }

  .db-reset-input-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .db-reset-input {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.875rem;
    letter-spacing: 0.1em;
  }

  .db-reset-input:focus {
    outline: none;
    border-color: rgb(239, 68, 68);
    box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.2);
  }

  .db-reset-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .db-reset-input-hint {
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .btn-db-reset {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 1.25rem;
    border: 1px solid rgb(239, 68, 68);
    border-radius: 0.375rem;
    background-color: rgb(239, 68, 68);
    color: #fff;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-db-reset:hover:not(:disabled) {
    background-color: rgb(220, 38, 38);
    border-color: rgb(220, 38, 38);
  }

  .btn-db-reset:disabled {
    background-color: transparent;
    color: rgba(239, 68, 68, 0.5);
    border-color: rgba(239, 68, 68, 0.3);
    cursor: not-allowed;
  }
</style>
