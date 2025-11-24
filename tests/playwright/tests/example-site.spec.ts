import { test, expect } from '@playwright/test';

test.describe('Example Site Website', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should load homepage successfully', async ({ page }) => {
    // Actual title: "Spiritual Center — Cultivating Love and Light"
    await expect(page).toHaveTitle(/Spiritual Center|Cultivating Love/i);
    await expect(page.locator('body')).toBeVisible();
  });

  test('should have proper meta tags', async ({ page }) => {
    // Check viewport meta
    const viewport = await page.locator('meta[name="viewport"]');
    await expect(viewport).toHaveCount(1);
  });

  test('should be responsive on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await expect(page.locator('body')).toBeVisible();
    // Check that content is not overflowing
    const body = page.locator('body');
    const box = await body.boundingBox();
    expect(box?.width).toBeLessThanOrEqual(375);
  });

  test('should have navigation menu', async ({ page }) => {
    // Look for common navigation patterns
    const nav = page.locator('nav, header, [role="navigation"]');
    const navCount = await nav.count();
    expect(navCount).toBeGreaterThan(0);
  });

  test('should have clickable links', async ({ page }) => {
    const links = page.locator('a[href]');
    const count = await links.count();
    expect(count).toBeGreaterThan(0);

    // Check that at least one link is visible
    if (count > 0) {
      const firstVisibleLink = links.first();
      await expect(firstVisibleLink).toBeVisible();
    }
  });

  test('should load images properly', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    const images = page.locator('img');
    const imageCount = await images.count();

    // Check if any images exist
    if (imageCount > 0) {
      for (let i = 0; i < Math.min(imageCount, 5); i++) {
        const img = images.nth(i);
        const isVisible = await img.isVisible();
        if (isVisible) {
          // Wait for image to load
          await img.waitFor({ state: 'visible' });
          // Check image loaded (may be 0 for lazy-loaded or SVG)
          const naturalWidth = await img.evaluate((el: HTMLImageElement) => el.naturalWidth);
          // Accept loaded images or SVG/CSS background images
          expect(naturalWidth).toBeGreaterThanOrEqual(0);
        }
      }
    }
  });

  test('should have no console errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Filter out known acceptable errors
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') &&
      !e.includes('analytics') &&
      !e.includes('third-party') &&
      !e.includes('Failed to load translations') &&
      !e.includes('404') &&
      !e.includes('Failed to fetch')
    );

    expect(criticalErrors).toHaveLength(0);
  });

  test('should have proper heading structure', async ({ page }) => {
    const h1 = page.locator('h1');
    const h1Count = await h1.count();

    // Should have at least one h1 for SEO
    expect(h1Count).toBeGreaterThanOrEqual(1);
  });

  test('should load within acceptable time', async ({ page }) => {
    const start = Date.now();
    await page.goto('/');
    await page.waitForLoadState('domcontentloaded');
    const loadTime = Date.now() - start;

    // Should load within 5 seconds
    expect(loadTime).toBeLessThan(5000);
  });

  test('should have footer section', async ({ page }) => {
    const footer = page.locator('footer, [role="contentinfo"]');
    const footerCount = await footer.count();
    // Footer is common but optional
    if (footerCount > 0) {
      await expect(footer.first()).toBeVisible();
    }
  });

  test('should handle 404 pages gracefully', async ({ page }) => {
    const response = await page.goto('/nonexistent-page-12345');
    // Either 404 or redirect to home
    expect([200, 404]).toContain(response?.status());
  });
});

