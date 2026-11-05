import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR page data update', () => {
  e2e('changing _label updates page title and sidebar without full reload', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    let fullReloadOccurred = false;

    page.on('load', () => {
      fullReloadOccurred = true;
    });

    await expect(page.locator('.td-page-title')).toHaveText('Alice (Dev)', {
      timeout: 10_000,
    });

    await testProject.modifyFile(
      'vault/people/alice.td',
      (content) => content.replace('"Alice (Dev)"', '"Alice Updated"'),
    );

    // Page title updates via HMR page-data event (fast)
    await expect(page.locator('.td-page-title')).toHaveText('Alice Updated', {
      timeout: 15_000,
    });
    // Sidebar updates via HMR site-data module invalidation (requires RPC round-trip)
    await expect(page.getByTestId('sidebar').locator('text=Alice Updated')).toBeVisible({
      timeout: 20_000,
    });

    expect(fullReloadOccurred).toBe(false);
  });

  e2e('adding a heading updates the TOC without full reload', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    let fullReloadOccurred = false;

    page.on('load', () => {
      fullReloadOccurred = true;
    });

    const rail = page.locator('.td-rail');
    const toc = page.locator('.td-rail .td-toc');

    await expect(rail).not.toBeVisible();

    await testProject.modifyFile(
      'vault/people/alice.td',
      (content) => content + '\n## Background\n\nSome background.\n',
    );

    await expect(rail).toBeVisible({
      timeout: 15_000,
    });
    await expect(toc.locator('text=Background')).toBeAttached({
      timeout: 15_000,
    });

    expect(fullReloadOccurred).toBe(false);
  });

  e2e('changing a frontmatter value updates the properties panel without full reload', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    let fullReloadOccurred = false;

    page.on('load', () => {
      fullReloadOccurred = true;
    });

    await expect(page.locator('.td-fm-box-value').first()).toBeVisible({
      timeout: 5_000,
    });

    await testProject.modifyFile(
      'vault/people/alice.td',
      (content) => content.replace('"developer"', '"designer"'),
    );

    await expect(page.locator('.td-fm-box-value >> text=designer').first()).toBeVisible({
      timeout: 15_000,
    });

    expect(fullReloadOccurred).toBe(false);
  });
});
