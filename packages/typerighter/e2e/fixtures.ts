import {
  mkdtemp, cp, rm, writeFile, readFile,
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

const EXAMPLES_DIR = path.resolve(import.meta.dirname, '../../../examples/project_tracker');
const BIN = path.resolve(import.meta.dirname, '../bin/typerighter.js');

export interface TestProject {
  // Absolute path to the isolated temp copy
  dir: string;
  // Port the dev server is running on
  port: number;
  // Modify a .td file and wait for HMR to propagate
  modifyFileAndWaitForHMR: (
    page: Page,
    relativePath: string,
    transform: (content: string) => string,
  ) => Promise<void>;
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
      const port = address.port;

      server.close(() => resolve(port));
    });
  });
}

export const e2e = test.extend<{
  testProject: TestProject;
}>({
  testProject: async ({}, use) => {
    // Create isolated temp copy, skip cache and node_modules
    const directory = await mkdtemp(path.join(tmpdir(), 'typerighter-e2e-'));

    await cp(EXAMPLES_DIR, directory, {
      recursive: true,
      filter: (src) => {
        const name = path.basename(src);

        return name !== '.typedown' && name !== 'node_modules';
      },
    });

    // Symlink node_modules from the original project
    const {
      symlink,
    } = await import('node:fs/promises');

    await symlink(
      path.join(EXAMPLES_DIR, 'node_modules'),
      path.join(directory, 'node_modules'),
      'dir',
    );

    const port = await getFreePort();

    // Start dev server
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
        stdio: [
          'ignore',
          'pipe',
          'pipe',
        ],
        env: {
          ...process.env,
          NODE_ENV: 'development',
        },
      },
    );

    // Wait for server to be ready by polling
    await waitForServer(`http://localhost:${port}`, 30_000);

    const modifyFileAndWaitForHMR = async (
      page: Page,
      relativePath: string,
      transform: (content: string) => string,
    ) => {
      const filePath = path.join(directory, relativePath);
      const original = await readFile(filePath, 'utf-8');
      const modified = transform(original);

      // Listen for HMR update before writing
      const hmrPromise = page.waitForEvent('console', {
        predicate: (message) => message.text().includes('[vite] hot updated'),
        timeout: 10_000,
      }).catch(() => {
        // Fall back to waiting for content change if console message is not emitted
      });

      await writeFile(filePath, modified, 'utf-8');
      await hmrPromise;
      // Small settle time for DOM updates
      await page.waitForTimeout(500);
    };

    await use({
      dir: directory,
      port,
      modifyFileAndWaitForHMR,
    });

    // Cleanup
    serverProcess.kill('SIGTERM');
    await rm(directory, {
      recursive: true,
      force: true,
    });
  },
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
