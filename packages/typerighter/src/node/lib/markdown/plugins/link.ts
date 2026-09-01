/*
* Copied from https://github.com/vuejs/vitepress/blob/main/src/node/markdown/plugins/link.ts
* Commit: c37bde6
* */

import {
  URL,
} from 'node:url';
import type {
  MarkdownItAsync,
} from 'markdown-it-async';
import {
  EXTERNAL_URL_RE,
  isUrlExternal,
  isUrlToPage,
  type MarkdownEnv,
} from '@/shared';

const INDEX_RE = /(^|.*\/)index\.(?:md|td)(#?.*)$/i;

// markdown-it plugin for:
// 1. Adding target="_blank" to external links
// 2. Normalizing internal links to end with `.html`
export function linkPlugin (
  md: MarkdownItAsync,
  externalAttrs: Record<string, string>,
  base: string,
  slugify: (string_: string) => string,
): void {
  // Track which line each link_open token came from (for dead link reporting)
  md.core.ruler.after('inline', 'typedown_link_lines', (state) => {
    for (const token of state.tokens) {
      if (token.type !== 'inline' || !token.children || !token.map) continue;

      const line = token.map[0] + 1;

      for (const child of token.children) {
        if (child.type === 'link_open') {
          child.meta ??= {};
          child.meta.vpLine = line;
        }
      }
    }
  });

  // Override link_open rendering to normalize hrefs and apply external attributes
  md.renderer.rules.link_open = function (
    tokens,
    index,
    options,
    env: MarkdownEnv,
    self,
  ): string {
    const token = tokens[index];
    const hrefIndex = token.attrIndex('href');

    if (
      0 <= hrefIndex
      && token.attrGet('class') !== 'header-anchor'
    ) {
      const hrefAttribute = (token.attrs ?? [])[hrefIndex];
      // Split off text fragment directives (`:~:text=...`) to restore later
      const [
        url,
        fragment,
      ] = hrefAttribute[1].split(':~:', 2);

      hrefAttribute[1] = url;

      if (isUrlExternal(url)) {
        // External link: apply configured attributes (e.g. target="_blank", rel="noreferrer")
        token.attrJoin('class', 'td-external-link');
        Object.entries(externalAttrs).forEach(([
          key,
          value,
        ]) => {
          token.attrSet(key, value);
        });
        // localhost links are tracked as potentially dead links
        if (url.replace(EXTERNAL_URL_RE, '').startsWith('//localhost:')) {
          pushLink(url, env, token.meta?.vpLine);
        }
        hrefAttribute[1] = url;
      } else {
        // Internal link: normalize the href
        // Use a dummy base to parse relative URLs without throwing
        const {
          pathname, protocol,
        } = new URL(url, 'http://a.com');

        if (
          !url.startsWith('#')             // not an anchor-only link
          && protocol.startsWith('http')   // not mailto:/tel:/etc
          && token.attrIndex('target') < 0 // not explicitly targeted
          && token.attrIndex('download') < 0 // not a download link
          && isUrlToPage(pathname)        // not a file download (.pdf, .png, etc.)
        ) {
          normalizeHref(hrefAttribute, env, token.meta?.vpLine);
        } else if (url.startsWith('#')) {
          // Anchor-only: slugify the hash
          hrefAttribute[1] = decodeURI(normalizeHash(hrefAttribute[1]));
        }

        // Prepend base path to absolute internal urls (e.g. /docs/foo -> /blog/docs/foo)
        if (hrefAttribute[1].startsWith('/')) {
          hrefAttribute[1] = `${base}${hrefAttribute[1]}`.replace(/\/+/g, '/');
        }
      }
      // Restore text fragment directive if present
      if (fragment) {
        hrefAttribute[1] += (hrefAttribute[1].includes('#') ? '' : '#') + ':~:' + fragment;
      }
    }

    return self.renderToken(tokens, index, options);
  };

  // Normalize an internal page href: strip .td/.md extensions, ensure relative prefix, track for dead link checking
  function normalizeHref (
    hrefAttribute: [string, string],
    env: MarkdownEnv,
    line?: number,
  ): void {
    let url = hrefAttribute[1];

    const indexMatch = url.match(INDEX_RE);

    if (indexMatch) {
      // index.md -> / (strip the filename, keep the hash)
      const [
        , path,
        hash,
      ] = indexMatch;

      url = path + normalizeHash(hash);
    } else {
      // Strip query and hash for extension processing
      let cleanUrl = url.replace(/[?#].*$/, '');

      // foo.md / foo.td -> foo.html
      if (cleanUrl.endsWith('.md') || cleanUrl.endsWith('.td')) {
        cleanUrl = cleanUrl.replace(/\.(?:md|td)$/, env.cleanUrls ? '' : '.html');
      }

      // ./foo -> ./foo.html
      if (
        !env.cleanUrls
        && !cleanUrl.endsWith('.html')
        && !cleanUrl.endsWith('/')
      ) {
        cleanUrl += '.html';
      }
      const parsed = new URL(url, 'http://a.com');

      url = cleanUrl + parsed.search + normalizeHash(parsed.hash);
    }

    // Ensure leading . for relative paths
    if (!url.startsWith('/') && !url.startsWith('./')) {
      url = './' + url;
    }

    // Track the link (without .html) for dead link checking
    pushLink(url.replace(/\.html$/, ''), env, line);

    // markdown-it encodes the uri, so decode it back
    hrefAttribute[1] = decodeURI(url);
  }

  // Slugify a hash fragment: "#Some Title" -> "#some-title"
  function normalizeHash (string_: string): string {
    return string_ ? encodeURI('#' + slugify(decodeURI(string_).slice(1))) : '';
  }

  // Collect links and their source lines into env for dead link reporting
  function pushLink (link: string, env: MarkdownEnv, line?: number): void {
    const links = env.links || (env.links = []);

    links.push(link);
    if (line !== undefined) {
      const linkLines = env.linkLines || (env.linkLines = []);

      linkLines[links.length - 1] = line;
    }
  }
}
