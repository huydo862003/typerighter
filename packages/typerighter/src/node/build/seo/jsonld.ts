// JSON-LD structured data builders

import {
  unslugify,
} from '@/shared';

export interface ArticleLdOptions {
  title: string;
  description: string;
  canonicalUrl?: string;
  author?: string;
  dateModified?: string;
}

// Build an Article JSON-LD block
export function buildArticleLd (options: ArticleLdOptions): string {
  return JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'Article',
    headline: options.title,
    description: options.description,
    ...(options.canonicalUrl !== undefined ? { url: options.canonicalUrl } : {}),
    ...(options.author !== undefined ? { author: { '@type': 'Person', name: options.author } } : {}),
    ...(options.dateModified !== undefined ? { dateModified: options.dateModified } : {}),
  });
}

// Build a BreadcrumbList JSON-LD block from a page path
export function buildBreadcrumbLd (pagePath: string, title: string): string | undefined {
  if (pagePath === '/') return undefined;

  const segments = pagePath.split('/').filter(Boolean);
  const items = segments.map((segment, index) => ({
    '@type': 'ListItem',
    position: index + 1,
    name: index === segments.length - 1 ? title : unslugify(segment),
  }));

  return JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: items,
  });
}
