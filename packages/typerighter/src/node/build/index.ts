import path from 'node:path';
import fs from 'node:fs/promises';
import {
  build, type InlineConfig,
} from 'vite';
import {
  prerenderHtmlPages,
} from './prerender';
import {
  generateClientAppEntry, typedown,
} from '../plugin/vite';
import {
  ProgressLogger,
} from '../lib/progress';
import {
  buildContentTree, buildDirectoryListingMap, CONTENT_EXTENSIONS, CONTENT_GLOB, type ContentTreeNode,
  path as tdpath,
} from '@/shared';
import type {
  AppContext,
} from '../context';

const DEFAULT_LAYOUT_IMPORT = 'typerighter/client/theme-default';

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
  const base = options.base ?? '/';

  const clientOutDir = path.join(outDir, '.client');
  const ssrOutDir = path.join(outDir, '.server');

  // Clear stale output from previous builds
  await fs.rm(outDir, { recursive: true, force: true });

  // 1. Fetch project metadata from the RPC server
  const tdContext = await ctx.getTdContext();

  const [
    config,
    files,
    schemaGroups,
    schemaNames,
  ] = await Promise.all([
    tdContext.getConfig(),
    tdContext.listFiles(),
    tdContext.listFilesGroupedBySchema(),
    tdContext.listSchemas(),
  ]);

  const schemaEntries = await Promise.all(
    schemaNames.map(async (name) => {
      const info = await tdContext.getSchema(name);

      return [name, info.properties] as const;
    }),
  );
  const schemas = Object.fromEntries(schemaEntries);

  const allItems = Object.values(schemaGroups).flat();
  const contentTree = buildContentTree(allItems);
  const siteConfig = JSON.stringify({ title: config.siteTitle, description: config.siteDescription });
  const siteData = JSON.stringify({ contentTree });

  // 2. Generate entry files inside the project so Vite can resolve 'typerighter/*' imports
  const tempDir = path.join(root, 'node_modules', '.typerighter');
  await fs.mkdir(tempDir, { recursive: true });
  const clientEntryPath = path.join(tempDir, 'client-entry.js');
  const ssrEntryPath = path.join(tempDir, 'ssr-entry.js');

  await Promise.all([
    fs.writeFile(clientEntryPath, generateClientAppEntry({
      contentDir: config.contentDir,
      siteTitle: config.siteTitle,
      siteDescription: config.siteDescription,
      contentTree,
      schemas,
    })),
    fs.writeFile(ssrEntryPath, generateSsrEntry({
      contentDir: config.contentDir,
      layoutImport: DEFAULT_LAYOUT_IMPORT,
      siteConfig,
      siteData,
    })),
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
    collectDirectoryPaths(contentTree.children, '', pagePaths);

    const phase3 = new ProgressLogger(logger, 'Pre-rendering pages...');

    await prerenderHtmlPages({
      ssrEntryPath: path.join(ssrOutDir, 'ssr-entry.js'),
      clientOutDir,
      outDir,
      base,
      pagePaths,
      progress: phase3,
    });

    phase3.done(`Pre-rendered ${pagePaths.length} pages`);

    // 6. Copy assets to the final output directory
    const clientAssetsDir = path.join(clientOutDir, 'assets');
    const outAssetsDir = path.join(outDir, 'assets');

    await fs.cp(clientAssetsDir, outAssetsDir, {
      recursive: true,
    }).catch(() => {
      // No assets to copy
    });

    await copyContentAssets(path.join(root, config.contentDir), outDir);
  } finally {
    // 7. Clean up intermediate directories
    await Promise.all([
      fs.rm(tempDir, { recursive: true, force: true }),
      fs.rm(clientOutDir, { recursive: true, force: true }),
      fs.rm(ssrOutDir, { recursive: true, force: true }),
    ]);
  }

  logger.log(`\nBuild complete. Output: ${outDir}`);
}

interface SsrEntryOptions {
  contentDir: string;
  layoutImport: string;
  siteConfig: string;
  siteData: string;
}

// Generate the SSR entry module used for pre-rendering
function generateSsrEntry (options: SsrEntryOptions): string {
  const glob = `/${options.contentDir}/${CONTENT_GLOB}`;
  const parsedConfig = JSON.parse(options.siteConfig);
  const parsedData = JSON.parse(options.siteData);
  const contentTree = parsedData.contentTree ?? { rootItems: [], children: [] };
  const directoryListingMap = buildDirectoryListingMap(contentTree.children ?? [], parsedConfig.title ?? '');
  const siteDataWithListings = JSON.stringify({ ...parsedData, directoryListings: directoryListingMap });

  return `
import { createTypedownApp } from 'typerighter/client';
import { TdDirectoryIndex } from 'typerighter/client/theme-default';
import { renderToString } from 'vue/server-renderer';
import { h } from 'vue';
import theme from '${options.layoutImport}';

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
  const base = ('/${options.contentDir}/' + pagePath).replace(/\\/+/g, '/').replace(/\\/$/, '');
  const page = findPage(base);
  if (page) return Promise.resolve(page);

  const dir = siteData.directoryListings[pagePath] || siteData.directoryListings[pagePath + '/'];
  if (dir) return Promise.resolve({
    default: { name: 'DirectoryIndex', render() { return h(TdDirectoryIndex); } },
    __pageData: { frontmatter: {}, headings: [], title: dir.title },
  });

  return Promise.resolve(undefined);
}

export async function render(url) {
  const { app, router } = await createTypedownApp(loadPageModule, theme.Layout, ${options.siteConfig}, siteData);
  await router.go(url, { replace: true });
  const html = await renderToString(app);
  return { html, pageData: router.route.data };
}
`;
}

// Copy non-.td files from content directory to output, preserving structure
async function copyContentAssets (contentDir: string, outDir: string): Promise<void> {
  const entries = await fs.readdir(contentDir, {
    recursive: true,
    withFileTypes: true,
  });

  const copies = entries
    .filter((entry) => entry.isFile() && !tdpath.isContentFile(entry.name))
    .map(async (entry) => {
      const src = path.join(entry.parentPath, entry.name);
      const relative = path.relative(contentDir, src);
      const dest = path.join(outDir, relative);

      return fs.mkdir(path.dirname(dest), { recursive: true })
        .then(() => fs.copyFile(src, dest));
    });

  await Promise.all(copies);
}

// Collect all directory paths from the content tree for pre-rendering
function collectDirectoryPaths (nodes: ContentTreeNode[], prefix: string, paths: string[]) {
  for (const node of nodes) {
    const dirPath = `${prefix}/${node.name}`;

    paths.push(dirPath);
    collectDirectoryPaths(node.children, dirPath, paths);
  }
}
