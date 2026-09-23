import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR content update', () => {
  e2e('modifying a .td file updates page without full reload', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    // Track full page reloads
    let fullReloadOccurred = false;

    page.on('load', () => {
      fullReloadOccurred = true;
    });

    const marker = `HMR_TEST_${Date.now()}`;

    await testProject.modifyFile(
      'vault/people/alice.td',
      (content) => content + `\n${marker}\n`,
    );

    // The marker text should appear on the page via HMR
    await expect(page.locator(`text=${marker}`)).toBeVisible({
      timeout: 15_000,
    });

    expect(fullReloadOccurred).toBe(false);
  });
});
