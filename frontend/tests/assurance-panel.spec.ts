import { expect, test as base, type Page } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  ALL_OUTCOME_ROW_IDS,
  HTML_LIKE_DETAIL,
  HTML_LIKE_LABEL,
  LARGE_GRAPH_VERSION,
  LARGE_PLAN_ID,
  LARGE_ROW_ID,
  LONG_DEPENDENCY_ID,
  MISSING_SELECTED_ID,
} from '../src/features/assurance-panel/fixtures';

const screenshotDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../test-results/G05-screenshots',
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
        const local = parsed.hostname === localHost && parsed.port === localPort;
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

const outcomeExpectations: readonly [string, string][] = [
  [LARGE_ROW_ID, 'Success'],
  ['stale-success', 'Success'],
  ['partial-success', 'Success'],
  ['unknown-freshness-success', 'Success'],
  ['unavailable-completeness-success', 'Success'],
  ['violation', 'Violation'],
  ['timeout', 'Timeout'],
  ['refused', 'Refused'],
  ['unreachable', 'Unreachable'],
  ['dns-nx-domain', 'DNS NXDOMAIN'],
  ['dns-server-failure', 'DNS server failure'],
  ['tls-failure', 'TLS failure'],
  ['malformed-response', 'Malformed response'],
  ['unsupported', 'Unsupported'],
  ['unavailable', 'Unavailable'],
  ['no-observation', 'No observation'],
];

async function openFixture(page: Page) {
  await page.goto('/#/assurance-panel');
  await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
  await expect(page.getByText('DEVELOPMENT FIXTURE — NOT LIVE DATA').first()).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Assurance panel — synthetic observations' }),
  ).toBeVisible();
}

function rowById(page: Page, id: string) {
  return page.locator(`[data-testid="assurance-row"][data-row-id="${id}"]`);
}

test('renders every distinct outcome and keeps large string IDs exact', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await expect(page.getByTestId('assurance-plan-id')).toHaveText(LARGE_PLAN_ID);
  await expect(page.getByTestId('assurance-plan-id')).not.toHaveText('9007199254740992');
  await expect(page.getByTestId('assurance-graph-version')).toHaveText(LARGE_GRAPH_VERSION);

  const renderedIds = await page.locator('[data-testid="assurance-row"]').evaluateAll((elements) =>
    elements.map((element) => element.getAttribute('data-row-id')),
  );
  expect(renderedIds).toEqual([...ALL_OUTCOME_ROW_IDS]);

  for (const [id, label] of outcomeExpectations) {
    await expect(rowById(page, id)).toContainText(label);
  }

  await expect(rowById(page, 'timeout')).not.toContainText('Violation');
  await expect(rowById(page, 'timeout')).not.toContainText('Unsupported');
  await expect(rowById(page, 'unsupported')).not.toContainText('Timeout');
  await expect(rowById(page, 'violation')).not.toContainText('Timeout');

  await expect(rowById(page, LARGE_ROW_ID)).toHaveAttribute('data-positive-accent', 'true');
  await expect(rowById(page, 'stale-success')).toHaveAttribute('data-positive-accent', 'false');
  await expect(rowById(page, 'partial-success')).toHaveAttribute('data-positive-accent', 'false');
  await expect(rowById(page, 'unknown-freshness-success')).toHaveAttribute(
    'data-positive-accent',
    'false',
  );
  await expect(rowById(page, 'unavailable-completeness-success')).toHaveAttribute(
    'data-positive-accent',
    'false',
  );
  await expect(rowById(page, 'stale-success')).toContainText('stale');
  await expect(rowById(page, 'partial-success')).toContainText('partial');
  await expect(rowById(page, 'unknown-freshness-success')).toContainText('unknown');
  await expect(rowById(page, 'unavailable-completeness-success')).toContainText('unavailable');
  await expect(rowById(page, 'stale-success')).toContainText('Success');
  await expect(rowById(page, 'partial-success')).toContainText('Success');

  for (const banned of ['Healthy', 'Admitted', 'Verified deployment', 'Safe to apply']) {
    await expect(page.locator('main')).not.toContainText(banned);
  }

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-all-outcomes.png'),
    fullPage: true,
  });
});

