// Worker thread that renders pages to HTML

import {
  pathToFileURL,
} from 'node:url';
import {
  parentPort, workerData,
} from 'node:worker_threads';
import {
  generateHtmlTemplate,
} from '../lib/html-template';
import type {
  PrerenderWorkerConfig, PrerenderWorkerResult,
} from './prerender';

const config = workerData as PrerenderWorkerConfig;
const ssrModule = await import(pathToFileURL(config.ssrEntryPath).href);

parentPort!.on('message', async (pagePath: string) => {
  try {
    const result = await ssrModule.render(pagePath);

    const html = generateHtmlTemplate({
      title: result.pageData.title ?? '',
      description: result.pageData.frontmatter.description !== undefined
        ? String(result.pageData.frontmatter.description)
        : '',
      siteTitle: config.siteTitle,
      base: config.base,
      entryScript: config.clientEntry,
      canonicalUrl: config.base + pagePath.replace(/^\//, ''),
      headExtra: config.headExtra,
      appContent: result.html,
    });

    const fileName = pagePath === '/'
      ? 'index.html'
      : `${pagePath.replace(/^\//, '')}.html`;

    const response: PrerenderWorkerResult = { pagePath, fileName, html };
    parentPort!.postMessage({ result: response });
  } catch (err) {
    parentPort!.postMessage({
      error: err instanceof Error ? err.message : String(err),
    });
  }
});
