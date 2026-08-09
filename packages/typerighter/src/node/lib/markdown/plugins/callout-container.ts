/*
* Copied from https://github.com/vuejs/vitepress/blob/main/src/node/markdown/plugins/containers.ts
* Commit: b6d9cb8
* */

/* This file lets you create block-level callout containers in Markdown
* Example: ::: warning :::
* */

import {
  container,
} from '@mdit/plugin-container';
import type {
  MarkdownItAsync,
} from 'markdown-it-async';
import type {
  RenderRule,
} from 'markdown-it/lib/renderer.mjs';

export function calloutContainerPlugin (
  md: MarkdownItAsync,
): void {
  for (const name of Object.keys(DEFAULT_TITLES)) {
    md.use(container, {
      name,
      openRender: createOpenRender(md, name),
      closeRender: () => (name === 'details' ? '</details>\n' : '</div>\n'),
    });
  }
}

const DEFAULT_TITLES: Record<string, string> = {
  tip: 'TIP',
  info: 'INFO',
  warning: 'WARNING',
  danger: 'DANGER',
  details: 'Details',
  note: 'NOTE',
  important: 'IMPORTANT',
  caution: 'CAUTION',
};

function createOpenRender (
  md: MarkdownItAsync,
  name: string,
): RenderRule {
  return (tokens, index) => {
    const token = tokens[index];

    // `::: warning Don't do this` -> `"Don't do this"`
    const info = token.info.trim().slice(name.length)
      .trim();

    // Build HTML attributes (e.g. `class="warning custom-block"`)
    token.attrJoin('class', `${name} custom-block`);
    const renderedAttrs = md.renderer.renderAttrs(token).trim();

    // Render the title as inline markdown so e.g. **bold** works in titles
    const title = md.renderInline(info || DEFAULT_TITLES[name]);

    // ::: details renders as <details><summary>
    if (name === 'details')
      return `<details ${renderedAttrs}><summary>${title}</summary>\n`;
    // When the user provides a custom title, omit the `-default` class
    const titleClass =
      'custom-block-title' + (info ? '' : ' custom-block-title-default');

    return `<div ${renderedAttrs}><p class="${titleClass}">${title}</p>\n`;
  };
}
