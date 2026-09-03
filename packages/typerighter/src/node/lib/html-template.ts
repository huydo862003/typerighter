// Shared HTML document shell used by both dev server and pre-renderer

import {
  escapeHtml,
} from '@/shared';

export interface HtmlTemplateOptions {
  /** Raw page title */
  title: string;
  /** Raw page description */
  description: string;
  /** Site title, appended as suffix when different from page title */
  siteTitle?: string;
  /** Base path, e.g. "/" or "/docs/" */
  base: string;
  /** Module script src, e.g. "/@typedown/app" or "assets/app-abc.js" */
  entryScript: string;
  /** HTML language attribute */
  lang?: string;
  /** Canonical URL for SEO */
  canonicalUrl?: string;
  /** Extra tags injected into <head> (CSS links, module preloads) */
  headExtra?: string;
  /** SSR-rendered content inside div#app */
  appContent?: string;
}

export function generateHtmlTemplate (options: HtmlTemplateOptions): string {
  const title = escapeHtml(options.title);
  const description = escapeHtml(options.description);

  const pageTitle = options.siteTitle !== undefined && options.title !== options.siteTitle
    ? `${title} - ${escapeHtml(options.siteTitle)}`
    : title;

  const canonical = options.canonicalUrl !== undefined
    ? `\n    <link rel="canonical" href="${escapeHtml(options.canonicalUrl)}">`
    : '';

  const headExtra = options.headExtra !== undefined
    ? options.headExtra + '\n'
    : '';

  const jsonLd = JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'Article',
    headline: options.title,
    description: options.description,
  });

  return `<!DOCTYPE html>
<html lang="${options.lang ?? 'en'}">
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>${pageTitle}</title>
    <meta name="description" content="${description}">${canonical}
    <link rel="icon" href="${options.base}favicon.svg" type="image/svg+xml">
    <meta property="og:type" content="article">
    <meta property="og:title" content="${title}">
    <meta property="og:description" content="${description}">
    <meta property="og:image" content="${options.base}og-image.png">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="${title}">
    <meta name="twitter:description" content="${description}">
    <meta name="twitter:image" content="${options.base}og-image.png">
    <script type="application/ld+json">${jsonLd}</script>
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
