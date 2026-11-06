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
    page, testProject,
  }) => {
    // override hover/pointer media features via CDP so CSS sees touch device
    const session = await page.context().newCDPSession(page);
    await session.send('Emulation.setEmulatedMedia', {
      features: [
        { name: 'hover', value: 'none' },
        { name: 'pointer', value: 'none' },
      ],
    });
    await testProject.goto(page, '/notes/index');

    const inlineToc = page.getByTestId('toc').first();
    const railToc = page.getByTestId('toc').last();

    await expect(inlineToc).toBeVisible({ timeout: 15_000 });
    await expect(railToc).toBeHidden();

    await expect(inlineToc.getByRole('link', { name: 'Overview' })).toBeVisible();
    await expect(inlineToc.getByRole('link', { name: 'Details', exact: true })).toBeVisible();
    await expect(inlineToc.getByRole('link', { name: 'Summary' })).toBeVisible();
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
