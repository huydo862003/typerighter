// HTML document shell used by both dev server and pre-renderer

import {
  escapeHtml,
} from '@/shared';

export interface HtmlTemplateOptions {
  /** Page title */
  title: string;
  /** Page description for meta tags */
  description: string;
  /** Site title, appended as suffix when different from page title */
  siteTitle?: string;
  /** Site author for meta and JSON-LD */
  author?: string;
  /** Base path (e.g. "/" or "/docs/") */
  base: string;
  /** Absolute site origin (e.g. "https://example.com") */
  origin?: string;
  /** Module script src (e.g. "assets/app-abc.js") */
  entryScript: string;
  /** HTML lang attribute */
  lang?: string;
  /** Canonical URL path (e.g. "/people/alice") */
  canonicalUrl?: string;
  /** Per-page OG image path, falls back to global og-image.png */
  ogImagePath?: string;
  /** Pre-built JSON-LD blocks to inject into head */
  jsonLdBlocks?: string[];
  /** Extra tags injected into head (CSS links, module preloads) */
  headExtra?: string;
  /** SSR-rendered content inside div#app */
  appContent?: string;
}

// Render a complete HTML document from the given options
export function renderHtmlDocument (options: HtmlTemplateOptions): string {
  const title = escapeHtml(options.title);
  const description = escapeHtml(options.description);

  const pageTitle = options.siteTitle !== undefined && options.title !== options.siteTitle
    ? `${title} - ${escapeHtml(options.siteTitle)}`
    : title;

  const origin = options.origin ?? '';
  const absoluteCanonical = options.canonicalUrl !== undefined
    ? origin + options.canonicalUrl
    : undefined;

  const canonical = absoluteCanonical !== undefined
    ? `\n    <link rel="canonical" href="${escapeHtml(absoluteCanonical)}">`
    : '';

  const ogUrl = absoluteCanonical !== undefined
    ? `\n    <meta property="og:url" content="${escapeHtml(absoluteCanonical)}">`
    : '';

  const ogImageSrc = options.ogImagePath !== undefined
    ? `${origin}${options.base}${options.ogImagePath}`
    : `${origin}${options.base}og-image.png`;

  const authorMeta = options.author !== undefined
    ? `\n    <meta name="author" content="${escapeHtml(options.author)}">`
    : '';

  const ogSiteName = options.siteTitle !== undefined
    ? `\n    <meta property="og:site_name" content="${escapeHtml(options.siteTitle)}">`
    : '';

  const rssFeed = origin !== ''
    ? `\n    <link rel="alternate" type="application/rss+xml" title="${options.siteTitle !== undefined ? escapeHtml(options.siteTitle) : 'RSS Feed'}" href="${origin}${options.base}feed.xml">`
    : '';

  const jsonLdTags = (options.jsonLdBlocks ?? [])
    .map((block) => `\n    <script type="application/ld+json">${block}</script>`)
    .join('');

  const headExtra = options.headExtra !== undefined
    ? options.headExtra + '\n'
    : '';

  return `<!DOCTYPE html>
<html lang="${options.lang ?? 'en'}">
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="generator" content="Typerighter">
    <meta name="robots" content="index, follow">
    <title>${pageTitle}</title>
    <meta name="description" content="${description}">${authorMeta}${canonical}
    <link rel="icon" href="${options.base}favicon.svg" type="image/svg+xml">${rssFeed}
    <meta property="og:type" content="article">${ogSiteName}${ogUrl}
    <meta property="og:title" content="${title}">
    <meta property="og:description" content="${description}">
    <meta property="og:image" content="${escapeHtml(ogImageSrc)}">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="${title}">
    <meta name="twitter:description" content="${description}">
    <meta name="twitter:image" content="${escapeHtml(ogImageSrc)}">${jsonLdTags}
    <script>
      (function () {
        var theme = localStorage.getItem('td-theme');
        var isDark = theme === 'dark' || (theme !== 'light' && matchMedia('(prefers-color-scheme: dark)').matches);
        if (isDark) document.documentElement.classList.add('dark');
      })()
    </script>
${headExtra}  </head>
  <body>
    <div id="app">${options.appContent ?? ''}</div>
    <script type="module" src="${options.base}${options.entryScript}"></script>
  </body>
</html>`;
}
