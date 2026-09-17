import { expect, test as base, type Page } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const screenshotDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../test-results/G04-screenshots',
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

async function openFixture(
  page: Page,
  viewport: { width: number; height: number },
) {
  await page.setViewportSize(viewport);
  await page.goto('/#/snapshot-panel');
  await expect(
    page.getByText('SYNTHETIC DEVELOPMENT FIXTURE — NOT LIVE DATA'),
  ).toBeVisible();
  await expect(page.locator('[data-snapshot-panel]')).toBeVisible();
}

async function selectCase(page: Page, id: string) {
  await page.getByLabel('Synthetic snapshot case').selectOption(id);
}

function panel(page: Page) {
  return page.locator('[data-snapshot-panel]');
}

test('shows committed labels, order, duplicates, and uncoerced scalars', async ({
  page,
}) => {
  await openFixture(page, { width: 1280, height: 800 });
  const root = panel(page);

  await expect(
    page.getByRole('heading', { name: 'Synthetic committed snapshot' }),
  ).toBeVisible();
  await expect(root.locator('[data-datastore-label]')).toHaveText(
    'Committed configuration',
  );
  await expect(root.locator('[data-snapshot-id]')).toHaveText(
    'snap-committed-synthetic-001',
  );
  await expect(root.locator('[data-source]')).toHaveText('Synthetic lab collector');
  await expect(root.locator('[data-collected]')).toHaveText(
    '2026-09-17T16:01:02.000Z',
  );
  await expect(root.locator('[data-received]')).toHaveText(
    '2026-09-17T16:01:03.000Z',
  );
  await expect(root.locator('[data-freshness]')).toHaveAttribute(
    'data-freshness',
    'fresh',
  );
  await expect(root.locator('[data-completeness]')).toHaveAttribute(
    'data-completeness',
    'complete',
  );
  await expect(root.getByText('Applied', { exact: true })).toHaveCount(0);

  const groups = root.locator('[data-group]');
  await expect(groups).toHaveCount(2);
  await expect(groups.nth(0)).toHaveAttribute('data-group-id', 'system');
  await expect(groups.nth(1)).toHaveAttribute('data-group-id', 'network');

  await expect(root.locator('[data-field-id="metric"] [data-value]')).toHaveText(
    '0',
  );
  await expect(root.locator('[data-field-id="disabled"] [data-value]')).toHaveText(
    'false',
  );
  await expect(root.locator('[data-field-id="vlan"] [data-value]')).toHaveText(
    '007',
  );

  const note = root.locator('[data-field-id="note"] [data-value]');
  expect(await note.evaluate((element) => element.textContent)).toBe('alpha  beta');
  expect(await note.evaluate((element) => getComputedStyle(element).whiteSpace)).toBe(
    'pre-wrap',
  );

  const interfaces = root.locator('[data-field-id="interfaces"] [data-value]');
  await expect(interfaces).toHaveText(['eth0', 'eth1', 'eth0', 'br-lan']);

  const tags = root.locator('[data-field-id="tags"] [data-value]');
  await expect(tags).toHaveCount(4);
  expect(await tags.allTextContents()).toEqual(['', 'dup', 'dup', '']);

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-committed.png'),
    fullPage: true,
  });
});

