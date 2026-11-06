import {
  e2e, expect,
} from '../fixtures';

// Helper: type a query and wait for results
async function searchFor (page, sidebar, query) {
  const searchInput = sidebar.locator('input[type="text"]');

  await expect(searchInput).toBeVisible({
    timeout: 10_000,
  });
  await searchInput.fill(query);

  return sidebar.locator('.td-search-result');
}

e2e.describe('search indexing', () => {
  // Each test searches for a unique word that only appears in search-content.td
  // in a specific content type, verifying that the indexer captures it

  e2e('indexes heading text', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);
    const sidebar = page.getByTestId('sidebar');
    const results = await searchFor(page, sidebar, 'Xylophone');

    await expect(results.first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(results.first()).toContainText('Search Content');
  });

  e2e('indexes paragraph text', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);
    const sidebar = page.getByTestId('sidebar');
    const results = await searchFor(page, sidebar, 'Kaleidoscope');

    await expect(results.first()).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('indexes text across bold and italic boundaries', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);
    const sidebar = page.getByTestId('sidebar');

    // Bold text
    const boldResults = await searchFor(page, sidebar, 'Periwinkle');

    await expect(boldResults.first()).toBeVisible({
      timeout: 15_000,
    });

    // Clear and search italic text
    await sidebar.locator('input[type="text"]').fill('');
    const italicResults = await searchFor(page, sidebar, 'Vermillion');

    await expect(italicResults.first()).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('indexes code block content', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);
    const sidebar = page.getByTestId('sidebar');
    const results = await searchFor(page, sidebar, 'fibonacci_sequence');

    await expect(results.first()).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('indexes table cell content', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);
    const sidebar = page.getByTestId('sidebar');
    const results = await searchFor(page, sidebar, 'Chrysanthemum');

    await expect(results.first()).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('indexes content inside containers', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);
    const sidebar = page.getByTestId('sidebar');
    const results = await searchFor(page, sidebar, 'Bioluminescence');

    await expect(results.first()).toBeVisible({
      timeout: 15_000,
    });
  });

  e2e('indexes content inside collapsible details', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);
    const sidebar = page.getByTestId('sidebar');
    const results = await searchFor(page, sidebar, 'Phantasmagoria');

    await expect(results.first()).toBeVisible({
      timeout: 15_000,
    });
  });
});
