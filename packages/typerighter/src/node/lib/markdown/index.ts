/*
* Based on https://github.com/vuejs/vitepress/blob/main/src/node/markdown/markdown.ts
* Commit: 2fa0ded
* */

import {
  headersPlugin,
} from '@mdit-vue/plugin-headers';
import {
  titlePlugin,
} from '@mdit-vue/plugin-title';
import {
  slugify,
} from '@mdit-vue/shared';
import {
  anchor,
} from '@mdit/plugin-anchor';
import {
  fullEmoji,
} from '@mdit/plugin-emoji';
import {
  tasklist,
} from '@mdit/plugin-tasklist';
import {
  MarkdownItAsync,
} from 'markdown-it-async';
import {
  katex,
} from '@mdit/plugin-katex';
import type {
  TdSiteConfig,
} from '@typerighter/rpc-client';
import type MarkdownIt from 'markdown-it';
import {
  calloutContainerPlugin,
} from './plugins/callout-container';
import {
  createHighlighter,
} from './plugins/highlight';
import {
  imagePlugin,
} from './plugins/image';
import {
  lineNumberPlugin,
} from './plugins/line-number';
import {
  linkPlugin,
} from './plugins/link';
import {
  preWrapperPlugin,
} from './plugins/pre-wrapper';
import {
  restoreEntities,
} from './plugins/restore-entities';
import {
  tablePlugin,
} from './plugins/table';
import {
  componentContainerPlugin,
} from './plugins/custom-container';

export type MarkdownRenderer = MarkdownItAsync;

export async function createMarkdownRenderer (
  config: TdSiteConfig,
): Promise<MarkdownRenderer> {
  const base = config.basePath;
  const highlight = await createHighlighter();

  const md = new MarkdownItAsync({
    linkify: true,
    highlight,
  }) as MarkdownIt & MarkdownItAsync;

  md.linkify.set({
    fuzzyLink: false,
  });
  restoreEntities(md);

  /* Typedown plugins */

  preWrapperPlugin(md);
  lineNumberPlugin(md);
  calloutContainerPlugin(md);
  componentContainerPlugin(md);
  imagePlugin(md);
  linkPlugin(md, {
    target: '_blank',
    rel: 'noreferrer',
  }, base, slugify);
  tablePlugin(md);

  /* Community plugins */

  fullEmoji(md);
  tasklist(md);
  anchor(md, {
    slugify,
    getTokensText: (tokens) => {
      return tokens
        .filter((token) => ![
          'html_inline',
          'emoji',
        ].includes(token.type))
        .map((token) => token.content)
        .join('');
    },
    permalink: (slug, _, state, index) => {
      const title =
        state.tokens[index + 1]?.children
          ?.filter((token) => [
            'text',
            'code_inline',
          ].includes(token.type))
          .reduce((accumulator, token) => accumulator + token.content, '')
          .trim() || '';

      const linkTokens = [
        Object.assign(new state.Token('link_open', 'a', 1), {
          attrs: [
            [
              'class',
              'header-anchor',
            ],
            [
              'href',
              `#${slug}`,
            ],
            [
              'aria-label',
              `Permalink to "${title}"`,
            ],
          ],
        }),
        Object.assign(new state.Token('html_inline', '', 0), {
          content: '&#8203;',
          meta: {
            isPermalinkSymbol: true,
          },
        }),
        new state.Token('link_close', 'a', -1),
      ];

      const space = new state.Token('text', '', 0);

      space.content = ' ';
      state.tokens[index + 1].children?.push(space, ...linkTokens);
    },
  });
  katex(md, {
    // Render a broken formula as a red error span instead of failing the build
    throwOnError: false,
    // MathML alongside the HTML keeps the maths readable to screen readers
    output: 'htmlAndMathml',
  });

  /* mdit-vue plugins */

  headersPlugin(md, {
    level: [
      2,
      3,
      4,
      5,
      6,
    ],
    slugify,
  });
  titlePlugin(md);

  return md;
}
