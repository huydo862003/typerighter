import fs from 'node:fs';
import path from 'node:path';
import {
  defineConfig,
} from '@playwright/test';

// On NixOS, set PLAYWRIGHT_CHROMIUM_EXECUTABLE in flake.nix shellHook to bypass revision check
// Falls back to scanning PLAYWRIGHT_BROWSERS_PATH if the env var is not set
function resolveChromiumExecutable (): string | undefined {
  const explicit = process.env['PLAYWRIGHT_CHROMIUM_EXECUTABLE'];

  if (explicit) return explicit;

  const browsersPath = process.env['PLAYWRIGHT_BROWSERS_PATH'];

  if (!browsersPath) return undefined;

  const directory = fs.readdirSync(browsersPath).find((entry) => entry.startsWith('chromium_headless_shell-'));

  if (!directory) return undefined;

  const binary = path.join(browsersPath, directory, 'chrome-headless-shell-linux64', 'chrome-headless-shell');

  return fs.existsSync(binary) ? binary : undefined;
}

const executablePath = resolveChromiumExecutable();

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  retries: 0,
  use: {
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'root',
      use: {
        browserName: 'chromium',
        ...(executablePath !== undefined && {
          launchOptions: {
            executablePath,
          },
        }),
      },
    },
    {
      name: 'base-path',
      use: {
        browserName: 'chromium',
        ...(executablePath !== undefined && {
          launchOptions: {
            executablePath,
          },
        }),
      },
    },
  ],
});
