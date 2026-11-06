// Pre-render all pages to static HTML files using worker threads for large vaults

import fs from 'node:fs/promises';
import path from 'node:path';
import {
  fileURLToPath,
} from 'node:url';
import type {
  ProgressLogger,
} from '../lib/progress';
import {
  renderHtmlDocument,
} from '../lib/html-template';
import {
  WorkerPool,
} from '../lib/worker-pool';
import {
  stripLeadingSlash,
} from '@/shared';
import {
  buildPageSeo,
} from './seo';
import {
  isCriticalFontFile,
} from './fonts';

export interface PrerenderContext {
  /** Absolute path to the SSR bundle entry */
  ssrEntryPath: string;
  /** Absolute path to the client output directory */
  clientOutDir: string;
  /** Absolute path to the final output directory */
  outDir: string;
  /** The base path (e.g. "/") */
  base: string;
  /** Absolute site URL for SEO (e.g. "https://example.com") */
  origin?: string;
  /** HTML lang attribute */
  lang?: string;
  /** List of page paths to render (e.g. ["/", "/posts/hello"]) */
  pageRoutes: string[];
  /** Site title for SEO title suffix */
  siteTitle: string;
  /** Site author for meta tag */
  author?: string;
  /** Progress logger for reporting render progress */
  progress?: ProgressLogger;
}

export interface PrerenderWorkerConfig {
  ssrEntryPath: string;
  base: string;
  origin?: string;
  lang?: string;
  siteTitle: string;
  author?: string;
  clientEntry: string;
  headExtra?: string;
}

export interface PrerenderWorkerResult {
  pageRoute: string;
  fileName: string;
  html: string;
}

const WORKER_THRESHOLD = 64;

// Pre-render all pages to static HTML files
export async function prerenderHtmlPages (context: PrerenderContext): Promise<void> {
  // 1. Resolve client assets (JS/CSS paths)
  const { clientEntry, cssFiles, fontFiles } = await resolveClientAssets(context.clientOutDir);

  const cssLinks = cssFiles
    .map((file) => `    <link rel="stylesheet" href="${context.base}${file}">`)
    .join('\n');

  const fontPreloads = fontFiles
    .map((file) => `    <link rel="preload" as="font" type="font/woff2" crossorigin href="${context.base}${file}">`)
    .join('\n');

  const headExtra = [cssLinks, fontPreloads].filter(Boolean).join('\n') || undefined;

  if (context.pageRoutes.length >= WORKER_THRESHOLD) {
    await prerenderWithWorkers(context, clientEntry, headExtra);
  } else {
    await prerenderSingleThread(context, clientEntry, headExtra);
  }
}

// Distribute pages across a pool of worker threads
async function prerenderWithWorkers (
  context: PrerenderContext,
  clientEntry: string,
  headExtra: string | undefined,
): Promise<void> {
  const pool = new WorkerPool<string, PrerenderWorkerResult>({
    filename: path.resolve(fileURLToPath(import.meta.url), '../prerenderWorker.js'),
    workerData: {
      ssrEntryPath: context.ssrEntryPath,
      base: context.base,
      origin: context.origin,
      lang: context.lang,
      siteTitle: context.siteTitle,
      author: context.author,
      clientEntry,
      headExtra,
    } satisfies PrerenderWorkerConfig,
  });

  let rendered = 0;
  const totalPages = context.pageRoutes.length;

  try {
    await Promise.all(context.pageRoutes.map(async (pageRoute) => {
      const result = await pool.run(pageRoute);
      const filepath = path.join(context.outDir, result.fileName);

      await fs.mkdir(path.dirname(filepath), { recursive: true });
      await fs.writeFile(filepath, result.html);

      rendered++;
      context.progress?.update(rendered, totalPages);
    }));
  } finally {
    await pool.destroy();
  }
}

// Single-threaded rendering for small vaults
async function prerenderSingleThread (
  context: PrerenderContext,
  clientEntry: string,
  headExtra: string | undefined,
): Promise<void> {
  const ssrModule = await import(context.ssrEntryPath);
  const render = await ssrModule.createRenderer();
  const totalPages = context.pageRoutes.length;
  let rendered = 0;

  for (const pageRoute of context.pageRoutes) {
    const result = await render(pageRoute);
    const seo = buildPageSeo(result.pageData, pageRoute, {
      base: context.base,
      origin: context.origin,
      author: context.author,
    });

    const html = renderHtmlDocument({
      title: seo.title,
      description: seo.description,
      siteTitle: context.siteTitle,
      author: context.author,
      base: context.base,
      origin: context.origin,
      lang: context.lang,
      entryScript: clientEntry,
      canonicalUrl: seo.canonicalUrl,
      ogType: seo.ogType,
      ogImagePath: seo.ogImagePath,
      jsonLdBlocks: seo.jsonLdBlocks,
      headExtra,
      appContent: result.html,
    });

    const fileName = seo.isHomepage
      ? 'index.html'
      : `${stripLeadingSlash(pageRoute)}.html`;
    const filepath = path.join(context.outDir, fileName);

    await fs.mkdir(path.dirname(filepath), { recursive: true });
    await fs.writeFile(filepath, html);

    rendered++;
    context.progress?.update(rendered, totalPages);
  }
}

// Read the Vite manifest to find the client entry and asset files
async function resolveClientAssets (clientOutDir: string): Promise<{
  clientEntry: string;
  cssFiles: string[];
  fontFiles: string[];
}> {
  const manifestPath = path.join(clientOutDir, '.vite', 'manifest.json');
  const raw = await fs.readFile(manifestPath, 'utf-8');
  const manifest = JSON.parse(raw) as Record<string, {
    file: string;
    isEntry?: boolean;
    css?: string[];
  }>;

  let clientEntry = '';
  const cssFiles: string[] = [];
  const fontFiles: string[] = [];

  for (const chunk of Object.values(manifest)) {
    if (chunk.isEntry) {
      clientEntry = chunk.file;

      if (chunk.css) {
        cssFiles.push(...chunk.css);
      }
    } else if (isCriticalFontFile(chunk.file)) {
      fontFiles.push(chunk.file);
    }
  }

  return { clientEntry, cssFiles, fontFiles };
}
