import {
  writeFile, unlink,
} from 'node:fs/promises';
import path from 'node:path';
import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR sidebar update', () => {
  e2e('adding a new .td file updates the sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    // Verify new person is not in sidebar yet
    const sidebar = page.getByTestId('sidebar');

    await expect(sidebar.locator('text=Zara')).not.toBeVisible();

    // Create a new .td file
    const newFile = path.join(testProject.dir, 'vault/people/zara.td');

    await writeFile(newFile, '---\n_type: Person\nname: Zara\n---\n\nZara is new here.\n', 'utf-8');

    // Wait for sidebar to update
    await expect(sidebar.locator('text=Zara')).toBeVisible({
      timeout: 10_000,
    });
  });

  e2e('removing a .td file updates the sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    // Find bob in sidebar
    const sidebar = page.getByTestId('sidebar');

    await expect(sidebar.locator('text=Bob')).toBeVisible({
      timeout: 5_000,
    });

    // Remove bob's file
    await unlink(path.join(testProject.dir, 'vault/people/bob.td'));

    // Bob should disappear from sidebar
    await expect(sidebar.locator('text=Bob')).not.toBeVisible({
      timeout: 10_000,
    });
  });
});
