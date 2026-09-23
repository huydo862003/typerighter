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
    await testProject.goto(page, '/people/bob');
    await expect(page.getByTestId('content').getByRole('heading')).toBeVisible({
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
    await testProject.goto(page, '/notes/index');

    // Delete the entire milestones folder
    await rm(path.join(testProject.dir, 'vault/notes'), {
      recursive: true,
      force: true,
    });

    // Should redirect away from the deleted page
    await page.waitForTimeout(3_000);
    const url = page.url();

    expect(url).not.toContain('/notes/');
  });
});
