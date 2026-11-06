// RSS feed generation

import type {
  TdSidebarItem,
} from '@typerighter/rpc-client';
import {
  escapeHtml, path as tdpath,
} from '@/shared';

// Generate an RSS 2.0 feed from sidebar items
export function generateRssFeed (
  items: TdSidebarItem[],
  base: string,
  origin: string,
  title: string,
  description: string,
  lang: string,
): string {
  const sorted = [...items]
    .filter((item) => item.fileMetadata.mtimeSecs > 0)
    .sort((first, second) => second.fileMetadata.mtimeSecs - first.fileMetadata.mtimeSecs)
    .slice(0, 50);

  const feedItems = sorted.map((item) => {
    const pageRoute = tdpath.stripExtension(item.filepath);
    const url = `${origin}${base}${pageRoute}`;
    const pubDate = new Date(item.fileMetadata.mtimeSecs * 1000).toUTCString();
    const itemTitle = escapeHtml(item.label ?? pageRoute);
    const itemDescription = item.excerpt !== undefined ? `<![CDATA[${item.excerpt}]]>` : '';

    return `    <item>
      <title>${itemTitle}</title>
      <link>${escapeHtml(url)}</link>
      <guid>${escapeHtml(url)}</guid>
      <pubDate>${pubDate}</pubDate>
      <description>${itemDescription}</description>
    </item>`;
  }).join('\n');

  return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>${escapeHtml(title)}</title>
    <link>${escapeHtml(origin + base)}</link>
    <description>${escapeHtml(description)}</description>
    <language>${lang}</language>
    <atom:link href="${escapeHtml(origin + base)}feed.xml" rel="self" type="application/rss+xml"/>
${feedItems}
  </channel>
</rss>
`;
}
