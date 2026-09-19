import {
  e2e, expect,
} from '../fixtures';

e2e.describe('dev server starts', () => {
  e2e('page loads and has content', async ({
    page, testProject,
  }) => {
    await page.goto(`http://localhost:${testProject.port}`);
    await expect(page).toHaveTitle(/.+/);
    const body = page.locator('body');

    await expect(body).not.toBeEmpty();
  });

  e2e('serves HTML with doctype', async ({
    testProject,
  }) => {
    const result = await fetch(`http://localhost:${testProject.port}`);
    const html = await result.text();

    expect(html.toLowerCase()).toContain('<!doctype html');
  });
});
