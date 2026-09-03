// Serves a virtual index.html so no physical file is needed in the vault

import type {
  IncomingMessage, ServerResponse,
} from 'node:http';
import type {
  ViteDevServer,
} from 'vite';
import type {
  TypedownPluginCache,
} from '..';
import {
  generateHtmlTemplate,
} from '@/node/lib/html-template';
import {
  VIRTUAL_APP_ID,
} from '@/node/plugin/vite/constants';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';
import {
  getUrlPath, stripTrailingSlash,
} from '@/shared';

export function virtualHtml (
  server: ViteDevServer,
  tdContext: TypedownContext,
  cache: TypedownPluginCache,
) {
  return (
    request: IncomingMessage,
    response: ServerResponse,
    next: () => void,
  ) => {
    if (!request.url) return next();

    const urlPath = stripTrailingSlash(getUrlPath(request.url));
    const base = stripTrailingSlash(cache.basePath);

    if (urlPath !== base && urlPath !== base + '/index.html') return next();

    tdContext
      .getConfig()
      .then(async (config) => {
        const html = await server.transformIndexHtml(
          request.url ?? '',
          getDevHtml(cache, config),
        );

        response.setHeader('Content-Type', 'text/html');
        response.end(html);
      })
      .catch(() => next());
  };
}

function getDevHtml (
  cache: TypedownPluginCache,
  config: {
    siteTitle: string;
    siteDescription: string;
  },
): string {
  if (cache.devHtml !== undefined) return cache.devHtml;

  cache.devHtml = generateHtmlTemplate({
    title: config.siteTitle,
    description: config.siteDescription,
    base: '/',
    entryScript: VIRTUAL_APP_ID,
  });

  return cache.devHtml;
}
