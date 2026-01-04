import { init, register, getLocaleFromNavigator, locale } from 'svelte-i18n';

import de from './de.json';
import en from './en.json';

// Register available locales with inline imports
register('de', () => Promise.resolve(de));
register('en', () => Promise.resolve(en));

// Get saved locale from localStorage or use browser default
function getInitialLocale(): string {
  if (typeof localStorage !== 'undefined') {
    const saved = localStorage.getItem('fuckupRSS_locale');
    if (saved && (saved === 'de' || saved === 'en')) {
      return saved;
    }
  }
  const browserLocale = getLocaleFromNavigator()?.split('-')[0];
  return browserLocale === 'en' ? 'en' : 'de';
}

// Initialize i18n
init({
  fallbackLocale: 'de',
  initialLocale: getInitialLocale(),
});

// Export function to change locale
export function setLocale(newLocale: string) {
  if (newLocale === 'de' || newLocale === 'en') {
    locale.set(newLocale);
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('fuckupRSS_locale', newLocale);
    }
  }
}

export { locale };
