import { init, register, getLocaleFromNavigator, locale } from "svelte-i18n";
import { invoke } from "@tauri-apps/api/core";

import de from "./de.json";
import en from "./en.json";

// Register available locales with inline imports
register("de", () => Promise.resolve(de));
register("en", () => Promise.resolve(en));

// Get initial locale from browser (DB will override later)
function getInitialLocale(): string {
  const browserLocale = getLocaleFromNavigator()?.split("-")[0];
  return browserLocale === "en" ? "en" : "de";
}

// Initialize i18n with browser default first
init({
  fallbackLocale: "de",
  initialLocale: getInitialLocale(),
});

// Load locale from database and apply it
export async function initLocaleFromDb() {
  try {
    const savedLocale = await invoke<string | null>("get_setting", { key: "locale" });
    if (savedLocale && (savedLocale === "de" || savedLocale === "en")) {
      locale.set(savedLocale);
    }
  } catch (e) {
    console.error("Failed to load locale from DB:", e);
  }
}

// Export function to change locale (saves to DB)
export async function setLocale(newLocale: string) {
  if (newLocale === "de" || newLocale === "en") {
    locale.set(newLocale);
    try {
      await invoke("set_setting", { key: "locale", value: newLocale });
    } catch (e) {
      console.error("Failed to save locale to DB:", e);
    }
  }
}

export { locale };
