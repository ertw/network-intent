import { expect, test as base, type Locator, type Page } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { ViewSelection } from '../src/contracts/presentation';

const screenshotDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../../docs/grok/reports/G03',
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

async function openViewControls(
  page: Page,
  viewport: { width: number; height: number },
) {
  await page.setViewportSize(viewport);
  await page.goto('/#/view-controls');
  await expect(
    page.getByRole('heading', {
      name: 'Development fixture only — not live data',
    }),
  ).toBeVisible();
  await expect(
    page.getByText('DEVELOPMENT FIXTURE — NOT LIVE DATA').first(),
  ).toBeVisible();
  await expect(page.getByText('SYNTHETIC', { exact: false }).first()).toBeVisible();
  await expect(page.getByRole('region', { name: 'View controls' })).toBeVisible();
}

async function expandAdvanced(page: Page) {
  await page.locator('summary').filter({ hasText: /^Advanced$/ }).click();
  await expect(page.getByRole('group', { name: 'Overlays' })).toBeVisible();
  await expect(
    page.getByRole('switch', { name: 'Combined overview' }),
  ).toBeVisible();
}

async function readJsonRegion(page: Page, name: string): Promise<unknown> {
  const text = (await page.getByRole('region', { name }).innerText()).trim();
  if (text === 'None') {
    return null;
  }
  return JSON.parse(text);
}

async function currentSelection(page: Page): Promise<ViewSelection> {
  return (await readJsonRegion(page, 'Current view selection')) as ViewSelection;
}

async function lastEvent(page: Page): Promise<ViewSelection | null> {
  return (await readJsonRegion(page, 'Last emitted view selection')) as
    | ViewSelection
    | null;
}

async function eventCount(page: Page): Promise<number> {
  return Number(
    (await page.getByRole('status', { name: 'Emitted event count' }).innerText()).trim(),
  );
}

async function tabUntilFocused(page: Page, locator: Locator, limit = 60) {
  for (let i = 0; i < limit; i++) {
    await page.keyboard.press('Tab');
    if (await locator.evaluate((element) => element === document.activeElement)) {
      return;
    }
  }
  throw new Error('Could not tab to the target control');
}

function primaryRadios(page: Page) {
  return page.getByRole('group', { name: 'Primary layer' }).getByRole('radio');
}

function overlayCheckboxes(page: Page) {
  return page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox');
}

test('renders five primary layers and no L5/L6', async ({ page }) => {
  await openViewControls(page, { width: 1280, height: 800 });

  await expect(primaryRadios(page)).toHaveCount(5);
  await expect(primaryRadios(page).nth(0)).toHaveAccessibleName('L1');
  await expect(primaryRadios(page).nth(1)).toHaveAccessibleName('L2');
  await expect(primaryRadios(page).nth(2)).toHaveAccessibleName('L3');
  await expect(primaryRadios(page).nth(3)).toHaveAccessibleName('L4');
  await expect(primaryRadios(page).nth(4)).toHaveAccessibleName('L7');
  await expect(primaryRadios(page).nth(2)).toBeChecked();

  await expect(page.getByRole('radio', { name: 'L5' })).toHaveCount(0);
  await expect(page.getByRole('radio', { name: 'L6' })).toHaveCount(0);
  await expect(page.getByRole('checkbox', { name: 'L5' })).toHaveCount(0);
  await expect(page.getByRole('checkbox', { name: 'L6' })).toHaveCount(0);

  await expect(page.getByRole('radio', { name: 'Intent' })).toBeChecked();
  await expect(page.getByRole('radio', { name: 'View' })).toBeChecked();
  await expect(
    page.getByRole('switch', { name: 'Assurance overlay' }),
  ).not.toBeChecked();
  await expect(page.getByRole('group', { name: 'Overlays' })).toHaveCount(0);
  await expect(page.getByRole('switch', { name: 'Combined overview' })).toHaveCount(0);
  await expect(page.getByRole('status', { name: 'Emitted event count' })).toHaveText(
    '0',
  );

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-default.png'),
    fullPage: true,
  });
});

