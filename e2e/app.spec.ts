import { test, expect } from './fixtures';

test.describe('App Layout', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should load the app without errors', async ({ page }) => {
    // Wait for the app to initialize
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
  });

  test('should display the sidebar', async ({ page }) => {
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    // Sidebar should be visible
    const sidebar = page.locator('aside, .sidebar, [class*="sidebar"]').first();
    await expect(sidebar).toBeVisible();
  });

  test('should display the main content area', async ({ page }) => {
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
    // Main content should exist
    await expect(page.locator('.flex-1')).toBeVisible();
  });
});

test.describe('Sidebar Functionality', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
  });

  test('should have a sync button', async ({ page }) => {
    // Look for sync/refresh button by title or class (icon button with refresh functionality)
    const syncButton = page.locator('button.icon-btn').first();
    await expect(syncButton).toBeVisible();
  });

  test('should have a settings button', async ({ page }) => {
    // Settings button
    const settingsButton = page.locator('button[title*="ettings"], button[title*="instellungen"]').first();
    await expect(settingsButton).toBeVisible();
  });

  test('should show add feed form when clicking add button', async ({ page }) => {
    // Find and click the add button
    const addButton = page.locator('button').filter({ hasText: '+' }).first();

    if (await addButton.isVisible()) {
      await addButton.click();
      // Form should appear with URL input
      const urlInput = page.locator('input[type="url"], input[placeholder*="URL"]');
      await expect(urlInput).toBeVisible();
    }
  });
});

test.describe('Settings Dialog', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
  });

  test('should open settings dialog', async ({ page }) => {
    // Find settings button (gear icon)
    const settingsButton = page.locator('button[title*="ettings"], button[title*="instellungen"]').first();

    if (await settingsButton.isVisible()) {
      await settingsButton.click();

      // Dialog should appear
      const dialog = page.locator('dialog[open], [role="dialog"]');
      await expect(dialog).toBeVisible();
    }
  });

  test('should close settings dialog with close button', async ({ page }) => {
    const settingsButton = page.locator('button[title*="ettings"], button[title*="instellungen"]').first();

    if (await settingsButton.isVisible()) {
      await settingsButton.click();

      // Wait for dialog
      const dialog = page.locator('dialog[open], [role="dialog"]');
      await expect(dialog).toBeVisible();

      // Find and click close button
      const closeButton = dialog.locator('button').filter({ hasText: /close|schließen|×/i }).first();
      if (await closeButton.isVisible()) {
        await closeButton.click();
        await expect(dialog).not.toBeVisible();
      }
    }
  });
});

test.describe('Theme Support', () => {
  test('should have theme classes on body', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });

    // Body should have a theme class
    const body = page.locator('body');
    const className = await body.getAttribute('class');
    // Theme class should be applied (mocha, latte, etc.)
    expect(className).toBeTruthy();
  });
});

test.describe('Accessibility', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.app-container')).toBeVisible({ timeout: 10000 });
  });

  test('should have buttons with accessible names', async ({ page }) => {
    const buttons = page.locator('button');
    const count = await buttons.count();

    for (let i = 0; i < Math.min(count, 5); i++) {
      const button = buttons.nth(i);
      if (await button.isVisible()) {
        const ariaLabel = await button.getAttribute('aria-label');
        const title = await button.getAttribute('title');
        const text = await button.innerText();

        // Button should have some accessible name
        const hasAccessibleName = !!(ariaLabel || title || text.trim());
        expect(hasAccessibleName).toBeTruthy();
      }
    }
  });

  test('should have headings', async ({ page }) => {
    // App should have at least one heading
    const headings = page.locator('h1, h2, h3, h4, h5, h6');
    const count = await headings.count();
    expect(count).toBeGreaterThan(0);
  });
});
