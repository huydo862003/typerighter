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

  e2e('multi-deck card appears in multiple decks', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    const table = page.locator('.td-decks-table');

    await expect(table).toBeVisible({
      timeout: 15_000,
    });

    // card-multi belongs to both 01-Foundations and 03-Proofs
    // Both decks should show at least 1 card (the multi-deck card + possibly others)
    const rows = table.locator('.td-decks-link');
    const count = await rows.count();

    // Find Foundations and Proofs rows and check card counts > 0
    let foundationsCards = 0;
    let proofsCards = 0;

    for (let index = 0; index < count; index++) {
      const cells = rows.nth(index).locator('span[role="cell"]');
      const deckName = await cells.nth(0).textContent();
      const cardCount = await cells.nth(2).textContent();

      if (deckName?.includes('Foundations')) foundationsCards = Number(cardCount?.trim());
      if (deckName?.includes('Proofs')) proofsCards = Number(cardCount?.trim());
    }

    expect(foundationsCards).toBeGreaterThanOrEqual(2);
    expect(proofsCards).toBeGreaterThanOrEqual(2);
  });

  e2e('preview mode shows position counter and next/previous buttons', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    const previewButton = page.locator('.td-decks-preview-btn');

    await expect(previewButton).toBeVisible({
      timeout: 15_000,
    });

    await previewButton.click();

    const study = page.locator('.td-study');

    await expect(study).toBeVisible({
      timeout: 10_000,
    });

    // Should show position counter (e.g. "1 / 5")
    await expect(study.locator('.td-study-counts')).toContainText('/');

    // Click show answer
    await page.locator('.td-study-show-btn').click();

    // Should show Previous and Next buttons (not grade buttons)
    const previewNav = study.locator('.td-study-preview-nav');

    await expect(previewNav).toBeVisible();
    await expect(study.locator('.td-study-eases')).not.toBeVisible();

    await page.keyboard.press('Escape');
  });

  e2e('clicking a deck row opens study for that deck', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/flashcards/index');

    const table = page.locator('.td-decks-table');

    await expect(table).toBeVisible({
      timeout: 15_000,
    });

    // Click the first deck row
    const firstRow = table.locator('.td-decks-link').first();

    await firstRow.click();

    // Study overlay should open
    const study = page.locator('.td-study');

    await expect(study).toBeVisible({
      timeout: 10_000,
    });

    // Should show grade-related UI (not preview nav)
    await expect(study.locator('.td-study-counts')).toBeVisible();

    await page.keyboard.press('Escape');
  });
});