test('overlay exclusion, canonical order, and primary change', async ({ page }) => {
  await openViewControls(page, { width: 1280, height: 800 });
  await page.getByRole('button', { name: 'Load overlay ordering case' }).click();
  await expandAdvanced(page);

  await expect(overlayCheckboxes(page)).toHaveCount(4);
  await expect(overlayCheckboxes(page).nth(0)).toHaveAccessibleName('L1');
  await expect(overlayCheckboxes(page).nth(1)).toHaveAccessibleName('L2');
  await expect(overlayCheckboxes(page).nth(2)).toHaveAccessibleName('L4');
  await expect(overlayCheckboxes(page).nth(3)).toHaveAccessibleName('L7');
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L3' }),
  ).toHaveCount(0);

  await expect(overlayCheckboxes(page).nth(0)).toBeChecked();
  await expect(overlayCheckboxes(page).nth(1)).toBeChecked();
  await expect(overlayCheckboxes(page).nth(2)).not.toBeChecked();
  await expect(overlayCheckboxes(page).nth(3)).not.toBeChecked();
  await expect(await eventCount(page)).toBe(0);

  await page
    .getByRole('group', { name: 'Overlays' })
    .getByRole('checkbox', { name: 'L4' })
    .click();
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L3',
    overlays: ['L1', 'L2', 'L4'],
    combinedOverview: false,
    stateView: 'intent',
    assuranceOverlay: false,
    editMode: 'view',
  });

  await page
    .getByRole('group', { name: 'Primary layer' })
    .getByRole('radio', { name: 'L1' })
    .click();
  const afterPrimary = await lastEvent(page);
  expect(afterPrimary?.primaryLayer).toBe('L1');
  expect(afterPrimary?.overlays).toEqual(['L2', 'L4']);
  expect(afterPrimary?.overlays).not.toContain('L1');
  expect(afterPrimary?.overlays).not.toContain('L3');

  await expect(overlayCheckboxes(page)).toHaveCount(4);
  await expect(overlayCheckboxes(page).nth(0)).toHaveAccessibleName('L2');
  await expect(overlayCheckboxes(page).nth(1)).toHaveAccessibleName('L3');
  await expect(overlayCheckboxes(page).nth(2)).toHaveAccessibleName('L4');
  await expect(overlayCheckboxes(page).nth(3)).toHaveAccessibleName('L7');
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L2' }),
  ).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L4' }),
  ).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L3' }),
  ).not.toBeChecked();

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-advanced.png'),
    fullPage: true,
  });
});

test('combined overview retains and restores layer choices', async ({ page }) => {
  await openViewControls(page, { width: 1280, height: 800 });
  await page.getByRole('button', { name: 'Load combined overview case' }).click();
  await expandAdvanced(page);

  await expect(
    page.getByRole('group', { name: 'Primary layer' }).getByRole('radio', { name: 'L2' }),
  ).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L1' }),
  ).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L7' }),
  ).toBeChecked();

  await page.getByRole('switch', { name: 'Combined overview' }).click();
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L2',
    overlays: ['L1', 'L7'],
    combinedOverview: true,
  });

  await expect(
    page.getByRole('group', { name: 'Primary layer' }).getByRole('radio', { name: 'L1' }),
  ).toBeDisabled();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L1' }),
  ).toBeDisabled();
  await expect(
    page.getByRole('switch', { name: 'Combined overview' }),
  ).toBeEnabled();

  const countWhileLocked = await eventCount(page);
  await page
    .getByRole('group', { name: 'Primary layer' })
    .getByRole('radio', { name: 'L4' })
    .click({ force: true });
  await expect(await eventCount(page)).toBe(countWhileLocked);
  await expect(await currentSelection(page)).toMatchObject({
    primaryLayer: 'L2',
    overlays: ['L1', 'L7'],
    combinedOverview: true,
  });

  await page.getByRole('switch', { name: 'Combined overview' }).click();
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L2',
    overlays: ['L1', 'L7'],
    combinedOverview: false,
  });
  await expect(
    page.getByRole('group', { name: 'Primary layer' }).getByRole('radio', { name: 'L2' }),
  ).toBeEnabled();
  await expect(
    page.getByRole('group', { name: 'Primary layer' }).getByRole('radio', { name: 'L2' }),
  ).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L1' }),
  ).toBeEnabled();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L1' }),
  ).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L7' }),
  ).toBeChecked();
});

test('state, assurance, and edit mode stay independent of layers', async ({
  page,
}) => {
  await openViewControls(page, { width: 1280, height: 800 });
  await page.getByRole('button', { name: 'Load overlay ordering case' }).click();

  await page.getByRole('radio', { name: 'Operational' }).click();
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L3',
    overlays: ['L1', 'L2'],
    stateView: 'operational',
    assuranceOverlay: false,
    editMode: 'view',
  });

  await page.getByRole('switch', { name: 'Assurance overlay' }).click();
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L3',
    overlays: ['L1', 'L2'],
    stateView: 'operational',
    assuranceOverlay: true,
    editMode: 'view',
  });

  await page.getByRole('radio', { name: 'Edit' }).click();
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L3',
    overlays: ['L1', 'L2'],
    stateView: 'operational',
    assuranceOverlay: true,
    editMode: 'edit',
  });

  await page
    .getByRole('group', { name: 'Primary layer' })
    .getByRole('radio', { name: 'L4' })
    .click();
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L4',
    overlays: ['L1', 'L2'],
    stateView: 'operational',
    assuranceOverlay: true,
    editMode: 'edit',
  });

  await expect(page.getByRole('radio', { name: 'Operational' })).toBeChecked();
  await expect(page.getByRole('switch', { name: 'Assurance overlay' })).toBeChecked();
  await expect(page.getByRole('radio', { name: 'Edit' })).toBeChecked();
  await expect(
    page.getByText(
      'View/Edit requests a mode change only. It does not authorize compiler, graph, source, adoption, or deployment operations.',
    ),
  ).toBeVisible();
});

