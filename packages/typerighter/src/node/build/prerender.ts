import fs from 'node:fs/promises';
import path from 'node:path';
import type {
  ProgressLogger,
} from '../lib/progress';
import {
  generateHtmlTemplate,
} from '../lib/html-template';

export interface PrerenderContext {
  /** Absolute path to the SSR bundle entry */
  ssrEntryPath: string;
  /** Absolute path to the client output directory */
  clientOutDir: string;
  /** Absolute path to the final output directory */
  outDir: string;
  /** The base path (e.g. "/") */
  base: string;
  /** List of page paths to render (e.g. ["/", "/posts/hello"]) */
  pagePaths: string[];
  /** Site title for SEO title suffix */
  siteTitle: string;
  /** Progress logger for reporting render progress */
  progress?: ProgressLogger;
}


// Pre-render all pages to static HTML files
export async function prerenderHtmlPages (context: PrerenderContext): Promise<void> {
  // 1. Load the SSR bundle and resolve client assets (JS/CSS paths)
  const ssrModule = await import(context.ssrEntryPath);
  const { clientEntry, cssFiles, jsFiles } = await resolveClientAssets(context.clientOutDir);

  // 2. Render all pages concurrently and write to disk
  const totalPages = context.pagePaths.length;
  let renderedPages = 0;

  await Promise.all(context.pagePaths.map(async (pagePath) => {
    const result = await ssrModule.render(pagePath);

    const cssLinks = cssFiles
      .map((file) => `    <link rel="stylesheet" href="${context.base}${file}">`)
      .join('\n');

    const modulePreloads = jsFiles
      .map((file) => `    <link rel="modulepreload" href="${context.base}${file}">`)
      .join('\n');

    const html = generateHtmlTemplate({
      title: result.pageData.title,
      description: result.pageData.frontmatter.description !== undefined
        ? String(result.pageData.frontmatter.description)
        : '',
      siteTitle: context.siteTitle,
      base: context.base,
      entryScript: clientEntry,
      canonicalUrl: context.base + pagePath.replace(/^\//, ''),
      headExtra: [
        cssLinks,
        modulePreloads,
      ].filter(Boolean).join('\n') || undefined,
      appContent: result.html,
    });

    const fileName = pagePath === '/'
      ? 'index.html'
      : `${pagePath.replace(/^\//, '')}.html`;
    const filepath = path.join(context.outDir, fileName);

    await fs.mkdir(path.dirname(filepath), { recursive: true });
    await fs.writeFile(filepath, html);

    renderedPages++;
    context.progress?.update(renderedPages, totalPages);
  }));
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
