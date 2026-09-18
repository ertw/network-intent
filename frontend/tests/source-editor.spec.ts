import { expect, test as base, type Page } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  CRLF_WITH_FINAL,
  LF_WITH_FINAL,
  MIXED_NEWLINES,
  UNICODE_SOURCE,
} from '../src/features/source-editor/fixtures';

const captureEvidence = process.env.CAPTURE_GROK_EVIDENCE === '1';
const screenshotDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  // Astra review capture preserves the original Grok evidence.
  process.env.CAPTURE_ASTRA_EVIDENCE === '1'
    ? '../../docs/grok/reports/G06/astra'
    : captureEvidence ? '../../docs/grok/reports/G06' : '../test-results/G06-screenshots',
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
  await page.goto('/#/source-editor');
  await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
  await expect(page.getByText('DEVELOPMENT FIXTURE — NOT LIVE DATA').first()).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Source editor — synthetic document' }),
  ).toBeVisible();
}

async function dispatchClipboard(
  page: Page,
  type: 'paste' | 'cut',
  text = 'PASTED',
) {
  await page.evaluate(
    ({ type: eventType, text: payload }) => {
      const content = document.querySelector('[data-testid="source-editor-content"]');
      if (!(content instanceof HTMLElement)) {
        throw new Error('missing editor content');
      }
      const data = new DataTransfer();
      data.setData('text/plain', payload);
      const event = new ClipboardEvent(eventType, { bubbles: true, cancelable: true });
      Object.defineProperty(event, 'clipboardData', { value: data });
      content.dispatchEvent(event);
    },
    { type, text },
  );
}

async function dispatchDrop(page: Page) {
  await page.evaluate(() => {
    const content = document.querySelector('[data-testid="source-editor-content"]');
    if (!(content instanceof HTMLElement)) {
      throw new Error('missing editor content');
    }
    const data = new DataTransfer();
    data.setData('text/plain', 'DROPPED');
    const event = new DragEvent('drop', {
      bubbles: true,
      cancelable: true,
      dataTransfer: data,
    });
    content.dispatchEvent(event);
  });
}

test('edit mode typing updates the parent source without formatting', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  const content = page.getByTestId('source-editor-content');
  await content.click();
  await page.keyboard.press('Meta+a');
  await page.keyboard.press('ArrowRight');
  await page.keyboard.type('Z');

  await expect(page.getByTestId('source-editor-change-count')).toHaveText('onChange calls: 1');
  await expect(page.getByTestId('source-editor-parent-source')).toHaveText(`${LF_WITH_FINAL}Z`);
  await expect(page.getByTestId('source-editor-parent-source')).toContainText('\tkeep-tab');

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-edit.png'),
    fullPage: true,
  });
});

test('view mode blocks typing, clipboard, drop, and user-edit commands', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await page.getByTestId('source-editor-editable').uncheck();
  await expect(page.getByTestId('source-editor')).toHaveAttribute('data-editable', 'false');

  const content = page.getByTestId('source-editor-content');
  await content.click();
  await page.keyboard.type('nope');
  await dispatchClipboard(page, 'paste');
  await dispatchClipboard(page, 'cut');
  await dispatchDrop(page);
  await page.getByTestId('source-editor-user-command').click();

  await expect(page.getByTestId('source-editor-change-count')).toHaveText('onChange calls: 0');
  await expect(page.getByTestId('source-editor-parent-source')).toHaveText(LF_WITH_FINAL);
  await expect(content).not.toHaveAttribute('contenteditable', 'true');

  await page.screenshot({
    path: path.join(screenshotDir, 'desktop-1280x800-view.png'),
    fullPage: true,
  });
});

test('external replacement does not echo onChange and ignored edits roll back once', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await page.getByTestId('source-editor-external-reset').click();
  await expect(page.getByTestId('source-editor-change-count')).toHaveText('onChange calls: 0');
  await expect(page.getByTestId('source-editor-parent-source')).toHaveText(LF_WITH_FINAL);

  await page.getByTestId('source-editor-ignore-change').check();
  await page.getByTestId('source-editor-content').click();
  await page.keyboard.press('Meta+a');
  await page.keyboard.press('ArrowRight');
  await page.keyboard.type('Q');
  await expect(page.getByTestId('source-editor-change-count')).toHaveText('onChange calls: 1');
  await expect(page.getByTestId('source-editor-parent-source')).toHaveText(LF_WITH_FINAL);
  await expect(page.getByTestId('source-editor-content')).not.toContainText('Q');
});

test('CRLF selection uses editor offsets and mixed newlines stay raw', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await page.getByLabel('Synthetic case').selectOption('crlf-final');
  await expect(page.getByTestId('source-editor')).toHaveAttribute('data-newline', 'crlf');
  await expect(page.getByTestId('source-editor-parent-source')).toHaveText(CRLF_WITH_FINAL);

  await page.getByTestId('source-editor-content').click();
  await page.keyboard.press('Meta+a');
  await expect(page.getByTestId('source-editor-last-selection')).toHaveText('Last selection: 0,11');
  expect(CRLF_WITH_FINAL.length).toBe(13);

  await page.getByLabel('Synthetic case').selectOption('mixed');
  await expect(page.getByTestId('source-editor-unsupported')).toBeVisible();
  await expect(page.getByTestId('source-editor-raw')).toHaveText(MIXED_NEWLINES);
  await expect(page.getByTestId('source-editor-host')).toHaveCount(0);
  await expect(page.getByTestId('source-editor-change-count')).toHaveText('onChange calls: 0');

  await page.getByLabel('Synthetic case').selectOption('unicode');
  await expect(page.getByTestId('source-editor-host')).toBeVisible();
  await expect(page.getByTestId('source-editor-parent-source')).toHaveText(UNICODE_SOURCE);
  await expect(page.getByTestId('source-editor').locator('img')).toHaveCount(0);
});

