import {
  e2e, expect,
} from '../fixtures';

e2e.describe('content', () => {
  e2e('TOC shows h2 and h3 headings', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/notes/index');

    const toc = page.getByTestId('toc');

    await expect(toc).toBeVisible({
      timeout: 15_000,
    });

    await expect(toc.getByRole('link', {
      name: 'Overview',
    })).toBeVisible();
    await expect(toc.getByRole('link', {
      name: 'Details',
    })).toBeVisible();
    await expect(toc.getByRole('link', {
      name: 'Summary',
    })).toBeVisible();
  });

  e2e('edit link points to the correct source file', async ({
    page,
    testProject,
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
      'https://github.com/test/repo/edit/main/notes/index.td',
    );
  });

  e2e('active sidebar item is visible after navigation', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/people/alice');

    const sidebar = page.getByTestId('sidebar');
    const activeLink = sidebar.getByRole('link', {
      name: /Alice/,
    }).first();

    await expect(activeLink).toBeVisible({
      timeout: 15_000,
    });
    await expect(activeLink).toBeInViewport();
  });
});
