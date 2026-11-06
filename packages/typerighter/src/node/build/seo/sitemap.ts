// Sitemap and robots.txt generation

import {
  escapeHtml, formatIsoDate, stripLeadingSlash,
} from '@/shared';

// Generate a sitemap.xml string from the list of page paths
// Uses absolute URLs when origin is configured
export function generateSitemap (
  pageRoutes: string[],
  base: string,
  origin?: string,
  mtimeMap?: Map<string, number>,
): string {
  const prefix = origin ?? '';
  const urls = pageRoutes
    .map((p) => {
      const loc = escapeHtml(prefix + base + stripLeadingSlash(p));
      const mtime = mtimeMap?.get(p);
      const lastmod = mtime !== undefined
        ? `<lastmod>${formatIsoDate(mtime * 1000)}</lastmod>`
        : '';

      return `  <url><loc>${loc}</loc>${lastmod}</url>`;
    })
    .join('\n');

  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;
}

// Generate robots.txt with a reference to the sitemap
export function generateRobotsTxt (origin: string, base: string): string {
  return `User-agent: *
Allow: /

Sitemap: ${origin}${base}sitemap.xml
`;
}
