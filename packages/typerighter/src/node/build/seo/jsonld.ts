import {
  ensureTrailingSlash, isIndexUrl, unslugify,
} from '@/shared';

export interface ArticleLdOptions {
  title: string;
  description: string;
  canonicalUrl: string;
  author?: string;
  dateModified?: string;
}

export interface WebSiteLdOptions {
  title: string;
  canonicalUrl: string;
}

export interface BreadcrumbLdOptions {
  pageRoute: string;
  title: string;
  /** Absolute site origin (e.g. "https://example.com") for item URLs */
  origin?: string;
  /** Base path (e.g. "/docs/") prepended to all item URLs */
  base: string;
}

// Build an Article JSON-LD block
export function buildArticleLd (options: ArticleLdOptions): string {
  return JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'Article',
    headline: options.title,
    description: options.description,
    url: options.canonicalUrl,
    ...(options.author !== undefined ? { author: { '@type': 'Person', name: options.author } } : {}),
    ...(options.dateModified !== undefined ? { dateModified: options.dateModified } : {}),
  });
}

// Build a WebSite JSON-LD block for the homepage
export function buildWebSiteLd (options: WebSiteLdOptions): string {
  return JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'WebSite',
    name: options.title,
    url: options.canonicalUrl,
  });
}

// Build a BreadcrumbList JSON-LD block from a page path
export function buildBreadcrumbLd (options: BreadcrumbLdOptions): string | undefined {
  const { pageRoute, title, origin = '' } = options;
  const isDirectoryIndex = isIndexUrl(pageRoute);
  const base = ensureTrailingSlash(options.base);

  if (pageRoute === '/') return undefined;

  const segments = pageRoute.split('/').filter(Boolean);
  const items = segments.map((segment, index) => {
    const isLast = index === segments.length - 1;
    const visibleSegments = isLast && isDirectoryIndex
      ? segments.slice(0, index)
      : segments.slice(0, index + 1);
    const itemUrl = visibleSegments.length > 0
      ? `${origin}${base}${visibleSegments.join('/')}`
      : `${origin}${base}`;

    return {
      '@type': 'ListItem',
      position: index + 1,
      name: isLast ? title : unslugify(segment),
      item: itemUrl,
    };
  });

  return JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: items,
  });
}
