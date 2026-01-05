import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn().mockResolvedValue(undefined),
}));

// Mock svelte-i18n
vi.mock('svelte-i18n', () => ({
  _: {
    subscribe: vi.fn((callback: (value: (key: string) => string) => void) => {
      callback((key: string) => key);
      return () => {};
    }),
  },
  locale: {
    subscribe: vi.fn((callback: (value: string) => void) => {
      callback('de');
      return () => {};
    }),
    set: vi.fn(),
  },
  init: vi.fn(),
  getLocaleFromNavigator: vi.fn(() => 'de'),
  addMessages: vi.fn(),
}));

// Suppress console.error for cleaner test output (optional)
// vi.spyOn(console, 'error').mockImplementation(() => {});
