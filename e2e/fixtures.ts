import { test as base, expect } from '@playwright/test';

// Mock data for Tauri API calls
export const mockPentacles = [
  {
    id: 1,
    url: 'https://example.com/feed.xml',
    title: 'Test Feed',
    description: 'A test feed',
    unread_count: 5,
    category_id: null,
    fetch_full_text: false,
    created_at: '2024-01-01T00:00:00Z',
  },
];

export const mockFnords = [
  {
    id: 1,
    pentacle_id: 1,
    guid: 'test-guid-1',
    title: 'Test Article 1',
    link: 'https://example.com/article1',
    content: '<p>Test content</p>',
    content_full: null,
    summary: null,
    author: 'Test Author',
    published_at: '2024-01-01T12:00:00Z',
    status: 'concealed',
    has_changes: false,
  },
  {
    id: 2,
    pentacle_id: 1,
    guid: 'test-guid-2',
    title: 'Test Article 2',
    link: 'https://example.com/article2',
    content: '<p>Another test</p>',
    content_full: null,
    summary: null,
    author: 'Test Author',
    published_at: '2024-01-01T13:00:00Z',
    status: 'concealed',
    has_changes: false,
  },
];

export const mockSettings = {
  locale: 'de',
  theme: 'mocha',
  sync_interval: 30,
  show_terminology_tooltips: true,
};

export const mockOllamaStatus = {
  available: true,
  models: ['qwen3-vl:8b', 'nomic-embed-text'],
  recommended_main: 'qwen3-vl:8b',
  recommended_embedding: 'nomic-embed-text',
  has_recommended_main: true,
  has_recommended_embedding: true,
};

// Script to inject Tauri mocks into the page
export const tauriMockScript = `
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => {
      console.log('Mocked invoke:', cmd, args);
      const mocks = {
        'get_pentacles': ${JSON.stringify(mockPentacles)},
        'get_fnords': ${JSON.stringify(mockFnords)},
        'get_fnord': ${JSON.stringify(mockFnords[0])},
        'get_settings': ${JSON.stringify(mockSettings)},
        'get_setting': 'de',
        'check_ollama': ${JSON.stringify(mockOllamaStatus)},
        'get_unprocessed_count': { without_summary: 0, without_analysis: 0 },
        'get_changed_fnords': [],
        'reset_all_changes': undefined,
        'sync_all_feeds': { success: true, total_new: 0, total_updated: 0, results: [] },
        'add_pentacle': ${JSON.stringify(mockPentacles[0])},
        'delete_pentacle': undefined,
        'update_fnord_status': undefined,
        'set_setting': undefined,
      };
      return mocks[cmd] ?? undefined;
    },
    transformCallback: () => {},
  };

  // Mock the event system
  window.__TAURI_INTERNALS__.invoke.transformCallback = (callback) => {
    return callback;
  };
`;

// Extended test fixture with Tauri mocks
export const test = base.extend({
  page: async ({ page }, use) => {
    // Inject Tauri mocks before any navigation
    await page.addInitScript(tauriMockScript);
    await use(page);
  },
});

export { expect };
