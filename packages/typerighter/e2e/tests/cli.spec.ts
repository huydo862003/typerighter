import {
  mkdir,
  mkdtemp,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises';
import {
  tmpdir,
} from 'node:os';
import path from 'node:path';
import {
  execFileSync,
  spawnSync,
} from 'node:child_process';
import {
  test, expect,
} from '@playwright/test';

const BIN = path.resolve(import.meta.dirname, '../../bin/typerighter.js');

test('init: creates scaffold files', async () => {
  const directory = await mkdtemp(path.join(tmpdir(), 'typerighter-init-'));

  try {
    execFileSync('node', [
      BIN,
      'init',
      directory,
      '--name',
      'test-vault',
      '--title',
      'Test Vault',
      '--description',
      'A test vault',
    ], {
      stdio: 'pipe',
      timeout: 30_000,
    });

    const yaml = await readFile(path.join(directory, 'typedown.yaml'), 'utf-8');
    const package_ = await readFile(path.join(directory, 'package.json'), 'utf-8');

    expect(yaml).toContain('title: "Test Vault"');
    expect(yaml).toContain('description: "A test vault"');
    expect(JSON.parse(package_).name).toBe('test-vault');
    expect(JSON.parse(package_).devDependencies).toHaveProperty('typerighter');

    const index = await readFile(path.join(directory, 'vault', 'index.td'), 'utf-8');

    expect(index).toContain('_type: Article');
  } finally {
    await rm(directory, {
      force: true,
      recursive: true,
    }).catch(() => {});
  }
});

test('init: skips existing files on re-run', async () => {
  const directory = await mkdtemp(path.join(tmpdir(), 'typerighter-init-'));

  const initializeArgs = [
    BIN,
    'init',
    directory,
    '--name',
    'test-vault',
    '--title',
    'Test Vault',
    '--description',
    'A test vault',
  ];

  try {
    execFileSync('node', initializeArgs, {
      stdio: 'pipe',
      timeout: 30_000,
    });

    // Write a sentinel value to an existing file
    await writeFile(path.join(directory, 'typedown.yaml'), '# sentinel\n');

    execFileSync('node', initializeArgs, {
      stdio: 'pipe',
      timeout: 30_000,
    });

    // File should not be overwritten
    const yaml = await readFile(path.join(directory, 'typedown.yaml'), 'utf-8');

    expect(yaml).toBe('# sentinel\n');
  } finally {
    await rm(directory, {
      force: true,
      recursive: true,
    }).catch(() => {});
  }
});

test('setup-editor neovim: appends config snippet to init.lua', async () => {
  const xdgDirectory = await mkdtemp(path.join(tmpdir(), 'typerighter-xdg-'));
  const nvimDirectory = path.join(xdgDirectory, 'nvim');

  try {
    await mkdir(nvimDirectory, {
      recursive: true,
    });
    await writeFile(path.join(nvimDirectory, 'init.lua'), ['require("lazy").setup({})'].join('\n'));

    execFileSync('node', [
      BIN,
      'setup-editor',
      'neovim',
    ], {
      env: {
        ...process.env,
        XDG_CONFIG_HOME: xdgDirectory,
      },
      stdio: 'pipe',
      timeout: 30_000,
    });

    const initializeLua = await readFile(path.join(nvimDirectory, 'init.lua'), 'utf-8');

    expect(initializeLua).toContain('huydo862003/typerighter');
  } finally {
    await rm(xdgDirectory, {
      force: true,
      recursive: true,
    }).catch(() => {});
  }
});

test('setup-editor neovim: does not duplicate config on re-run', async () => {
  const xdgDirectory = await mkdtemp(path.join(tmpdir(), 'typerighter-xdg-'));
  const nvimDirectory = path.join(xdgDirectory, 'nvim');
  const args = [
    BIN,
    'setup-editor',
    'neovim',
  ];
  const env = {
    ...process.env,
    XDG_CONFIG_HOME: xdgDirectory,
  };

  try {
    await mkdir(nvimDirectory, {
      recursive: true,
    });
    await writeFile(path.join(nvimDirectory, 'init.lua'), 'require("lazy").setup({})');

    execFileSync('node', args, {
      env,
      stdio: 'pipe',
      timeout: 30_000,
    });
    execFileSync('node', args, {
      env,
      stdio: 'pipe',
      timeout: 30_000,
    });

    const initializeLua = await readFile(path.join(nvimDirectory, 'init.lua'), 'utf-8');
    const occurrences = (initializeLua.match(/huydo862003\/typerighter/g) ?? []).length;

    expect(occurrences).toBe(1);
  } finally {
    await rm(xdgDirectory, {
      force: true,
      recursive: true,
    }).catch(() => {});
  }
});

test('setup-editor neovim: exits with error when no config dir found', async () => {
  const emptyHome = await mkdtemp(path.join(tmpdir(), 'typerighter-home-'));

  try {
    const result = spawnSync('node', [
      BIN,
      'setup-editor',
      'neovim',
    ], {
      encoding: 'utf-8',
      env: {
        ...process.env,
        HOME: emptyHome,
        XDG_CONFIG_HOME: emptyHome,
      },
      stdio: 'pipe',
      timeout: 30_000,
    });

    expect(result.status).toBe(1);
  } finally {
    await rm(emptyHome, {
      force: true,
      recursive: true,
    }).catch(() => {});
  }
});

test('setup-editor: exits with error for unknown editor', async () => {
  const result = spawnSync('node', [
    BIN,
    'setup-editor',
    'unknown-editor',
  ], {
    encoding: 'utf-8',
    stdio: 'pipe',
    timeout: 10_000,
  });

  expect(result.status).toBe(1);
});
