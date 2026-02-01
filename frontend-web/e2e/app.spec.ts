import { test, expect } from '@playwright/test';

test.describe('Application', () => {
  test('loads homepage', async ({ page }) => {
    await page.goto('/');

    await expect(page.locator('h1')).toContainText('Pika');
  });

  test('has navigation menu', async ({ page }) => {
    await page.goto('/');

    const menuButton = page.getByTestId('session-list-button');
    await expect(menuButton).toBeVisible();
  });

  test('opens settings dialog', async ({ page }) => {
    await page.goto('/');

    const settingsButton = page.getByRole('button', { name: /settings/i });
    await settingsButton.click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
  });
});

test.describe('Responsive Design', () => {
  test('mobile view', async ({ page, viewport }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');

    const menuButton = page.getByTestId('session-list-button');
    await expect(menuButton).toBeVisible();

    const header = page.locator('header');
    await expect(header).toBeVisible();
  });

  test('desktop view', async ({ page, viewport }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto('/');

    const header = page.locator('header');
    await expect(header).toBeVisible();
  });
});
