import {
  test,
} from '@playwright/test';
import {
  expect, createTestProjectFixture, type TestProject,
} from '../fixtures';

const components = test.extend<{
  testProject: TestProject;
}>({
  testProject: [
    createTestProjectFixture('vault-components'),
    {
      scope: 'test',
    },
  ],
});

// Navigate to /demo once before each test
components.beforeEach(async ({
  page, testProject,
}) => {
  await testProject.goto(page, '/demo');
});

// Vue replaces custom element tags with the component's root element
// :::td-alert becomes <div class="td-alert"> in the DOM, so selectors use class names
components.describe('TdAlert', () => {
  components('renders in page', async ({
    page,
  }) => {
    await expect(page.locator('.td-alert').first()).toBeVisible({
      timeout: 15_000,
    });
  });

  components('kind prop is passed through as data-kind', async ({
    page,
  }) => {
    await expect(page.locator('.td-alert[data-kind="info"]')).toBeVisible({
      timeout: 15_000,
    });
  });

  components('slot content renders inside component', async ({
    page,
  }) => {
    await expect(page.locator('.td-alert').first()).toContainText('This is an alert component.');
  });
});

components.describe('TdDataTable', () => {
  components('renders component element', async ({
    page,
  }) => {
    await expect(page.locator('.td-data-table')).toBeVisible({
      timeout: 15_000,
    });
  });

  components('caption prop renders as figcaption', async ({
    page,
  }) => {
    const caption = page.locator('.td-data-table .td-data-table__caption');

    await expect(caption).toBeVisible({
      timeout: 15_000,
    });
    await expect(caption).toHaveText('Monthly sales');
  });

  components('striped prop sets data-striped attribute', async ({
    page,
  }) => {
    await expect(page.locator('.td-data-table')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.locator('.td-data-table')).toHaveAttribute('data-striped', 'true');
  });

  components('markdown table in slot renders as <table> inside component', async ({
    page,
  }) => {
    const table = page.locator('.td-data-table table');

    await expect(table).toBeVisible({
      timeout: 15_000,
    });
    await expect(table).toContainText('jan');
    await expect(table).toContainText('$1,200');
  });
});

components.describe('TdTabs', () => {
  components('renders tab bar with correct labels', async ({
    page,
  }) => {
    const tabs = page.locator('.td-tabs [role="tab"]');

    await expect(page.locator('.td-tabs')).toBeVisible({
      timeout: 15_000,
    });
    await expect(tabs).toHaveCount(3);
    await expect(tabs.nth(0)).toHaveText('Overview');
    await expect(tabs.nth(1)).toHaveText('Details');
    await expect(tabs.nth(2)).toHaveText('Notes');
  });

  components('active prop sets initial active tab', async ({
    page,
  }) => {
    await expect(page.locator('.td-tabs')).toBeVisible({
      timeout: 15_000,
    });
    // active="1" -> Details is initially active (0-indexed)
    await expect(page.locator('.td-tabs')).toHaveAttribute('data-active-tab', '1');
    await expect(page.locator('.td-tabs [role="tab"][aria-selected="true"]')).toHaveText('Details');
  });

  components('clicking a tab updates active state reactively', async ({
    page,
  }) => {
    await expect(page.locator('.td-tabs')).toBeVisible({
      timeout: 15_000,
    });

    await page.locator('.td-tabs [role="tab"]').nth(2)
      .click();

    await expect(page.locator('.td-tabs')).toHaveAttribute('data-active-tab', '2');
    await expect(page.locator('.td-tabs [role="tab"][aria-selected="true"]')).toHaveText('Notes');
  });

  components('slot markdown with bold and list renders inside panel', async ({
    page,
  }) => {
    const panel = page.locator('.td-tabs .td-tabs__panel');

    await expect(panel).toBeVisible({
      timeout: 15_000,
    });
    await expect(panel).toContainText('This is the tab panel content.');
    await expect(panel.locator('strong')).toHaveText('bold text');
    await expect(panel.locator('li')).toHaveCount(2);
  });
});

components.describe('TdCard', () => {
  components('renders with title', async ({
    page,
  }) => {
    await expect(page.locator('.td-card')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.locator('.td-card .td-card__title')).toHaveText('Getting started');
  });

  components('icon prop renders in header', async ({
    page,
  }) => {
    const icon = page.locator('.td-card .td-card__icon');

    await expect(icon).toBeVisible({
      timeout: 15_000,
    });
    await expect(icon).toHaveText('🚀');
  });

  components('footer prop renders conditionally', async ({
    page,
  }) => {
    await expect(page.locator('.td-card .td-card__footer')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.locator('.td-card .td-card__footer')).toHaveText('Last updated: today');
  });

  components('data-has-footer attribute reflects footer presence', async ({
    page,
  }) => {
    await expect(page.locator('.td-card')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.locator('.td-card')).toHaveAttribute('data-has-footer', 'true');
  });

  components('slot markdown ordered list renders inside card body', async ({
    page,
  }) => {
    const items = page.locator('.td-card .td-card__body ol li');

    await expect(page.locator('.td-card')).toBeVisible({
      timeout: 15_000,
    });
    await expect(items).toHaveCount(3);
    await expect(items.nth(0)).toContainText('Install the package');
    await expect(items.nth(2)).toContainText('Run the dev server');
  });
});

components.describe('contract', () => {
  components('components.json has expected shape for all components', async ({
    testProject,
  }) => {
    const {
      readFile,
    } = await import('node:fs/promises');
    const contract = JSON.parse(
      await readFile(`${testProject.dir}/.typedown/components.json`, 'utf-8'),
    );

    expect(Object.keys(contract)).toEqual(
      expect.arrayContaining([
        'TdAlert',
        'TdDataTable',
        'TdTabs',
        'TdCard',
      ]),
    );
    expect(contract.TdTabs.props.labels.required).toBe(true);
    expect(contract.TdCard.props.title.required).toBe(true);
    expect(contract.TdDataTable.props.caption.required).toBe(false);
  });
});

components.describe('HMR', () => {
  components('new component added at runtime becomes available after HMR', async ({
    page,
    testProject,
  }) => {
    await testProject.writeFile(
      'components/TdBadge.vue',
      '<template><span class="td-badge"><slot /></span></template>',
    );

    // Read existing contract and add TdBadge without clobbering other entries
    const {
      readFile,
    } = await import('node:fs/promises');
    const existing = JSON.parse(
      await readFile(`${testProject.dir}/.typedown/components.json`, 'utf-8'),
    );

    await testProject.writeFile(
      '.typedown/components.json',
      JSON.stringify({
        ...existing,
        TdBadge: {
          source: './components/TdBadge.vue',
          props: {},
        },
      }, undefined, 2),
    );
    await testProject.modifyFile(
      'vault/demo.td',
      (content) => `${content}\n:::td-badge\nNew badge\n:::\n`,
    );

    // Reload the page so the app module re-evaluates with the new component import
    await testProject.goto(page, '/demo');

    await expect(page.locator('.td-badge')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.locator('.td-badge')).toContainText('New badge');
  });
});
