import {
  readFile, access,
} from 'node:fs/promises';
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
    await expect(page.locator('.td-data-table')).toHaveAttribute('data-striped');
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let contract: Record<string, any>;

  components.beforeEach(async ({
    testProject,
  }) => {
    contract = JSON.parse(
      await readFile(`${testProject.dir}/.typedown/components.json`, 'utf-8'),
    );
  });

  components('components.json contains all registered components', async () => {
    expect(Object.keys(contract)).toEqual(
      expect.arrayContaining([
        'TdAlert',
        'TdDataTable',
        'TdTabs',
        'TdCard',
        'TdComplex',
      ]),
    );
  });

  components('string prop type is mapped correctly', async () => {
    // Required props: { type: "string" }; optional props: { type: ["string", "null"] }
    expect(contract.TdCard.props.title).toEqual({
      type: 'string',
    });
    expect(contract.TdCard.props.icon).toEqual({
      type: [
        'string',
        'null',
      ],
    });
  });

  components('boolean prop type is mapped correctly', async () => {
    expect(contract.TdDataTable.props.striped).toEqual({
      type: [
        'boolean',
        'null',
      ],
    });
  });

  components('number prop type is mapped correctly', async () => {
    expect(contract.TdTabs.props.active).toEqual({
      type: [
        'number',
        'null',
      ],
    });
    expect(contract.TdTabs.props.labels).toEqual({
      type: 'string',
    });
  });

  components('unsupported prop type is omitted, component still registered', async () => {
    // TdComplex has title: string (kept) and data: Record<string, unknown> (omitted)
    expect(contract.TdComplex).toBeDefined();
    expect(contract.TdComplex.props.title).toEqual({
      type: 'string',
    });
    expect(contract.TdComplex.props.data).toBeUndefined();
  });
});

// Fixture variant that starts without a pre-existing components.json
const componentsFresh = test.extend<{
  testProject: TestProject;
}>({
  testProject: [
    createTestProjectFixture('vault-components', {
      withoutContract: true,
    }),
    {
      scope: 'test',
    },
  ],
});

// Wait for the plugin to write the file (it writes async during configureServer)
async function waitForContract (filePath: string, timeoutMs = 15_000): Promise<string> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    try {
      return await readFile(filePath, 'utf-8');
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
  }
  throw new Error(`components.json not created within ${timeoutMs}ms`);
}

componentsFresh.describe('contract (fresh)', () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let contract: Record<string, any>;

  componentsFresh.beforeEach(async ({
    testProject,
  }) => {
    const raw = await waitForContract(`${testProject.dir}/.typedown/components.json`);

    contract = JSON.parse(raw);
  });

  componentsFresh('plugin generates components.json from scratch on startup', async () => {
    expect(Object.keys(contract)).toEqual(
      expect.arrayContaining([
        'TdAlert',
        'TdDataTable',
        'TdTabs',
        'TdCard',
        'TdComplex',
      ]),
    );
  });

  componentsFresh('generated contract has correct prop types', async () => {
    expect(contract.TdTabs.props.labels).toEqual({
      type: 'string',
    });
    expect(contract.TdTabs.props.active).toEqual({
      type: [
        'number',
        'null',
      ],
    });
    expect(contract.TdDataTable.props.striped).toEqual({
      type: [
        'boolean',
        'null',
      ],
    });
    expect(contract.TdCard.props.title).toEqual({
      type: 'string',
    });
    expect(contract.TdCard.props.footer).toEqual({
      type: [
        'string',
        'null',
      ],
    });
  });

  componentsFresh('unsupported prop type omitted in generated contract', async () => {
    expect(contract.TdComplex.props.title).toEqual({
      type: 'string',
    });
    expect(contract.TdComplex.props.data).toBeUndefined();
  });
});

// Vault with no theme.components config - plugin should not create components.json
const componentsNone = test.extend<{
  testProject: TestProject;
}>({
  testProject: [
    createTestProjectFixture('vault-root'),
    {
      scope: 'test',
    },
  ],
});

