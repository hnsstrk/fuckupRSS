// Settings store using Svelte 5 runes
// Persists to localStorage

interface Settings {
  showTerminologyTooltips: boolean;
  theme: 'dark' | 'light';
}

const STORAGE_KEY = 'fuckupRSS_settings';

function loadSettings(): Settings {
  if (typeof localStorage === 'undefined') {
    return { showTerminologyTooltips: true, theme: 'dark' };
  }
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      return JSON.parse(saved);
    }
  } catch {
    // Ignore parse errors
  }
  return { showTerminologyTooltips: true, theme: 'dark' };
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

  set theme(value: 'dark' | 'light') {
    this.#settings.theme = value;
    saveSettings(this.#settings);
    this.applyTheme();
  }

  applyTheme() {
    if (typeof document !== 'undefined') {
      document.documentElement.classList.toggle('light', this.#settings.theme === 'light');
    }
  }

  // Initialize theme on load
  init() {
    this.applyTheme();
  }
}

export const settings = new SettingsStore();
