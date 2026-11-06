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

  e2e('theme toggle adds transition class during switch', async ({
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

    // Listen for the transition class being added
    const hadTransitionClass = await page.evaluate(() => {
      return new Promise((resolve) => {
        const observer = new MutationObserver(() => {
          if (document.documentElement.classList.contains('td-theme-transition')) {
            observer.disconnect();
            resolve(true);
          }
        });

        observer.observe(document.documentElement, {
          attributes: true,
          attributeFilter: ['class'],
        });

        // Timeout fallback
        setTimeout(() => {
          observer.disconnect();
          resolve(false);
        }, 3000);
      });
    });

    await toggle.click();

    expect(hadTransitionClass).toBe(true);

    // Transition class should be removed after animation completes
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
