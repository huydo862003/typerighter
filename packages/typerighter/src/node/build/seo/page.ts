import type {
  OgType, PageData,
} from '@/shared/types/ssg';
import {
  formatIsoDate, getParentUrl, isIndexUrl, stripLeadingSlash, truncate,
} from '@/shared';
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
  const description = buildPageDescription(pageData);
  // Index pages are served at the parent directory URL by static hosts
  const canonicalRoute = isIndexUrl(pageRoute) ? getParentUrl(pageRoute) + '/' : pageRoute;
  const isHomepage = canonicalRoute === '/';
  const canonicalUrl = (config.origin ?? '') + config.base + stripLeadingSlash(canonicalRoute);
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

// Priority: seoMetadata > frontmatter.description > excerpt > heading fallback
function buildPageDescription (pageData: PageData): string {
  if (pageData.seoMetadata?.description !== undefined) return pageData.seoMetadata.description;

  if (pageData.frontmatter.description !== undefined) return String(pageData.frontmatter.description);

  if (pageData.excerpt !== undefined) return truncate(pageData.excerpt, DESCRIPTION_MAX_LENGTH);

  return buildDescriptionFromHeadings(pageData);
}

// fallback: title + h2 headings joined as a summary
function buildDescriptionFromHeadings (pageData: PageData): string {
  const titles = pageData.headings
    .filter((heading) => heading.level === 2)
    .slice(0, 5)
    .map((heading) => heading.title);

  if (titles.length === 0) return truncate(pageData.title, DESCRIPTION_MAX_LENGTH);

  return truncate(`${pageData.title}: ${titles.join(', ')}`, DESCRIPTION_MAX_LENGTH);
}
