import path from 'node:path';
import fs from 'node:fs/promises';
import {
  build, type InlineConfig,
} from 'vite';
import {
  prerenderHtmlPages,
} from './prerender';
import {
  typedown,
  THEME_ID, PAGES_ID, SITE_DATA_ID, SEARCH_INDEX_ID, PAGE_DATA_EVENT,
} from '../plugin/vite';
import {
  generateAppEntry, generateThemeModule, resolveUserComponents, type UserComponent,
} from '../lib/codegen';
import {
  ProgressLogger,
} from '../lib/progress';
import {
  buildContentTree, buildDirectoryListingMap, CONTENT_EXTENSIONS, CONTENT_GLOB, type ContentSummary, type ContentTreeEntry,
  getIndexUrl, getNodeIndexItem, path as tdpath,
} from '@/shared';
import type {
  AppContext,
} from '../context';
import {
  generateSitemap, generateRobotsTxt, generateRssFeed,
} from './seo';


export interface BuildOptions {
  /** Output directory for the final build (default: "dist") */
  outDir?: string;
  /** Base public path (default: "/") */
  base?: string;
  /** Additional Vite config overrides */
  viteConfig?: InlineConfig;
}

// Build the site into static HTML files
export async function buildSite (ctx: AppContext, options: BuildOptions = {}): Promise<void> {
  const { root, logger } = ctx;
  const outDir = path.resolve(root, options.outDir ?? 'dist');
  const clientOutDir = path.join(outDir, '.client');
  const ssrOutDir = path.join(outDir, '.server');

  // Clear stale output from previous builds
  await fs.rm(outDir, { recursive: true, force: true });

  // 1. Fetch project metadata from the RPC server
  const tdContext = await ctx.getTdContext();

  const [
    config,
    files,
    sidebarItems,
  ] = await Promise.all([
    tdContext.getConfig(),
    tdContext.listFiles(),
    tdContext.listSidebar(),
  ]);

  // Coerce null -> undefined for optional fields from JSON-RPC
  const origin = config.origin ?? undefined;
  const author = config.author ?? undefined;
  const license = config.license ?? undefined;
  const repo = config.repo ?? undefined;

  const rawBase = options.base ?? config.basePath ?? '/';
  const base = rawBase.endsWith('/') ? rawBase : rawBase + '/';

  const contentItems: ContentSummary[] = sidebarItems.map((item) => ({
    ...item,
    header: {},
  }));
  const contentTree = buildContentTree(contentItems);
  const siteConfig = JSON.stringify({ title: config.siteTitle, description: config.siteDescription, basePath: base, origin, lang: config.lang, rootDir: config.rootDir, repo, author, license, nav: config.nav });
  const siteData = JSON.stringify({ ready: true, contentTree });

  // 2. Generate entry files inside the project so Vite can resolve 'typerighter/*' imports
  const tempDir = path.join(root, 'node_modules', '.typerighter');
  await fs.mkdir(tempDir, { recursive: true });
  const clientEntryPath = path.join(tempDir, 'client-entry.js');
  const ssrEntryPath = path.join(tempDir, 'ssr-entry.js');
  const themeModulePath = path.join(tempDir, 'theme.js');

  const userComponents = await resolveUserComponents(config.components, root);
  const themeModule = generateThemeModule(
    config.layout ?? undefined,
    root,
  );

  await Promise.all([
    fs.writeFile(clientEntryPath, generateAppEntry({
      rootDir: config.rootDir,
      basePath: base,
      origin,
      lang: config.lang,
      siteTitle: config.siteTitle,
      siteDescription: config.siteDescription,
      repo,
      author,
      license,
      nav: config.nav,
      userComponents,
    }, {
      theme: THEME_ID,
      pages: PAGES_ID,
      siteData: SITE_DATA_ID,
      searchIndex: SEARCH_INDEX_ID,
      pageDataEvent: PAGE_DATA_EVENT,
    })),
    fs.writeFile(ssrEntryPath, generateSsrEntry({
      rootDir: config.rootDir,
      siteConfig,
      siteData,
      userComponents,
    })),
    fs.writeFile(themeModulePath, themeModule),
  ]);

  try {
    const plugins = typedown({ context: ctx });
    const customLogger = ctx.createViteLogger();

    // 3. Build the client bundle (JS/CSS for the browser)
    const phase1 = new ProgressLogger(logger, 'Building client bundle...');

    await build({
      ...options.viteConfig,
      root,
      base,
      plugins,
      customLogger,
      logLevel: 'silent',
      build: {
        ...options.viteConfig?.build,
        outDir: clientOutDir,
        manifest: true,
        emptyOutDir: true,
        rollupOptions: {
          ...options.viteConfig?.build?.rollupOptions,
          input: clientEntryPath,
        },
      },
    });

    const clientChunks = await fs.readdir(path.join(clientOutDir, 'assets')).catch(() => []);

    phase1.done(`Client bundle (${clientChunks.length} chunks)`);

    // 4. Build the SSR bundle (Node-runnable entry for pre-rendering)
    const phase2 = new ProgressLogger(logger, 'Building SSR bundle...');

    await build({
      ...options.viteConfig,
      root,
      base,
      plugins,
      customLogger,
      logLevel: 'silent',
      build: {
        ...options.viteConfig?.build,
        outDir: ssrOutDir,
        ssr: ssrEntryPath,
        emptyOutDir: true,
      },
    });

    phase2.done('SSR bundle');

    // 5. Pre-render each page to a static HTML file
    const pagePaths = files.map((file) => {
      const withoutExtension = tdpath.stripExtension(file);

      return withoutExtension === 'index'
        ? '/'
        : `/${withoutExtension}`;
    });

    // Add directory index pages (root + all subdirectories)
    pagePaths.push('/');
    collectDirectoryPaths(contentTree.entries, '', pagePaths);

    const phase3 = new ProgressLogger(logger, 'Pre-rendering pages...');

    await prerenderHtmlPages({
      ssrEntryPath: path.join(ssrOutDir, 'ssr-entry.js'),
      clientOutDir,
      outDir,
      base,
      origin,
      lang: config.lang,
      pagePaths,
      siteTitle: config.siteTitle,
      author,
      progress: phase3,
    });

    phase3.done(`Pre-rendered ${pagePaths.length} pages`);

    // 6. Generate sitemap.xml and robots.txt
    const mtimeMap = new Map<string, number>();

    for (const item of sidebarItems) {
      const withoutExtension = tdpath.stripExtension(item.filepath);
      const pagePath = withoutExtension === 'index' ? '/' : `/${withoutExtension}`;

      mtimeMap.set(pagePath, item.fileMetadata.mtime);
    }

    const sitemap = generateSitemap(pagePaths, base, origin, mtimeMap);
    const writes: Promise<void>[] = [
      fs.writeFile(path.join(outDir, 'sitemap.xml'), sitemap),
    ];

    if (origin !== undefined) {
      writes.push(fs.writeFile(path.join(outDir, 'robots.txt'), generateRobotsTxt(origin, base)));
      writes.push(fs.writeFile(path.join(outDir, 'feed.xml'), generateRssFeed(
        sidebarItems, base, origin, config.siteTitle, config.siteDescription, config.lang,
      )));
    }

    await Promise.all(writes);

    // 7. Copy assets to the final output directory
    const clientAssetsDir = path.join(clientOutDir, 'assets');
    const outAssetsDir = path.join(outDir, 'assets');

    await fs.cp(clientAssetsDir, outAssetsDir, {
      recursive: true,
    }).catch(() => {
      // No assets to copy
    });

    await copyVaultAssets(path.join(root, config.rootDir), outDir);

    // 8. Copy public directory to output
    const publicDir = path.join(root, config.publicDir);

    await fs.cp(publicDir, outDir, {
      recursive: true,
    }).catch(() => {
      // No public directory
    });
  } finally {
    // 8. Clean up intermediate directories
    await Promise.all([
      fs.rm(tempDir, { recursive: true, force: true }),
      fs.rm(clientOutDir, { recursive: true, force: true }),
      fs.rm(ssrOutDir, { recursive: true, force: true }),
    ]);
  }

  logger.log(`\nBuild complete. Output: ${outDir}`);
}

