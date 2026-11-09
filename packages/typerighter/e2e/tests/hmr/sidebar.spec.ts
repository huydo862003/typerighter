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

    const sidebar = page.getByTestId('sidebar');

    // Expand windowed sidebar so all entries are visible
    const moreButton = sidebar.locator('button.td-tree-more').last();

    if (await moreButton.isVisible()) {
      await moreButton.click();
    }

    // Verify new person is not in sidebar yet
    await expect(sidebar.locator('text=Zara')).not.toBeVisible();

    // Create a new .td file
    const newFile = path.join(testProject.dir, 'vault/people/zara.td');

    await writeFile(newFile, '---\n_type: Person\nname: Zara\n---\n\nZara is new here.\n', 'utf-8');

    // Wait for sidebar to update, expanding the windowed list if needed
    await expect(async () => {
      const more = sidebar.locator('button.td-tree-more').last();

      if (await more.isVisible()) {
        await more.click();
      }

      await expect(sidebar.locator('text=Zara')).toBeVisible();
    }).toPass({
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