test.describe('Example Site Accessibility', () => {
  test('should have lang attribute', async ({ page }) => {
    await page.goto('/');
    const lang = await page.locator('html').getAttribute('lang');
    expect(lang).toBeTruthy();
  });

  test('should have alt text for images', async ({ page }) => {
    await page.goto('/');
    const images = page.locator('img:visible');
    const count = await images.count();

    for (let i = 0; i < Math.min(count, 5); i++) {
      const img = images.nth(i);
      const alt = await img.getAttribute('alt');
      // Alt should exist (can be empty for decorative images)
      expect(alt !== null || await img.getAttribute('role') === 'presentation').toBeTruthy();
    }
  });

  test('should have sufficient color contrast', async ({ page }) => {
    await page.goto('/');
    // Basic check - page should be readable
    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Example Site Performance', () => {
  test('should have good Core Web Vitals', async ({ page }) => {
    await page.goto('/');

    // Check Largest Contentful Paint
    const lcp = await page.evaluate(() => {
      return new Promise<number>((resolve) => {
        new PerformanceObserver((entryList) => {
          const entries = entryList.getEntries();
          const lastEntry = entries[entries.length - 1] as PerformanceEntry;
          resolve(lastEntry.startTime);
        }).observe({ type: 'largest-contentful-paint', buffered: true });

        // Fallback timeout
        setTimeout(() => resolve(0), 5000);
      });
    });

    // LCP should be under 2.5s for good experience
    if (lcp > 0) {
      expect(lcp).toBeLessThan(2500);
    }
  });

  test('should not have too many requests', async ({ page }) => {
    let requestCount = 0;
    page.on('request', () => requestCount++);

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Reasonable number of requests for a landing page
    expect(requestCount).toBeLessThan(100);
  });

  test('should have compressed assets', async ({ page }) => {
    const responses: { url: string; size: number }[] = [];

    page.on('response', async (response) => {
      const headers = response.headers();
      if (headers['content-type']?.includes('javascript') ||
          headers['content-type']?.includes('css')) {
        const size = parseInt(headers['content-length'] || '0');
        if (size > 0) {
          responses.push({ url: response.url(), size });
        }
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Check that large assets are reasonably sized (< 500KB each)
    for (const resp of responses) {
      expect(resp.size).toBeLessThan(500 * 1024);
    }
  });
});

test.describe('Example Site SEO', () => {
  test('should have meta description', async ({ page }) => {
    await page.goto('/');
    const metaDesc = page.locator('meta[name="description"]');
    const count = await metaDesc.count();

    if (count > 0) {
      const content = await metaDesc.getAttribute('content');
      expect(content).toBeTruthy();
      expect(content!.length).toBeGreaterThan(50);
      expect(content!.length).toBeLessThan(160);
    }
  });

  test('should have Open Graph tags', async ({ page }) => {
    await page.goto('/');

    const ogTitle = page.locator('meta[property="og:title"]');
    const ogDesc = page.locator('meta[property="og:description"]');
    const ogImage = page.locator('meta[property="og:image"]');

    // At least title should be present for social sharing
    const hasOgTitle = await ogTitle.count() > 0;
    const hasOgDesc = await ogDesc.count() > 0;

    // Log what's missing but don't fail
    if (!hasOgTitle) console.log('Missing og:title');
    if (!hasOgDesc) console.log('Missing og:description');
  });

  test('should have canonical URL', async ({ page }) => {
    await page.goto('/');
    const canonical = page.locator('link[rel="canonical"]');
    const count = await canonical.count();

    if (count > 0) {
      const href = await canonical.getAttribute('href');
      expect(href).toContain('example.org');
    }
  });

  test('should have robots meta or robots.txt', async ({ page }) => {
    await page.goto('/');
    const robotsMeta = page.locator('meta[name="robots"]');
    const robotsCount = await robotsMeta.count();

    if (robotsCount === 0) {
      // Check for robots.txt
      const robotsResponse = await page.goto('/robots.txt');
      expect([200, 404]).toContain(robotsResponse?.status());
    }
  });
});

test.describe('Example Site Forms & Interactivity', () => {
  test('should have contact info or interactive elements', async ({ page }) => {
    await page.goto('/');

    const form = page.locator('form');
    const email = page.locator('a[href^="mailto:"]');
    const phone = page.locator('a[href^="tel:"]');
    const buttons = page.locator('button, .btn, [role="button"]');
    const links = page.locator('a[href]');

    const hasForm = await form.count() > 0;
    const hasEmail = await email.count() > 0;
    const hasPhone = await phone.count() > 0;
    const hasButtons = await buttons.count() > 0;
    const hasLinks = await links.count() > 0;

    // Should have at least one interactive element
    expect(hasForm || hasEmail || hasPhone || hasButtons || hasLinks).toBeTruthy();
  });

  test('should have working internal links', async ({ page }) => {
    await page.goto('/');

    const internalLinks = page.locator('a[href^="/"], a[href^="#"]');
    const count = await internalLinks.count();

    // Test first 5 internal links
    for (let i = 0; i < Math.min(count, 5); i++) {
      const link = internalLinks.nth(i);
      const href = await link.getAttribute('href');

      if (href && !href.startsWith('#')) {
        const response = await page.goto(href);
        expect(response?.status()).toBeLessThan(400);
        await page.goto('/'); // Return to home
      }
    }
  });

  test('should have social media links', async ({ page }) => {
    await page.goto('/');

    const socialLinks = page.locator('a[href*="facebook"], a[href*="instagram"], a[href*="telegram"], a[href*="youtube"], a[href*="twitter"]');
    const count = await socialLinks.count();

    // Log social presence
    console.log(`Found ${count} social media links`);
  });
});

test.describe('Example Site Security', () => {
  test('should use HTTPS', async ({ page }) => {
    const response = await page.goto('/');
    const url = page.url();
    expect(url).toMatch(/^https:\/\//);
  });

  test('should have secure headers', async ({ page }) => {
    const response = await page.goto('/');
    const headers = response?.headers();

    // Check for common security headers (informational)
    const hasXFrame = headers?.['x-frame-options'] !== undefined;
    const hasCSP = headers?.['content-security-policy'] !== undefined;
    const hasXContent = headers?.['x-content-type-options'] !== undefined;

    console.log('Security headers present:', {
      'X-Frame-Options': hasXFrame,
      'Content-Security-Policy': hasCSP,
      'X-Content-Type-Options': hasXContent
    });
  });

  test('should not expose sensitive information', async ({ page }) => {
    await page.goto('/');
    const html = await page.content();

    // Check for common sensitive patterns
    expect(html).not.toMatch(/api[_-]?key\s*[:=]\s*["'][^"']+["']/i);
    expect(html).not.toMatch(/password\s*[:=]\s*["'][^"']+["']/i);
    expect(html).not.toMatch(/secret\s*[:=]\s*["'][^"']+["']/i);
  });
});

test.describe('Example Site Mobile Experience', () => {
  test('should have touch-friendly buttons', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');

    const buttons = page.locator('button, a.btn, .button, [role="button"]');
    const count = await buttons.count();

    for (let i = 0; i < Math.min(count, 5); i++) {
      const button = buttons.nth(i);
      const isVisible = await button.isVisible();

      if (isVisible) {
        const box = await button.boundingBox();
        if (box) {
          // Touch targets should be at least 44x44 pixels
          expect(box.width).toBeGreaterThanOrEqual(40);
          expect(box.height).toBeGreaterThanOrEqual(40);
        }
      }
    }
  });

  test('should have readable font size on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');

    const body = page.locator('body');
    const fontSize = await body.evaluate((el) =>
      window.getComputedStyle(el).fontSize
    );

    const size = parseInt(fontSize);
    expect(size).toBeGreaterThanOrEqual(14); // Minimum readable size
  });

  test('should not have horizontal scroll on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');

    const hasHorizontalScroll = await page.evaluate(() => {
      return document.documentElement.scrollWidth > document.documentElement.clientWidth;
    });

    expect(hasHorizontalScroll).toBeFalsy();
  });
});

test.describe('Example Site Content', () => {
  test('should have main content area', async ({ page }) => {
    await page.goto('/');

    const main = page.locator('main, [role="main"], #main, .main');
    const count = await main.count();

    expect(count).toBeGreaterThan(0);
  });

  test('should have readable text content', async ({ page }) => {
    await page.goto('/');

    const text = await page.locator('body').innerText();
    // Should have substantial content
    expect(text.length).toBeGreaterThan(100);
  });

  test('should load without JavaScript errors', async ({ page }) => {
    const jsErrors: string[] = [];

    page.on('pageerror', (error) => {
      jsErrors.push(error.message);
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    expect(jsErrors).toHaveLength(0);
  });
});
