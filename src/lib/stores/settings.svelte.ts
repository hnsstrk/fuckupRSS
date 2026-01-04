// Settings store using Svelte 5 runes
// Persists to localStorage

export type Theme = 'mocha' | 'macchiato' | 'frappe' | 'latte';

interface Settings {
  showTerminologyTooltips: boolean;
  theme: Theme;
}

const STORAGE_KEY = 'fuckupRSS_settings';
const THEME_CLASSES: Theme[] = ['mocha', 'macchiato', 'frappe', 'latte'];

function loadSettings(): Settings {
  if (typeof localStorage === 'undefined') {
    return { showTerminologyTooltips: true, theme: 'mocha' };
  }
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const parsed = JSON.parse(saved);
      // Migrate old 'dark'/'light' theme values
      if (parsed.theme === 'dark') {
        parsed.theme = 'mocha';
      } else if (parsed.theme === 'light') {
        parsed.theme = 'latte';
      }
      return parsed;
    }
  } catch {
    // Ignore parse errors
  }
  return { showTerminologyTooltips: true, theme: 'mocha' };
}

function saveSettings(settings: Settings) {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  }
}

class SettingsStore {
  #settings = $state<Settings>(loadSettings());

  get showTerminologyTooltips() {
    return this.#settings.showTerminologyTooltips;
  }

  set showTerminologyTooltips(value: boolean) {
    this.#settings.showTerminologyTooltips = value;
    saveSettings(this.#settings);
  }

  get theme() {
    return this.#settings.theme;
  }

  set theme(value: Theme) {
    this.#settings.theme = value;
    saveSettings(this.#settings);
    this.applyTheme();
  }

  applyTheme() {
    if (typeof document !== 'undefined') {
      // Remove all theme classes
      THEME_CLASSES.forEach(cls => {
        document.documentElement.classList.remove(cls);
      });
      // Add current theme class
      document.documentElement.classList.add(this.#settings.theme);
    }
  }

  // Initialize theme on load
  init() {
    this.applyTheme();
  }
}

export const settings = new SettingsStore();
