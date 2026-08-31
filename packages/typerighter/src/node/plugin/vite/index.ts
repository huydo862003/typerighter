import {
  createReadStream,
  existsSync,
  statSync,
} from 'node:fs';
import {
  resolve,
} from 'node:path';
import pc from 'picocolors';
import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import {
  normalizePath,
  type Plugin, type ViteDevServer,
} from 'vite';
import {
  renderToVueSfc,
} from '../../lib/render';
import {
  createAppContext, resolveProjectRoot, type AppContext,
} from '../../context';
import type {
  TypedownContext,
} from '../../lib/typedown-context';
import {
  VIRTUAL_APP_ID, RESOLVED_VIRTUAL_APP_ID,
  PAGES_ID, RESOLVED_PAGES_ID,
  SITE_DATA_ID, RESOLVED_SITE_DATA_ID,
  SEARCH_INDEX_ID, RESOLVED_SEARCH_INDEX_ID,
} from './constants';
import {
  SearchIndexer, type PageIndexInput,
} from './search';
import {
  resolveAliases,
} from './alias';
import {
  buildContentTree,
  buildDirectoryListingMap,
  CONTENT_EXTENSIONS,
  escapeHtml,
  getTdContentUrl,
  getTdResourceTitle,
  path,
} from '@/shared';
import type {
  ContentTree,
} from '@/shared';

const COMMON_MIME_TYPES: Record<string, string> = {
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  png: 'image/png',
  gif: 'image/gif',
  webp: 'image/webp',
  svg: 'image/svg+xml',
  avif: 'image/avif',
  ico: 'image/x-icon',
  pdf: 'application/pdf',
  zip: 'application/zip',
  mp3: 'audio/mpeg',
  mp4: 'video/mp4',
  woff: 'font/woff',
  woff2: 'font/woff2',
  ttf: 'font/ttf',
};

export interface ClientAppEntryOptions {
  /** Vault root directory relative to project root */
  rootDir?: string;
  /** URL base path */
  basePath?: string;
  /** Site title */
  siteTitle: string;
  /** Site description */
  siteDescription: string;
  /** Repository URL */
  repo?: string;
}

// Create the typedown vite plugin with the Vue plugin bundled
export interface TypedownPluginOptions {
  // @internal Used by buildSite to share the RPC connection
  context?: AppContext;
}

export function generateClientAppEntry (options: ClientAppEntryOptions): string {
  const {
    rootDir: rootDirectory = '.',
  } = options;

  const siteConfig = JSON.stringify({
    title: options.siteTitle,
    description: options.siteDescription,
    basePath: options.basePath ?? '/',
    repo: options.repo,
  });

  return `
import 'typerighter/style.css';
import 'typerighter/fonts.css';
import { createTypedownApp } from 'typerighter/client';
import { TdDirectoryIndex, TdGlossaryIndex } from 'typerighter/client/theme-default';
import { isIndexUrl, getDirectoryFromPageUrl } from 'typerighter/shared';
import { h } from 'vue';
import theme from 'typerighter/client/theme-default';
import { pages as initialPages } from '${PAGES_ID}';
import initialSiteData from '${SITE_DATA_ID}';
import searchIndex from '${SEARCH_INDEX_ID}';

let pages = initialPages;
const contentExts = ${JSON.stringify(CONTENT_EXTENSIONS)};

function findPage(base) {
  for (const ext of contentExts) {
    const key = base + ext;
    if (pages[key]) return pages[key];
  }
}

async function loadPageModule(pagePath) {
  const base = ('/${rootDirectory}/' + pagePath).replace(/\\/+/g, '/').replace(/\\/$/, '');
  const loader = findPage(base);
  if (loader) return loader();

  if (isIndexUrl(pagePath)) {
    const dirPath = getDirectoryFromPageUrl(pagePath);
    const dir = siteData.value.directoryListings[dirPath];
    if (dir) return {
      default: { name: 'DirectoryIndex', render() { return h(TdDirectoryIndex); } },
      __pageData: { frontmatter: {}, headings: [], title: dir.title },
    };
  }

  return undefined;
}

const { app, searchIndex: searchIndexRef, siteData } = await createTypedownApp(loadPageModule, theme.Layout, ${siteConfig}, { ...initialSiteData, searchIndex });
app.mount('#app');

// Accept HMR so modules update without a full page reload
if (import.meta.hot) {
  import.meta.hot.accept('${PAGES_ID}', (m) => {
    if (m) pages = m.pages;
  });

  import.meta.hot.accept('${SEARCH_INDEX_ID}', (m) => {
    if (m) searchIndexRef.value = m.default;
  });

  import.meta.hot.accept('${SITE_DATA_ID}', (m) => {
    if (m) siteData.value = m.default;
  });
}
`;
}

