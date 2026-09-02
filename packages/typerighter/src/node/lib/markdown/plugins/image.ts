/*
* Copied from https://github.com/vuejs/vitepress/blob/main/src/node/markdown/plugins/image.ts
* Commit: 078786a
* */

/* markdown-it plugin for:
* - Normalizing image source
* - Auto adding width and height to images to avoid layout shift
*/

import fs from 'node:fs';
import path from 'node:path';
import type {
  MarkdownItAsync,
} from 'markdown-it-async';
import type Token from 'markdown-it/lib/token.mjs';
import {
  getImageDimensions,
} from '@/node/lib/image-dimensions';
import {
  EXTERNAL_URL_RE, type MarkdownEnv,
} from '@/shared';

export function imagePlugin (
  md: MarkdownItAsync,
): void {
  const imageRule = md.renderer.rules.image ?? md.renderer.renderToken.bind(md.renderer);

  md.renderer.rules.image = function (tokens, index, options, env: MarkdownEnv, self): string {
    const token = tokens[index];

    let url = token.attrGet('src');

    if (url && !EXTERNAL_URL_RE.test(url)) {
      // Normalize relative "foo.png" to "./foo.png" and decode for processing by bundlers
      if (!/^\.*?\//.test(url)) {
        url = './' + url;
      }

      url = decodeURIComponent(url);
      token.attrSet('src', url);

      addImageDimensions(token, url, env);
    }

    if (!token.attrGet('loading')) {
      token.attrSet('loading', 'lazy');
    }

    return imageRule(tokens, index, options, env, self);
  };
}

// Add width/height attributes to avoid layout shift
function addImageDimensions (
  token: Token,
  url: string,
  env: MarkdownEnv,
): void {
  const width = token.attrGet('width');
  const height = token.attrGet('height');

  if (width && height) return;

  const dimensions = resolveImageDimensions(url, env);

  if (!dimensions) return;

  const aspectRatio = dimensions.width / dimensions.height;

  if (!width) {
    const newWidth = height ? +height * aspectRatio : dimensions.width;

    if (Number.isFinite(newWidth)) {
      token.attrSet('width', Math.round(newWidth).toString());
    }
  }

  if (!height) {
    const newHeight = width ? +width / aspectRatio : dimensions.height;

    if (Number.isFinite(newHeight)) {
      token.attrSet('height', Math.round(newHeight).toString());
    }
  }
}

function resolveImageDimensions (url: string, env: MarkdownEnv): {
  width: number;
  height: number;
} | undefined {
  try {
    const imagePath = resolveLocalImage(url, env);

    return imagePath ? getImageDimensions(fs.readFileSync(imagePath)) : undefined;
  } catch {
    // Best-effort: may fail if the file doesn't exist or the format is unsupported
    return undefined;
  }
}

// Resolve a relative image path against the current file's directory
function resolveLocalImage (src: string, env: MarkdownEnv): string | undefined {
  // Absolute paths can't be resolved without a public directory
  if (src.startsWith('/')) return undefined;

  const {
    realPath, path: envPath,
  } = env;

  return path.resolve(path.dirname(realPath ?? envPath), src);
}
