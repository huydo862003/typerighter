import path from "node:path";
import fs from "node:fs/promises";
import { build, type InlineConfig } from "vite";
import { prerenderHtmlPages } from "./prerender";
import {
  typedown,
  THEME_ID,
  PAGES_ID,
  SITE_DATA_ID,
  SEARCH_INDEX_ID,
  PAGE_DATA_EVENT,
} from "../plugin/vite";
import { generateAppEntry, generateThemeModule } from "../lib/codegen";
import { resolveUserComponents, type UserComponent } from "../lib/components";
import { ProgressLogger } from "../lib/progress";
import {
  buildContentTree,
  buildDirectoryListingMap,
  CONTENT_GLOB,
  type ContentSummary,
  type ContentTreeEntry,
  ensureTrailingSlash,
  getIndexUrl,
  getNodeIndexItem,
  path as tdpath,
} from "@/shared";
import type { AppContext } from "../context";
import { buildSitemap, buildRobotsTxt, buildRssFeed } from "./seo";

export interface BuildOptions {
  /** Output directory for the final build (default: "dist") */
  outDir?: string;
  /** Base public path (default: "/") */
  base?: string;
  /** Additional Vite config overrides */
  viteConfig?: InlineConfig;
}

// Build the site into static HTML files
export async function buildSite(
  ctx: AppContext,
  options: BuildOptions = {},
): Promise<void> {
  const { root, logger } = ctx;
  const outDir = path.resolve(root, options.outDir ?? "dist");
  const clientOutDir = path.join(outDir, ".client");
  const ssrOutDir = path.join(outDir, ".server");

  // Clear stale output from previous builds
  await fs.rm(outDir, { recursive: true, force: true });

  // 1. Fetch project metadata from the RPC server
  const tdContext = await ctx.getTdContext();

  const [config, files, sidebarItems, schemaNames] = await Promise.all([
    tdContext.getConfig(),
    tdContext.listFiles(),
    tdContext.listSidebar(),
    tdContext.listSchemas(),
  ]);

  const schemaEntries = await Promise.all(
    schemaNames.map(async (name) => {
      const info = await tdContext.getSchema(name);

      return [name, info.properties] as const;
    }),
  );
  const schemas = Object.fromEntries(schemaEntries);

  // Coerce null -> undefined for optional fields from JSON-RPC
  const origin = config.origin ?? undefined;
  const author = config.author ?? undefined;
  const license = config.license ?? undefined;
  const repo = config.repo ?? undefined;

  const rawBase = options.base ?? config.basePath ?? "/";
  const base = ensureTrailingSlash(rawBase);

  const contentItems: ContentSummary[] = sidebarItems.map((item) => ({
    ...item,
    header: {},
  }));
  const contentTree = buildContentTree(contentItems);
  const siteConfig = JSON.stringify({
    title: config.siteTitle,
    description: config.siteDescription,
    basePath: base,
    origin,
    lang: config.lang,
    rootDir: config.rootDir,
    repo,
    author,
    license,
    nav: config.nav,
  });
  const siteData = JSON.stringify({ ready: true, contentTree, schemas });

  // 2. Generate entry files inside the project so Vite can resolve 'typerighter/*' imports
  const tempDir = path.join(root, "node_modules", ".typerighter");
  await fs.mkdir(tempDir, { recursive: true });
  const clientEntryPath = path.join(tempDir, "client-entry.js");
  const ssrEntryPath = path.join(tempDir, "ssr-entry.js");
  const themeModulePath = path.join(tempDir, "theme.js");

  const userComponents = await resolveUserComponents(config.components, root);
  const themeModule = generateThemeModule(config.layout ?? undefined, root);

  await Promise.all([
    fs.writeFile(
      clientEntryPath,
      generateAppEntry(
        {
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
        },
        {
          theme: THEME_ID,
          pages: PAGES_ID,
          siteData: SITE_DATA_ID,
          searchIndex: SEARCH_INDEX_ID,
          pageDataEvent: PAGE_DATA_EVENT,
        },
      ),
    ),
    fs.writeFile(
      ssrEntryPath,
      generateSsrEntry({
        rootDir: config.rootDir,
        siteConfig,
        siteData,
        themeModulePath,
        userComponents,
      }),
    ),
    fs.writeFile(themeModulePath, themeModule),
  ]);

  try {
    const customLogger = ctx.createViteLogger();

    // 3 & 4. Build client and SSR bundles concurrently
    const bundlePhase = new ProgressLogger(logger, "Building bundles...");
    const tBundles = performance.now();

    await Promise.all([
      // Client bundle: produces JS/CSS for the browser plus a Vite manifest
      build({
        ...options.viteConfig,
        root,
        base,
        plugins: typedown({ context: ctx }),
        customLogger,
        logLevel: "silent",
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
      }),
      // SSR bundle: Node-runnable entry used for pre-rendering
      build({
        ...options.viteConfig,
        root,
        base,
        plugins: typedown({ context: ctx }),
        customLogger,
        logLevel: "silent",
        build: {
          ...options.viteConfig?.build,
          outDir: ssrOutDir,
          ssr: ssrEntryPath,
          emptyOutDir: true,
        },
      }),
    ]);

    const clientChunks = await fs
      .readdir(path.join(clientOutDir, "assets"))
      .catch(() => [] as string[]);

    const bundlesMs = performance.now() - tBundles;
    bundlePhase.done(`Client (${clientChunks.length} chunks) + SSR`);
    if (process.env.TYPEDOWN_VERBOSE === '1') process.stderr.write(
      `[timing:phase] bundles ${bundlesMs.toFixed(0)}ms\n`
    );

    // 5. Pre-render each page to a static HTML file
    const pageRoutes = files.map((file) => {
      const withoutExtension = tdpath.stripExtension(file);

      return withoutExtension === "index" ? "/" : `/${withoutExtension}`;
    });

    // Add directory index pages (root + all subdirectories)
    pageRoutes.push("/");
    collectDirectoryPaths(contentTree.entries, pageRoutes);

    const phase3 = new ProgressLogger(logger, "Pre-rendering pages...");

    await prerenderHtmlPages({
      ssrEntryPath: path.join(ssrOutDir, "ssr-entry.js"),
      clientOutDir,
      outDir,
      base,
      origin,
      lang: config.lang,
      pageRoutes,
      siteTitle: config.siteTitle,
      author,
      progress: phase3,
    });

    phase3.done(`Pre-rendered ${pageRoutes.length} pages`);

    // 6. Generate sitemap.xml and robots.txt
    const mtimeMap = new Map<string, number>();

    for (const item of sidebarItems) {
      const withoutExtension = tdpath.stripExtension(item.filepath);
      const pageRoute =
        withoutExtension === "index" ? "/" : `/${withoutExtension}`;

      mtimeMap.set(pageRoute, item.fileMetadata.mtimeSecs);
    }

    const sitemap = buildSitemap(pageRoutes, base, origin, mtimeMap);
    const writes: Promise<void>[] = [
      fs.writeFile(path.join(outDir, "sitemap.xml"), sitemap),
    ];

    if (origin !== undefined) {
      writes.push(
        fs.writeFile(
          path.join(outDir, "robots.txt"),
          buildRobotsTxt(origin, base),
        ),
      );
      writes.push(
        fs.writeFile(
          path.join(outDir, "feed.xml"),
          buildRssFeed(
            sidebarItems,
            base,
            origin,
            config.siteTitle,
            config.siteDescription,
            config.lang,
          ),
        ),
      );
    }

    await Promise.all(writes);

    // 7. Copy assets to the final output directory
    const clientAssetsDir = path.join(clientOutDir, "assets");
    const outAssetsDir = path.join(outDir, "assets");

    await fs
      .cp(clientAssetsDir, outAssetsDir, {
        recursive: true,
      })
      .catch(() => {
        // No assets to copy
      });

    await copyVaultAssets(path.join(root, config.rootDir), outDir);

    // 8. Copy public directory to output
    const publicDir = path.join(root, config.publicDir);

    await fs
      .cp(publicDir, outDir, {
        recursive: true,
      })
      .catch(() => {
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
  themeModulePath: string;
  userComponents?: UserComponent[];
}

// Generate the SSR entry module used for pre-rendering
function generateSsrEntry(options: SsrEntryOptions): string {
  const glob = `/${options.rootDir}/${CONTENT_GLOB}`;
  const parsedConfig = JSON.parse(options.siteConfig);
  const parsedData = JSON.parse(options.siteData);
  const contentTree = parsedData.contentTree ?? { entries: [] };
  const directoryListingMap = buildDirectoryListingMap(
    contentTree.entries ?? [],
    parsedConfig.title ?? "",
  );
  const siteDataWithListings = JSON.stringify({
    ...parsedData,
    directoryListings: directoryListingMap,
  });

  const userComponents = options.userComponents ?? [];
  const componentImports = userComponents
    .map(
      (component) =>
        `import ${component.name} from ${JSON.stringify(component.absolutePath)};`,
    )
    .join("\n");
  const componentRegistrations = userComponents
    .map(
      (component) =>
        `app.component(${JSON.stringify(component.name)}, ${component.name});`,
    )
    .join("\n");

  const ssrHeader = [
    componentImports,
    `import { createTypedownApp } from 'typerighter/client';`,
  ]
    .filter(Boolean)
    .join("\n");

  return `
${ssrHeader}
import { renderToString } from 'vue/server-renderer';
import { shallowRef } from 'vue';
import theme from ${JSON.stringify(options.themeModulePath)};

const rawPages = import.meta.glob('${glob}', { eager: true });
const pages = Object.fromEntries(Object.entries(rawPages).map(([k, v]) => [k, () => Promise.resolve(v)]));
const siteData = shallowRef({ ...${siteDataWithListings}, pages });

export async function createRenderer() {
  const { app, router } = await createTypedownApp({ ...${options.siteConfig}, theme }, siteData);
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
async function copyVaultAssets(rootDir: string, outDir: string): Promise<void> {
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

      return fs
        .mkdir(path.dirname(dest), { recursive: true })
        .then(() => fs.copyFile(src, dest));
    });

  await Promise.all(copies);
}

// Collect directory index paths from the content tree for pre-rendering
// Only adds dirs without an index.td (those are already covered by files.map)
function collectDirectoryPaths(
  entries: ContentTreeEntry[],
  paths: string[],
  prefix: string = "",
) {
  for (const entry of entries) {
    if (entry.kind !== "dir") continue;

    const dirPath = `${prefix}/${entry.node.name}`;

    if (!getNodeIndexItem(entry.node)) {
      paths.push(getIndexUrl(dirPath));
    }

    collectDirectoryPaths(entry.node.entries, paths, dirPath);
  }
}
