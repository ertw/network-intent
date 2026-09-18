import { expect, test as base, type Page } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  ALL_KIND_IDS,
  HTML_LIKE,
  LONG_TEXT,
  MISSING_SELECTED_ID,
} from '../src/features/changes-panel/cases';

const captureEvidence = process.env.CAPTURE_GROK_EVIDENCE === '1';
const screenshotDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  // Astra review capture preserves the original Grok evidence.
  process.env.CAPTURE_ASTRA_EVIDENCE === '1'
    ? '../../docs/grok/reports/G08/astra'
    : captureEvidence ? '../../docs/grok/reports/G08' : '../test-results/G08-screenshots',
);

const localHost = '127.0.0.1';
const localPort = '4173';
const heading =
  'Component development fixtures — no live device data or admission decisions';

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
        const local = parsed.origin === `http://${localHost}:${localPort}`;
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

async function openFixture(page: Page) {
  await page.goto('/#/changes-panel');
  await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
  await expect(page.getByText('DEVELOPMENT FIXTURE — NOT LIVE DATA').first()).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Changes panel — synthetic supplied list' }),
  ).toBeVisible();
}

function rowById(page: Page, id: string) {
  return page.locator(`[data-testid="changes-row"][data-change-id="${id}"]`);
}

test('renders supplied kinds in order and shows before/after without mutation controls', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  const rendered = await page.locator('[data-testid="changes-row"]').evaluateAll((elements) =>
    elements.map((element) => ({
      id: element.getAttribute('data-change-id'),
      kind: element.textContent,
    })),
  );
  expect(rendered.map((entry) => entry.id)).toEqual([...ALL_KIND_IDS]);
  await expect(rowById(page, 'added-row')).toContainText('Added');
  await expect(rowById(page, 'removed-row')).toContainText('Removed');
  await expect(rowById(page, 'modified-row')).toContainText('Modified');
  await expect(rowById(page, 'blocked-row')).toContainText('Blocked');

  await expect(page.getByRole('button', { name: /approve|accept|apply|adopt|merge|undo/i })).toHaveCount(
    0,
  );

  await rowById(page, 'added-row').click();
  await expect(page.getByTestId('changes-select-count')).toHaveText('onSelect calls: 1');
  await expect(page.getByTestId('changes-before-empty')).toHaveText('No fields supplied');
  await expect(page.getByTestId('changes-after')).toContainText('synthetic-lab');

  await rowById(page, 'removed-row').click();
  await expect(page.getByTestId('changes-after-empty')).toHaveText('No fields supplied');
  await expect(page.getByTestId('changes-explanation')).toHaveText('No explanation supplied');

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-all-kinds.png'),
    fullPage: true,
  });
});

test('blocked, HTML-like, unknown, and empty field states stay literal', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await rowById(page, 'blocked-row').click();
  const detail = page.getByTestId('changes-detail');
  await expect(detail).toContainText(HTML_LIKE);
  await expect(detail).toContainText('Redacted');
  await expect(detail).toContainText('Unknown');
  await expect(page.getByTestId('changes-panel').locator('img')).toHaveCount(0);
  await expect(page.getByTestId('changes-panel').locator('script')).toHaveCount(0);

  await rowById(page, 'long-row').click();
  await expect(page.getByTestId('changes-explanation')).toContainText(LONG_TEXT.slice(0, 40));

  await page.getByLabel('Synthetic case').selectOption('whitespace-empty');
  await expect(page.getByTestId('changes-detail')).toContainText('Empty value');
  await expect(page.getByTestId('changes-detail')).toContainText('Empty list');
  await expect(page.getByTestId('changes-after').locator('[data-list] li')).toHaveCount(4);
  await expect(page.getByTestId('changes-after')).toContainText('alpha  beta');

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-detail.png'),
    fullPage: true,
  });
});

test('selectedId is controlled and empty or unknown ids invent no detail', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await rowById(page, 'modified-row').click();
  await expect(page.getByTestId('changes-detail-kind')).toHaveText('Modified');

  await page.getByTestId('changes-ignore-select').check();
  await rowById(page, 'added-row').click();
  await expect(page.getByTestId('changes-last-select')).toHaveText('Last onSelect id: added-row');
  await expect(page.getByTestId('changes-detail-kind')).toHaveText('Modified');
  await expect(rowById(page, 'modified-row')).toHaveAttribute('aria-pressed', 'true');

  await page.getByTestId('changes-ignore-select').uncheck();
  await page.getByTestId('changes-missing-id').click();
  await expect(page.getByTestId('changes-detail-kind')).toHaveCount(0);
  await expect(page.getByTestId('changes-detail')).not.toContainText(MISSING_SELECTED_ID);

  await page.getByLabel('Synthetic case').selectOption('empty');
  await expect(page.getByTestId('changes-empty')).toHaveText('No changes supplied');
  await expect(page.locator('[data-testid="changes-row"]')).toHaveCount(0);

  await page.getByRole('button', { name: 'Reset fixture' }).click();
  await expect(page.getByTestId('changes-select-count')).toHaveText('onSelect calls: 0');
});

test('keyboard selection works and before/after stack at 390px', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openFixture(page);

  const target = rowById(page, 'modified-row');
  for (let index = 0; index < 40; index += 1) {
    await page.keyboard.press('Tab');
    if (await target.evaluate((element) => element === document.activeElement)) {
      break;
    }
  }
  await expect(target).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('changes-last-select')).toHaveText('Last onSelect id: modified-row');

  const beforeBox = await page.getByTestId('changes-before').boundingBox();
  const afterBox = await page.getByTestId('changes-after').boundingBox();
  expect(beforeBox && afterBox).toBeTruthy();
  if (beforeBox && afterBox) {
    expect(afterBox.y).toBeGreaterThan(beforeBox.y + beforeBox.height - 1);
  }

  const overflowX = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflowX).toBeLessThanOrEqual(1);

  await page.screenshot({
    path: path.join(screenshotDir, 'mobile-390x844-before-after.png'),
    fullPage: true,
  });
});

test('exact whitespace, duplicate and empty entries remain inspectable', async ({ page }) => {
  await openFixture(page);
  await page.getByLabel('Synthetic case').selectOption('whitespace-empty');
  const after = page.getByTestId('changes-after');
  expect(await after.locator('[data-field-id="spaces"] [data-value]').textContent()).toBe('alpha  beta');
  await expect(after.locator('[data-field-id="spaces"] [data-value]')).toHaveCSS('white-space', 'pre-wrap');
  expect(await after.locator('[data-list] [data-value]').allTextContents()).toEqual(['', 'dup', 'dup', '']);
  expect(await page.getByTestId('changes-before').locator('[data-value]').textContent()).toBe('');
  await page.getByLabel('Synthetic case').selectOption('all-kinds');
  await rowById(page, 'long-row').click();
  const value = page.getByTestId('changes-before').locator('[data-value]');
  expect(await value.textContent()).toBe(LONG_TEXT);
  await value.focus();
  await page.keyboard.press('End');
  expect(await value.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
});
