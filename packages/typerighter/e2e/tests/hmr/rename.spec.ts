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
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Bob's link should point to /people/bob
    await expect(sidebar.getByRole('link', {
      name: /Bob/,
    })).toHaveAttribute('href', /\/people\/bob$/, {
      timeout: 5_000,
    });

    // Rename bob.td to bobby.td
    await rename(
      path.join(testProject.dir, 'vault/people/bob.td'),
      path.join(testProject.dir, 'vault/people/bobby.td'),
    );

    // Rename fires as delete + create so the link may briefly disappear
    // Wait for the new href to appear
    await expect(sidebar.getByRole('link', {
      name: /Bob/,
    })).toHaveAttribute('href', /\/people\/bobby$/, {
      timeout: 15_000,
    });
  });
});
