import {
  e2e, expect,
} from '../../fixtures';

e2e.describe('SSG hidden folder navigation', () => {
  e2e('hidden page shows prev/next scoped to its folder', async ({
    page,
    testProject,
  }) => {
    // Navigate directly to the middle hidden page
    await testProject.goto(page, '/hidden-folder/bravo');

    const nav = page.locator('nav[aria-label="Page navigation"]');

    // Should show prev/next within the hidden folder
    await expect(nav).toBeVisible({
      timeout: 10_000,
    });

    // Previous should be Alpha
    const previousLink = nav.locator('.td-prevnext-link.is-previous');

    await expect(previousLink).toBeVisible();
    await expect(previousLink.locator('.td-prevnext-title')).toHaveText('Alpha (Hidden)');

    // Next should be Charlie
    const nextLink = nav.locator('.td-prevnext-link.is-next');

    await expect(nextLink).toBeVisible();
    await expect(nextLink.locator('.td-prevnext-title')).toHaveText('Charlie (Hidden)');
  });

  e2e('first hidden page has no previous link', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/hidden-folder/alpha');

    const nav = page.locator('nav[aria-label="Page navigation"]');

    await expect(nav).toBeVisible({
      timeout: 10_000,
    });

    // No previous link
    await expect(nav.locator('.td-prevnext-link.is-previous')).not.toBeVisible();

    // Next should be Bravo
    const nextLink = nav.locator('.td-prevnext-link.is-next');

    await expect(nextLink).toBeVisible();
    await expect(nextLink.locator('.td-prevnext-title')).toHaveText('Bravo (Hidden)');
  });

  e2e('last hidden page has no next link', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/hidden-folder/charlie');

    const nav = page.locator('nav[aria-label="Page navigation"]');

    await expect(nav).toBeVisible({
      timeout: 10_000,
    });

    // Previous should be Bravo
    const previousLink = nav.locator('.td-prevnext-link.is-previous');

    await expect(previousLink).toBeVisible();
    await expect(previousLink.locator('.td-prevnext-title')).toHaveText('Bravo (Hidden)');

    // No next link
    await expect(nav.locator('.td-prevnext-link.is-next')).not.toBeVisible();
  });

  e2e('hidden page shows edit link at the bottom', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/hidden-folder/alpha');

    const editLink = page.locator('.td-page-edit');

    await expect(editLink).toBeVisible({
      timeout: 10_000,
    });
    await expect(editLink).toHaveText('Edit this page');
  });
});
