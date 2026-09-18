import { expect, test as base, type Page } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  HTML_LIKE_EDGE,
  HTML_LIKE_LABEL,
  LONG_LABEL,
} from '../src/features/graph-renderer/fixtures';

const captureEvidence = process.env.CAPTURE_GROK_EVIDENCE === '1';
const screenshotDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  // Astra review capture preserves the original Grok evidence.
  process.env.CAPTURE_ASTRA_EVIDENCE === '1'
    ? '../../docs/grok/reports/G07/astra'
    : captureEvidence ? '../../docs/grok/reports/G07' : '../test-results/G07-screenshots',
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
  await page.goto('/#/graph-renderer');
  await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
  await expect(page.getByText('DEVELOPMENT FIXTURE — NOT LIVE DATA').first()).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Graph renderer — synthetic projection' }),
  ).toBeVisible();
}

async function waitForCanvas(page: Page) {
  await expect(page.getByTestId('graph-canvas')).toBeVisible({ timeout: 10_000 });
  await expect(page.locator('[data-testid="graph-canvas-node"]').first()).toBeVisible();
}

test('real ELK and Svelte Flow render the chain with finite positions', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);
  await waitForCanvas(page);

  await expect(page.getByTestId('graph-node-item')).toHaveCount(3);
  await expect(page.getByTestId('graph-edge-item')).toHaveCount(2);
  await expect(page.locator('.svelte-flow__edge')).toHaveCount(2);
  await expect(page.getByTestId('graph-node-list')).toContainText('Chain one');
  await expect(page.getByTestId('graph-node-list')).toContainText('Chain three');
  await expect(page.getByTestId('graph-edge-list')).toContainText('one to two');

  const positions = await page.locator('[data-testid="graph-canvas-node"]').evaluateAll((elements) =>
    elements.map((element) => ({
      id: element.getAttribute('data-node-id'),
      x: Number(element.getAttribute('data-x')),
      y: Number(element.getAttribute('data-y')),
    })),
  );
  expect(positions.map((entry) => entry.id).sort()).toEqual(['n1', 'n2', 'n3']);
  for (const position of positions) {
    expect(Number.isFinite(position.x), `${position.id} x`).toBe(true);
    expect(Number.isFinite(position.y), `${position.id} y`).toBe(true);
  }

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-chain.png'),
    fullPage: true,
  });
});

test('selection is controlled, pan/zoom work, and mutation keys do not edit the graph', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);
  await waitForCanvas(page);

  await page.locator('[data-testid="graph-node-item"][data-node-id="n2"]').click();
  await expect(page.getByTestId('graph-select-count')).toHaveText('onSelect calls: 1');
  await expect(page.getByTestId('graph-last-select')).toHaveText('Last onSelect id: n2');
  await expect(page.locator('[data-testid="graph-node-item"][data-node-id="n2"]')).toHaveAttribute(
    'aria-pressed',
    'true',
  );

  await page.getByTestId('graph-ignore-select').check();
  await page.locator('[data-testid="graph-node-item"][data-node-id="n1"]').click();
  await expect(page.getByTestId('graph-last-select')).toHaveText('Last onSelect id: n1');
  await expect(page.locator('[data-testid="graph-node-item"][data-node-id="n2"]')).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await expect(page.locator('[data-testid="graph-node-item"][data-node-id="n1"]')).toHaveAttribute(
    'aria-pressed',
    'false',
  );

  const viewportBefore = await page.locator('.svelte-flow__viewport').getAttribute('style');
  await page.getByTestId('svelte-flow__controls').locator('button').nth(0).click();
  await expect
    .poll(async () => page.locator('.svelte-flow__viewport').getAttribute('style'))
    .not.toBe(viewportBefore);

  await page.keyboard.press('Delete');
  await page.keyboard.press('Backspace');
  await expect(page.getByTestId('graph-node-item')).toHaveCount(3);
  await expect(page.locator('.svelte-flow__edge')).toHaveCount(2);
});

