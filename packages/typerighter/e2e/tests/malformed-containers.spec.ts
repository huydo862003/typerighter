import {
  e2e, expect,
} from '../fixtures';

e2e.describe('malformed containers', () => {
  e2e('page with malformed containers loads without crashing', async ({
    page,
    testProject,
  }) => {
    // The page has empty-label containers and unclosed containers
    // It must not produce invalid HTML that crashes the Vue renderer
    await testProject.goto(page, '/malformed-containers');

    // The page should render - check that the heading is visible
    await expect(page.getByRole('heading', { name: 'Valid containers' })).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('valid containers inside the page still render', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/malformed-containers');

    // The valid details block should work
    const details = page.locator('details.td-callout');

    await expect(details.first()).toBeVisible({
      timeout: 10_000,
    });

    // The valid tip callout should render
    const tip = page.locator('.tip.td-callout');

    await expect(tip).toBeVisible();
  });

  e2e('page does not show Vue compilation errors', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/malformed-containers');

    // Wait for content to load
    await expect(page.getByRole('heading', { name: 'Valid containers' })).toBeVisible({
      timeout: 15_000,
    });

    // Check no Vite/Vue error overlay appeared
    const errorOverlay = page.locator('vite-error-overlay');

    await expect(errorOverlay).not.toBeVisible();
  });
});
