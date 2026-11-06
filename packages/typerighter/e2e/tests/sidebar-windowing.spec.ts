import {
  e2e, expect,
} from '../fixtures';

e2e.describe('sidebar windowing', () => {
  // The people/ directory has 15 person files (> WINDOW_SIZE of 10)
  // so windowing should be active

  e2e('shows "more..." below when viewing an early entry', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Alice is near the top, so there should be hidden entries below
    await expect(sidebar.getByRole('button', {
      name: /more\.\.\./,
    })).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('shows "more..." above when viewing a late entry', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/olivia');

    const sidebar = page.getByTestId('sidebar');

    // Olivia is near the end alphabetically, so entries above should be hidden
    // There should be at least one "more..." button
    const moreButtons = sidebar.getByRole('button', {
      name: /more\.\.\./,
    });

    await expect(moreButtons.first()).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('clicking "more..." expands all entries', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    const moreButton = sidebar.getByRole('button', {
      name: /more\.\.\./,
    }).first();

    await expect(moreButton).toBeVisible({
      timeout: 15_000,
    });
    await moreButton.click();

    // After expanding, no more buttons should be visible
    await expect(sidebar.getByRole('button', {
      name: /more\.\.\./,
    })).not.toBeVisible();

    // All people should now be visible
    await expect(sidebar.getByRole('link', {
      name: /Olivia/,
    }).first()).toBeVisible();
    await expect(sidebar.getByRole('link', {
      name: /Alice/,
    }).first()).toBeVisible();
  });

  e2e('overview entry is always visible regardless of window position', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/olivia');

    const sidebar = page.getByTestId('sidebar');

    // Overview link should always be present even when viewing a late entry
    await expect(sidebar.getByRole('link', {
      name: /Overview/,
    }).first()).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('current page is always visible in the window', async ({
    page,
    testProject,
  }) => {
    // Navigate to a middle entry
    await testProject.goto(page, '/people/jack');

    const sidebar = page.getByTestId('sidebar');

    // Jack should be visible since the window centers around the current page
    await expect(sidebar.getByRole('link', {
      name: /Jack/,
    }).first()).toBeVisible({
      timeout: 15_000,
    });
  });
});
