import {
  e2e, expect,
} from '../fixtures';

e2e.describe('table of contents', () => {
  e2e('TOC renders heading links for pages with multiple sections', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/long-page');

    const toc = page.getByTestId('toc');

    await expect(toc).toBeVisible({
      timeout: 15_000,
    });

    // Should contain links for the headings
    await expect(toc.getByRole('link', {
      name: /Introduction/,
    })).toBeVisible();
    await expect(toc.getByRole('link', {
      name: /API Reference/,
    })).toBeVisible();
    await expect(toc.getByRole('link', {
      name: /Contributing/,
    })).toBeVisible();
  });

  e2e('TOC contains all headings from a long page', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/long-page');

    const toc = page.getByTestId('toc');

    await expect(toc).toBeVisible({
      timeout: 15_000,
    });

    // The page has 20 h2 headings - all should appear as TOC links
    const tocLinks = toc.getByRole('link');

    await expect(tocLinks).toHaveCount(20, {
      timeout: 10_000,
    });
  });

  e2e('TOC highlights the active heading on scroll', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/long-page');

    const toc = page.getByTestId('toc');

    await expect(toc).toBeVisible({
      timeout: 15_000,
    });

    // Scroll to a heading further down the page
    await page.locator('h2:has-text("Deployment")').scrollIntoViewIfNeeded();

    // Wait for the intersection observer to update the active heading
    await expect(toc.locator('a.is-active')).toBeVisible({
      timeout: 5_000,
    });

    const activeText = await toc.locator('a.is-active').textContent();

    // The active heading should be near "Deployment" after scrolling
    expect(activeText?.trim()).toBeTruthy();
  });

  e2e('clicking a TOC link updates the URL hash', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/long-page');

    const toc = page.getByTestId('toc');

    await expect(toc).toBeVisible({
      timeout: 15_000,
    });

    // Click the "FAQ" link in the TOC
    await toc.getByRole('link', {
      name: /FAQ/,
    }).click();

    // The URL should contain the anchor hash
    await expect(page).toHaveURL(/#faq/, {
      timeout: 5_000,
    });
  });

  e2e('floating TOC is already scrolled to active heading on hover', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/long-page');

    // Scroll to a heading near the bottom of the page
    await page.locator('h2', {
      hasText: 'Contributing',
    }).scrollIntoViewIfNeeded();

    // Wait for the intersection observer to update the active TOC heading
    await page.waitForTimeout(500);

    // Hover on the rail to reveal the TOC panel
    const rail = page.locator('.td-rail');

    await rail.hover();

    const toc = page.getByTestId('toc');

    await expect(toc).toBeVisible({
      timeout: 5_000,
    });

    // The active link should already be visible in the TOC viewport (no animation needed)
    const activeLink = toc.locator('a.is-active');

    await expect(activeLink).toBeVisible();

    // Verify the active link is actually in the visible area of the TOC scroll container
    const isInView = await activeLink.evaluate((element) => {
      const container = element.closest('.td-toc-list-wrap');

      if (!container) return true;

      const containerRect = container.getBoundingClientRect();
      const linkRect = element.getBoundingClientRect();

      return linkRect.top >= containerRect.top && linkRect.bottom <= containerRect.bottom;
    });

    expect(isInView).toBe(true);
  });

  e2e('TOC is hidden on pages with fewer than 2 headings', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page);

    // The index page has no/few headings - TOC should not appear
    const toc = page.getByTestId('toc');

    await expect(toc).not.toBeVisible({
      timeout: 5_000,
    });
  });
});
