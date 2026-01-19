// Settings store using Svelte 5 runes
// Persists to SQLite database via Tauri

import { invoke } from '@tauri-apps/api/core';
import { log, type LogLevel } from '../logger';

// Theme mode: light, dark, or follow system
export type ThemeMode = 'light' | 'dark' | 'system';

// Dark themes (user can choose one when mode is 'dark' or 'system' with dark OS)
export type DarkTheme =
  // Catppuccin
  | 'mocha'
  | 'macchiato'
  | 'frappe'
  // Ayu
  | 'ayu-dark'
  | 'ayu-mirage'
  // Gruvbox
  | 'gruvbox-dark'
  // Tokyo Night
  | 'tokyo-night'
  | 'tokyo-storm'
  // Solarized
  | 'solarized-dark'
  // Dracula
  | 'dracula'
  // Nord
  | 'nord'
  // Everforest
  | 'everforest';

// Light themes (user can choose one when mode is 'light' or 'system' with light OS)
export type LightTheme =
  // Catppuccin
  | 'latte'
  // Ayu
  | 'ayu-light'
  // Gruvbox
  | 'gruvbox-light'
  // Tokyo Night
  | 'tokyo-day'
  // Solarized
  | 'solarized-light';

// All available themes
export type Theme = DarkTheme | LightTheme;

interface Settings {
  locale: string;
  themeMode: ThemeMode; // 'light', 'dark', or 'system'
  darkTheme: DarkTheme; // The dark theme preference (used when mode is dark or system-dark)
  lightTheme: LightTheme; // The light theme preference (used when mode is light or system-light)
  show_terminology_tooltips: boolean;
  sync_interval: number; // in minutes
  sync_on_start: boolean;
  log_level: LogLevel;
  embedding_model: string; // Embedding model for keyword similarity
  enable_headless_browser: boolean; // Use headless browser for JavaScript-rendered content
}

const THEME_CLASSES: Theme[] = [
  // Catppuccin
  'mocha',
  'macchiato',
  'frappe',
  'latte',
  // Ayu
  'ayu-dark',
  'ayu-mirage',
  'ayu-light',
  // Gruvbox
  'gruvbox-dark',
  'gruvbox-light',
  // Tokyo Night
  'tokyo-night',
  'tokyo-storm',
  'tokyo-day',
  // Solarized
  'solarized-dark',
  'solarized-light',
  // Dracula
  'dracula',
  // Nord
  'nord',
  // Everforest
  'everforest',
];

const DARK_THEMES: DarkTheme[] = [
  'mocha',
  'macchiato',
  'frappe',
  'ayu-dark',
  'ayu-mirage',
  'gruvbox-dark',
  'tokyo-night',
  'tokyo-storm',
  'solarized-dark',
  'dracula',
  'nord',
  'everforest',
];

const LIGHT_THEMES: LightTheme[] = [
  'latte',
  'ayu-light',
  'gruvbox-light',
  'tokyo-day',
  'solarized-light',
];

const THEME_MODES: ThemeMode[] = ['light', 'dark', 'system'];

const LOG_LEVELS: LogLevel[] = ['error', 'warn', 'info', 'debug', 'trace'];

// Default embedding model (matches Rust RECOMMENDED_EMBEDDING_MODEL)
const DEFAULT_EMBEDDING_MODEL = 'snowflake-arctic-embed2';

