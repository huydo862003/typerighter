import {
  writeFile, copyFile, mkdir,
} from 'node:fs/promises';
import path from 'node:path';
import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR file operations', () => {
  e2e('copying a .td file adds it to the sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Copy alice.td to dave.td
    // The copy keeps alice's _label, so dave appears with a link to /people/dave
    await copyFile(
      path.join(testProject.dir, 'vault/people/alice.td'),
      path.join(testProject.dir, 'vault/people/dave.td'),
    );

    // A new link pointing to /people/dave should appear
    await expect(sidebar.locator('a[href*="/people/dave"]')).toBeVisible({
      timeout: 10_000,
    });
  });

  e2e('adding a .td file in a new nested folder updates the sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Create a new folder with a file inside
    const newDir = path.join(testProject.dir, 'vault/reports');

    await mkdir(newDir, {
      recursive: true,
    });
    await writeFile(
      path.join(newDir, 'weekly.td'),
      '---\ntitle: Weekly Report\n---\n\nThis week went well.\n',
      'utf-8',
    );

    // The new folder should appear in sidebar
    await expect(sidebar.locator('text=Reports')).toBeVisible({
      timeout: 10_000,
    });
  });

  e2e('adding a typed .td file in an existing folder updates the sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Add a new Person entry
    await writeFile(
      path.join(testProject.dir, 'vault/people/eve.td'),
      '---\n_type: Person\n_label: "Eve (QA)"\nname: Eve\nrole: qa\n---\n\nEve tests things.\n',
      'utf-8',
    );

    // Eve should appear in the sidebar
    await expect(sidebar.locator('text=Eve (QA)')).toBeVisible({
      timeout: 10_000,
    });
  });
});
