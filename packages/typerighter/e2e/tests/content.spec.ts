import {
  e2e, expect,
} from '../fixtures';

e2e.describe('content', () => {
  e2e('rail TOC is visible on mouse devices', async ({
    page, testProject,
  }) => {
    await testProject.goto(page, '/notes/index');

    // default chromium context is a mouse device (hover: hover, pointer: fine)
    // rail TOC is the last [data-testid="toc"] in the DOM
    const railToc = page.getByTestId('toc').last();

    await expect(railToc).toBeVisible({ timeout: 15_000 });

    await expect(railToc.getByRole('link', { name: 'Overview' })).toBeVisible();
    await expect(railToc.getByRole('link', { name: 'Details', exact: true })).toBeVisible();
    await expect(railToc.getByRole('link', { name: 'Summary' })).toBeVisible();
  });

  e2e('inline TOC is visible on touch devices, rail is hidden', async ({
    browser, testProject,
  }) => {
    // isMobile sets hover:none and pointer:coarse at the browser level so CSS
    // media queries see a touch device (CDP setEmulatedMedia is unreliable here)
    const touchContext = await browser.newContext({ isMobile: true });
    const touchPage = await touchContext.newPage();

    await touchPage.goto(testProject.url('/notes/index'));
    await touchPage.waitForFunction(
      () => 0 < (document.querySelector('#app')?.children.length ?? 0),
      { timeout: 30_000 },
    );

    const inlineToc = touchPage.getByTestId('toc').first();
    const railToc = touchPage.getByTestId('toc').last();

    await expect(inlineToc).toBeVisible({ timeout: 15_000 });
    await expect(railToc).toBeHidden();

    await expect(inlineToc.getByRole('link', { name: 'Overview' })).toBeVisible();
    await expect(inlineToc.getByRole('link', { name: 'Details', exact: true })).toBeVisible();
    await expect(inlineToc.getByRole('link', { name: 'Summary' })).toBeVisible();

    await touchContext.close();
  });

  e2e(
    'edit link points to the correct source file',
    async ({
      page, testProject,
    }) => {
      await testProject.goto(page, '/notes/index');

      const editLink = page.getByRole('link', {
        name: 'Edit this page',
      });

      await expect(editLink).toBeVisible({
        timeout: 15_000,
      });

      await expect(editLink).toHaveAttribute(
        'href',
        'https://github.com/test/repo/edit/main/vault/notes/index.td',
      );
    },
  );

  e2e(
    'active sidebar item is visible after navigation',
    async ({
      page, testProject,
    }) => {
      await testProject.goto(page, '/people/alice');

      const sidebar = page.getByTestId('sidebar');
      const activeLink = sidebar
        .getByRole('link', {
          name: /Alice/,
        })
        .first();

      await expect(activeLink).toBeVisible({
        timeout: 15_000,
      });
      await expect(activeLink).toBeInViewport();
    },
  );
});
