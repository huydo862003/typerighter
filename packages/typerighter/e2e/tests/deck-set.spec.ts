import {
  e2e, expect,
} from '../fixtures';

e2e.describe('deck set', () => {
  e2e('renders deck table with correct deck names', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    // Wait for deck table to render
    const table = page.locator('.td-decks-table');

    await expect(table).toBeVisible({
      timeout: 15_000,
    });

    // Should have deck rows (not counting header)
    const rows = table.locator('.td-decks-link');

    await expect(rows).not.toHaveCount(0);
  });

  e2e('decks are sorted by numeric prefix', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    const table = page.locator('.td-decks-table');

    await expect(table).toBeVisible({
      timeout: 15_000,
    });

    // Get all deck names in order
    const rows = table.locator('.td-decks-link');
    const names: string[] = [];
    const count = await rows.count();

    for (let index = 0; index < count; index++) {
      const cell = rows.nth(index).locator('span[role="cell"]')
        .first();
      const text = await cell.textContent();

      if (text) names.push(text.trim());
    }

    // 01-Foundations should come before 02-Type Theory, 03-Proofs, then Review (no prefix = last)
    const foundationsIndex = names.findIndex((name) => name.includes('Foundations'));
    const typeTheoryIndex = names.findIndex((name) => name.includes('Type'));
    const proofsIndex = names.findIndex((name) => name.includes('Proofs'));
    const reviewIndex = names.findIndex((name) => name.includes('Review'));

    expect(foundationsIndex).toBeGreaterThanOrEqual(0);
    expect(typeTheoryIndex).toBeGreaterThan(foundationsIndex);
    expect(proofsIndex).toBeGreaterThan(typeTheoryIndex);
    expect(reviewIndex).toBeGreaterThan(proofsIndex);
  });

  e2e('numeric prefix is stripped from displayed deck names', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    const table = page.locator('.td-decks-table');

    await expect(table).toBeVisible({
      timeout: 15_000,
    });

    // Names should not contain "01-", "02-", "03-" prefixes
    const rows = table.locator('.td-decks-link');
    const count = await rows.count();

    for (let index = 0; index < count; index++) {
      const cell = rows.nth(index).locator('span[role="cell"]')
        .first();
      const text = await cell.textContent();

      expect(text).not.toMatch(/^\d+-/);
    }
  });

  e2e('study all due button is visible', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    const studyButton = page.locator('.td-decks-study-btn');

    await expect(studyButton).toBeVisible({
      timeout: 15_000,
    });
    await expect(studyButton).toContainText('Study all due');
  });

  e2e('preview button is visible', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    const previewButton = page.locator('.td-decks-preview-btn');

    await expect(previewButton).toBeVisible({
      timeout: 15_000,
    });
    await expect(previewButton).toHaveText('Preview');
  });

  e2e('clicking preview opens study overlay', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    const previewButton = page.locator('.td-decks-preview-btn');

    await expect(previewButton).toBeVisible({
      timeout: 15_000,
    });

    await previewButton.click();

    // Study overlay should appear
    const study = page.locator('.td-study');

    await expect(study).toBeVisible({
      timeout: 10_000,
    });

    // Should show "Show answer" button
    await expect(page.locator('.td-study-show-btn')).toBeVisible();

    // Press Escape to close
    await page.keyboard.press('Escape');

    await expect(study).not.toBeVisible();
  });
});
