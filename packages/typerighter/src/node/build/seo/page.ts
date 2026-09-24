// Extract per-page SEO data from PageData

import type {
  PageData,
} from '@/shared/types/ssg';
import {
  formatIsoDate,
} from '@/shared';

// Resolve the effective page title, preferring _meta override
export function resolvePageTitle (pageData: PageData): string {
  return pageData.meta?.title ?? pageData.title;
}

// Resolve the effective page description, preferring _meta override
export function resolvePageDescription (pageData: PageData): string {
  if (pageData.meta?.description !== undefined) return pageData.meta.description;

  const description = pageData.frontmatter.description;

  return description !== undefined ? String(description) : '';
}

// Extract dateModified from file mtime as YYYY-MM-DD
export function extractDateModified (pageData: PageData): string | undefined {
  const mtime = pageData.metadata?.mtime;

  return mtime !== undefined
    ? formatIsoDate(mtime)
    : undefined;
}

// Extract per-page og:image path from _meta
export function extractOgImagePath (pageData: PageData): string | undefined {
  return pageData.meta?.image;
}