interface SsrEntryOptions {
  rootDir: string;
  siteConfig: string;
  siteData: string;
  userComponents?: UserComponent[];
}

// Generate the SSR entry module used for pre-rendering
function generateSsrEntry (options: SsrEntryOptions): string {
  const glob = `/${options.rootDir}/${CONTENT_GLOB}`;
  const parsedConfig = JSON.parse(options.siteConfig);
  const parsedData = JSON.parse(options.siteData);
  const contentTree = parsedData.contentTree ?? { entries: [] };
  const directoryListingMap = buildDirectoryListingMap(contentTree.entries ?? [], parsedConfig.title ?? '');
  const siteDataWithListings = JSON.stringify({ ...parsedData, directoryListings: directoryListingMap });

  const userComponents = options.userComponents ?? [];
  const componentImports = userComponents
    .map((c) => `import ${c.name} from ${JSON.stringify(c.absolutePath)};`)
    .join('\n');
  const componentRegistrations = userComponents
    .map((c) => `app.component(${JSON.stringify(c.name)}, ${c.name});`)
    .join('\n');

  const ssrHeader = [
    componentImports,
    `import { createTypedownApp } from 'typerighter/client';`,
  ].filter(Boolean).join('\n');

  return `
${ssrHeader}
import { TdDirectoryIndex, TdGlossaryIndex } from 'typerighter/client/theme-default';
import { isIndexUrl, getDirectoryFromPageUrl } from 'typerighter/shared';
import { renderToString } from 'vue/server-renderer';
import { h } from 'vue';
import theme from './theme.js';

const pages = import.meta.glob('${glob}', { eager: true });
const contentExts = ${JSON.stringify(CONTENT_EXTENSIONS)};
const siteData = ${siteDataWithListings};

function findPage(base) {
  for (const ext of contentExts) {
    const key = base + ext;
    if (pages[key]) return pages[key];
  }
}

function loadPageModule(pagePath) {
  const base = ('/${options.rootDir}/' + pagePath).replace(/\\/+/g, '/').replace(/\\/$/, '');
  const page = findPage(base);
  if (page) return Promise.resolve(page);

  if (isIndexUrl(pagePath)) {
    const dirPath = getDirectoryFromPageUrl(pagePath);
    const dir = siteData.directoryListings[dirPath];
    if (dir) return Promise.resolve({
      default: { name: 'DirectoryIndex', render() { return h(TdDirectoryIndex); } },
      pageData: { frontmatter: {}, headings: [], title: dir.title },
    });
  }

  return Promise.resolve(undefined);
}

export async function createRenderer() {
  const { app, router } = await createTypedownApp(loadPageModule, theme, ${options.siteConfig}, siteData);
  ${componentRegistrations}
  return async function render(url) {
    await router.go(url, { replace: true });
    const html = await renderToString(app);
    return { html, pageData: router.route.data };
  };
}
`;
}

// Copy non-.td files from vault root to output, preserving structure
async function copyVaultAssets (rootDir: string, outDir: string): Promise<void> {
  const entries = await fs.readdir(rootDir, {
    recursive: true,
    withFileTypes: true,
  });

  const copies = entries
    .filter((entry) => entry.isFile() && !tdpath.isContentFile(entry.name))
    .map(async (entry) => {
      const src = path.join(entry.parentPath, entry.name);
      const relative = path.relative(rootDir, src);
      const dest = path.join(outDir, relative);

      return fs.mkdir(path.dirname(dest), { recursive: true })
        .then(() => fs.copyFile(src, dest));
    });

  await Promise.all(copies);
}


// Collect directory index paths from the content tree for pre-rendering
// Only adds dirs without an index.td (those are already covered by files.map)
function collectDirectoryPaths (entries: ContentTreeEntry[], prefix: string, paths: string[]) {
  for (const entry of entries) {
    if (entry.kind !== 'dir') continue;

    const dirPath = `${prefix}/${entry.node.name}`;

    if (!getNodeIndexItem(entry.node)) {
      paths.push(getIndexUrl(dirPath));
    }

    collectDirectoryPaths(entry.node.entries, dirPath, paths);
  }
}