export function typedown (options: TypedownPluginOptions = {}): Plugin[] {
  let context = options.context;
  let server: ViteDevServer | undefined;

  const searchIndexer = new SearchIndexer();
  let searchIndexReady: Promise<void> | undefined;

  function resolveTdContext () {
    if (context === undefined) throw new Error('typedown plugin not initialized');

    return context.getTdContext();
  }

  const vuePlugin = vue({
    include: /\.(?:vue|td)$/,
  });

  const typedownPlugin: Plugin = {
    name: 'vite-plugin-typedown',

    enforce: 'pre',

    async config (userConfig) {
      if (!context) {
        context = createAppContext(resolveProjectRoot(userConfig.root ?? process.cwd()));
      }

      const tdContext = await resolveTdContext();
      const tdConfig = await tdContext.getConfig();

      return {
        publicDir: tdConfig.publicDir,
        server: {
          port: 8686,
          strictPort: false,
        },
        resolve: {
          alias: resolveAliases(),
        },
        ssr: {
          // Bundle typerighter into SSR entry to avoid dual Vue instances (as vite in ssr doesnt bundle node_modules)
          noExternal: ['typerighter'],
        },
      };
    },

    async buildStart () {
      // In production builds, run vault check upfront and fail on errors
      if (server) return;

      const tdContext = await resolveTdContext();
      const report = await tdContext.checkVault();

      printDiagnostics(report.diagnostics);

      if (0 < report.errorCount) {
        const lines = report.diagnostics
          .filter((diagnostic) => diagnostic.severity === 'error')
          .map((diagnostic) => `  ${diagnostic.filepath}:${diagnostic.line}:${diagnostic.column} - ${diagnostic.message} (${diagnostic.code})`);

        this.error(
          `Vault check failed with ${report.errorCount} error(s):\n${lines.join('\n')}`,
        );
      }
    },

    // Resolve virtual modules
    resolveId (id) {
      if (id === '/' + VIRTUAL_APP_ID || id === VIRTUAL_APP_ID) {
        return RESOLVED_VIRTUAL_APP_ID;
      }
      if (id === PAGES_ID) {
        return RESOLVED_PAGES_ID;
      }
      if (id === SITE_DATA_ID) {
        return RESOLVED_SITE_DATA_ID;
      }
      if (id === SEARCH_INDEX_ID) {
        return RESOLVED_SEARCH_INDEX_ID;
      }
    },

    // Serve virtual modules
    async load (id) {
      if (id === RESOLVED_SEARCH_INDEX_ID) {
        if (!searchIndexReady) {
          const tdContext = await resolveTdContext();

          searchIndexReady = indexAllPages(tdContext, searchIndexer);
        }
        await searchIndexReady;

        return `export default ${JSON.stringify(searchIndexer.serialize())}`;
      }

      if (id === RESOLVED_PAGES_ID) {
        const tdContext = await resolveTdContext();
        const config = await tdContext.getConfig();
        const rootDirectory = config.rootDir ?? '.';
        const glob = rootDirectory === '.' ? '/**/*.{td,md}' : `/${rootDirectory}/**/*.{td,md}`;

        return `export const pages = import.meta.glob('${glob}');`;
      }

      if (id === RESOLVED_SITE_DATA_ID) {
        const tdContext = await resolveTdContext();
        const siteData = await fetchSiteData(tdContext);

        return `export default ${JSON.stringify(siteData)}`;
      }

      if (id !== RESOLVED_VIRTUAL_APP_ID) return;

      const tdContext = await resolveTdContext();
      const config = await tdContext.getConfig();

      return generateClientAppEntry({
        ...config,
        rootDir: config.rootDir,
      });
    },

    async configureServer (devServer) {
      server = devServer;
      const tdContext = await resolveTdContext();

      // Print vault diagnostics after the server URL is shown
      devServer.httpServer?.once('listening', async () => {
        const report = await tdContext.checkVault();

        if (0 < report.diagnostics.length) {
          console.error('');
          printDiagnostics(report.diagnostics);
        }
      });

      // Config changes affect the rendering pipeline itself, requires full reload
      tdContext.rpc.onConfigChanged(() => {
        if (!server) return;
        invalidateVirtualAppModule(server);
        invalidatePages(server);
        invalidateSiteData(server);
        hmrFullReload(server);
      });

      tdContext.rpc.onContentChanged(({
        content,
      }: {
        content: string;
      }) => {
        if (!server) return;

        // Incrementally re-index the changed file
        getPageIndexInput(tdContext, content)
          .then((page) => {
            if (page && server) {
              searchIndexer.addPage(page);
              invalidateSearchIndex(server);
            }
          })
          .catch(() => {});

        tdContext.getConfig()
          .then((config) => {
            if (!server) return;

            const absolute = normalizePath(
              resolve(server.config.root, config.rootDir, content),
            );

            // A single file can back several modules (`?vue&type=template`, `&type=style`)
            const modules = server.moduleGraph.getModulesByFile(absolute);

            if (!modules?.size) return; // not transformed yet, nothing to invalidate

            const updates = [...modules].map((module_) => {
              server?.moduleGraph.invalidateModule(module_);

              return makeHmrUpdate(module_);
            });

            server.hot.send({
              type: 'update',
              updates,
            });
          })
          .catch(() => {});

      });

      // Full re-index when files are added or removed
      function handleContentListChange () {
        if (!server) return;
        searchIndexReady = indexAllPages(tdContext, searchIndexer);
        searchIndexReady.then(() => {
          if (server) invalidateSearchIndex(server);
        });
        invalidatePages(server);
        invalidateSiteData(server);
      }

      tdContext.rpc.onContentCreated(handleContentListChange);
      tdContext.rpc.onContentDeleted(handleContentListChange);

      // Schema changes affect all pages using that schema and sidebar data
      function handleSchemaChange () {
        if (!server) return;
        invalidateSiteData(server);
        hmrInvalidateAll(server);
      }

      tdContext.rpc.onSchemaChanged(handleSchemaChange);
      tdContext.rpc.onSchemaCreated(handleSchemaChange);
      tdContext.rpc.onSchemaDeleted(handleSchemaChange);

      let cachedRootDirectory = 'vault';

      tdContext.getConfig()
        .then((config) => {
          cachedRootDirectory = config.rootDir;
        })
        .catch(() => {});

      // Middleware to serve assets (images, PDFs, etc.) from rootDir
      devServer.middlewares.use((request, result, next) => {
        if (!request.url || request.method !== 'GET' || result.writableEnded || !server) return next();

        const urlPath = request.url.split('?')[0].split('#')[0];

        if (urlPath.startsWith('/@') || urlPath.startsWith('/node_modules')) return next();

        const relativePath = urlPath.replace(/^\//, '');
        const contentFilePath = resolve(server.config.root, cachedRootDirectory, relativePath);

        if (existsSync(contentFilePath)) {
          try {
            const stat = statSync(contentFilePath);

            if (stat.isFile()) {
              const extension = path.extname(contentFilePath)
                .slice(1)
                .toLowerCase();
              const mimeType = COMMON_MIME_TYPES[extension] ?? 'application/octet-stream';

              result.setHeader('Content-Type', mimeType);
              result.setHeader('Content-Length', stat.size);
              createReadStream(contentFilePath).pipe(result);

              return;
            }
          } catch {}
        }

        next();
      });

      // Serve a default index.html for SPA routing
      return () => {
        devServer.middlewares.use((request, result, next) => {
          if (!request.url || result.writableEnded) return next();

          if (request.url.startsWith('/@') || request.url.startsWith('/node_modules') || request.url.includes('.')) {
            return next();
          }

          tdContext.getConfig().then((config) => {
            const title = escapeHtml(config.siteTitle);
            const description = escapeHtml(config.siteDescription);
            const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${title}</title>
  <meta name="description" content="${description}">
  <link rel="icon" href="/favicon.svg" type="image/svg+xml">
  <meta property="og:type" content="website">
  <meta property="og:title" content="${title}">
  <meta property="og:description" content="${description}">
  <meta property="og:image" content="/og-image.png">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="${title}">
  <meta name="twitter:description" content="${description}">
  <meta name="twitter:image" content="/og-image.png">
</head>
<body>
  <div id="app"></div>
  <script type="module" src="/${VIRTUAL_APP_ID}"></script>
</body>
</html>`;

            result.setHeader('Content-Type', 'text/html');
            result.end(html);
          })
            .catch(next);
        });
      };
    },

    async transform (_, id) {
      const cleanId = id.split('?')[0];

      if (!path.isContentFile(cleanId)) return;

      if (path.isTypeFile(cleanId)) {
        return {
          code: '<script>import { TdNotFound } from \'typerighter/client/theme-default\'; export default TdNotFound; export const __pageData = { frontmatter: {}, headings: [], title: \'\' };</script>',
          map: null,
        };
      }

      const tdContext = await resolveTdContext();
      const config = await tdContext.getConfig();
      const rootDirectory = config.rootDir;
      const relativePath = cleanId.includes(rootDirectory)
        ? cleanId.slice(cleanId.indexOf(rootDirectory) + rootDirectory.length + 1)
        : cleanId;

      try {
        const resource = await tdContext.getFile(relativePath);
        const {
          vueSrc,
        } = await renderToVueSfc(tdContext, resource, relativePath);

        return {
          code: vueSrc,
          map: null,
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);

        this.error(`[typedown] Failed to transform ${relativePath}: ${message}`);
      }
    },

    handleHotUpdate ({
      file,
    }) {
      if (path.isContentFile(file)) return [];
    },

  };

  return [
    typedownPlugin,
    vuePlugin,
    ...tailwindcss(),
  ];
}

// Fetch site data (content tree, schemas, directory listings) from the RPC server
async function fetchSiteData (context: TypedownContext): Promise<{
  contentTree: ContentTree;
  schemas: Record<string, unknown>;
  directoryListings: ReturnType<typeof buildDirectoryListingMap>;
}> {
  const [
    config,
    schemaGroups,
    schemaNames,
  ] = await Promise.all([
    context.getConfig(),
    context.listFilesGroupedBySchema(),
    context.listSchemas(),
  ]);

  const schemaEntries = await Promise.all(
    schemaNames.map(async (name) => {
      const info = await context.getSchema(name);

      return [
        name,
        info.properties,
      ] as const;
    }),
  );
  const schemas = Object.fromEntries(schemaEntries);

  const allItems = Object.values(schemaGroups).flat();
  const contentTree = buildContentTree(allItems);
  const directoryListings = buildDirectoryListingMap(contentTree.children, config.siteTitle);

  return {
    contentTree,
    schemas,
    directoryListings,
  };
}

// Render a single file into a PageIndexInput for the search indexer
async function getPageIndexInput (context: TypedownContext, filepath: string): Promise<PageIndexInput | undefined> {
  try {
    const resource = await context.getFile(filepath);
    const html = await context.md.renderAsync(resource.content, {
      path: filepath,
      relativePath: filepath,
      cleanUrls: true,
    });

    return {
      id: getTdContentUrl(filepath),
      title: getTdResourceTitle(resource.header, filepath),
      html,
    };
  } catch {
    return undefined;
  }
}

// Get all .td modules from the module graph
function getTdModules (server: ViteDevServer) {
  return [...server.moduleGraph.idToModuleMap.entries()]
    .filter(([id]) => id.endsWith('.td'))
    .map(([
      , module_,
    ]) => module_);
}

// Full page reload for config changes
function hmrFullReload (server: ViteDevServer): void {
  for (const module_ of getTdModules(server)) {
    server.moduleGraph.invalidateModule(module_);
  }

  server.hot.send({
    type: 'full-reload',
  });
}

// Invalidate all .td modules and trigger HMR
function hmrInvalidateAll (server: ViteDevServer): void {
  const modules = getTdModules(server);
  const updates = modules.map((module_) => {
    server.moduleGraph.invalidateModule(module_);

    return makeHmrUpdate(module_);
  });

  if (0 < updates.length) {
    server.hot.send({
      type: 'update',
      updates,
    });
  }
}

// Render all pages and rebuild the search index
async function indexAllPages (context: TypedownContext, indexer: SearchIndexer): Promise<void> {
  const files = await context.listFiles();
  const pages = await Promise.all(files.map((filepath) => getPageIndexInput(context, filepath)));

  indexer.addAll(pages.filter((page): page is PageIndexInput => page !== undefined));
}

// Invalidate the pages virtual module so import.meta.glob re-scans the filesystem
function invalidatePages (server: ViteDevServer): void {
  const module_ = server.moduleGraph.getModuleById(RESOLVED_PAGES_ID);

  if (!module_) return;
  server.moduleGraph.invalidateModule(module_);
  server.hot.send({
    type: 'update',
    updates: [makeHmrUpdate(module_)],
  });
}

// Invalidate the search index virtual module and push an HMR update
function invalidateSearchIndex (server: ViteDevServer): void {
  const module_ = server.moduleGraph.getModuleById(RESOLVED_SEARCH_INDEX_ID);

  if (!module_) return;
  server.moduleGraph.invalidateModule(module_);
  server.hot.send({
    type: 'update',
    updates: [makeHmrUpdate(module_)],
  });
}

// Invalidate the site data virtual module and push an HMR update
function invalidateSiteData (server: ViteDevServer): void {
  const module_ = server.moduleGraph.getModuleById(RESOLVED_SITE_DATA_ID);

  if (!module_) return;
  server.moduleGraph.invalidateModule(module_);
  server.hot.send({
    type: 'update',
    updates: [makeHmrUpdate(module_)],
  });
}

// Invalidate the virtual app module so it regenerates with fresh siteData
function invalidateVirtualAppModule (server: ViteDevServer): void {
  const module_ = server.moduleGraph.getModuleById(RESOLVED_VIRTUAL_APP_ID);

  if (module_) {
    server.moduleGraph.invalidateModule(module_);
  }
}

// Build an HMR update payload for a module
function makeHmrUpdate (module_: {
  url: string;
}) {
  return {
    type: 'js-update' as const,
    path: module_.url,
    acceptedPath: module_.url,
    timestamp: Date.now(),
  };
}

// Print vault diagnostics to stderr with severity prefix and location
function printDiagnostics (diagnostics: Array<{
  severity: string;
  filepath: string;
  line: number;
  column: number;
  message: string;
  code: string;
}>) {
  for (const diagnostic of diagnostics) {
    const location = `${diagnostic.filepath}:${diagnostic.line}:${diagnostic.column}`;
    const prefix = diagnostic.severity === 'error' ? pc.red('error') : pc.yellow('warn');

    console.error(`  ${prefix} ${location} ${diagnostic.message} ${pc.dim(`(${diagnostic.code})`)}`);
  }
}
