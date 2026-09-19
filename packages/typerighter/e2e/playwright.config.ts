import {
  defineConfig,
} from '@playwright/test';

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
      },
    },
    {
      name: 'base-path',
      use: {
        browserName: 'chromium',
      },
    },
  ],
});
