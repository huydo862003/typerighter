import {
  e2e, expect,
} from '../fixtures';

e2e.describe('sidebar pagination', () => {
  e2e(
    'shows "more" button when a folder has more than 10 items',
    async ({
      page, testProject,
    }) => {
      // Navigate to a page inside the notes folder (15 items) so it expands
      await testProject.goto(page, '/notes/note-1');

      const sidebar = page.getByTestId('sidebar');
      const notesFolder = sidebar.getByRole('treeitem', {
        name: /Notes/,
      }).first();

      // The folder should show at most 10 items initially, so "more" appears
      const moreButton = notesFolder.getByRole('button', {
        name: /more/i,
      });

      await expect(moreButton).toBeVisible({
        timeout: 15_000,
      });
    },
  );

  e2e(
    'clicking "more" reveals additional items',
    async ({
      page, testProject,
    }) => {
      await testProject.goto(page, '/notes/note-1');

      const sidebar = page.getByTestId('sidebar');
      const notesFolder = sidebar.getByRole('treeitem', {
        name: /Notes/,
      }).first();

      const moreButton = notesFolder.getByRole('button', {
        name: /more/i,
      });

      await expect(moreButton).toBeVisible({
        timeout: 15_000,
      });

      const countBefore = await sidebar.getByRole('link').count();

      await moreButton.click();

      const countAfter = await sidebar.getByRole('link').count();

      expect(countAfter).toBeGreaterThan(countBefore);
    },
  );
});