test('empty, invalid, cycle, parallel, and layer fixtures keep supplied data', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await page.getByLabel('Synthetic case').selectOption('empty');
  await expect(page.getByTestId('graph-empty')).toHaveText('No graph to display');
  await expect(page.getByTestId('graph-canvas')).toHaveCount(0);

  await page.getByLabel('Synthetic case').selectOption('dangling');
  await expect(page.getByTestId('graph-invalid')).toBeVisible();
  await expect(page.getByTestId('graph-invalid-issue')).toContainText('absent');
  await expect(page.getByTestId('graph-edge-item')).toHaveCount(1);

  await page.getByLabel('Synthetic case').selectOption('cycle');
  await waitForCanvas(page);
  await expect(page.locator('.svelte-flow__edge')).toHaveCount(3);

  await page.getByLabel('Synthetic case').selectOption('parallel');
  await waitForCanvas(page);
  await expect(page.locator('.svelte-flow__edge')).toHaveCount(2);
  await expect(page.getByTestId('graph-edge-list')).toContainText('parallel A');
  await expect(page.getByTestId('graph-edge-list')).toContainText('parallel B');

  await page.getByLabel('Synthetic case').selectOption('layers');
  await waitForCanvas(page);
  for (const layer of ['L1', 'L2', 'L3', 'L4', 'L7']) {
    await expect(page.locator(`[data-testid="graph-node-item"][data-layer="${layer}"]`)).toHaveCount(
      1,
    );
  }

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-layers.png'),
    fullPage: true,
  });
});

test('HTML-like labels stay text and the 390px viewport does not overflow', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openFixture(page);
  await page.getByLabel('Synthetic case').selectOption('html-long');
  await waitForCanvas(page);

  await expect(page.getByTestId('graph-node-list')).toContainText(HTML_LIKE_LABEL);
  await expect(page.getByTestId('graph-node-list')).toContainText(LONG_LABEL);
  await expect(page.getByTestId('graph-edge-list')).toContainText(HTML_LIKE_EDGE);
  await expect(page.getByTestId('graph-renderer').locator('img')).toHaveCount(0);
  await expect(page.getByTestId('graph-renderer').locator('script')).toHaveCount(0);

  await page.getByTestId('graph-second').check();
  await expect(page.getByTestId('graph-renderer')).toHaveCount(2);

  const overflowX = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflowX).toBeLessThanOrEqual(1);

  await page.screenshot({
    path: path.join(screenshotDir, 'mobile-390x844-html-long.png'),
    fullPage: true,
  });
});

for (const fixture of ['duplicate-nodes', 'duplicate-edges', 'overlapping']) {
  test(`invalid ${fixture} remains inspectable without a rendering exception`, async ({ page }) => {
    await openFixture(page);
    await page.getByLabel('Synthetic case').selectOption(fixture);
    await expect(page.getByTestId('graph-invalid')).toBeVisible();
    await expect(page.getByTestId('graph-canvas')).toHaveCount(0);
    await page.getByLabel('Synthetic case').selectOption('chain');
    await waitForCanvas(page);
  });
}

test('canvas requests respect retained selection and keyboard selection', async ({ page }) => {
  await openFixture(page);
  await waitForCanvas(page);
  const first = page.locator('.svelte-flow__node[data-id="n1"]');
  const second = page.locator('.svelte-flow__node[data-id="n2"]');
  await page.locator('[data-node-id="n2"][data-testid="graph-node-item"]').click();
  await expect(second).toHaveClass(/selected/);
  await page.getByTestId('graph-ignore-select').check();
  await first.click();
  await expect(page.getByTestId('graph-last-select')).toHaveText('Last onSelect id: n1');
  await expect(first).not.toHaveClass(/selected/);
  await expect(second).toHaveClass(/selected/);
  await page.getByTestId('graph-ignore-select').uncheck();
  await first.focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('[data-node-id="n1"][data-testid="graph-node-item"]')).toHaveAttribute('aria-pressed', 'true');
});