test('history shortcuts emit fixture events without mutating text', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openFixture(page);

  await page.getByTestId('source-editor-content').click();
  await page.keyboard.press('Meta+z');
  await expect(page.getByTestId('source-editor-history-count')).toHaveText(
    'onHistoryRequest calls: 1',
  );
  await expect(page.getByTestId('source-editor-last-history')).toHaveText(
    'Last history request: undo',
  );
  await expect(page.getByTestId('source-editor-parent-source')).toHaveText(LF_WITH_FINAL);

  await page.keyboard.press('Meta+Shift+z');
  await expect(page.getByTestId('source-editor-last-history')).toHaveText(
    'Last history request: redo',
  );
  await page.keyboard.press('Control+y');
  await expect(page.getByTestId('source-editor-history-count')).toHaveText(
    'onHistoryRequest calls: 3',
  );
  await expect(page.getByTestId('source-editor-parent-source')).toHaveText(LF_WITH_FINAL);

  await page.getByTestId('source-editor-editable').uncheck();
  await page.getByTestId('source-editor-content').click();
  await page.keyboard.press('Meta+z');
  await expect(page.getByTestId('source-editor-history-count')).toHaveText(
    'onHistoryRequest calls: 3',
  );
});

test('mount/unmount, clamped selection, and the narrow viewport stay usable', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openFixture(page);

  await page.getByTestId('source-editor-clamp-selection').click();
  await page.getByTestId('source-editor-content').click();
  await page.getByTestId('source-editor-mounted').uncheck();
  await expect(page.getByTestId('source-editor')).toHaveCount(0);
  await page.getByTestId('source-editor-mounted').check();
  await expect(page.getByTestId('source-editor-host')).toBeVisible();
  await page.getByTestId('source-editor-second').check();
  await expect(page.getByTestId('source-editor')).toHaveCount(2);

  const overflowX = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflowX).toBeLessThanOrEqual(1);

  await page.screenshot({
    path: path.join(screenshotDir, 'mobile-390x844-edit.png'),
    fullPage: true,
  });
});

test('exact LF and CRLF edits preserve separators, tabs, Unicode and final newlines', async ({ page }) => {
  await openFixture(page);
  for (const [key, original] of [
    ['lf-final', LF_WITH_FINAL], ['lf-no-final', 'alpha\nbeta'],
    ['crlf-final', CRLF_WITH_FINAL], ['crlf-no-final', 'alpha\r\nbeta'],
    ['unicode', UNICODE_SOURCE],
  ]) {
    await page.getByLabel('Synthetic case').selectOption(key!);
    const parent = page.getByTestId('source-editor-parent-source');
    expect(await parent.textContent()).toBe(original);
    const content = page.getByTestId('source-editor-content');
    const identity = await content.elementHandle();
    await content.focus();
    await page.keyboard.press('Meta+a');
    await page.keyboard.press('ArrowRight');
    await page.keyboard.type('Z');
    await expect.poll(() => parent.textContent()).toBe(`${original}Z`);
    expect(await identity!.evaluate((element) => element.isConnected)).toBe(true);
    await dispatchClipboard(page, 'paste', '😀\t two  spaces');
    await expect.poll(() => parent.textContent()).toBe(`${original}Z😀\t two  spaces`);
    await page.keyboard.press('Backspace');
    await expect.poll(() => parent.textContent()).toBe(`${original}Z😀\t two  space`);
  }
});

test('external selection, view replacement and repeated mounts do not echo callbacks', async ({ page }) => {
  await openFixture(page);
  await page.getByTestId('source-editor-set-selection').click();
  await expect(page.getByTestId('source-editor-selection-count')).toHaveText('onSelectionChange calls: 0');
  await page.getByTestId('source-editor-content').focus();
  await expect.poll(() => page.evaluate(() => window.getSelection()?.toString())).toBe('alpha');
  await page.getByTestId('source-editor-ignore-selection').check();
  await page.getByTestId('source-editor-content').focus();
  await page.keyboard.press('ArrowRight');
  await expect.poll(() => page.evaluate(() => window.getSelection()?.toString())).toBe('alpha');
  await page.getByTestId('source-editor-nonfinite-selection').click();
  await page.getByTestId('source-editor-content').focus();
  await expect.poll(() => page.evaluate(() => window.getSelection()?.toString())).toBe('alpha');
  await page.getByLabel('Synthetic case').selectOption('crlf-final');
  await page.getByTestId('source-editor-editable').uncheck();
  await page.getByTestId('source-editor-external-reset').click();
  await expect(page.getByTestId('source-editor-content')).toContainText('keep-tab');
  await expect(page.getByTestId('source-editor-change-count')).toHaveText('onChange calls: 0');
  for (let i = 0; i < 3; i++) {
    await page.getByTestId('source-editor-mounted').uncheck();
    await page.getByTestId('source-editor-mounted').check();
  }
  await page.getByTestId('source-editor-editable').check();
  await page.getByTestId('source-editor-user-command').click();
  await expect(page.getByTestId('source-editor-change-count')).toHaveText('onChange calls: 1');
  await page.getByLabel('Synthetic case').selectOption('standalone-cr');
  expect(await page.getByTestId('source-editor-raw').textContent()).toBe('alpha\rbeta');
  await expect(page.getByTestId('source-editor-host')).toHaveCount(0);
});
