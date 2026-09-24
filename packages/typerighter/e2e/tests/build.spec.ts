// Build artifact tests: sitemap, robots.txt, HTML SEO tags

import {
  mkdtemp, cp, rm, readFile, writeFile, symlink, access,
} from 'node:fs/promises';
import {
  tmpdir,
} from 'node:os';
import path from 'node:path';
import {
  execFileSync,
} from 'node:child_process';
import {
  test, expect,
} from '@playwright/test';

const FIXTURES_DIR = path.resolve(import.meta.dirname, '../fixtures');
const BIN = path.resolve(import.meta.dirname, '../../bin/typerighter.js');

async function buildFixture (
  fixture: string,
  configPatch?: (yaml: string) => string,
): Promise<string> {
  const fixtureDirectory = path.join(FIXTURES_DIR, fixture);
  const directory = await mkdtemp(path.join(tmpdir(), `typerighter-build-${fixture}-`));

  await cp(fixtureDirectory, directory, {
    recursive: true,
    dereference: true,
    filter: (source) => {
      const name = path.basename(source);

      return name !== '.typedown' && name !== 'node_modules';
    },
  });

  await symlink(
    path.join(fixtureDirectory, 'node_modules'),
    path.join(directory, 'node_modules'),
    'dir',
  );

  if (configPatch !== undefined) {
    const configPath = path.join(directory, 'typedown.yaml');
    const yaml = await readFile(configPath, 'utf-8');

    await writeFile(configPath, configPatch(yaml));
  }

  execFileSync('node', [
    BIN,
    'build',
  ], {
    cwd: directory,
    timeout: 60_000,
    stdio: 'pipe',
  });

  return directory;
}

async function fileExists (filePath: string): Promise<boolean> {
  return access(filePath).then(() => true, () => false);
}

test('build without origin: sitemap has relative URLs', async () => {
  const directory = await buildFixture('vault-root');

  try {
    const sitemap = await readFile(path.join(directory, 'dist', 'sitemap.xml'), 'utf-8');

    expect(sitemap).toContain('<urlset');
    expect(sitemap).toContain('<loc>/');
    expect(sitemap).not.toContain('https://');
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {});
  }
});

test('build without origin: no robots.txt', async () => {
  const directory = await buildFixture('vault-root');

  try {
    expect(await fileExists(path.join(directory, 'dist', 'robots.txt'))).toBe(false);
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {});
  }
});

test('build without origin: HTML has meta tags', async () => {
  const directory = await buildFixture('vault-root');

  try {
    const html = await readFile(path.join(directory, 'dist', 'index.html'), 'utf-8');

    expect(html).toContain('<meta name="description"');
    expect(html).toContain('<meta property="og:title"');
    expect(html).toContain('<meta name="twitter:card"');
    expect(html).toContain('application/ld+json');
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {});
  }
});

test('build: sitemap has lastmod dates', async () => {
  const directory = await buildFixture('vault-root');

  try {
    const sitemap = await readFile(path.join(directory, 'dist', 'sitemap.xml'), 'utf-8');

    expect(sitemap).toMatch(/<lastmod>\d{4}-\d{2}-\d{2}<\/lastmod>/);
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {});
  }
});

test('build: HTML has og:site_name', async () => {
  const directory = await buildFixture('vault-root');

  try {
    const html = await readFile(path.join(directory, 'dist', 'index.html'), 'utf-8');

    expect(html).toContain('og:site_name');
    expect(html).toContain('E2E Test Vault');
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {});
  }
});

test('build with origin: sitemap has absolute URLs', async () => {
  const directory = await buildFixture('vault-root', (yaml) => yaml + '\n  origin: "https://example.com"\n');

  try {
    const sitemap = await readFile(path.join(directory, 'dist', 'sitemap.xml'), 'utf-8');

    expect(sitemap).toContain('https://example.com/');
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {});
  }
});

test('build with origin: robots.txt references sitemap', async () => {
  const directory = await buildFixture('vault-root', (yaml) => yaml + '\n  origin: "https://example.com"\n');

  try {
    const robots = await readFile(path.join(directory, 'dist', 'robots.txt'), 'utf-8');

    expect(robots).toContain('User-agent: *');
    expect(robots).toContain('Sitemap: https://example.com/sitemap.xml');
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {});
  }
});

test('build with origin: HTML has absolute SEO URLs', async () => {
  const directory = await buildFixture('vault-root', (yaml) => yaml + '\n  origin: "https://example.com"\n');

  try {
    const html = await readFile(path.join(directory, 'dist', 'index.html'), 'utf-8');

    expect(html).toContain('rel="canonical" href="https://example.com/');
    expect(html).toContain('og:url" content="https://example.com/');
    expect(html).toContain('og:image" content="https://example.com/');
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {});
  }
});
