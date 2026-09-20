import {
  mkdtemp, cp, rm, writeFile, readFile, symlink,
} from 'node:fs/promises';
import {
  tmpdir,
} from 'node:os';
import path from 'node:path';
import {
  type ChildProcess, spawn,
} from 'node:child_process';
import {
  test, type Page,
} from '@playwright/test';

const FIXTURES_DIR = path.resolve(import.meta.dirname, 'fixtures');
const BIN = path.resolve(import.meta.dirname, '../bin/typerighter.js');

export type FixtureName = 'vault-root' | 'vault-base';

export interface TestProject {
  // Absolute path to the isolated temp copy
  dir: string;
  // Port the dev server is running on
  port: number;
  // Base path from typedown.yaml, empty string for root
  basePath: string;
  // Full URL for a vault-relative path
  url: (vaultPath?: string) => string;
  // Navigate and wait for the Vue app to render (CSR needs JS to execute)
  goto: (page: Page, vaultPath?: string) => Promise<void>;
  // Modify a vault file in place
  modifyFile: (
    relativePath: string,
    transform: (content: string) => string,
  ) => Promise<void>;
}

function createTestProjectFixture (fixture: FixtureName) {
  return async ({}, use: (project: TestProject) => Promise<void>) => {
    const fixtureDirectory = path.join(FIXTURES_DIR, fixture);

    // Create isolated temp copy
    // Skip .typedown (stale cache) and node_modules (symlinked separately for speed)
    const directory = await mkdtemp(path.join(tmpdir(), `typerighter-e2e-${fixture}-`));

    await cp(fixtureDirectory, directory, {
      recursive: true,
      // Dereference symlinks so tests modify the copy, not the source
      dereference: true,
      filter: (src) => {
        const name = path.basename(src);

        return name !== '.typedown' && name !== 'node_modules';
      },
    });

    // Symlink node_modules from the fixture source (resolved by pnpm workspace)
    await symlink(
      path.join(fixtureDirectory, 'node_modules'),
      path.join(directory, 'node_modules'),
      'dir',
    );

    const port = await getFreePort();
    const basePath = await readBasePath(directory);

    const serverProcess: ChildProcess = spawn(
      'node',
      [
        BIN,
        'dev',
        '--port',
        String(port),
      ],
      {
        cwd: directory,
        // Inherit stderr for debug, ignore stdout to avoid pipe buffer blocking
        stdio: [
          'ignore',
          'ignore',
          'inherit',
        ],
        env: {
          ...process.env,
          NODE_ENV: 'development',
        },
      },
    );

    await waitForServer(`http://localhost:${port}${basePath}/`, 30_000);

    const url = (vaultPath = '') => `http://localhost:${port}${basePath}${vaultPath}`;

    const goto = async (page: Page, vaultPath = '') => {
      await page.goto(url(vaultPath));
      await page.waitForFunction(
        () => 0 < document.querySelector('#app')?.children.length ?? 0,
        {
          timeout: 15_000,
        },
      );
    };

    const modifyFile = async (
      relativePath: string,
      transform: (content: string) => string,
    ) => {
      const filePath = path.join(directory, relativePath);
      const original = await readFile(filePath, 'utf-8');

      await writeFile(filePath, transform(original), 'utf-8');
    };

    await use({
      dir: directory,
      port,
      basePath,
      url,
      goto,
      modifyFile,
    });

    serverProcess.kill('SIGTERM');
    // Wait for the server to finish writing cache before removing the temp dir
    await new Promise<void>((resolve) => {
      serverProcess.on('exit', resolve);
      setTimeout(resolve, 5_000);
    });
    await rm(directory, {
      recursive: true,
      force: true,
    }).catch(() => {});
  };
}

// Find a free port by binding to 0 and releasing
async function getFreePort (): Promise<number> {
  const {
    createServer,
  } = await import('node:net');

  return new Promise((resolve, reject) => {
    const server = createServer();

    server.listen(0, () => {
      const address = server.address();

      if (address === null || typeof address === 'string') {
        server.close(() => reject(new Error('Could not get port')));

        return;
      }
      server.close(() => resolve(address.port));
    });
  });
}

// Read base_path from typedown.yaml
async function readBasePath (directory: string): Promise<string> {
  const yaml = await readFile(path.join(directory, 'typedown.yaml'), 'utf-8');
  const match = yaml.match(/base_path:\s*"?([^"\n]+)"?/);

  return match?.[1]?.trim() ?? '';
}

// Picks vault-root or vault-base based on the Playwright project name
export const e2e = test.extend<{
  testProject: TestProject;
}>({
  testProject: [
    async ({}, use, testInfo) => {
      const fixture: FixtureName = testInfo.project.name === 'base-path'
        ? 'vault-base'
        : 'vault-root';

      await createTestProjectFixture(fixture)({}, use);
    },
    {
      scope: 'test',
    },
  ],
});

async function waitForServer (url: string, timeoutMs: number): Promise<void> {
  const start = Date.now();

  while (Date.now() - start < timeoutMs) {
    try {
      const result = await fetch(url);

      if (result.ok) return;
    } catch {
      // Server not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Server at ${url} did not start within ${timeoutMs}ms`);
}

export {
  expect,
} from '@playwright/test';
