import {
  test as base,
} from '@playwright/test';
import {
  expect, createTestProjectFixture, type TestProject,
} from '../fixtures';

const components = base.extend<{ testProject: TestProject }>({
  testProject: [createTestProjectFixture('vault-components'), {
    scope: 'test',
  }],
});

components.describe('custom components', () => {
  components('component renders in page', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/demo');

    const alert = page.locator('td-alert').first();

    await expect(alert).toBeVisible({
      timeout: 15_000,
    });
  });

  components('prop is passed through to component', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/demo');

    const alertWithKind = page.locator('td-alert[kind="info"]');

    await expect(alertWithKind).toBeVisible({
      timeout: 15_000,
    });
  });

  components('component slot content renders', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/demo');

    const alert = page.locator('td-alert').first();

    await expect(alert).toContainText('This is an alert component.');
  });

  components('components.json contract has expected shape', async ({
    testProject,
  }) => {
    const {
      readFile,
    } = await import('node:fs/promises');
    const contractPath = `${testProject.dir}/.typedown/components.json`;
    const content = await readFile(contractPath, 'utf-8');
    const contract = JSON.parse(content);

    expect(contract).toHaveProperty('TdAlert');
    expect(contract.TdAlert).toHaveProperty('source');
    expect(contract.TdAlert.props).toHaveProperty('kind');
  });

  components('new component added at runtime becomes available after HMR', async ({
    page,
    testProject,
  }) => {
    await testProject.goto(page, '/demo');

    // Add a new component file and contract entry
    await testProject.writeFile(
      'components/TdBadge.vue',
      '<template><span class="td-badge"><slot /></span></template>',
    );
    await testProject.writeFile(
      '.typedown/components.json',
      JSON.stringify({
        TdAlert: {
          source: './components/TdAlert.vue',
          props: {
            kind: {
              type: 'string',
              required: false,
            },
          },
        },
        TdBadge: {
          source: './components/TdBadge.vue',
          props: {},
        },
      }, undefined, 2),
    );
    await testProject.modifyFile('vault/demo.td', (content) => content + '\n:::td-badge\nNew badge\n:::\n');

    await expect(page.locator('td-badge')).toBeVisible({
      timeout: 15_000,
    });
  });
});
