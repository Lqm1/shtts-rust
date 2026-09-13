import { test, expect } from '@playwright/test';
test('loads packaged WASM under the Pages path and produces playable audio', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('./');
  await expect(page.getByRole('button', { name: '音声を生成', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: '音声を生成', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('音声を生成しました');
  await expect.poll(() => page.locator('audio').evaluate(audio => audio.duration)).toBeGreaterThan(0);
  expect(errors).toEqual([]);
});