test('distinguishes empty scalar, empty list, unknown, and redacted fields', async ({
  page,
}) => {
  await openFixture(page, { width: 1280, height: 800 });
  await selectCase(page, 'redacted-unknown');
  await expect(
    page.getByRole('heading', { name: 'Synthetic redacted and unknown fields' }),
  ).toBeVisible();

  const root = panel(page);
  const kinds = root.locator('[data-field]');
  await expect(kinds).toHaveCount(4);
  await expect(kinds.nth(0)).toHaveAttribute('data-kind', 'scalar');
  await expect(kinds.nth(1)).toHaveAttribute('data-kind', 'ordered_list');
  await expect(kinds.nth(2)).toHaveAttribute('data-kind', 'redacted');
  await expect(kinds.nth(3)).toHaveAttribute('data-kind', 'unknown');

  const emptyScalar = root.locator('[data-field-id="empty-scalar"]');
  const emptyValue = emptyScalar.locator('[data-value]');
  await expect(emptyValue).toHaveCount(1);
  expect(await emptyValue.evaluate((element) => element.textContent)).toBe('');
  await expect(emptyScalar.getByLabel('Empty value')).toHaveCount(1);
  await expect(emptyScalar.getByText('Empty value', { exact: true })).toBeVisible();
  expect(await emptyScalar.locator('[data-value]').textContent()).toBe('');
  await expect(emptyScalar.getByText('Empty list')).toHaveCount(0);
  await expect(emptyScalar.getByText('Redacted')).toHaveCount(0);

  const emptyList = root.locator('[data-field-id="empty-list"]');
  await expect(emptyList.getByText('Empty list', { exact: true })).toBeVisible();
  await expect(emptyList.locator('[data-value]')).toHaveCount(0);
  await expect(emptyList.getByLabel('Empty value')).toHaveCount(0);
  await expect(emptyList.getByText('Redacted')).toHaveCount(0);

  const redacted = root.locator('[data-field-id="community-string"]');
  await expect(redacted.getByText('Redacted', { exact: true })).toBeVisible();
  await expect(redacted.locator('[data-value]')).toHaveCount(0);
  await expect(redacted.locator('[data-unknown-reason]')).toHaveCount(0);
  await expect(redacted.getByLabel('Empty value')).toHaveCount(0);
  await expect(redacted.getByRole('button')).toHaveCount(0);
  await expect(redacted.locator('[title]')).toHaveCount(0);
  expect(await redacted.getAttribute('title')).toBeNull();
  await expect(redacted).not.toContainText('secret');
  await expect(redacted).not.toContainText('community-value');

  const unknown = root.locator('[data-field-id="uptime"]');
  await expect(unknown.locator('[data-unknown-reason]')).toHaveText(
    'Synthetic collector did not return uptime',
  );
  await expect(unknown.getByText('Redacted', { exact: true })).toHaveCount(0);
  await expect(unknown.getByText('Empty list', { exact: true })).toHaveCount(0);
  await expect(unknown.getByLabel('Empty value')).toHaveCount(0);
  await expect(unknown.locator('[data-value]')).toHaveCount(0);

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-redacted-unknown.png'),
    fullPage: true,
  });
});

test('keeps HTML-like field text literal and does not create nodes from it', async ({
  page,
}) => {
  await openFixture(page, { width: 1280, height: 800 });
  await selectCase(page, 'long-text-html');
  await expect(
    page.getByRole('heading', {
      name: 'Synthetic long text and HTML-like values',
    }),
  ).toBeVisible();

  const root = panel(page);
  const snippet = '<img src="x" onerror="alert(1)">';
  await expect(root).toContainText(snippet);
  await expect(root).toContainText('<script>document.title="pwned"</script>');
  await expect(root.locator('img')).toHaveCount(0);
  await expect(root.locator('script')).toHaveCount(0);
  await expect(
    page.locator('[data-snapshot-panel] img, [data-snapshot-panel] script'),
  ).toHaveCount(0);
});

test('updates from external demo props without mutation controls', async ({
  page,
}) => {
  await openFixture(page, { width: 1280, height: 800 });
  const root = panel(page);

  await expect(root.locator('[data-datastore-label]')).toHaveText(
    'Committed configuration',
  );
  await expect(page.locator('[data-demo-selection-count]')).toHaveText(
    'Fixture selections: 0',
  );

  await selectCase(page, 'operational-complete');
  await expect(
    page.getByRole('heading', { name: 'Synthetic operational snapshot' }),
  ).toBeVisible();
  await expect(root.locator('[data-datastore-label]')).toHaveText(
    'Operational state',
  );
  await expect(root.locator('[data-snapshot-id]')).toHaveText(
    'snap-operational-synthetic-004',
  );
  await expect(root).not.toContainText('synthetic-lab');
  await expect(page.locator('[data-demo-selection-count]')).toHaveText(
    'Fixture selections: 1',
  );

  await expect(root.getByRole('button')).toHaveCount(0);
  await expect(root.getByRole('textbox')).toHaveCount(0);
  await expect(root.getByRole('combobox')).toHaveCount(0);
  await expect(root.locator('input, textarea, select, button')).toHaveCount(0);
  await expect(root.locator('[contenteditable]')).toHaveCount(0);
  await expect(root.getByRole('button', { name: /edit|save|apply|retry/i })).toHaveCount(
    0,
  );

  await page.getByRole('button', { name: 'Reset synthetic case' }).click();
  await expect(
    page.getByRole('heading', { name: 'Synthetic committed snapshot' }),
  ).toBeVisible();
  await expect(root.locator('[data-datastore-label]')).toHaveText(
    'Committed configuration',
  );
  await expect(page.locator('[data-demo-reset-count]')).toHaveText(
    'Fixture resets: 1',
  );
});

