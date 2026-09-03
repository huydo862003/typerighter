// Serves static assets (images, PDFs, etc.) from the vault root directory

import {
  createReadStream, existsSync, statSync,
} from 'node:fs';
import type {
  IncomingMessage, ServerResponse,
} from 'node:http';
import {
  resolve,
} from 'node:path';
import type {
  ViteDevServer,
} from 'vite';
import type {
  TypedownPluginCache,
} from '..';
import {
  getUrlPath, path,
} from '@/shared';

const COMMON_MIME_TYPES: Record<string, string> = {
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  png: 'image/png',
  gif: 'image/gif',
  webp: 'image/webp',
  svg: 'image/svg+xml',
  avif: 'image/avif',
  ico: 'image/x-icon',
  pdf: 'application/pdf',
  zip: 'application/zip',
  mp3: 'audio/mpeg',
  mp4: 'video/mp4',
  webm: 'video/webm',
  woff2: 'font/woff2',
  woff: 'font/woff',
  ttf: 'font/ttf',
};

export function vaultAssets (server: ViteDevServer, cache: TypedownPluginCache) {
  return (
    request: IncomingMessage,
    response: ServerResponse,
    next: () => void,
  ) => {
    if (!request.url || request.method !== 'GET' || response.writableEnded)
      return next();

    const urlPath = getUrlPath(request.url);

    if (urlPath.startsWith('/@') || urlPath.startsWith('/node_modules'))
      return next();

    const relativePath = urlPath.replace(/^\//, '');
    const contentFilePath = resolve(
      server.config.root,
      cache.rootDirectory,
      relativePath,
    );

    if (existsSync(contentFilePath)) {
      try {
        const stat = statSync(contentFilePath);

        if (stat.isFile()) {
          const extension = path
            .extname(contentFilePath)
            .slice(1)
            .toLowerCase();
          const mimeType =
            COMMON_MIME_TYPES[extension] ?? 'application/octet-stream';

          response.setHeader('Content-Type', mimeType);
          response.setHeader('Content-Length', stat.size);
          createReadStream(contentFilePath).pipe(response);

          return;
        }
      } catch { }
    }

    next();
  };
}
