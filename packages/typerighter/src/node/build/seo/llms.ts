import type {
  TdSidebarItem,
} from '@typerighter/rpc-client';
import {
  isIndexFile, path as tdpath, unslugify,
} from '@/shared';

// llms.txt per the llmstxt.org spec: H1 title, optional description blockquote, then sections grouping pages by top-level directory
export function buildLlmsTxt (
  items: TdSidebarItem[],
  base: string,
  origin: string,
  siteTitle: string,
  siteDescription: string | undefined,
): string {
  const lines: string[] = [];

  lines.push(`# ${siteTitle}`);
  lines.push('');

  if (siteDescription) {
    lines.push(`> ${siteDescription}`);
    lines.push('');
  }

  // Group pages by top-level directory
  const sections = new Map<string, TdSidebarItem[]>();

  for (const item of items) {
    const parts = item.filepath.split('/');
    const section = parts.length > 1 ? parts[0] : '';

    if (!sections.has(section)) sections.set(section, []);
    sections.get(section)!.push(item);
  }

  for (const [section, sectionItems] of sections) {
    const sectionLabel = section ? unslugify(section) : 'General';

    lines.push(`## ${sectionLabel}`);
    lines.push('');

    for (const item of sectionItems) {
      const route = tdpath.stripExtension(item.filepath);
      // Use directory path for index files so the URL matches how static hosts serve them
      const pageRoute = isIndexFile(item.filepath)
        ? route.replace(/\/index$/, '/').replace(/^index$/, '')
        : route;
      const url = `${origin}${base}${pageRoute}`;
      const label = item.label ?? unslugify(tdpath.filestem(item.filepath));
      const description = item.excerpt ? `: ${item.excerpt}` : '';

      lines.push(`- [${label}](${url})${description}`);
    }

    lines.push('');
  }

  return lines.join('\n');
}
