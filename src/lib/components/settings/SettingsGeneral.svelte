<script lang="ts">
  import { _ } from "svelte-i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
  import {
    settings,
    type DarkTheme,
    type LightTheme,
    type ThemeMode,
  } from "../../stores/settings.svelte";
  import { type LogLevel } from "../../logger";
  import { setLocale, locale } from "../../i18n";
  import { appState, toasts } from "../../stores/state.svelte";
  import type { OpmlFeedPreview, OpmlImportResult } from "../../types";

  // Local state for form
  let selectedLocale = $state("de");
  let showTooltips = $state(true);
  let selectedThemeMode = $state<ThemeMode>("system");
  let selectedDarkTheme = $state<DarkTheme>("mocha");
  let selectedLightTheme = $state<LightTheme>("latte");
  let syncInterval = $state(30);
  let syncOnStart = $state(true);
  let enableHeadlessBrowser = $state(false);
  let selectedLogLevel = $state<LogLevel>("info");

  // Dropdown open states
  let langDropdownOpen = $state(false);
  let darkThemeDropdownOpen = $state(false);
  let lightThemeDropdownOpen = $state(false);
  let logLevelDropdownOpen = $state(false);

  // OPML Import state
  let opmlPreview = $state<OpmlFeedPreview[]>([]);
  let opmlContent = $state<string | null>(null);
  let opmlImporting = $state(false);
  let opmlResult = $state<OpmlImportResult | null>(null);
  let opmlError = $state<string | null>(null);

  // OPML Export state
  let opmlExporting = $state(false);
  let opmlExportResult = $state<string | null>(null);
  let opmlExportError = $state<string | null>(null);

  // Short Content Analysis state
  interface ShortContentStats {
    total_fetched: number;
    content_null_or_empty: number;
    content_under_200: number;
    content_200_to_500: number;
    content_over_500: number;
    by_feed: { pentacle_id: number; pentacle_title: string; short_articles: number }[];
  }
  interface RefetchResponse {
    total_found: number;
    processed: number;
    improved: number;
    failed: number;
  }
  let shortContentAnalyzing = $state(false);
  let shortContentStats = $state<ShortContentStats | null>(null);
  let shortContentError = $state<string | null>(null);
  let shortContentRefetching = $state(false);
  let shortContentRefetchResult = $state<RefetchResponse | null>(null);

  const localeOptions = [
    { value: "de", labelKey: "settings.languageGerman" },
    { value: "en", labelKey: "settings.languageEnglish" },
  ];

  const themeModeOptions: { value: ThemeMode; labelKey: string }[] = [
    { value: "light", labelKey: "settings.themeModeLight" },
    { value: "dark", labelKey: "settings.themeModeDark" },
    { value: "system", labelKey: "settings.themeModeSystem" },
  ];

  interface ThemeOption<T> {
    value: T;
    labelKey: string;
    family: string;
  }

  const darkThemeOptions: ThemeOption<DarkTheme>[] = [
    { value: "mocha", labelKey: "settings.themes.mocha", family: "catppuccin" },
    { value: "macchiato", labelKey: "settings.themes.macchiato", family: "catppuccin" },
    { value: "frappe", labelKey: "settings.themes.frappe", family: "catppuccin" },
    { value: "ayu-dark", labelKey: "settings.themes.ayu-dark", family: "ayu" },
    { value: "ayu-mirage", labelKey: "settings.themes.ayu-mirage", family: "ayu" },
    { value: "gruvbox-dark", labelKey: "settings.themes.gruvbox-dark", family: "gruvbox" },
    { value: "tokyo-night", labelKey: "settings.themes.tokyo-night", family: "tokyoNight" },
    { value: "tokyo-storm", labelKey: "settings.themes.tokyo-storm", family: "tokyoNight" },
    { value: "solarized-dark", labelKey: "settings.themes.solarized-dark", family: "solarized" },
    { value: "dracula", labelKey: "settings.themes.dracula", family: "dracula" },
    { value: "nord", labelKey: "settings.themes.nord", family: "nord" },
    { value: "everforest", labelKey: "settings.themes.everforest", family: "everforest" },
  ];

  const lightThemeOptions: ThemeOption<LightTheme>[] = [
    { value: "latte", labelKey: "settings.themes.latte", family: "catppuccin" },
    { value: "ayu-light", labelKey: "settings.themes.ayu-light", family: "ayu" },
    { value: "gruvbox-light", labelKey: "settings.themes.gruvbox-light", family: "gruvbox" },
    { value: "tokyo-day", labelKey: "settings.themes.tokyo-day", family: "tokyoNight" },
    { value: "solarized-light", labelKey: "settings.themes.solarized-light", family: "solarized" },
  ];

  const themeFamilyLabels: Record<string, string> = {
    catppuccin: "settings.themeFamily.catppuccin",
    ayu: "settings.themeFamily.ayu",
    gruvbox: "settings.themeFamily.gruvbox",
    tokyoNight: "settings.themeFamily.tokyoNight",
    solarized: "settings.themeFamily.solarized",
    dracula: "settings.themeFamily.dracula",
    nord: "settings.themeFamily.nord",
    everforest: "settings.themeFamily.everforest",
  };

  const logLevelOptions: { value: LogLevel; label: string }[] = [
    { value: "error", label: "Error" },
    { value: "warn", label: "Warn" },
    { value: "info", label: "Info" },
    { value: "debug", label: "Debug" },
    { value: "trace", label: "Trace" },
  ];

  export function init() {
    selectedLocale = $locale || "de";
    showTooltips = settings.showTerminologyTooltips;
    selectedThemeMode = settings.themeMode;
    selectedDarkTheme = settings.darkTheme;
    selectedLightTheme = settings.lightTheme;
    syncInterval = settings.syncInterval;
    syncOnStart = settings.syncOnStart;
    enableHeadlessBrowser = settings.enableHeadlessBrowser;
    selectedLogLevel = settings.logLevel;
  }

  export function closeAllDropdowns() {
    langDropdownOpen = false;
    darkThemeDropdownOpen = false;
    lightThemeDropdownOpen = false;
    logLevelDropdownOpen = false;
  }

  function handleTooltipsChange(checked: boolean) {
    showTooltips = checked;
    settings.showTerminologyTooltips = checked;
  }

  function handleSyncIntervalChange(value: number) {
    syncInterval = value;
    settings.syncInterval = value;
  }

  function handleSyncOnStartChange(checked: boolean) {
    syncOnStart = checked;
    settings.syncOnStart = checked;
  }

  function handleEnableHeadlessBrowserChange(checked: boolean) {
    enableHeadlessBrowser = checked;
    settings.enableHeadlessBrowser = checked;
  }

  async function selectLocale(value: string) {
    selectedLocale = value;
    langDropdownOpen = false;
    await setLocale(value);
  }

  function selectThemeMode(value: ThemeMode) {
    selectedThemeMode = value;
    settings.themeMode = value;
  }

  function selectDarkTheme(value: DarkTheme) {
    selectedDarkTheme = value;
    darkThemeDropdownOpen = false;
    settings.darkTheme = value;
  }

  function selectLightTheme(value: LightTheme) {
    selectedLightTheme = value;
    lightThemeDropdownOpen = false;
    settings.lightTheme = value;
  }

  function selectLogLevel(value: LogLevel) {
    selectedLogLevel = value;
    logLevelDropdownOpen = false;
    settings.logLevel = value;
  }

  function toggleLangDropdown() {
    langDropdownOpen = !langDropdownOpen;
    darkThemeDropdownOpen = false;
    lightThemeDropdownOpen = false;
    logLevelDropdownOpen = false;
  }

  function toggleDarkThemeDropdown() {
    darkThemeDropdownOpen = !darkThemeDropdownOpen;
    langDropdownOpen = false;
    lightThemeDropdownOpen = false;
    logLevelDropdownOpen = false;
  }

  function toggleLightThemeDropdown() {
    lightThemeDropdownOpen = !lightThemeDropdownOpen;
    langDropdownOpen = false;
    darkThemeDropdownOpen = false;
    logLevelDropdownOpen = false;
  }

  function toggleLogLevelDropdown() {
    logLevelDropdownOpen = !logLevelDropdownOpen;
    langDropdownOpen = false;
    darkThemeDropdownOpen = false;
    lightThemeDropdownOpen = false;
  }

  function getLocaleLabelKey(value: string): string {
    return localeOptions.find((o) => o.value === value)?.labelKey || "";
  }

  function getThemeDisplayName<T extends string>(
    value: T,
    options: ThemeOption<T>[]
  ): string {
    const option = options.find((o) => o.value === value);
    if (!option) return value;

    const familyLabelKey = themeFamilyLabels[option.family];
    const familyName = familyLabelKey ? $_(familyLabelKey) : "";
    const themeName = $_(option.labelKey);

    return `${familyName} ${themeName}`;
  }

  // OPML Import handlers
  async function handleSelectOpmlFile() {
    opmlError = null;
    opmlResult = null;
    opmlPreview = [];
    opmlContent = null;

    try {
      const filePath = await open({
        multiple: false,
        filters: [
          {
            name: "OPML",
            extensions: ["opml", "xml"],
          },
        ],
      });

      if (!filePath) return;

      const content = await readTextFile(filePath as string);
      opmlContent = content;
      const preview = await invoke<OpmlFeedPreview[]>("parse_opml_preview", {
        content,
      });
      opmlPreview = preview;
    } catch (e) {
      opmlError = String(e);
    }
  }

  async function handleImportOpml() {
    if (opmlPreview.length === 0 || !opmlContent) return;

    opmlImporting = true;
    opmlError = null;

    try {
      const result = await invoke<OpmlImportResult>("import_opml", {
        content: opmlContent,
        skipExisting: true,
      });

      opmlResult = result;
      opmlPreview = [];
      opmlContent = null;

      await appState.loadPentacles();
    } catch (e) {
      opmlError = String(e);
    } finally {
      opmlImporting = false;
    }
  }

  function handleClearOpmlPreview() {
    opmlPreview = [];
    opmlContent = null;
    opmlResult = null;
    opmlError = null;
  }

  async function handleExportOpml() {
    opmlExporting = true;
    opmlExportResult = null;
    opmlExportError = null;

    try {
      if (appState.pentacles.length === 0) {
        opmlExportError = $_("settings.opml.noFeedsToExport");
        opmlExporting = false;
        return;
      }

      const opmlExportContent = await invoke<string>("export_opml");

      const filePath = await save({
        filters: [
          {
            name: "OPML",
            extensions: ["opml"],
          },
        ],
        defaultPath: "fuckupRSS-feeds.opml",
      });

      if (!filePath) {
        opmlExporting = false;
        return;
      }

      await writeTextFile(filePath, opmlExportContent);

      opmlExportResult = $_("settings.opml.exportSuccess", {
        values: { count: appState.pentacles.length },
      });
    } catch (e) {
      opmlExportError = String(e);
    } finally {
      opmlExporting = false;
    }
  }

  // Short Content Analysis handlers
  async function handleAnalyzeShortContent() {
    shortContentAnalyzing = true;
    shortContentError = null;
    shortContentStats = null;
    shortContentRefetchResult = null;

    try {
      const stats = await invoke<ShortContentStats>("get_short_content_stats");
      shortContentStats = stats;
    } catch (e) {
      shortContentError = String(e);
    } finally {
      shortContentAnalyzing = false;
    }
  }

  async function handleRefetchShortContent() {
    if (!enableHeadlessBrowser) return;

    shortContentRefetching = true;
    shortContentError = null;
    shortContentRefetchResult = null;

    try {
      const result = await invoke<RefetchResponse>("refetch_short_articles");
      shortContentRefetchResult = result;
      // Refresh stats after refetch
      const stats = await invoke<ShortContentStats>("get_short_content_stats");
      shortContentStats = stats;
    } catch (e) {
      shortContentError = String(e);
    } finally {
      shortContentRefetching = false;
    }
  }