test('mounted layout races, failure, empty/invalid replacement and unmount keep latest input', async ({ page }) => {
  // Only this async race test substitutes the local layout adapter. Other tests use ELK.
  await page.route('**/src/features/graph-renderer/elk-layout.ts', (route) => route.fulfill({
    contentType: 'application/javascript',
    body: `window.pendingLayouts = [];
      export function createBundledLayout() {
        return graph => new Promise((resolve, reject) => {
          window.pendingLayouts.push({ graph, resolve, reject });
        });
      }`,
  }));
  async function settle(index: number, reject = false) {
    await page.evaluate(({ index, reject }) => {
      const pending = (window as unknown as { pendingLayouts: Array<{
        graph: { children: Array<{ id: string; x?: number; y?: number }> };
        resolve: (graph: unknown) => void; reject: (error: Error) => void;
      }> }).pendingLayouts[index]!;
      if (reject) pending.reject(new Error('synthetic current failure'));
      else pending.resolve({ ...pending.graph, children: pending.graph.children.map((node, i) => ({ ...node, x: i * 280, y: 0 })) });
    }, { index, reject });
  }
  async function pendingCount(count: number) {
    await expect.poll(() => page.evaluate(() =>
      (window as unknown as { pendingLayouts: unknown[] }).pendingLayouts.length)).toBe(count);
  }
  await openFixture(page); // A: chain
  await pendingCount(1);
  await page.getByLabel('Synthetic case').selectOption('single'); // B
  await pendingCount(2);
  await settle(1);
  await waitForCanvas(page);
  await settle(0, true); // older rejection must not replace B
  await expect(page.getByTestId('graph-canvas-node')).toHaveAttribute('data-node-id', 'only');
  await page.getByLabel('Synthetic case').selectOption('chain'); // C
  await pendingCount(3);
  await expect(page.getByTestId('graph-canvas')).toHaveCount(0);
  await page.getByLabel('Synthetic case').selectOption('disconnected'); // D
  await pendingCount(4);
  await settle(3);
  await settle(2); // reverse successes
  await expect(page.getByTestId('graph-canvas-node')).toHaveCount(2);
  await page.getByLabel('Synthetic case').selectOption('single');
  await pendingCount(5);
  await settle(4, true);
  await expect(page.getByTestId('graph-error')).toContainText('synthetic current failure');
  await expect(page.getByTestId('graph-canvas')).toHaveCount(0);
  await page.getByLabel('Synthetic case').selectOption('chain');
  await pendingCount(6);
  await page.getByLabel('Synthetic case').selectOption('empty');
  await settle(5);
  await expect(page.getByTestId('graph-empty')).toBeVisible();
  await page.getByLabel('Synthetic case').selectOption('chain');
  await pendingCount(7);
  await page.getByLabel('Synthetic case').selectOption('duplicate-nodes');
  await settle(6);
  await expect(page.getByTestId('graph-invalid')).toBeVisible();
  await expect(page.getByTestId('graph-canvas')).toHaveCount(0);
  await page.getByLabel('Synthetic case').selectOption('chain');
  await pendingCount(8);
  await page.evaluate(() => { location.hash = '/changes-panel'; });
  await expect(page.getByTestId('graph-renderer')).toHaveCount(0);
  await settle(7);
  await expect(page.getByTestId('graph-renderer')).toHaveCount(0);
});

test('canvas pan, edge selection, drag/connect prevention and instance IDs', async ({ page }) => {
  await openFixture(page);
  await waitForCanvas(page);
  const node = page.locator('.svelte-flow__node[data-id="n1"]');
  const before = await node.getAttribute('style');
  const box = (await node.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 40, box.y + box.height / 2 + 30, { steps: 5 });
  await page.mouse.up();
  expect(await node.getAttribute('style')).toBe(before);
  const pane = page.locator('.svelte-flow__pane');
  const paneBox = (await pane.boundingBox())!;
  const transform = await page.locator('.svelte-flow__viewport').getAttribute('style');
  await page.mouse.move(paneBox.x + 30, paneBox.y + 30);
  await page.mouse.down();
  await page.mouse.move(paneBox.x + 75, paneBox.y + 55, { steps: 5 });
  await page.mouse.up();
  await expect.poll(() => page.locator('.svelte-flow__viewport').getAttribute('style')).not.toBe(transform);
  const edge = page.locator('.svelte-flow__edge[data-id="e1"]');
  await edge.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('graph-last-select')).toHaveText('Last onSelect id: e1');
  await expect(edge).toHaveClass(/selected/);
  await page.keyboard.press('Delete');
  await page.keyboard.press('Backspace');
  await expect(page.locator('.svelte-flow__edge')).toHaveCount(2);
  await page.getByTestId('graph-ignore-select').check();
  await edge.press('Escape');
  await expect(edge).toHaveClass(/selected/);
  await expect(page.getByTestId('graph-last-select')).toHaveText('Last onSelect id: null');
  const handles = page.locator('.svelte-flow__handle');
  expect(await handles.evaluateAll((elements) => elements.every((element) => !element.classList.contains('connectable')))).toBe(true);
  await page.getByTestId('graph-second').check();
  await expect(page.getByTestId('graph-canvas')).toHaveCount(2);
  const ids = await page.getByTestId('graph-renderer').locator('[id]').evaluateAll((elements) => elements.map((element) => element.id));
  expect(new Set(ids).size).toBe(ids.length);
});
