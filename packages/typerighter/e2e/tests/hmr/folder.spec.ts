import {
  mkdir, writeFile, rm,
} from 'node:fs/promises';
import path from 'node:path';
import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR folder operations', () => {
  e2e('adding a folder with .td files updates the sidebar', async ({
    page,
    testProject,
  }) => {
    await page.goto(`http://localhost:${testProject.port}`);
    await page.waitForLoadState('networkidle');

    const sidebar = page.locator('nav, [class*="sidebar"], aside');

    await expect(sidebar.locator('text=Reports')).not.toBeVisible();

    // Create a new folder with a file
    const reportsDirectory = path.join(testProject.dir, 'vault/reports');

    await mkdir(reportsDirectory, {
      recursive: true,
    });
    await writeFile(
      path.join(reportsDirectory, 'q1.td'),
      '---\ntitle: Q1 Report\n---\n\nQuarterly report.\n',
      'utf-8',
    );

    // New folder group should appear in sidebar
    await expect(sidebar.locator('text=Q1 Report')).toBeVisible({
      timeout: 10_000,
    });
  });

  e2e('deleting a folder removes its entries from the sidebar', async ({
    page,
    testProject,
  }) => {
    await page.goto(`http://localhost:${testProject.port}`);
    await page.waitForLoadState('networkidle');

    const sidebar = page.locator('nav, [class*="sidebar"], aside');

    // Milestones folder should be visible
    await expect(sidebar.locator('text=Alpha Preview')).toBeVisible({
      timeout: 5_000,
    });

    // Remove the milestones folder
    await rm(path.join(testProject.dir, 'vault/milestones'), {
      recursive: true,
      force: true,
    });

    await expect(sidebar.locator('text=Alpha Preview')).not.toBeVisible({
      timeout: 10_000,
    });
  });
});
