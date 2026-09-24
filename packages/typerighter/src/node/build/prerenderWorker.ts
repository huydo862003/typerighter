// Worker thread that renders pages to HTML

import {
  pathToFileURL,
} from 'node:url';
import {
  parentPort, workerData,
} from 'node:worker_threads';
import {
  renderHtmlDocument,
} from '../lib/htmlTemplate';
import type {
  PrerenderWorkerConfig, PrerenderWorkerResult,
} from './prerender';
import {
  stripLeadingSlash,
} from '@/shared';
import {
  resolvePageTitle, resolvePageDescription, extractDateModified, extractOgImagePath,
  buildArticleLd, buildBreadcrumbLd,
} from './seo';

const config = workerData as PrerenderWorkerConfig;
const ssrModule = await import(pathToFileURL(config.ssrEntryPath).href);

parentPort!.on('message', async (pagePath: string) => {
  try {
    const result = await ssrModule.render(pagePath);

    const title = resolvePageTitle(result.pageData);
    const description = resolvePageDescription(result.pageData);
    const dateModified = extractDateModified(result.pageData);
    const canonicalUrl = config.base + stripLeadingSlash(pagePath);

    const jsonLdBlocks = [
      buildArticleLd({ title, description, canonicalUrl, author: config.author, dateModified }),
      buildBreadcrumbLd(pagePath, title),
    ].filter((block): block is string => block !== undefined);

    const html = renderHtmlDocument({
      title,
      description,
      siteTitle: config.siteTitle,
      author: config.author,
      base: config.base,
      origin: config.origin,
      lang: config.lang,
      entryScript: config.clientEntry,
      canonicalUrl,
      ogImagePath: extractOgImagePath(result.pageData),
      jsonLdBlocks,
      headExtra: config.headExtra,
      appContent: result.html,
    });

    const fileName = pagePath === '/'
      ? 'index.html'
      : `${stripLeadingSlash(pagePath)}.html`;

    const response: PrerenderWorkerResult = { pagePath, fileName, html };
    parentPort!.postMessage({ result: response });
  } catch (err) {
    parentPort!.postMessage({
      error: err instanceof Error ? err.message : String(err),
    });
  }
});
