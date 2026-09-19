import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR frontmatter change', () => {
  e2e('changing frontmatter updates the page without full reload', async ({
    page,
    testProject,
  }) => {
    await page.goto(`http://localhost:${testProject.port}/people/alice`);
    await page.waitForLoadState('networkidle');

    let fullReloadOccurred = false;

    page.on('load', () => {
      fullReloadOccurred = true;
    });

    // Change alice's name in frontmatter
    await testProject.modifyFileAndWaitForHMR(
      page,
      'vault/people/alice.td',
      (content) => content.replace(/name:.*/, 'name: "Alice Updated"'),
    );

    // The updated name should appear
    await expect(page.locator('text=Alice Updated')).toBeVisible({
      timeout: 5_000,
    });
    expect(fullReloadOccurred).toBe(false);
  });
});
