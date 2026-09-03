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
  buildSite,
} from '../../build';
import {
  renderToVueSfc,
} from '../../lib/render';
import {
  createAppContext, resolveProjectRoot, type AppContext,
} from '../../context';
import {
  isRpcCancelled,
  type TypedownContext,
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
  vaultAssets, virtualHtml,
} from './middleware';
import {
  type ContentTree,
  buildContentTree,
  buildDirectoryListingMap,
  CONTENT_EXTENSIONS,
  getTdContentUrl,
  getTdResourceTitle,
  path,
  type ContentSummary,
} from '@/shared';

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

export interface TypedownPluginCache {
  searchIndex: Promise<void> | undefined;
  siteData: ReturnType<typeof fetchSiteData> | undefined;
  devHtml: string | undefined;
  basePath: string;
  rootDirectory: string;
}

export interface TypedownPluginOptions {
  // @internal Used by buildSite to share the RPC connection
  context?: AppContext;
  /** Vault directory relative to the host project root */
  root?: string;
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
import('typerighter/katex.css');
import { createTypedownApp } from 'typerighter/client';
import { TdDirectoryIndex, TdGlossaryIndex } from 'typerighter/client/theme-default';
import { isIndexUrl, getDirectoryFromPageUrl } from 'typerighter/shared';
import { h } from 'vue';
import theme from 'typerighter/client/theme-default';
import { pages as initialPages } from '${PAGES_ID}';
import initialSiteData from '${SITE_DATA_ID}';
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

const { app, searchIndex: searchIndexRef, siteData } = await createTypedownApp(loadPageModule, theme.Layout, ${siteConfig}, initialSiteData);
app.mount('#app');

// Load search index in the background after the app is mounted
import('${SEARCH_INDEX_ID}').then((m) => { searchIndexRef.value = m.default; });

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
  let hostOutDirectory: string;
  let hostBase: string;

  const searchIndexer = new SearchIndexer();

  const cache: TypedownPluginCache = {
    searchIndex: undefined,
    siteData: undefined,
    devHtml: undefined,
    basePath: '/',
    rootDirectory: 'vault',
  };

  function resolveTdContext () {
    if (context === undefined) throw new Error('typedown plugin not initialized');

    return context.getTdContext();
  }

  const typedownPlugin: Plugin = {
    name: 'vite-plugin-typedown',

    enforce: 'pre',

    async config (userConfig) {
      if (!context) {
        const root = options.root
          ? resolve(userConfig.root ?? process.cwd(), options.root)
          : resolveProjectRoot(userConfig.root ?? process.cwd());

        context = createAppContext(root);
      }

      hostBase = userConfig.base ?? '/';

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

    configResolved (config) {
      hostOutDirectory = resolve(config.root, config.build.outDir);
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
        if (!cache.searchIndex) {
          const tdContext = await resolveTdContext();

          cache.searchIndex = indexAllPages(tdContext, searchIndexer);
        }

        await cache.searchIndex;

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
        if (!cache.siteData) {
          cache.siteData = fetchSiteData(await resolveTdContext());
        }

        return `export default ${JSON.stringify(await cache.siteData)}`;
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

      // Prefetch eagerly so data is ready by the time virtual modules are loaded
      cache.searchIndex = indexAllPages(tdContext, searchIndexer);
      cache.siteData = fetchSiteData(tdContext);

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
        cache.devHtml = undefined;
        cache.siteData = undefined;
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
        cache.searchIndex = indexAllPages(tdContext, searchIndexer);
        cache.searchIndex.then(() => {
          if (server) invalidateSearchIndex(server);
        });
        cache.siteData = undefined;
        invalidatePages(server);
        invalidateSiteData(server);
      }

      tdContext.rpc.onContentCreated(handleContentListChange);
      tdContext.rpc.onContentDeleted(handleContentListChange);

      // Schema changes affect all pages using that schema and sidebar data
      function handleSchemaChange () {
        if (!server) return;
        cache.siteData = undefined;
        invalidateSiteData(server);
        hmrInvalidateAll(server);
      }

      tdContext.rpc.onSchemaChanged(handleSchemaChange);
      tdContext.rpc.onSchemaCreated(handleSchemaChange);
      tdContext.rpc.onSchemaDeleted(handleSchemaChange);

      const initialConfig = await tdContext.getConfig();

      cache.basePath = initialConfig.basePath;
      cache.rootDirectory = initialConfig.rootDir;

      tdContext.rpc.onConfigChanged((updated: {
        basePath: string;
        rootDir: string;
      }) => {
        cache.basePath = updated.basePath;
        cache.rootDirectory = updated.rootDir;
        cache.devHtml = undefined;
      });

      devServer.middlewares.use(vaultAssets(devServer, cache));
      devServer.middlewares.use(virtualHtml(devServer, tdContext, cache));
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
        // Cancellation during rapid edits is expected, skip the transform
        if (isRpcCancelled(error)) return;

        const message = error instanceof Error ? error.message : String(error);

        this.error(`[typedown] Failed to transform ${relativePath}: ${message}`);
      }
    },

    handleHotUpdate ({
      file,
    }) {
      if (path.isContentFile(file)) return [];
    },

    async closeBundle () {
      // When context was provided externally (CLI), the caller manages the build
      if (options.context) return;
      if (!context) return;

      try {
        const tdContext = await context.getTdContext();
        const config = await tdContext.getConfig();
        const basePath = config.basePath;
        const subpath = basePath.startsWith(hostBase)
          ? basePath.slice(hostBase.length)
          : basePath.replace(/^\//, '');

        await buildSite(context, {
          outDir: resolve(hostOutDirectory, subpath),
          base: basePath,
        });
      } finally {
        context.dispose();
      }
    },
  };

  const vuePlugin = vue({
    include: options.root
      ? new RegExp(`${options.root.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/.*\\.(?:vue|td)$`)
      : /\.(?:vue|td)$/,
  });

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
    sidebarItems,
    schemaNames,
  ] = await Promise.all([
    context.getConfig(),
    context.listSidebar(),
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

  // Build content tree from lightweight sidebar items
  const contentItems: ContentSummary[] = sidebarItems.map((item) => ({
    ...item,
    header: {},
  }));
  const contentTree = buildContentTree(contentItems);
  const directoryListings = buildDirectoryListingMap(contentTree.entries, config.siteTitle);

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
      title: getTdResourceTitle(filepath, resource.label),
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

// Fetch all pages as search index inputs
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