test('does not mutate input and external prop updates do not emit', async ({
  page,
}) => {
  await openViewControls(page, { width: 1280, height: 800 });
  await expect(await eventCount(page)).toBe(0);

  await page.getByRole('button', { name: 'Load overlays including primary' }).click();
  await expect(await eventCount(page)).toBe(0);
  await expect(await lastEvent(page)).toBeNull();
  await expect(await currentSelection(page)).toMatchObject({
    primaryLayer: 'L4',
    overlays: ['L4', 'L1'],
  });

  await expandAdvanced(page);
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L4' }),
  ).toHaveCount(0);
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L1' }),
  ).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Primary layer' }).getByRole('radio', { name: 'L4' }),
  ).toBeChecked();

  await page.getByRole('button', { name: 'Replace selection from outside' }).click();
  await expect(await eventCount(page)).toBe(0);
  await expect(await lastEvent(page)).toBeNull();
  await expect(await currentSelection(page)).toMatchObject({
    primaryLayer: 'L7',
    overlays: ['L1', 'L4'],
    combinedOverview: false,
    stateView: 'compare',
    assuranceOverlay: true,
    editMode: 'edit',
  });
  await expect(
    page.getByRole('group', { name: 'Primary layer' }).getByRole('radio', { name: 'L7' }),
  ).toBeChecked();
  await expect(page.getByRole('radio', { name: 'Compare' })).toBeChecked();
  await expect(page.getByRole('radio', { name: 'Edit' })).toBeChecked();
  await expect(page.getByRole('switch', { name: 'Assurance overlay' })).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L1' }),
  ).toBeChecked();
  await expect(
    page.getByRole('group', { name: 'Overlays' }).getByRole('checkbox', { name: 'L4' }),
  ).toBeChecked();

  await page.getByRole('radio', { name: 'Intent' }).click();
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L7',
    overlays: ['L1', 'L4'],
    stateView: 'intent',
    assuranceOverlay: true,
    editMode: 'edit',
  });
  await expect(await eventCount(page)).toBe(1);
});

test('keyboard focus, layer arrows, and overlay space toggle', async ({ page }) => {
  await openViewControls(page, { width: 1280, height: 800 });
  await expandAdvanced(page);

  const layerL3 = page
    .getByRole('group', { name: 'Primary layer' })
    .getByRole('radio', { name: 'L3' });
  await tabUntilFocused(page, layerL3);

  const outline = await layerL3.evaluate((element) => {
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

  await page.keyboard.press('ArrowLeft');
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L2',
    overlays: [],
    editMode: 'view',
  });
  await expect(
    page.getByRole('group', { name: 'Primary layer' }).getByRole('radio', { name: 'L2' }),
  ).toBeChecked();

  const overlayL1 = page
    .getByRole('group', { name: 'Overlays' })
    .getByRole('checkbox', { name: 'L1' });
  await tabUntilFocused(page, overlayL1);
  await page.keyboard.press('Space');
  await expect.poll(async () => lastEvent(page)).toMatchObject({
    primaryLayer: 'L2',
    overlays: ['L1'],
  });
  await expect(overlayL1).toBeChecked();
});

test('fits a 390-pixel viewport without horizontal scrolling', async ({ page }) => {
  await openViewControls(page, { width: 390, height: 844 });
  await expect(primaryRadios(page)).toHaveCount(5);
  await expect(page.getByRole('radio', { name: 'Device configuration' })).toBeVisible();
  const overflowDefault = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflowDefault).toBeLessThanOrEqual(1);

  await page.screenshot({
    path: path.join(screenshotDir, 'mobile-390x844-default.png'),
    fullPage: true,
  });

  await expandAdvanced(page);
  const overflowAdvanced = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflowAdvanced).toBeLessThanOrEqual(1);
});


test('requests do not override a value retained by the parent', async ({ page }) => {
  await openViewControls(page, { width: 1280, height: 800 });
  await page.getByLabel('Hold supplied selection (fixture only)').check();
  await page.getByRole('radio', { name: 'Operational', exact: true }).click();
  await expect.poll(async () => eventCount(page)).toBe(1);
  await expect(page.getByRole('radio', { name: 'Intent', exact: true })).toBeChecked();
  await expect(page.getByRole('radio', { name: 'Operational', exact: true })).not.toBeChecked();
  await page.getByRole('switch', { name: 'Assurance overlay' }).click();
  await expect.poll(async () => eventCount(page)).toBe(2);
  await expect(page.getByRole('switch', { name: 'Assurance overlay' })).not.toBeChecked();
  await expandAdvanced(page);
  await page.getByRole('radio', { name: 'L1', exact: true }).click();
  await expect(page.getByRole('radio', { name: 'L3', exact: true })).toBeChecked();
  await page.getByRole('checkbox', { name: 'L1', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'L1', exact: true })).not.toBeChecked();
  await page.getByRole('switch', { name: 'Combined overview' }).click();
  await expect(page.getByRole('switch', { name: 'Combined overview' })).not.toBeChecked();
  await expect(page.getByRole('radio', { name: 'L3', exact: true })).toBeEnabled();
});
