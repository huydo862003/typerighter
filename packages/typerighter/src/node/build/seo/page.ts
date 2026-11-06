// Resolve per-page SEO fields from PageData

import type {
  PageData,
} from '@/shared/types/ssg';
import {
  formatIsoDate, isIndexUrl, stripLeadingSlash, truncate,
} from '@/shared';
import type {
  OgType,
} from '../lib/html-template';
import {
  buildArticleLd, buildBreadcrumbLd, buildWebSiteLd,
} from './jsonld';

const DESCRIPTION_MAX_LENGTH = 160;

export interface PageSeoConfig {
  base: string;
  origin?: string;
  author?: string;
}

export interface PageSeo {
  title: string;
  description: string;
  canonicalUrl: string;
  isHomepage: boolean;
  ogType: OgType;
  ogImagePath: string | undefined;
  jsonLdBlocks: string[];
}

// Build the complete SEO payload for a page
export function buildPageSeo (pageData: PageData, pageRoute: string, config: PageSeoConfig): PageSeo {
  const title = pageData.seoMetadata?.title ?? pageData.title;
  const description = resolvePageDescription(pageData);
  const isHomepage = pageRoute === '/';
  const canonicalUrl = config.base + stripLeadingSlash(pageRoute);
  const dateModified = pageData.fileMetadata?.mtimeSecs !== undefined
    ? formatIsoDate(pageData.fileMetadata.mtimeSecs * 1000)
    : undefined;

  const jsonLdBlocks = [
    isHomepage
      ? buildWebSiteLd({ title, canonicalUrl })
      : buildArticleLd({ title, description, canonicalUrl, author: config.author, dateModified }),
    buildBreadcrumbLd({
      pageRoute,
      title,
      origin: config.origin,
      base: config.base,
      isDirectoryIndex: isIndexUrl(pageRoute),
    }),
  ].filter((block): block is string => block !== undefined);

  return {
    title,
    description,
    canonicalUrl,
    isHomepage,
    ogType: isHomepage ? 'website' : 'article',
    ogImagePath: pageData.seoMetadata?.image,
    jsonLdBlocks,
  };
}

// Resolve the effective page description, preferring _meta override
// Priority: seoMetadata > frontmatter.description > body excerpt > heading-based fallback
function resolvePageDescription (pageData: PageData): string {
  if (pageData.seoMetadata?.description !== undefined) return pageData.seoMetadata.description;

  if (pageData.frontmatter.description !== undefined) return String(pageData.frontmatter.description);

  if (pageData.excerpt !== undefined) return truncate(pageData.excerpt, DESCRIPTION_MAX_LENGTH);

  return buildDescriptionFromHeadings(pageData);
}

// Last-resort: build a description from the page title and h2 section headings
function buildDescriptionFromHeadings (pageData: PageData): string {
  const titles = pageData.headings
    .filter((heading) => heading.level === 2)
    .slice(0, 5)
    .map((heading) => heading.title);

  if (titles.length === 0) return truncate(pageData.title, DESCRIPTION_MAX_LENGTH);

  return truncate(`${pageData.title}: ${titles.join(', ')}`, DESCRIPTION_MAX_LENGTH);
}
