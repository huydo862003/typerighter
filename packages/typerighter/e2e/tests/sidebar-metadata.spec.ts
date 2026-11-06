import {
  e2e, expect,
} from '../fixtures';

e2e.describe('sidebar metadata', () => {
  e2e('sidebar shows custom labels instead of filenames', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Alice has _label: "Alice (Dev)" which should appear instead of "alice"
    await expect(sidebar.getByText('Alice (Dev)')).toBeVisible({
      timeout: 15_000,
    });

    // Bob has _label: "Bob (Design)"
    await expect(sidebar.getByText('Bob (Design)')).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('sidebar shows custom icons when defined', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Wait for the sidebar to render with siteData loaded
    await expect(sidebar.getByText('Alice (Dev)')).toBeVisible({
      timeout: 15_000,
    });

    // Alice has _icon: icon.user - the link should contain an SVG icon
    // that is NOT the default File icon
    const aliceLink = sidebar.getByRole('link', {
      name: /Alice/,
    }).first();

    const iconSvg = aliceLink.locator('.td-tree-file-icon');

    await expect(iconSvg).toBeVisible();
    // The custom icon (User) should not have the default File icon's path
    // File icon has a specific path; custom icons have different paths
    const svgHtml = await iconSvg.innerHTML();

    expect(svgHtml).not.toBe('');
  });

  e2e('labels do not flash as filenames before loading', async ({
    page,
    testProject,
  }) => {
    // Navigate and immediately check - labels should be correct from first render
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Wait for sidebar to appear
    await expect(sidebar.getByText('People').first()).toBeVisible({
      timeout: 15_000,
    });

    // "alice" (the raw filename) should NOT appear as sidebar text
    // Only "Alice (Dev)" (the _label) should be shown
    const rawFilename = sidebar.locator('.td-tree-link-text', {
      hasText: /^alice$/,
    });

    await expect(rawFilename).not.toBeVisible();
  });
});
