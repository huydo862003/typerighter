import type {
  TdBuiltResource,
} from '@typerighter/rpc-client';
import type {
  TypedownContext,
} from '../typedown-context';
import type {
  MarkdownEnv,
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
  const env: MarkdownEnv = {
    path: filepath,
    relativePath: filepath,
    cleanUrls: true,
  };

  const html = await context.md.renderAsync(resource.content, env);
  const isIndex = path.filestem(filepath) === INDEX_FILENAME;
  const title = env.title || (isIndex
    ? getTdIndexTitle(filepath, (await context.getConfig()).siteTitle)
    : getTdResourceTitle(resource.header, filepath));

  const pageData = {
    schema: resource.schema,
    frontmatter: resource.header,
    headings: env.headers ?? [],
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
    `<template><div class="typedown-content">\n${neutralizeInterpolation(html)}\n</div></template>`,
  ].join('\n');

  return {
    vueSrc,
    pageData,
  };
}

/**
 * The rendered HTML becomes a Vue template, so `{{ ... }}` in authored content would be evaluated as an expression
 */
function neutralizeInterpolation (html: string): string {
  return html.replaceAll('{{', '&#123;&#123;');
}
