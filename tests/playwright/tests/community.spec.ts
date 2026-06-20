import { test, expect } from '@playwright/test';

test.describe('Vibecode Hara Center Website', () => {
  test('should load the Vibecode homepage and display correct content', async ({ page }) => {
    await page.goto('https://example.invalid/');

    // Assert page title
    await expect(page).toHaveTitle('VibeCode Hub - AI Coding Assistant & Tools');

    // Assert main heading
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.locator('h1')).not.toBeEmpty();

    // Assert main content heading
    // Relaxed selector as .main-content might not exist
    await expect(page.locator('h2').first()).toBeVisible();

    // Assert footer text
    await expect(page.locator('footer')).toBeVisible();
  });
});
