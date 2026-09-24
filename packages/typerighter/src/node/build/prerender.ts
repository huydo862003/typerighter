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
} from '../lib/htmlTemplate';
import {
  WorkerPool,
} from '../lib/worker-pool';
import {
  stripLeadingSlash,
} from '@/shared';
import {
  resolvePageTitle, resolvePageDescription, extractDateModified, extractOgImagePath,
  buildArticleLd, buildBreadcrumbLd,
} from './seo';

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
  pagePaths: string[];
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
  pagePath: string;
  fileName: string;
  html: string;
}

const WORKER_THRESHOLD = 64;

// Pre-render all pages to static HTML files
export async function prerenderHtmlPages (context: PrerenderContext): Promise<void> {
  // 1. Resolve client assets (JS/CSS paths)
  const { clientEntry, cssFiles, jsFiles } = await resolveClientAssets(context.clientOutDir);

  const cssLinks = cssFiles
    .map((file) => `    <link rel="stylesheet" href="${context.base}${file}">`)
    .join('\n');

  const modulePreloads = jsFiles
    .map((file) => `    <link rel="modulepreload" href="${context.base}${file}">`)
    .join('\n');

  const headExtra = [cssLinks, modulePreloads].filter(Boolean).join('\n') || undefined;

  if (context.pagePaths.length >= WORKER_THRESHOLD) {
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
  const totalPages = context.pagePaths.length;

  try {
    await Promise.all(context.pagePaths.map(async (pagePath) => {
      const result = await pool.run(pagePath);
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

// Single-threaded async batching for small vaults
async function prerenderSingleThread (
  context: PrerenderContext,
  clientEntry: string,
  headExtra: string | undefined,
): Promise<void> {
  const ssrModule = await import(context.ssrEntryPath);
  const totalPages = context.pagePaths.length;
  let rendered = 0;

  const BATCH_SIZE = 32;

  for (let i = 0; i < totalPages; i += BATCH_SIZE) {
    const batch = context.pagePaths.slice(i, i + BATCH_SIZE);

    await Promise.all(batch.map(async (pagePath) => {
      const result = await ssrModule.render(pagePath);

      const title = resolvePageTitle(result.pageData);
      const description = resolvePageDescription(result.pageData);
      const dateModified = extractDateModified(result.pageData);
      const canonicalUrl = context.base + stripLeadingSlash(pagePath);

      const jsonLdBlocks = [
        buildArticleLd({ title, description, canonicalUrl, author: context.author, dateModified }),
        buildBreadcrumbLd(pagePath, title),
      ].filter((block): block is string => block !== undefined);

      const html = renderHtmlDocument({
        title,
        description,
        siteTitle: context.siteTitle,
        author: context.author,
        base: context.base,
        origin: context.origin,
        lang: context.lang,
        entryScript: clientEntry,
        canonicalUrl,
        ogImagePath: extractOgImagePath(result.pageData),
        jsonLdBlocks,
        headExtra,
        appContent: result.html,
      });

      const fileName = pagePath === '/'
        ? 'index.html'
        : `${stripLeadingSlash(pagePath)}.html`;
      const filepath = path.join(context.outDir, fileName);

      await fs.mkdir(path.dirname(filepath), { recursive: true });
      await fs.writeFile(filepath, html);

      rendered++;
      context.progress?.update(rendered, totalPages);
    }));
  }
}

// Read the Vite manifest to find the client entry and asset files
async function resolveClientAssets (clientOutDir: string): Promise<{
  clientEntry: string;
  cssFiles: string[];
  jsFiles: string[];
}> {
  const manifestPath = path.join(clientOutDir, '.vite', 'manifest.json');
  const raw = await fs.readFile(manifestPath, 'utf-8');
  const manifest = JSON.parse(raw) as Record<string, {
    file: string;
    isEntry?: boolean;
    css?: string[];
    imports?: string[];
  }>;

  let clientEntry = '';
  const cssFiles: string[] = [];
  const jsFiles: string[] = [];

  for (const chunk of Object.values(manifest)) {
    if (chunk.isEntry) {
      clientEntry = chunk.file;

      if (chunk.css) {
        cssFiles.push(...chunk.css);
      }
    } else if (chunk.file.endsWith('.js')) {
      jsFiles.push(chunk.file);
    }
  }

  return { clientEntry, cssFiles, jsFiles };
}
