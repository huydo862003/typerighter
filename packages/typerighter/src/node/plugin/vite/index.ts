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
} from '../../lib/typedown-context';
import {
  VIRTUAL_APP_ID, RESOLVED_VIRTUAL_APP_ID,
  PAGES_ID, RESOLVED_PAGES_ID,
  SITE_DATA_ID, RESOLVED_SITE_DATA_ID,
  SEARCH_INDEX_ID, RESOLVED_SEARCH_INDEX_ID,
} from './constants';
import {
  VirtualApp,
} from './virtual/app';
import {
  VirtualPages,
} from './virtual/pages';
import {
  VirtualSiteData,
} from './virtual/site-data';
import {
  VirtualSearchIndex,
} from './virtual/search-index';
import {
  resolveAliases,
} from './alias';
import {
  vaultAssets, virtualHtml,
} from './middleware';
import {
  path,
} from '@/shared';

// Re-export for SSG build
export {
  generate as generateClientAppEntry,
  type AppEntryOptions as ClientAppEntryOptions,
} from './virtual/app';

export interface TypedownPluginCache {
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

export function typedown (options: TypedownPluginOptions = {}): Plugin[] {
  let context = options.context;
  let server: ViteDevServer | undefined;
  let hostOutDirectory: string;
  let hostBase: string;

  const virtualApp = new VirtualApp();
  const virtualPages = new VirtualPages();
  const virtualSiteData = new VirtualSiteData();
  const virtualSearchIndex = new VirtualSearchIndex();

  const cache: TypedownPluginCache = {
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

    // Resolve virtual module IDs
    resolveId (id) {
      if (id === '/' + VIRTUAL_APP_ID || id === VIRTUAL_APP_ID) return RESOLVED_VIRTUAL_APP_ID;
      if (id === PAGES_ID) return RESOLVED_PAGES_ID;
      if (id === SITE_DATA_ID) return RESOLVED_SITE_DATA_ID;
      if (id === SEARCH_INDEX_ID) return RESOLVED_SEARCH_INDEX_ID;
    },

    // Serve virtual modules
    async load (id) {
      const context = await resolveTdContext();

      if (id === RESOLVED_SEARCH_INDEX_ID) return virtualSearchIndex.load(context);
      if (id === RESOLVED_SITE_DATA_ID) return virtualSiteData.load(context);
      if (id === RESOLVED_PAGES_ID) return virtualPages.load(context);
      if (id === RESOLVED_VIRTUAL_APP_ID) return virtualApp.load(context);
    },

    async configureServer (devServer) {
      server = devServer;
      const tdContext = await resolveTdContext();
      const config = await tdContext.getConfig();
      const rootDirectory = resolve(config.rootDir);

      // Populate search index and fetch site data in background
      virtualSearchIndex.index(rootDirectory);
      virtualSiteData.fetch(tdContext, server);

      // Print vault diagnostics after the server URL is shown
      devServer.httpServer?.once('listening', async () => {
        const report = await tdContext.checkVault();

        if (0 < report.diagnostics.length) {
          console.error('');
          printDiagnostics(report.diagnostics);
        }
      });

      // Config changes require full reload
      tdContext.rpc.onConfigChanged(() => {
        if (!server) return;
        cache.devHtml = undefined;
        virtualSiteData.clear();
        virtualApp.invalidate(server);
        virtualPages.invalidate(server);
        hmrFullReload(server);
        virtualSiteData.fetch(tdContext, server);
      });

      // Content changed: re-index file + invalidate .td modules
      tdContext.rpc.onContentChanged(({
        content,
      }: {
        content: string;
      }) => {
        if (!server) return;

        virtualSearchIndex.reindex(rootDirectory, content);
        virtualSearchIndex.invalidate(server);

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

      // Files added or removed: full re-index
      function handleContentListChange () {
        if (!server) return;
        virtualSearchIndex.index(rootDirectory);
        virtualSearchIndex.invalidate(server);
        virtualPages.invalidate(server);
        virtualSiteData.clear();
        virtualSiteData.fetch(tdContext, server);
      }

      tdContext.rpc.onContentCreated(handleContentListChange);
      tdContext.rpc.onContentDeleted(handleContentListChange);

      // Schema changes affect all pages and sidebar data
      function handleSchemaChange () {
        if (!server) return;
        virtualSiteData.clear();
        hmrInvalidateAll(server);
        virtualSiteData.fetch(tdContext, server);
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