class SettingsStore {
  #settings = $state<Settings>({
    locale: 'de',
    themeMode: 'system',
    darkTheme: 'mocha',
    lightTheme: 'latte',
    show_terminology_tooltips: true,
    sync_interval: 30,
    sync_on_start: true,
    log_level: import.meta.env.DEV ? 'debug' : 'info',
    embedding_model: DEFAULT_EMBEDDING_MODEL,
    enable_headless_browser: false,
  });
  #initialized = false;
  // Default to dark - will be updated by Tauri command on init
  #systemDark = $state(true);
  #mediaQuery: MediaQueryList | null = null;

  constructor() {
    // Apply default dark theme immediately to prevent flash of light theme
    if (typeof document !== 'undefined') {
      document.documentElement.classList.add('mocha');
    }
  }

  get showTerminologyTooltips() {
    return this.#settings.show_terminology_tooltips;
  }

  set showTerminologyTooltips(value: boolean) {
    this.#settings.show_terminology_tooltips = value;
    this.#saveSetting('showTerminologyTooltips', value ? 'true' : 'false');
  }

  // Theme mode: 'light', 'dark', or 'system'
  get themeMode(): ThemeMode {
    return this.#settings.themeMode;
  }

  set themeMode(value: ThemeMode) {
    this.#settings.themeMode = value;
    this.#saveSetting('theme_mode', value);
    this.applyTheme();
  }

  // The dark theme preference (for when mode is dark or system-dark)
  get darkTheme(): DarkTheme {
    return this.#settings.darkTheme;
  }

  set darkTheme(value: DarkTheme) {
    this.#settings.darkTheme = value;
    this.#saveSetting('dark_theme', value);
    this.applyTheme();
  }

  // The light theme preference (for when mode is light or system-light)
  get lightTheme(): LightTheme {
    return this.#settings.lightTheme;
  }

  set lightTheme(value: LightTheme) {
    this.#settings.lightTheme = value;
    this.#saveSetting('light_theme', value);
    this.applyTheme();
  }

  // Returns the currently active theme based on mode and system preference
  get theme(): Theme {
    if (this.#settings.themeMode === 'dark') {
      return this.#settings.darkTheme;
    } else if (this.#settings.themeMode === 'light') {
      return this.#settings.lightTheme;
    } else {
      // system mode
      return this.#systemDark ? this.#settings.darkTheme : this.#settings.lightTheme;
    }
  }

  // Setter for theme - auto-detects if dark or light and sets appropriate
  set theme(value: Theme) {
    if (DARK_THEMES.includes(value as DarkTheme)) {
      this.darkTheme = value as DarkTheme;
    } else if (LIGHT_THEMES.includes(value as LightTheme)) {
      this.lightTheme = value as LightTheme;
    }
  }

  // Whether the system prefers dark mode
  get systemPrefersDark(): boolean {
    return this.#systemDark;
  }

  // Available theme lists for UI
  get availableDarkThemes(): DarkTheme[] {
    return DARK_THEMES;
  }

  get availableLightThemes(): LightTheme[] {
    return LIGHT_THEMES;
  }

  get availableThemeModes(): ThemeMode[] {
    return THEME_MODES;
  }

  get locale() {
    return this.#settings.locale;
  }

  set locale(value: string) {
    this.#settings.locale = value;
    this.#saveSetting('locale', value);
  }

  get syncInterval() {
    return this.#settings.sync_interval;
  }

  set syncInterval(value: number) {
    this.#settings.sync_interval = value;
    this.#saveSetting('syncInterval', String(value));
  }

  get syncOnStart() {
    return this.#settings.sync_on_start;
  }

  set syncOnStart(value: boolean) {
    this.#settings.sync_on_start = value;
    this.#saveSetting('syncOnStart', value ? 'true' : 'false');
  }

  get logLevel(): LogLevel {
    return this.#settings.log_level;
  }

  set logLevel(value: LogLevel) {
    this.#settings.log_level = value;
    log.setLevel(value);
    this.#saveSetting('logLevel', value);
  }

  get availableLogLevels(): LogLevel[] {
    return LOG_LEVELS;
  }

  get embeddingModel(): string {
    return this.#settings.embedding_model;
  }

  set embeddingModel(value: string) {
    this.#settings.embedding_model = value;
    this.#saveSetting('embedding_model', value);
  }

  get defaultEmbeddingModel(): string {
    return DEFAULT_EMBEDDING_MODEL;
  }

  get enableHeadlessBrowser(): boolean {
    return this.#settings.enable_headless_browser;
  }

  set enableHeadlessBrowser(value: boolean) {
    this.#settings.enable_headless_browser = value;
    this.#saveSetting('enable_headless_browser', value ? 'true' : 'false');
  }

  async #saveSetting(key: string, value: string) {
    try {
      await invoke('set_setting', { key, value });
      log.debug(`Setting saved: ${key}=${value}`);
    } catch (e) {
      log.error('Failed to save setting:', e);
    }
  }

  applyTheme() {
    if (typeof document !== 'undefined') {
      // Remove all theme classes
      THEME_CLASSES.forEach((cls) => {
        document.documentElement.classList.remove(cls);
      });

      // Determine active theme based on mode
      let activeTheme: Theme;
      if (this.#settings.themeMode === 'dark') {
        activeTheme = this.#settings.darkTheme;
      } else if (this.#settings.themeMode === 'light') {
        activeTheme = this.#settings.lightTheme;
      } else {
        // system mode - follow OS preference
        activeTheme = this.#systemDark ? this.#settings.darkTheme : this.#settings.lightTheme;
      }

      document.documentElement.classList.add(activeTheme);
    }
  }

  #handleSystemThemeChange = async (_e: MediaQueryListEvent | MediaQueryList) => {
    // Re-detect using Tauri command (more reliable on Linux/Wayland)
    // Don't trust the media query event value directly
    this.#systemDark = await this.#detectSystemTheme();
    this.applyTheme();
  };

  async #detectSystemTheme(): Promise<boolean> {
    try {
      // Use Tauri command to detect system theme (more reliable on Linux/Wayland)
      const systemTheme = await invoke<string>('get_system_theme');
      return systemTheme === 'dark';
    } catch {
      // Fallback to CSS media query if Tauri command fails
      if (typeof window !== 'undefined') {
        return window.matchMedia('(prefers-color-scheme: dark)').matches;
      }
      return true; // Default to dark
    }
  }

  #setupSystemThemeListener() {
    if (typeof window === 'undefined') return;

    // Setup CSS media query listener as fallback and for real-time updates
    this.#mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    // Listen for system theme changes (may work on some systems)
    this.#mediaQuery.addEventListener('change', this.#handleSystemThemeChange);
  }

  // Initialize settings from database
  async init() {
    if (this.#initialized) return;

    // Setup system theme listener for real-time updates
    this.#setupSystemThemeListener();

    // Detect system theme using Tauri command (more reliable on Linux/Wayland)
    this.#systemDark = await this.#detectSystemTheme();

    try {
      const dbSettings = await invoke<Record<string, unknown>>('get_settings');
      this.#settings.locale = (dbSettings.locale as string) || 'de';

      // Load theme mode (default: 'system')
      const savedThemeMode = dbSettings.theme_mode as string;
      if (savedThemeMode && THEME_MODES.includes(savedThemeMode as ThemeMode)) {
        this.#settings.themeMode = savedThemeMode as ThemeMode;
      } else {
        this.#settings.themeMode = 'system';
      }

      // Load dark theme preference
      // Try new key first, fall back to legacy 'theme' key for backward compatibility
      const savedDarkTheme = (dbSettings.dark_theme as string) || (dbSettings.theme as string);
      if (savedDarkTheme && DARK_THEMES.includes(savedDarkTheme as DarkTheme)) {
        this.#settings.darkTheme = savedDarkTheme as DarkTheme;
      } else {
        this.#settings.darkTheme = 'mocha';
      }

      // Load light theme preference (default: 'latte')
      const savedLightTheme = dbSettings.light_theme as string;
      if (savedLightTheme && LIGHT_THEMES.includes(savedLightTheme as LightTheme)) {
        this.#settings.lightTheme = savedLightTheme as LightTheme;
      } else {
        this.#settings.lightTheme = 'latte';
      }

      this.#settings.show_terminology_tooltips = (dbSettings.showTerminologyTooltips as boolean) ?? true;
      this.#settings.sync_interval = (dbSettings.syncInterval as number) ?? 30;
      this.#settings.sync_on_start = (dbSettings.syncOnStart as boolean) ?? true;

      // Load log level (default based on environment)
      const savedLogLevel = dbSettings.logLevel as string;
      if (savedLogLevel && LOG_LEVELS.includes(savedLogLevel as LogLevel)) {
        this.#settings.log_level = savedLogLevel as LogLevel;
      }
      // Apply log level to logger
      log.setLevel(this.#settings.log_level);

      // Load embedding model
      const savedEmbeddingModel = dbSettings.embedding_model as string;
      if (savedEmbeddingModel) {
        this.#settings.embedding_model = savedEmbeddingModel;
      }

      // Load headless browser setting
      this.#settings.enable_headless_browser = (dbSettings.enable_headless_browser as boolean) ?? false;

      log.info('Settings loaded successfully');

      this.#initialized = true;
    } catch (e) {
      log.error('Failed to load settings:', e);
    }

    this.applyTheme();
  }

  // Cleanup method (call on app unmount if needed)
  destroy() {
    if (this.#mediaQuery) {
      this.#mediaQuery.removeEventListener('change', this.#handleSystemThemeChange);
    }
  }
}

export const settings = new SettingsStore();
