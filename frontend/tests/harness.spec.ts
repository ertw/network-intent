import { expect, test as base } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const screenshotDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../test-results/harness-screenshots',
);

const localHost = '127.0.0.1';
const localPort = '4173';

const test = base.extend<{ safety: { assert: () => void } }>({
  safety: [
    async ({ page }, use) => {
      const pageErrors: string[] = [];
      const leaked: string[] = [];

      page.on('pageerror', (error) => {
        pageErrors.push(String(error));
      });

      page.on('request', (request) => {
        const url = request.url();
        if (url.startsWith('data:') || url.startsWith('blob:')) {
          return;
        }
        let parsed: URL;
        try {
          parsed = new URL(url);
        } catch {
          leaked.push(url);
          return;
        }
        const local =
          parsed.hostname === localHost && parsed.port === localPort;
        if (!local) {
          leaked.push(url);
        }
      });

      await use({
        assert: () => {
          expect(pageErrors, pageErrors.join('\n')).toEqual([]);
          expect(leaked, leaked.join('\n')).toEqual([]);
        },
      });

      expect(pageErrors, pageErrors.join('\n')).toEqual([]);
      expect(leaked, leaked.join('\n')).toEqual([]);
    },
    { auto: true },
  ],
});

const heading =
  'Component development fixtures — no live device data or admission decisions';

test('renders the harness heading without leaving the local origin', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Development fixtures' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'harness-example' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'harness-discovery' })).toBeVisible();
  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-home.png'),
    fullPage: true,
  });
});

test('loads the harness-example fixture at both required viewports', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/#/harness-example');
  await expect(
    page.getByRole('heading', {
      name: 'Development fixture only — not live data',
    }),
  ).toBeVisible();
  await expect(page.getByText('DEVELOPMENT FIXTURE — NOT LIVE DATA').first()).toBeVisible();
  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-harness-example.png'),
    fullPage: true,
  });

  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
  await expect(
    page.getByRole('heading', {
      name: 'Development fixture only — not live data',
    }),
  ).toBeVisible();
  const overflowX = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflowX).toBeLessThanOrEqual(1);
  await page.screenshot({
    path: path.join(screenshotDir, 'mobile-390x844-harness-example.png'),
    fullPage: true,
  });
});

test('discovers a second fixture without a shared registry', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/#/harness-discovery');
  await expect(
    page.getByRole('heading', {
      name: 'Second development fixture — glob discovery check',
    }),
  ).toBeVisible();
});

test('unknown fixture routes stay local and show a useful empty state', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/#/not-a-fixture');
  await expect(page.getByRole('heading', { name: 'Unknown fixture' })).toBeVisible();
  await expect(page.getByText('No fixture named not-a-fixture.', { exact: false })).toBeVisible();
  await expect(page.getByText('does not fetch from the network', { exact: false })).toBeVisible();
  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-unknown.png'),
    fullPage: true,
  });

  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#/missing-harness-fixture');
  await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
  await expect(page.getByText('No fixture named missing-harness-fixture.', { exact: false })).toBeVisible();
  await page.screenshot({
    path: path.join(screenshotDir, 'mobile-390x844-unknown.png'),
    fullPage: true,
  });
});

test('keyboard navigation shows a focus ring and opens a fixture', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');

  const target = page.getByRole('link', { name: 'harness-discovery', exact: true });
  const linkCount = await page.getByRole('navigation').getByRole('link').count();
  for (let i = 0; i < linkCount; i++) {
    await page.keyboard.press('Tab');
    if (await target.evaluate(element => element === document.activeElement)) break;
  }
  const focused = page.locator(':focus');
  await expect(focused).toHaveRole('link');
  await expect(focused).toHaveAttribute('href', '#/harness-discovery');

  const outline = await focused.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      width: style.outlineWidth,
      style: style.outlineStyle,
      offset: style.outlineOffset,
    };
  });
  expect(outline.style).not.toBe('none');
  expect(Number.parseFloat(outline.width)).toBeGreaterThanOrEqual(3);

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-keyboard-focus.png'),
    fullPage: true,
  });

  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/#\/harness-discovery$/);
  await expect(
    page.getByRole('heading', {
      name: 'Second development fixture — glob discovery check',
    }),
  ).toBeVisible();
});


test('long and HTML-like unknown routes remain literal and fit the narrow viewport', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const name = '<img src=x onerror=alert(1)>' + 'x'.repeat(240);
  await page.goto('/#/' + encodeURIComponent(name));
  await expect(page.getByRole('heading', { name: 'Unknown fixture' })).toBeVisible();
  await expect(page.locator('main')).toContainText(name);
  await expect(page.locator('main img')).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
});

test('failed local module load shows an error and navigation can recover', async ({ page }) => {
  await page.route('**/features/harness-example/Demo.svelte*', route => route.abort());
  await page.goto('/#/harness-example');
  await expect(page.getByRole('heading', { name: 'Fixture failed to load' })).toBeVisible();
  await page.getByRole('link', { name: 'harness-discovery' }).click();
  await expect(page.getByRole('heading', { name: 'Second development fixture — glob discovery check' })).toBeVisible();
});
