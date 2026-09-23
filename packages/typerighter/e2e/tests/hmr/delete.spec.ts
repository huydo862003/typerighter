import {
  unlink,
} from 'node:fs/promises';
import path from 'node:path';
import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR file delete', () => {
  e2e('deleting a .td file removes it from the sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    await expect(sidebar.locator('text=Carol')).toBeVisible({
      timeout: 5_000,
    });

    await unlink(path.join(testProject.dir, 'vault/people/carol.td'));

    await expect(sidebar.locator('text=Carol')).not.toBeVisible({
      timeout: 10_000,
    });
  });
});
