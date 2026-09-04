import type {
  TdBuiltResource, TdHeading,
} from '@typerighter/rpc-client';
import type {
  TypedownContext,
} from '../typedown-context';
import {
  postprocessHtml,
} from './postprocess';
import type {
  MarkdownHeading,
  PageData,
} from '@/shared';
import {
  getTdIndexTitle, getTdResourceTitle, INDEX_FILENAME, path,
} from '@/shared';

export interface VueRenderResult {
  /** The rendered Vue SFC string */
  vueSrc: string;
  /** Page metadata */
  pageData: PageData;
}

// Render a built resource to a Vue SFC string
export async function renderToVueSfc (
  context: TypedownContext,
  resource: TdBuiltResource,
  filepath: string,
): Promise<VueRenderResult> {
  const html = await postprocessHtml(resource.content);
  const isIndex = path.filestem(filepath) === INDEX_FILENAME;
  const title = resource.title || (isIndex
    ? getTdIndexTitle(filepath, (await context.getConfig()).siteTitle)
    : getTdResourceTitle(filepath, resource.label));

  const pageData: PageData = {
    schema: resource.schema,
    schemaLabel: resource.schemaLabel,
    label: resource.label,
    icon: resource.icon,
    frontmatter: resource.header,
    headings: buildHeadingTree(resource.headings),
    title,
    metadata: resource.metadata,
  };
  const pageDataJson = JSON.stringify(JSON.stringify(pageData));

  const vueSrc = [
    '<script>',
    `export const __pageData = JSON.parse(${pageDataJson})`,
    `export default { name: ${JSON.stringify(filepath)} }`,
    '</script>',
    // Inlined rather than bound with `v-html`: the content has to pass through the Vue compiler for custom components and their slots to resolve
    `<template><div class="typedown-content">\n${escapeVueInterpolation(html)}\n</div></template>`,
  ].join('\n');

  return {
    vueSrc,
    pageData,
  };
}

// Build a nested heading tree from a flat list
// h3 nests under the preceding h2, h4 under preceding h3, etc
function buildHeadingTree (flat: TdHeading[]): MarkdownHeading[] {
  const root: MarkdownHeading[] = [];
  const stack: MarkdownHeading[] = [];

  for (const heading of flat) {
    const node: MarkdownHeading = {
      level: heading.level,
      title: heading.title,
      slug: heading.slug,
      link: `#${heading.slug}`,
      children: [],
    };

    // Pop stack until we find a parent with a lower level
    while (0 < stack.length && heading.level <= stack[stack.length - 1].level) {
      stack.pop();
    }

    if (0 < stack.length) {
      stack[stack.length - 1].children.push(node);
    } else {
      root.push(node);
    }

    stack.push(node);
  }

  return root;
}

// The rendered HTML becomes a Vue template, so {{ ... }} would be evaluated as an expression
function escapeVueInterpolation (html: string): string {
  return html.replaceAll('{{', '&#123;&#123;');
}