test('controlled selection shows supplied detail without reconciling sources', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  const detail = page.getByTestId('assurance-detail');
  await expect(detail.getByText('Observation source')).toHaveCount(0);

  await rowById(page, 'stale-success').click();
  await expect(page.getByTestId('assurance-select-count')).toHaveText('onSelect calls: 1');
  await expect(page.getByTestId('assurance-last-select-id')).toHaveText(
    'Last onSelect id: stale-success',
  );
  await expect(rowById(page, 'stale-success')).toHaveAttribute('aria-pressed', 'true');

  await expect(detail.getByText('Observation source')).toBeVisible();
  await expect(detail).toContainText('synthetic-probe-adapter-east');
  await expect(detail).toContainText('synthetic-lab-collector-west');
  await expect(detail).toContainText('2026-01-01T00:00:00.000Z');
  await expect(detail).toContainText('2026-01-02T12:34:56.000Z');
  await expect(detail).toContainText('stale');
  await expect(detail).toContainText('complete');
  await expect(detail).toContainText('Success');
  await expect(page.getByText('Healthy', { exact: true })).toHaveCount(0);

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-stale-success-detail.png'),
    fullPage: true,
  });

  await rowById(page, LARGE_ROW_ID).click();
  await expect(page.getByTestId('assurance-last-select-id')).toHaveText(
    `Last onSelect id: ${LARGE_PLAN_ID}`,
  );
  await expect(detail).toContainText(LONG_DEPENDENCY_ID);
  await expect(detail).toContainText('dep-upstream-dns');

  await rowById(page, 'unreachable').click();
  await expect(detail).toContainText('Device-local / no endpoint supplied');

  await rowById(page, 'unknown-freshness-success').click();
  await expect(detail.getByText('Unknown', { exact: true })).toHaveCount(2);
  await expect(detail).toContainText('unknown');

  await rowById(page, 'html-like').click();
  await expect(rowById(page, 'html-like')).toContainText(HTML_LIKE_LABEL);
  await expect(detail).toContainText(HTML_LIKE_DETAIL);
  await expect(page.getByTestId('assurance-panel').locator('img')).toHaveCount(0);
  await expect(page.getByTestId('assurance-panel').locator('script')).toHaveCount(0);
  await expect(page.getByTestId('assurance-rows-frozen')).toHaveText('Source rows frozen: yes');
});

test('selectedId is controlled and unknown ids invent no detail', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await rowById(page, 'violation').click();
  await expect(page.getByTestId('assurance-detail')).toContainText('Observation source');
  await expect(page.getByTestId('assurance-select-count')).toHaveText('onSelect calls: 1');

  await page.getByRole('button', { name: 'Select unknown id' }).click();
  await expect(page.getByTestId('assurance-select-count')).toHaveText('onSelect calls: 1');
  await expect(page.getByTestId('assurance-detail').getByText('Observation source')).toHaveCount(0);
  await expect(page.getByTestId('assurance-detail')).not.toContainText(MISSING_SELECTED_ID);
  await expect(rowById(page, 'violation')).toHaveAttribute('aria-pressed', 'false');

  await page.getByLabel('Synthetic case').selectOption('unknown-selection');
  await expect(page.getByTestId('assurance-detail').getByText('Observation source')).toHaveCount(0);
  await expect(page.locator('[data-testid="assurance-row"]')).toHaveCount(ALL_OUTCOME_ROW_IDS.length);

  await page.getByLabel('Synthetic case').selectOption('empty');
  await expect(page.getByTestId('assurance-empty')).toHaveText('No assurance observations');
  await expect(page.locator('[data-testid="assurance-row"]')).toHaveCount(0);
  await expect(page.getByTestId('assurance-detail').getByText('Observation source')).toHaveCount(0);

  await page.getByLabel('Synthetic case').selectOption('no-plan');
  await expect(page.getByTestId('assurance-plan-id')).toHaveText('No plan');
  await expect(page.getByTestId('assurance-graph-version')).toHaveText('Unknown graph version');
  await expect(page.locator('[data-testid="assurance-row"]')).toHaveCount(ALL_OUTCOME_ROW_IDS.length);

  await page.getByRole('button', { name: 'Reset fixture' }).click();
  await expect(page.getByTestId('assurance-plan-id')).toHaveText(LARGE_PLAN_ID);
  await expect(page.getByTestId('assurance-select-count')).toHaveText('onSelect calls: 0');
  await expect(page.getByTestId('assurance-last-select-id')).toHaveText('Last onSelect id: none');
});

test('keyboard selection is available and the narrow viewport stays readable', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openFixture(page);

  const target = rowById(page, 'stale-success');
  for (let i = 0; i < 40; i += 1) {
    await page.keyboard.press('Tab');
    if (await target.evaluate((element) => element === document.activeElement)) {
      break;
    }
  }
  await expect(target).toBeFocused();

  const outline = await target.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      width: style.outlineWidth,
      style: style.outlineStyle,
      offset: style.outlineOffset,
    };
  });
  expect(outline.style).not.toBe('none');
  expect(Number.parseFloat(outline.width)).toBeGreaterThanOrEqual(3);

  await page.keyboard.press('Enter');
  await expect(page.getByTestId('assurance-last-select-id')).toHaveText(
    'Last onSelect id: stale-success',
  );
  await expect(page.getByTestId('assurance-detail')).toContainText('synthetic-probe-adapter-east');

  await rowById(page, LARGE_ROW_ID).click();
  await expect(page.getByTestId('assurance-detail')).toContainText(LONG_DEPENDENCY_ID);

  const overflowX = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflowX).toBeLessThanOrEqual(1);

  await page.screenshot({
    path: path.join(screenshotDir, 'mobile-390x844-selection.png'),
    fullPage: true,
  });
});
