// Settings store using Svelte 5 runes
// Persists to SQLite database via Tauri

import { invoke } from '@tauri-apps/api/core';
import { log, type LogLevel } from '../logger';

// Dark themes (user can choose one)
export type DarkTheme = 'mocha' | 'macchiato' | 'frappe';
// All available themes
export type Theme = DarkTheme | 'latte';

interface Settings {
  locale: string;
  darkTheme: DarkTheme; // The dark theme preference (used when system is dark)
  show_terminology_tooltips: boolean;
  sync_interval: number; // in minutes
  sync_on_start: boolean;
  log_level: LogLevel;
}

const THEME_CLASSES: Theme[] = ['mocha', 'macchiato', 'frappe', 'latte'];
const DARK_THEMES: DarkTheme[] = ['mocha', 'macchiato', 'frappe'];

const LOG_LEVELS: LogLevel[] = ['error', 'warn', 'info', 'debug', 'trace'];

class SettingsStore {
  #settings = $state<Settings>({
    locale: 'de',
    darkTheme: 'mocha',
    show_terminology_tooltips: true,
    sync_interval: 30,
    sync_on_start: true,
    log_level: import.meta.env.DEV ? 'debug' : 'info',
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

  // The dark theme preference (for when system is in dark mode)
  get darkTheme(): DarkTheme {
    return this.#settings.darkTheme;
  }

  set darkTheme(value: DarkTheme) {
    this.#settings.darkTheme = value;
    this.#saveSetting('theme', value);
    this.applyTheme();
  }

  // Legacy getter for backward compatibility - returns the currently active theme
  get theme(): Theme {
    return this.#systemDark ? this.#settings.darkTheme : 'latte';
  }

  // Legacy setter - maps to darkTheme if it's a dark theme
  set theme(value: Theme) {
    if (DARK_THEMES.includes(value as DarkTheme)) {
      this.darkTheme = value as DarkTheme;
    }
    // Ignore 'latte' - it's only used automatically when system is light
  }

  // Whether the system prefers dark mode
  get systemPrefersDark(): boolean {
    return this.#systemDark;
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
      // Apply theme based on system preference
      const activeTheme = this.#systemDark ? this.#settings.darkTheme : 'latte';
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
      // Load the dark theme preference (stored as 'theme' in DB for backward compatibility)
      const savedTheme = dbSettings.theme as string;
      if (savedTheme && DARK_THEMES.includes(savedTheme as DarkTheme)) {
        this.#settings.darkTheme = savedTheme as DarkTheme;
      } else {
        this.#settings.darkTheme = 'mocha';
      }
      this.#settings.show_terminology_tooltips = (dbSettings.show_terminology_tooltips as boolean) ?? true;
      this.#settings.sync_interval = (dbSettings.sync_interval as number) ?? 30;
      this.#settings.sync_on_start = (dbSettings.sync_on_start as boolean) ?? true;

      // Load log level (default based on environment)
      const savedLogLevel = dbSettings.log_level as string;
      if (savedLogLevel && LOG_LEVELS.includes(savedLogLevel as LogLevel)) {
        this.#settings.log_level = savedLogLevel as LogLevel;
      }
      // Apply log level to logger
      log.setLevel(this.#settings.log_level);
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