componentsNone.describe('contract (no components)', () => {
  componentsNone('components.json is not created when no components are configured', async ({
    testProject,
  }) => {
    // Server is already responsive by the time the test runs, so any contract write has occurred
    await expect(access(`${testProject.dir}/.typedown/components.json`)).rejects.toThrow();
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

components.describe('component check', () => {
  // Positive: correct usage renders without issues
  components('known component with required prop renders correctly', async ({
    page,
  }) => {
    await expect(page.locator('.td-card .td-card__title')).toHaveText('Getting started');
  });

  // Negative: unknown component emits diagnostic, page still renders
  // Uses check-unknown-component.td in the fixture (checked at startup)
  components('unknown component emits warning', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/check-unknown-component');
    await expect(page.locator('.td-content')).toBeVisible({
      timeout: 15_000,
    });

    const hasWarning = testProject.serverStderr.some(
      (line) => line.includes('td-nonexistent'),
    );

    expect(hasWarning).toBe(true);
  });

  // Negative: missing required prop emits diagnostic, component still renders
  // Uses check-missing-prop.td in the fixture (TdCard without title)
  components('missing required prop emits warning', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/check-missing-prop');
    await expect(page.locator('.td-content')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.locator('.td-card')).toBeVisible({
      timeout: 15_000,
    });

    const hasWarning = testProject.serverStderr.some(
      (line) => line.includes('title'),
    );

    expect(hasWarning).toBe(true);
  });

  // Negative: unknown prop emits diagnostic, component still renders
  // Uses check-unknown-prop.td in the fixture (TdAlert with severity prop)
  components('unknown prop emits warning', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/check-unknown-prop');
    await expect(page.locator('.td-content')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.locator('.td-alert')).toBeVisible({
      timeout: 15_000,
    });

    const hasWarning = testProject.serverStderr.some(
      (line) => line.includes('severity'),
    );

    expect(hasWarning).toBe(true);
  });
});

components.describe('layout slots', () => {
  components('layout_top slot renders banner above header', async ({
    page,
  }) => {
    const banner = page.locator('[data-testid="layout-slot-banner"]');

    await expect(banner).toBeVisible({
      timeout: 15_000,
    });
    await expect(banner).toContainText('Test announcement banner');
  });

  components('doc_after slot renders feedback widget after content', async ({
    page,
  }) => {
    const footer = page.locator('[data-testid="layout-slot-doc-footer"]');

    await expect(footer).toBeVisible({
      timeout: 15_000,
    });
    await expect(footer).toContainText('Was this page helpful?');
  });

  components('custom layout component replaces default layout', async ({
    page,
    testProject,
  }) => {
    // Switch to a fully custom layout by rewriting the config
    await testProject.writeFile(
      'typedown.yaml',
      [
        'version: "1"',
        'vault:',
        '  root_dir: "vault"',
        'site:',
        '  title: "E2E Component Vault"',
        '  description: "Test fixture"',
        'theme:',
        '  components:',
        '    - "./components"',
        '  layout: "./custom-layout/TdCustomLayout.vue"',
      ].join('\n'),
    );

    await testProject.goto(page, '/demo');

    await expect(page.locator('[data-testid="custom-layout"]')).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.locator('[data-testid="custom-header"]')).toContainText('Custom Layout');
    // Default header should not be present
    await expect(page.locator('.td-header')).not.toBeVisible();
  });

  components('layout_top slot appears before the header', async ({
    page,
  }) => {
    const banner = page.locator('[data-testid="layout-slot-banner"]');
    const header = page.locator('.td-header');

    await expect(banner).toBeVisible({
      timeout: 15_000,
    });

    const bannerBox = await banner.boundingBox();
    const headerBox = await header.boundingBox();

    expect(bannerBox).not.toBeNull();
    expect(headerBox).not.toBeNull();
    expect(bannerBox!.y).toBeLessThan(headerBox!.y);
  });
});
