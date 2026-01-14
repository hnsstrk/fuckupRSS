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

export const mockUnprocessedCount = {
  total: 5,
  with_content: 3,
};

export const mockSyncResponse = {
  success: true,
  total_new: 5,
  total_updated: 2,
  results: [
    {
      pentacle_id: 1,
      pentacle_title: 'Test Feed',
      new_articles: 5,
      updated_articles: 2,
      full_text_fetched: 5,
      error: null,
    },
  ],
};

// Script to inject Tauri mocks into the page
export const tauriMockScript = `
  // Track event listeners for cleanup
  const eventListeners = new Map();
  let listenerId = 0;

  // Track invoke calls for testing
  window.__INVOKE_CALLS__ = [];

  // Mutable state for dynamic testing
  window.__MOCK_STATE__ = {
    unprocessedCount: { total: 0, with_content: 0 },
    syncCallCount: 0,
  };

  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => {
      console.log('Mocked invoke:', cmd, args);
      // Track all invoke calls
      window.__INVOKE_CALLS__.push({ cmd, args, timestamp: Date.now() });

      const mocks = {
        'get_pentacles': ${JSON.stringify(mockPentacles)},
        'get_fnords': ${JSON.stringify(mockFnords)},
        'get_fnord': ${JSON.stringify(mockFnords[0])},
        'get_settings': ${JSON.stringify(mockSettings)},
        'get_setting': null,
        'check_ollama': ${JSON.stringify(mockOllamaStatus)},
        'get_unprocessed_count': () => window.__MOCK_STATE__.unprocessedCount,
        'get_changed_fnords': [],
        'reset_all_changes': undefined,
        'sync_all_feeds': () => {
          // After sync, simulate new unprocessed articles
          window.__MOCK_STATE__.syncCallCount++;
          window.__MOCK_STATE__.unprocessedCount = { total: 5, with_content: 3 };
          return ${JSON.stringify(mockSyncResponse)};
        },
        'add_pentacle': ${JSON.stringify(mockPentacles[0])},
        'delete_pentacle': undefined,
        'update_fnord_status': undefined,
        'set_setting': undefined,
        'get_prompts': { summary_prompt: 'Test prompt', analysis_prompt: 'Test analysis' },
        'get_default_prompts': { summary_prompt: 'Test prompt', analysis_prompt: 'Test analysis' },
        'get_loaded_models': { models: [] },
        'set_prompts': undefined,
        'calculate_keyword_quality_scores': { updated_count: 0, avg_score: 0, low_quality_count: 0 },
        'ensure_models_loaded': undefined,
        'get_system_theme': 'dark',
      };
      const result = mocks[cmd];
      return typeof result === 'function' ? result() : (result ?? undefined);
    },
    transformCallback: (callback, once) => {
      const id = listenerId++;
      return id;
    },
    metadata: {
      currentWebview: { windowLabel: 'main', label: 'main' },
      currentWindow: { label: 'main' }
    },
  };

  // Mock Tauri event plugin
  window.__TAURI_PLUGIN_EVENT__ = {
    listen: async (event, handler) => {
      const id = listenerId++;
      eventListeners.set(id, { event, handler });
      console.log('Mocked listen:', event, 'id:', id);
      return () => {
        eventListeners.delete(id);
        console.log('Unlistened:', event, 'id:', id);
      };
    },
    emit: async (event, payload) => {
      console.log('Mocked emit:', event, payload);
      for (const [id, listener] of eventListeners) {
        if (listener.event === event) {
          listener.handler({ payload, event, id });
        }
      }
    },
    once: async (event, handler) => {
      const id = listenerId++;
      const wrappedHandler = (e) => {
        handler(e);
        eventListeners.delete(id);
      };
      eventListeners.set(id, { event, handler: wrappedHandler });
      return () => eventListeners.delete(id);
    },
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
