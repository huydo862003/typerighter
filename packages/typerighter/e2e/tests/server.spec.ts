import {
  e2e, expect,
} from '../fixtures';

e2e.describe('dev server starts', () => {
  e2e('page loads and has content', async ({
    page, testProject,
  }) => {
    await testProject.goto(page);
    await expect(page).toHaveTitle(/.+/);
  });

  e2e('serves HTML with doctype', async ({
    testProject,
  }) => {
    const result = await fetch(testProject.url());
    const html = await result.text();

    expect(html.toLowerCase()).toContain('<!doctype html');
  });
});
