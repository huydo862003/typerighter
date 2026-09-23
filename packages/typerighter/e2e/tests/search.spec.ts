import {
  e2e, expect,
} from '../fixtures';

e2e.describe('search', () => {
  e2e('typing in search shows results with titles', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    // The sidebar search input
    const searchInput = page.getByTestId('sidebar').locator('input[type="text"]');

    await expect(searchInput).toBeVisible({
      timeout: 10_000,
    });

    await searchInput.fill('Alice');

    // Search results should appear with a matching title
    const resultLink = page.getByTestId('sidebar').locator('.td-search-result');

    await expect(resultLink.first()).toBeVisible({
      timeout: 10_000,
    });
    await expect(resultLink.first()).toContainText('Alice');

    // The result should show the absolute filepath
    const resultPath = resultLink.first().locator('.td-search-result-path');

    await expect(resultPath).toBeVisible();
    await expect(resultPath).toContainText('/people/alice');
  });

  e2e('search results show contextual excerpts', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const searchInput = page.getByTestId('sidebar').locator('input[type="text"]');

    await expect(searchInput).toBeVisible({
      timeout: 10_000,
    });

    // Search for a word that appears in body content
    await searchInput.fill('writes');

    // Wait for excerpts to load (async, filled after page component mounts)
    const excerpt = page.getByTestId('sidebar').locator('.td-search-result-excerpt');

    await expect(excerpt.first()).toBeVisible({
      timeout: 15_000,
    });

    // The excerpt should contain the matched term in context
    await expect(excerpt.first()).toContainText('writes', {
      timeout: 5_000,
    });
  });

  e2e('clearing search restores the sidebar tree', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');
    const searchInput = sidebar.locator('input[type="text"]');

    await expect(searchInput).toBeVisible({
      timeout: 10_000,
    });

    // Type to trigger search
    await searchInput.fill('Alice');
    await expect(sidebar.locator('.td-search-result').first()).toBeVisible({
      timeout: 10_000,
    });

    // Clear search
    await searchInput.fill('');

    // Sidebar tree should be back (Bob should be visible since People folder is expanded)
    await expect(sidebar.locator('text=Bob')).toBeVisible({
      timeout: 5_000,
    });
  });
});
