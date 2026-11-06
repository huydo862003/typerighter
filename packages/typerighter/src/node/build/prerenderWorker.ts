// Worker thread that renders pages to HTML

import {
  pathToFileURL,
} from 'node:url';
import {
  parentPort, workerData,
} from 'node:worker_threads';
import {
  renderHtmlDocument,
} from '../lib/html-template';
import type {
  PrerenderWorkerConfig, PrerenderWorkerResult,
} from './prerender';
import {
  stripLeadingSlash,
} from '@/shared';
import {
  buildPageSeo,
} from './seo';

const config = workerData as PrerenderWorkerConfig;
const ssrModule = await import(pathToFileURL(config.ssrEntryPath).href);
const render = await ssrModule.createRenderer();

parentPort!.on('message', async (pageRoute: string) => {
  try {
    const result = await render(pageRoute);
    const seo = buildPageSeo(result.pageData, pageRoute, {
      base: config.base,
      origin: config.origin,
      author: config.author,
    });

    const html = renderHtmlDocument({
      title: seo.title,
      description: seo.description,
      siteTitle: config.siteTitle,
      author: config.author,
      base: config.base,
      origin: config.origin,
      lang: config.lang,
      entryScript: config.clientEntry,
      canonicalUrl: seo.canonicalUrl,
      ogType: seo.ogType,
      ogImagePath: seo.ogImagePath,
      jsonLdBlocks: seo.jsonLdBlocks,
      headExtra: config.headExtra,
      appContent: result.html,
    });

    const fileName = seo.isHomepage
      ? 'index.html'
      : `${stripLeadingSlash(pageRoute)}.html`;

    const response: PrerenderWorkerResult = { pageRoute, fileName, html };
    parentPort!.postMessage({ result: response });
  } catch (err) {
    parentPort!.postMessage({
      error: err instanceof Error ? err.message : String(err),
    });
  }
});
