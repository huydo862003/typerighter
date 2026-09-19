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
    await testProject.goto(page);

    const sidebar = page.getByTestId('sidebar');

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

    // New file should appear in sidebar with unslugified filename
    await expect(sidebar.getByRole('link', {
      name: /Q1/,
    })).toBeVisible({
      timeout: 10_000,
    });
  });

  e2e('deleting a folder removes its entries from the sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);

    const sidebar = page.getByTestId('sidebar');

    // Milestones folder should be visible
    await expect(sidebar.locator('text=Notes')).toBeVisible({
      timeout: 5_000,
    });

    // Remove the milestones folder
    await rm(path.join(testProject.dir, 'vault/notes'), {
      recursive: true,
      force: true,
    });

    await expect(sidebar.locator('text=Notes')).not.toBeVisible({
      timeout: 10_000,
    });
  });
});
