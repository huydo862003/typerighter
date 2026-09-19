import {
  writeFile, readFile,
} from 'node:fs/promises';
import path from 'node:path';
import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR schema change', () => {
  e2e('modifying a schema file triggers full reload', async ({
    page,
    testProject,
  }) => {
    await page.goto(`http://localhost:${testProject.port}/people/alice`);
    await page.waitForLoadState('networkidle');

    // Schema changes trigger full reload, not HMR update
    const reloadPromise = page.waitForEvent('load', {
      timeout: 15_000,
    });

    // Add a field to Person schema
    const schemaPath = path.join(testProject.dir, 'vault/_types/people/Person.td');
    const original = await readFile(schemaPath, 'utf-8');

    await writeFile(
      schemaPath,
      original.replace('  email:', '  website:\n    type: string\n  email:'),
      'utf-8',
    );

    await reloadPromise;

    // Page should still render after schema change
    await expect(page.locator('body')).not.toBeEmpty();
  });
});