</script>

<!-- Language Dropdown -->
<div class="setting-group">
  <span class="label">{$_("settings.language")}</span>
  <div class="custom-select">
    <button
      type="button"
      class="select-trigger"
      aria-label={$_("settings.language")}
      onclick={toggleLangDropdown}
    >
      <span>{$_(getLocaleLabelKey(selectedLocale))}</span>
      <i class="arrow fa-solid {langDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
    </button>
    {#if langDropdownOpen}
      <div class="select-options">
        {#each localeOptions as option (option.value)}
          <button
            type="button"
            class="select-option {selectedLocale === option.value ? 'selected' : ''}"
            onclick={() => selectLocale(option.value)}
          >
            {$_(option.labelKey)}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- Theme Mode Selection -->
<div class="setting-group">
  <span class="label">{$_("settings.themeMode")}</span>
  <div class="theme-mode-buttons">
    {#each themeModeOptions as option (option.value)}
      <button
        type="button"
        class="theme-mode-btn {selectedThemeMode === option.value ? 'active' : ''}"
        onclick={() => selectThemeMode(option.value)}
      >
        {$_(option.labelKey)}
      </button>
    {/each}
  </div>
  <p class="setting-description">{$_("settings.themeModeDescription")}</p>
</div>

<!-- Dark Theme Selection -->
<div class="setting-group">
  <span class="label">{$_("settings.darkTheme")}</span>
  <div class="custom-select">
    <button
      type="button"
      class="select-trigger"
      aria-label={$_("settings.darkTheme")}
      onclick={toggleDarkThemeDropdown}
    >
      <span>{getThemeDisplayName(selectedDarkTheme, darkThemeOptions)}</span>
      <i class="arrow fa-solid {darkThemeDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
    </button>
    {#if darkThemeDropdownOpen}
      <div class="select-options">
        {#each darkThemeOptions as option (option.value)}
          <button
            type="button"
            class="select-option {selectedDarkTheme === option.value ? 'selected' : ''}"
            onclick={() => selectDarkTheme(option.value)}
          >
            {getThemeDisplayName(option.value, darkThemeOptions)}
          </button>
        {/each}
      </div>
    {/if}
  </div>
  <p class="setting-description">{$_("settings.darkThemeDescription")}</p>
</div>

<!-- Light Theme Selection -->
<div class="setting-group">
  <span class="label">{$_("settings.lightTheme")}</span>
  <div class="custom-select">
    <button
      type="button"
      class="select-trigger"
      aria-label={$_("settings.lightTheme")}
      onclick={toggleLightThemeDropdown}
    >
      <span>{getThemeDisplayName(selectedLightTheme, lightThemeOptions)}</span>
      <i class="arrow fa-solid {lightThemeDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
    </button>
    {#if lightThemeDropdownOpen}
      <div class="select-options">
        {#each lightThemeOptions as option (option.value)}
          <button
            type="button"
            class="select-option {selectedLightTheme === option.value ? 'selected' : ''}"
            onclick={() => selectLightTheme(option.value)}
          >
            {getThemeDisplayName(option.value, lightThemeOptions)}
          </button>
        {/each}
      </div>
    {/if}
  </div>
  <p class="setting-description">{$_("settings.lightThemeDescription")}</p>
</div>

<div class="setting-group checkbox-group">
  <label>
    <input
      type="checkbox"
      checked={showTooltips}
      onchange={(e) => handleTooltipsChange(e.currentTarget.checked)}
    />
    <span class="checkbox-label">{$_("settings.tooltips")}</span>
  </label>
  <p class="setting-description">{$_("settings.tooltipsDescription")}</p>
</div>

<!-- Sync Settings -->
<div class="setting-group">
  <span class="label">{$_("settings.sync.title")}</span>
</div>

<div class="setting-group">
  <label class="label" for="sync-interval">{$_("settings.sync.interval")}</label>
  <div class="slider-row">
    <input
      id="sync-interval"
      type="range"
      min="5"
      max="120"
      step="5"
      value={syncInterval}
      onchange={(e) => handleSyncIntervalChange(parseInt(e.currentTarget.value))}
      class="slider"
    />
    <span class="slider-value">
      {$_("settings.sync.minutes", { values: { count: syncInterval } })}
    </span>
  </div>
  <p class="setting-description">{$_("settings.sync.intervalDescription")}</p>
</div>

<div class="setting-group checkbox-group">
  <label>
    <input
      type="checkbox"
      checked={syncOnStart}
      onchange={(e) => handleSyncOnStartChange(e.currentTarget.checked)}
    />
    <span class="checkbox-label">{$_("settings.sync.onStart")}</span>
  </label>
  <p class="setting-description">{$_("settings.sync.onStartDescription")}</p>
</div>

<div class="setting-group checkbox-group">
  <label>
    <input
      type="checkbox"
      checked={enableHeadlessBrowser}
      onchange={(e) => handleEnableHeadlessBrowserChange(e.currentTarget.checked)}
    />
    <span class="checkbox-label">{$_("settings.enableHeadlessBrowser")}</span>
  </label>
  <p class="setting-description">{$_("settings.enableHeadlessBrowserDescription")}</p>
</div>

<!-- OPML Import -->
<div class="setting-group">
  <span class="label">{$_("settings.opml.title")}</span>
  <p class="setting-description">{$_("settings.opml.description")}</p>
</div>

<div class="opml-section">
  <button
    type="button"
    class="btn-action"
    onclick={handleSelectOpmlFile}
    disabled={opmlImporting}
  >
    {$_("settings.opml.selectFile")}
  </button>

  {#if opmlError}
    <div class="opml-error">{opmlError}</div>
  {/if}

  {#if opmlResult}
    <div class="opml-result">
      <p>
        {$_("settings.opml.importResult", {
          values: {
            imported: opmlResult.imported,
            skipped: opmlResult.skipped,
            total: opmlResult.total_feeds,
          },
        })}
      </p>
      {#if opmlResult.errors.length > 0}
        <div class="opml-errors">
          {#each opmlResult.errors as error, i (i)}
            <div class="opml-error-item">{error}</div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if opmlPreview.length > 0}
    <div class="opml-preview">
      <div class="opml-preview-header">
        <span>
          {$_("settings.opml.feedsFound", { values: { count: opmlPreview.length } })}
        </span>
        <button type="button" class="btn-small" onclick={handleClearOpmlPreview}>
          {$_("settings.opml.clear")}
        </button>
      </div>
      <div class="opml-feed-list">
        {#each opmlPreview as feed (feed.url)}
          <div class="opml-feed-item" class:exists={feed.already_exists}>
            <div class="opml-feed-info">
              <span class="opml-feed-title">
                {feed.title || feed.url}
              </span>
              {#if feed.category}
                <span class="opml-feed-category">{feed.category}</span>
              {/if}
            </div>
            {#if feed.already_exists}
              <span class="opml-feed-exists">
                {$_("settings.opml.alreadyExists")}
              </span>
            {/if}
          </div>
        {/each}
      </div>
      <div class="opml-preview-actions">
        <button
          type="button"
          class="btn-action"
          onclick={handleImportOpml}
          disabled={opmlImporting || opmlPreview.every((f) => f.already_exists)}
        >
          {#if opmlImporting}
            {$_("settings.opml.importing")}
          {:else}
            {$_("settings.opml.import")}
          {/if}
        </button>
        <span class="opml-preview-info">
          {$_("settings.opml.newFeeds", {
            values: { count: opmlPreview.filter((f) => !f.already_exists).length },
          })}
        </span>
      </div>
    </div>
  {/if}
</div>

<!-- OPML Export -->
<div class="opml-section opml-export">
  <div class="export-row">
    <div class="export-info">
      <span class="export-label">{$_("settings.opml.export")}</span>
      <p class="export-desc">{$_("settings.opml.exportDescription")}</p>
    </div>
    <button
      type="button"
      class="btn-action"
      onclick={handleExportOpml}
      disabled={opmlExporting || appState.pentacles.length === 0}
    >
      {#if opmlExporting}
        {$_("settings.opml.exporting")}
      {:else}
        {$_("settings.opml.exportButton")}
      {/if}
    </button>
  </div>

  {#if opmlExportError}
    <div class="opml-error">{opmlExportError}</div>
  {/if}

  {#if opmlExportResult}
    <div class="opml-result">{opmlExportResult}</div>
  {/if}
</div>

<!-- Short Content Analysis -->
<div class="setting-group">
  <span class="label">{$_("settings.maintenance.shortContent.title")}</span>
  <p class="setting-description">{$_("settings.maintenance.shortContent.analyzeDesc")}</p>
</div>

<div class="short-content-section">
  <div class="short-content-row">
    <button
      type="button"
      class="btn-action"
      onclick={handleAnalyzeShortContent}
      disabled={shortContentAnalyzing}
    >
      {#if shortContentAnalyzing}
        <i class="fa-solid fa-spinner fa-spin"></i>
        {$_("settings.maintenance.shortContent.analyzing")}
      {:else}
        <i class="fa-solid fa-magnifying-glass-chart"></i>
        {$_("settings.maintenance.shortContent.analyze")}
      {/if}
    </button>
  </div>

  {#if shortContentError}
    <div class="short-content-error">{shortContentError}</div>
  {/if}

  {#if shortContentStats}
    {@const totalShort = shortContentStats.content_null_or_empty + shortContentStats.content_under_200 + shortContentStats.content_200_to_500}
    <div class="short-content-stats">
      <div class="stats-header">
        {$_("settings.maintenance.shortContent.found", {
          values: { count: totalShort }
        })}
      </div>
      <div class="stats-breakdown">
        <div class="stat-item null-empty">
          <span class="stat-label">{$_("settings.maintenance.shortContent.breakdown.nullEmpty")}</span>
          <span class="stat-value">{shortContentStats.content_null_or_empty}</span>
        </div>
        <div class="stat-item very-short">
          <span class="stat-label">{$_("settings.maintenance.shortContent.breakdown.veryShort")}</span>
          <span class="stat-value">{shortContentStats.content_under_200}</span>
        </div>
        <div class="stat-item short">
          <span class="stat-label">{$_("settings.maintenance.shortContent.breakdown.short")}</span>
          <span class="stat-value">{shortContentStats.content_200_to_500}</span>
        </div>
        <div class="stat-item ok">
          <span class="stat-label">{$_("settings.maintenance.shortContent.breakdown.ok")}</span>
          <span class="stat-value">{shortContentStats.content_over_500}</span>
        </div>
      </div>

      {#if totalShort > 0}
        <div class="refetch-section">
          <p class="setting-description">{$_("settings.maintenance.shortContent.refetchDesc")}</p>

          {#if !enableHeadlessBrowser}
            <div class="headless-warning">
              <i class="fa-solid fa-triangle-exclamation"></i>
              {$_("settings.maintenance.shortContent.headlessRequired")}
            </div>
          {:else}
            <button
              type="button"
              class="btn-action btn-refetch"
              onclick={handleRefetchShortContent}
              disabled={shortContentRefetching}
            >
              {#if shortContentRefetching}
                <i class="fa-solid fa-spinner fa-spin"></i>
                {$_("settings.maintenance.shortContent.refetching")}
              {:else}
                <i class="fa-solid fa-rotate"></i>
                {$_("settings.maintenance.shortContent.refetch")}
              {/if}
            </button>
          {/if}

          {#if shortContentRefetchResult}
            <div class="refetch-result">
              {$_("settings.maintenance.shortContent.refetchResult", {
                values: {
                  improved: shortContentRefetchResult.improved,
                  total: shortContentRefetchResult.processed
                }
              })}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Log Level (Dev Mode) -->
{#if import.meta.env.DEV}
  <div class="setting-group">
    <span class="label">{$_("settings.logLevel")}</span>
    <div class="custom-select">
      <button
        type="button"
        class="select-trigger"
        aria-label={$_("settings.logLevel")}
        onclick={toggleLogLevelDropdown}
      >
        <span class="log-level-display">
          <span class="log-level-badge {selectedLogLevel}">{selectedLogLevel.toUpperCase()}</span>
        </span>
        <i class="arrow fa-solid {logLevelDropdownOpen ? 'fa-caret-up' : 'fa-caret-down'}"></i>
      </button>
      {#if logLevelDropdownOpen}
        <div class="select-options">
          {#each logLevelOptions as option (option.value)}
            <button
              type="button"
              class="select-option {selectedLogLevel === option.value ? 'selected' : ''}"
              onclick={() => selectLogLevel(option.value)}
            >
              <span class="log-level-badge {option.value}">{option.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <p class="setting-description">{$_("settings.logLevelDescription")}</p>
  </div>
{/if}

<style>
  .setting-group {
    margin-bottom: 1.25rem;
    max-width: 600px;
  }

  .setting-group > label,
  .setting-group > .label {
    display: block;
    margin-bottom: 0.375rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  /* Custom Select */
  .custom-select {
    position: relative;
  }

  .select-trigger {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 1rem;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    text-align: left;
  }

  .select-trigger:hover {
    border-color: var(--accent-primary);
  }

  .arrow {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .select-options {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 0.25rem;
    background-color: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    overflow: hidden;
    z-index: 100;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    max-height: 200px;
    overflow-y: auto;
  }

  .select-option {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 1rem;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.15s;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .select-option:hover {
    background-color: var(--bg-muted);
  }

  .select-option.selected {
    background-color: var(--bg-muted);
    color: var(--accent-primary);
  }

  /* Theme Mode Buttons */
  .theme-mode-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .theme-mode-btn {
    flex: 1;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-default);
    border-radius: 0.375rem;
    background-color: var(--bg-overlay);
    color: var(--text-primary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .theme-mode-btn:hover {
    border-color: var(--accent-primary);
  }

  .theme-mode-btn.active {
    background-color: var(--accent-primary);
    color: var(--text-on-accent);
    border-color: var(--accent-primary);
  }

  /* Checkbox */
  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .checkbox-group input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent-primary);
  }

  .checkbox-label {
    font-weight: 500;
  }

  .setting-description {
    margin: 0.25rem 0 0 0;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  /* Slider */
  .slider-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .slider {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    appearance: none;
    background: var(--bg-overlay);
    cursor: pointer;
  }

  .slider::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent-primary);
    cursor: pointer;
    transition: transform 0.15s;
  }

  .slider::-webkit-slider-thumb:hover {
    transform: scale(1.1);
  }

  .slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent-primary);
    cursor: pointer;
    border: none;
  }

  .slider-value {
    min-width: 6rem;
    text-align: right;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  /* Log Level Badges */
  .log-level-display {
    display: flex;
    align-items: center;
  }

  .log-level-badge {
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 600;
    font-family: monospace;
  }

  .log-level-badge.error {
    background-color: color-mix(in srgb, var(--status-error) 20%, transparent);
    color: var(--status-error);
  }

  .log-level-badge.warn {
    background-color: color-mix(in srgb, var(--status-warning) 20%, transparent);
    color: var(--status-warning);
  }

  .log-level-badge.info {
    background-color: color-mix(in srgb, var(--accent-info) 20%, transparent);
    color: var(--accent-info);
  }

  .log-level-badge.debug {
    background-color: color-mix(in srgb, var(--accent-primary) 20%, transparent);
    color: var(--accent-primary);
  }

  .log-level-badge.trace {
    background-color: color-mix(in srgb, var(--text-muted) 20%, transparent);
    color: var(--text-muted);
  }

  /* OPML Import */
  .opml-section {
    max-width: 600px;
    margin-bottom: 1.5rem;
  }

  .opml-error {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.375rem;
    color: var(--status-error);
    font-size: 0.875rem;
  }

  .opml-result {
    margin-top: 0.5rem;
    padding: 0.75rem;
    background-color: rgba(166, 227, 161, 0.1);
    border-radius: 0.375rem;
    color: var(--status-success);
    font-size: 0.875rem;
  }

  .opml-result p {
    margin: 0;
  }

  .opml-errors {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid rgba(243, 139, 168, 0.2);
  }

  .opml-error-item {
    font-size: 0.75rem;
    color: var(--status-error);
    margin-top: 0.25rem;
  }

  .opml-preview {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .opml-preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .btn-small {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--text-muted);
    border-radius: 0.25rem;
    background: none;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-small:hover {
    border-color: var(--text-primary);
    color: var(--text-primary);
  }

  .opml-feed-list {
    max-height: 200px;
    overflow-y: auto;
    margin-bottom: 0.75rem;
  }

  .opml-feed-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.375rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }

  .opml-feed-item:hover {
    background-color: var(--bg-muted);
  }

  .opml-feed-item.exists {
    opacity: 0.6;
  }

  .opml-feed-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    flex: 1;
    min-width: 0;
  }

  .opml-feed-title {
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .opml-feed-category {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .opml-feed-exists {
    font-size: 0.75rem;
    color: var(--status-warning);
    white-space: nowrap;
    margin-left: 0.5rem;
  }

  .opml-preview-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-default);
  }

  .opml-preview-info {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* OPML Export */
  .opml-export {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-default);
  }

  .export-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .export-info {
    flex: 1;
  }

  .export-label {
    font-weight: 500;
    color: var(--text-primary);
  }

  .export-desc {
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

  /* Short Content Analysis */
  .short-content-section {
    max-width: 600px;
    margin-bottom: 1.5rem;
  }

  .short-content-row {
    margin-bottom: 0.75rem;
  }

  .short-content-error {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.375rem;
    color: var(--status-error);
    font-size: 0.875rem;
  }

  .short-content-stats {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .stats-header {
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 0.75rem;
  }

  .stats-breakdown {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .stat-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.375rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }

  .stat-item.null-empty {
    background-color: rgba(243, 139, 168, 0.15);
  }

  .stat-item.very-short {
    background-color: rgba(250, 179, 135, 0.15);
  }

  .stat-item.short {
    background-color: rgba(249, 226, 175, 0.15);
  }

  .stat-item.ok {
    background-color: rgba(166, 227, 161, 0.15);
  }

  .stat-label {
    color: var(--text-secondary);
  }

  .stat-value {
    font-weight: 600;
    color: var(--text-primary);
  }

  .refetch-section {
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-default);
  }

  .refetch-section .setting-description {
    margin-bottom: 0.5rem;
  }

  .headless-warning {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: rgba(250, 179, 135, 0.15);
    border-radius: 0.375rem;
    color: var(--status-warning);
    font-size: 0.875rem;
  }

  .btn-refetch {
    margin-top: 0.5rem;
  }

  .btn-action i {
    margin-right: 0.375rem;
  }

  .refetch-result {
    margin-top: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: rgba(166, 227, 161, 0.1);
    border-radius: 0.375rem;
    color: var(--status-success);
    font-size: 0.875rem;
  }
</style>
