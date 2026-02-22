import { test, expect } from '@playwright/test';

test.describe('Application', () => {
  test('loads homepage', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByRole('heading', { name: 'Pika' })).toBeVisible();
    await expect(page.getByText('No session selected')).toBeVisible();
  });

  test('has desktop command palette trigger', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByTestId('command-palette-button')).toBeVisible();
  });

  test('opens settings dialog', async ({ page }) => {
    await page.goto('/');

    const settingsButton = page.getByRole('button', { name: /settings/i });
    await settingsButton.click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(page.getByText('AI Settings')).toBeVisible();
  });
});

test.describe('Responsive Design', () => {
  test('mobile view', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');

    const menuButton = page.getByRole('button', { name: /menu/i });
    await expect(menuButton).toBeVisible();
    await menuButton.click();
    await expect(page.getByRole('menuitem', { name: 'Switch Session' })).toBeVisible();

    const header = page.locator('header');
    await expect(header).toBeVisible();
  });

  test('desktop view', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto('/');

    const header = page.locator('header');
    await expect(header).toBeVisible();
  });
});
