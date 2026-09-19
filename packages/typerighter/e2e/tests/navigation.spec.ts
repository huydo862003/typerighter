import {
  e2e, expect,
} from '../fixtures';

e2e.describe('navigation', () => {
  e2e('clicking a sidebar link navigates to the correct page', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);

    const sidebar = page.getByTestId('sidebar');

    // Expand the People folder (collapsed by default on root page)
    const peopleFolder = sidebar.getByText('People').first();

    await expect(peopleFolder).toBeVisible({ timeout: 15_000 });
    await peopleFolder.click();

    // Now click alice inside the expanded folder
    const aliceLink = sidebar.getByRole('link', {
      name: /Alice/,
    }).first();

    await expect(aliceLink).toBeVisible({ timeout: 5_000 });
    await aliceLink.click();

    await expect(page).toHaveURL(/\/people\/alice/);
  });

  e2e('navigating between pages does not cause full reload', async ({
    page,
    testProject,
  }) => {
    // Start at alice so People folder is auto-expanded
    await testProject.goto(page, '/people/alice');

    await page.evaluate(() => {
      document.body.dataset.navMarker = 'true';
    });

    const sidebar = page.getByTestId('sidebar');
    const bobLink = sidebar.getByRole('link', {
      name: /Bob/,
    }).first();

    await expect(bobLink).toBeVisible({ timeout: 15_000 });
    await bobLink.click();

    await expect(page).toHaveURL(/\/people\/bob/);

    const markerExists = await page.evaluate(() => document.body.dataset.navMarker);

    expect(markerExists).toBe('true');
  });
});
