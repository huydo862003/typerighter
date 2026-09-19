import {
  rename,
} from 'node:fs/promises';
import path from 'node:path';
import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR file rename', () => {
  e2e('renaming a .td file updates the sidebar', async ({
    page,
    testProject,
  }) => {
    await page.goto(`http://localhost:${testProject.port}/people/alice`);
    await page.waitForLoadState('networkidle');

    const sidebar = page.locator('nav, [class*="sidebar"], aside');

    await expect(sidebar.locator('text=Bob')).toBeVisible({
      timeout: 5_000,
    });

    // Rename bob.td to bobby.td
    await rename(
      path.join(testProject.dir, 'vault/people/bob.td'),
      path.join(testProject.dir, 'vault/people/bobby.td'),
    );

    // Bob should disappear, Bobby should appear
    await expect(sidebar.locator('text=Bobby')).toBeVisible({
      timeout: 10_000,
    });
  });
});
