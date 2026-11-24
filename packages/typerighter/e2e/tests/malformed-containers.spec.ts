import {
  e2e, expect,
} from '../fixtures';

e2e.describe('container edge cases', () => {
  e2e('page with containers loads without crashing', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/malformed-containers');

    await expect(page.getByRole('heading', {
      name: 'Valid containers',
    })).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('valid containers render correctly', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/malformed-containers');

    // Wait for content to fully render
    await expect(page.getByRole('heading', {
      name: 'Valid containers',
    })).toBeVisible({
      timeout: 15_000,
    });

    // Details block renders with the new td-details class
    const details = page.locator('.td-details');

    await expect(details.first()).toBeVisible({
      timeout: 10_000,
    });

    // Tip callout still uses td-callout
    const tip = page.locator('.tip.td-callout');

    await expect(tip).toBeVisible({
      timeout: 10_000,
    });
  });

  e2e('nested containers render without Vue errors', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/malformed-containers');

    await expect(page.getByRole('heading', {
      name: 'Nested containers',
    })).toBeVisible({
      timeout: 15_000,
    });

    // No Vite/Vue error overlay
    const errorOverlay = page.locator('vite-error-overlay');

    await expect(errorOverlay).not.toBeVisible();
  });

  e2e('container with code block renders both', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/malformed-containers');

    // The code block inside the details should render
    await expect(page.getByRole('heading', {
      name: 'Container with code block',
    })).toBeVisible({
      timeout: 15_000,
    });
  });
});
