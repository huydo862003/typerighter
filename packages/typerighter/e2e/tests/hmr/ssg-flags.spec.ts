import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('HMR _ssg flag reactivity', () => {
  e2e('toggling show_in_sidebar: false hides item from sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Alice is visible initially
    await expect(sidebar.getByText('Alice (Dev)')).toBeVisible({
      timeout: 15_000,
    });

    // Hide Alice from sidebar
    await testProject.modifyFile('vault/people/alice.td', (content) =>
      content.replace(
        `---
_type: Person`,
        `---
_type: Person
_ssg:
  show_in_sidebar: false`,
      ));

    // Alice should disappear from sidebar
    await expect(sidebar.getByText('Alice (Dev)')).not.toBeVisible({
      timeout: 10_000,
    });
  });

  e2e('toggling show_in_sidebar: true shows item in sidebar', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');

    // Zoe starts hidden
    await expect(sidebar.getByText('Zoe (Hidden)')).not.toBeVisible({
      timeout: 10_000,
    });

    // Unhide Zoe
    await testProject.modifyFile('vault/people/zoe.td', (content) =>
      content.replace('show_in_sidebar: false', 'show_in_sidebar: true'));

    // Zoe should appear in sidebar
    await expect(sidebar.getByText('Zoe (Hidden)')).toBeVisible({
      timeout: 10_000,
    });
  });

  e2e('toggling show_in_search: false removes page from search results', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);

    const sidebar = page.getByTestId('sidebar');
    const searchInput = sidebar.locator('input[type="text"]');

    await expect(searchInput).toBeVisible({
      timeout: 10_000,
    });

    // Alice should appear in search initially
    await searchInput.fill('Alice');
    const results = sidebar.locator('.td-search-result');

    await expect(results.first()).toBeVisible({
      timeout: 10_000,
    });
    await searchInput.fill('');

    // Hide Alice from search
    await testProject.modifyFile('vault/people/alice.td', (content) =>
      content.replace(
        `---
_type: Person`,
        `---
_type: Person
_ssg:
  show_in_search: false`,
      ));

    // After change, Alice should not appear in search
    await expect(async () => {
      await searchInput.fill('');
      await searchInput.fill('Alice writes code');
      await expect(results).toHaveCount(0);
    }).toPass({
      timeout: 10_000,
    });
  });

  e2e('toggling show_in_search: true restores page in search results', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);

    const sidebar = page.getByTestId('sidebar');
    const searchInput = sidebar.locator('input[type="text"]');

    await expect(searchInput).toBeVisible({
      timeout: 10_000,
    });

    // Zygomorphic starts hidden from search
    await searchInput.fill('Zygomorphic');
    const results = sidebar.locator('.td-search-result');

    await page.waitForTimeout(1_000);
    await expect(results).toHaveCount(0);
    await searchInput.fill('');

    // Enable search on the hidden-from-search page
    await testProject.modifyFile('vault/hidden-from-search.td', (content) =>
      content.replace('show_in_search: false', 'show_in_search: true'));

    // Now Zygomorphic should appear in search
    await expect(async () => {
      await searchInput.fill('');
      await searchInput.fill('Zygomorphic');
      await expect(results.first()).toBeVisible();
    }).toPass({
      timeout: 10_000,
    });
  });
});
