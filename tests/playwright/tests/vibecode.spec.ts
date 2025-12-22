import { test, expect } from '@playwright/test';

test.describe('Vibecode Hara Center Website', () => {
  test('should load the Vibecode homepage and display correct content', async ({ page }) => {
    await page.goto('https://vibecode.hara.center/');

    // Assert page title
    await expect(page).toHaveTitle('Vibecode - Hara Center');

    // Assert main heading
    await expect(page.locator('header #branding h1')).toContainText('Vibecode - Hara Center');

    // Assert main content heading
    await expect(page.locator('.main-content h2')).toContainText('Welcome to Vibecode at Hara Center');

    // Assert footer text
    await expect(page.locator('footer p')).toContainText('Vibecode © 2023 Hara Center. All rights reserved.');
  });
});
