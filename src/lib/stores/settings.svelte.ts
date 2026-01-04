// Settings store using Svelte 5 runes
// Persists to SQLite database via Tauri

import { invoke } from '@tauri-apps/api/core';

export type Theme = 'mocha' | 'macchiato' | 'frappe' | 'latte';

interface Settings {
  locale: string;
  theme: Theme;
  show_terminology_tooltips: boolean;
}

const THEME_CLASSES: Theme[] = ['mocha', 'macchiato', 'frappe', 'latte'];

class SettingsStore {
  #settings = $state<Settings>({
    locale: 'de',
    theme: 'mocha',
    show_terminology_tooltips: true,
  });
  #initialized = false;

  get showTerminologyTooltips() {
    return this.#settings.show_terminology_tooltips;
  }

  set showTerminologyTooltips(value: boolean) {
    this.#settings.show_terminology_tooltips = value;
    this.#saveSetting('showTerminologyTooltips', value ? 'true' : 'false');
  }

  get theme() {
    return this.#settings.theme;
  }

  set theme(value: Theme) {
    this.#settings.theme = value;
    this.#saveSetting('theme', value);
    this.applyTheme();
  }

  get locale() {
    return this.#settings.locale;
  }

  set locale(value: string) {
    this.#settings.locale = value;
    this.#saveSetting('locale', value);
  }

  async #saveSetting(key: string, value: string) {
    try {
      await invoke('set_setting', { key, value });
    } catch (e) {
      console.error('Failed to save setting:', e);
    }
  }

  applyTheme() {
    if (typeof document !== 'undefined') {
      // Remove all theme classes
      THEME_CLASSES.forEach((cls) => {
        document.documentElement.classList.remove(cls);
      });
      // Add current theme class
      document.documentElement.classList.add(this.#settings.theme);
    }
  }

  // Initialize settings from database
  async init() {
    if (this.#initialized) return;

    try {
      const dbSettings = await invoke<Settings>('get_settings');
      this.#settings.locale = dbSettings.locale;
      this.#settings.theme = (dbSettings.theme as Theme) || 'mocha';
      this.#settings.show_terminology_tooltips = dbSettings.show_terminology_tooltips;
      this.#initialized = true;
    } catch (e) {
      console.error('Failed to load settings:', e);
    }

    this.applyTheme();
  }
}

export const settings = new SettingsStore();
