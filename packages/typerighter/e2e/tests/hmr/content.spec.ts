import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR content update', () => {
  e2e('modifying a .td file updates page without full reload', async ({
    page,
    testProject,
  }) => {
    // Navigate to a known page
    await page.goto(`http://localhost:${testProject.port}/people/alice`);
    await page.waitForLoadState('networkidle');

    // Track full page reloads
    let fullReloadOccurred = false;

    page.on('load', () => {
      fullReloadOccurred = true;
    });

    const marker = `HMR_TEST_${Date.now()}`;

    // Modify the file and wait for HMR
    await testProject.modifyFileAndWaitForHMR(
      page,
      'vault/people/alice.td',
      (content) => content + `\n${marker}\n`,
    );

    // The marker text should appear on the page
    await expect(page.locator(`text=${marker}`)).toBeVisible({
      timeout: 5_000,
    });

    // No full reload should have happened
    expect(fullReloadOccurred).toBe(false);
  });
});
