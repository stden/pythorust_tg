import { test, expect } from '@playwright/test';

test.describe('Example Domain Website', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should load homepage successfully', async ({ page }) => {
    await expect(page).toHaveTitle(/Example Domain/i);
    await expect(page.locator('body')).toBeVisible();
  });

  test('should have proper heading', async ({ page }) => {
    await expect(page.locator('h1')).toContainText('Example Domain');
  });

  test('should have more information link', async ({ page }) => {
    const link = page.locator('a', { hasText: 'More information' }).or(page.locator('a', { hasText: 'Learn more' }));
    await expect(link).toBeVisible();
    await expect(link).toHaveAttribute('href', /iana\.org\/domains\/example/);
  });

  test('should have readable text', async ({ page }) => {
    const text = await page.locator('p').first().innerText();
    expect(text.length).toBeGreaterThan(10);
  });
});