test('keeps all four datastore labels distinct and empty-group warnings', async ({
  page,
}) => {
  await openFixture(page, { width: 1280, height: 800 });
  const root = panel(page);

  await selectCase(page, 'session-staging-partial');
  await expect(root.locator('[data-datastore-label]')).toHaveText('Session staging');
  await expect(root.locator('[data-completeness-warning]')).toContainText('Partial');
  await expect(root.locator('[data-missing-item]')).toHaveText([
    'firewall.defaults',
    'dhcp.lan.leasetime',
  ]);
  await expect(root.locator('[data-freshness]')).toHaveAttribute(
    'data-freshness',
    'stale',
  );

  await selectCase(page, 'configuration-in-use-unavailable');
  await expect(root.locator('[data-datastore-label]')).toHaveText(
    'Configuration in use',
  );
  await expect(root.locator('[data-snapshot-id]')).toHaveText('No snapshot');
  await expect(root.locator('[data-collected]')).toHaveText('Unknown');
  await expect(root.locator('[data-received]')).toHaveText('Unknown');
  await expect(root.locator('[data-freshness]')).toHaveAttribute(
    'data-freshness',
    'unknown',
  );
  await expect(root.locator('[data-completeness-warning]')).toContainText(
    'Unavailable',
  );
  await expect(root.locator('[data-completeness-warning]')).toContainText(
    'not proof of no configuration',
  );
  await expect(root.locator('[data-empty-groups]')).toHaveText(
    'No configuration details',
  );
  await expect(root.locator('[data-reason]')).toHaveText(
    'Synthetic collector reported this datastore unavailable',
  );

  await selectCase(page, 'empty-groups-operational-partial');
  await expect(root.locator('[data-datastore-label]')).toHaveText('Operational state');
  await expect(root.locator('[data-empty-groups]')).toHaveText(
    'No operational details',
  );
  await expect(root.locator('[data-completeness-warning]')).toBeVisible();

  await selectCase(page, 'empty-groups-committed-complete');
  await expect(root.locator('[data-empty-groups]')).toHaveText(
    'No configuration details',
  );
  await expect(root.locator('[data-completeness-warning]')).toHaveCount(0);

  await selectCase(page, 'null-snapshot-unknown-time');
  await expect(root.locator('[data-snapshot-id]')).toHaveText('No snapshot');
  await expect(root.locator('[data-collected]')).toHaveText('Unknown');
  await expect(root.locator('[data-received]')).toHaveText('Unknown');
  await expect(root.locator('[data-reason-empty]')).toHaveText('None supplied');

  await expect(root.getByText('Applied', { exact: true })).toHaveCount(0);
});

test('scrolls long values from the keyboard and fits 390x844', async ({ page }) => {
  await openFixture(page, { width: 390, height: 844 });
  await selectCase(page, 'long-text-html');
  await expect(
    page.getByRole('heading', {
      name: 'Synthetic long text and HTML-like values',
    }),
  ).toBeVisible();

  const longValue = panel(page).locator('[data-field-id="long-banner"] [data-value]');
  await expect(longValue).toBeVisible();
  await longValue.focus();
  await expect(longValue).toBeFocused();

  const before = await longValue.evaluate((element) => element.scrollTop);
  await page.keyboard.press('End');
  await page.keyboard.press('PageDown');
  const after = await longValue.evaluate((element) => ({
    scrollTop: element.scrollTop,
    overflowY: getComputedStyle(element).overflowY,
    ellipsis: getComputedStyle(element).textOverflow,
  }));
  expect(after.scrollTop).toBeGreaterThan(before);
  expect(['auto', 'scroll']).toContain(after.overflowY);
  expect(after.ellipsis).not.toBe('ellipsis');

  const overflowX = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflowX).toBeLessThanOrEqual(1);

  await page.screenshot({
    path: path.join(screenshotDir, 'mobile-390x844-long-text.png'),
    fullPage: true,
  });
});


test('modified navigation keys are not consumed by the scroll helper', async ({ page }) => {
  await openFixture(page, { width: 390, height: 844 });
  await selectCase(page, 'long-text-html');
  const value = panel(page).locator('[data-field-id="long-banner"] [data-value]');
  for (const modifier of ['shiftKey', 'ctrlKey', 'metaKey', 'altKey']) {
    const result = await value.evaluate((element, modifier) => {
      element.scrollTop = 0;
      const event = new KeyboardEvent('keydown', {
        key: 'End', bubbles: true, cancelable: true, [modifier]: true,
      });
      element.dispatchEvent(event);
      return { prevented: event.defaultPrevented, top: element.scrollTop };
    }, modifier);
    expect(result).toEqual({ prevented: false, top: 0 });
  }
});
