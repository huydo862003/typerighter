import {
  e2e, expect,
} from '../fixtures';

e2e.describe('navigation', () => {
  e2e('clicking a sidebar link navigates to the correct page', async ({
    page,
    testProject,
  }) => {
    await page.goto(`http://localhost:${testProject.port}`);
    await page.waitForLoadState('networkidle');

    // Find and click a link to alice in the sidebar
    const sidebar = page.getByTestId('sidebar');
    const aliceLink = sidebar.locator('a', {
      hasText: 'Alice',
    }).first();

    await aliceLink.click();

    // URL should contain alice
    await expect(page).toHaveURL(/\/people\/alice/);

    // Page content should relate to alice
    const content = page.getByTestId('content');

    await expect(content.locator('text=Alice')).toBeVisible({
      timeout: 5_000,
    });
  });

  e2e('navigating between pages does not cause full reload', async ({
    page,
    testProject,
  }) => {
    await page.goto(`http://localhost:${testProject.port}/people/alice`);
    await page.waitForLoadState('networkidle');

    // Inject a marker into the DOM to detect full reload
    await page.evaluate(() => {
      document.body.dataset.navMarker = 'true';
    });

    // Navigate to another page via sidebar
    const sidebar = page.getByTestId('sidebar');
    const bobLink = sidebar.locator('a', {
      hasText: 'Bob',
    }).first();

    await bobLink.click();

    await expect(page).toHaveURL(/\/people\/bob/);

    // Marker should still exist if it was a client-side navigation
    const markerExists = await page.evaluate(() => document.body.dataset.navMarker);

    expect(markerExists).toBe('true');
  });
});
