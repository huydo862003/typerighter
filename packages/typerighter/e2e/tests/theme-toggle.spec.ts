import {
  e2e, expect,
} from '../fixtures';

e2e.describe('theme toggle', () => {
  e2e('toggle switches between light and dark mode', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);

    const html = page.locator('html');

    // Should start without .dark class (light mode default)
    await expect(html).not.toHaveClass(/dark/, {
      timeout: 15_000,
    });

    // Click the theme toggle button
    const toggle = page.getByRole('button', {
      name: /Switch to dark mode/,
    });

    await expect(toggle).toBeVisible({
      timeout: 5_000,
    });
    await toggle.click();

    // Should now have .dark class
    await expect(html).toHaveClass(/dark/);
  });

  e2e('transition class is cleaned up after toggle', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);

    const html = page.locator('html');
    const toggle = page.getByRole('button', {
      name: /Switch to dark mode/,
    });

    await expect(toggle).toBeVisible({
      timeout: 15_000,
    });
    await toggle.click();

    // After the transition completes, the class should be removed
    await expect(html).not.toHaveClass(/td-theme-transition/, {
      timeout: 2_000,
    });
  });

  e2e('theme persists across navigation', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);

    // Switch to dark mode
    const toggle = page.getByRole('button', {
      name: /Switch to dark mode/,
    });

    await expect(toggle).toBeVisible({
      timeout: 15_000,
    });
    await toggle.click();

    await expect(page.locator('html')).toHaveClass(/dark/);

    // Navigate to another page
    await testProject.goto(page, '/people/alice');

    // Dark mode should persist
    await expect(page.locator('html')).toHaveClass(/dark/);
  });
});
