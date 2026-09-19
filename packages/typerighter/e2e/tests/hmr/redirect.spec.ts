import {
  unlink, rm,
} from 'node:fs/promises';
import path from 'node:path';
import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR redirect on delete', () => {
  e2e('viewing a deleted file redirects away', async ({
    page,
    testProject,
  }) => {
    // Navigate to bob's page
    await page.goto(`http://localhost:${testProject.port}/people/bob`);
    await page.waitForLoadState('networkidle');
    await expect(page.locator('text=Bob')).toBeVisible({
      timeout: 5_000,
    });

    // Delete bob's file while viewing it
    await unlink(path.join(testProject.dir, 'vault/people/bob.td'));

    // Should redirect to a valid page or show not-found
    await page.waitForTimeout(3_000);
    const url = page.url();

    expect(url).not.toContain('/people/bob');
  });

  e2e('viewing a page in a deleted folder redirects away', async ({
    page,
    testProject,
  }) => {
    // Navigate to a milestone page
    await page.goto(`http://localhost:${testProject.port}/milestones/alpha-preview`);
    await page.waitForLoadState('networkidle');

    // Delete the entire milestones folder
    await rm(path.join(testProject.dir, 'vault/milestones'), {
      recursive: true,
      force: true,
    });

    // Should redirect away from the deleted page
    await page.waitForTimeout(3_000);
    const url = page.url();

    expect(url).not.toContain('/milestones/');
  });
});
