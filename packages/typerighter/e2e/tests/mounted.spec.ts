// Tests for typedown({ root }): mounting typedown inside a host Vite app.
// The vault lives under docs/ (not at the Vite project root), so the pages
// glob and loadPageModule must account for that path offset.

import { test } from '@playwright/test';
import { createMountedTestProjectFixture, expect, type TestProject } from '../fixtures';

const mounted = test.extend<{ testProject: TestProject }>({
  testProject: [
    createMountedTestProjectFixture('vault-mounted', 'docs'),
    { scope: 'test' },
  ],
});

mounted.describe('mounted typedown (root option)', () => {
  mounted('home page loads with vault content', async ({ page, testProject }) => {
    await testProject.goto(page);
    await expect(page.locator('body')).toContainText('Welcome to the mounted vault');
  });

  mounted('non-root page loads with correct content', async ({ page, testProject }) => {
    await testProject.goto(page, '/about');
    await expect(page.locator('body')).toContainText('About page content');
  });

  mounted('page title reflects site title from typedown.yaml', async ({ page, testProject }) => {
    await testProject.goto(page);
    await expect(page).toHaveTitle(/Mounted E2E Test/);
  });

  mounted('unknown path renders vue app (not a vite 404)', async ({ page, testProject }) => {
    await testProject.goto(page, '/does-not-exist');
    await expect(page.locator('#app')).not.toBeEmpty();
  });
});
