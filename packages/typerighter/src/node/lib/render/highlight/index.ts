/*
* Based on https://github.com/vuejs/vitepress/blob/main/src/node/markdown/plugins/highlight.ts
* Commit: fccc617
* */

import {
  transformerMetaHighlight,
  transformerNotationDiff,
  transformerNotationErrorLevel,
  transformerNotationFocus,
  transformerNotationHighlight,
} from '@shikijs/transformers';
import type {
  ShikiTransformer,
} from 'shiki';
import {
  extractLanguage,
  isShellLanguage,
} from './language';
import {
  createHighlighter,
  transformerLineNumbers,
} from '@/shared/utils/highlight';
import {
  consoleLogger,
} from '@/node/lib/logger';

// Create a Shiki-based syntax highlighter with all features enabled
export async function createServerHighlighter (): Promise<(string_: string, language: string, attrs: string, lineNumberStart?: number) => Promise<string>> {
  const highlight = await createHighlighter();

  // Annotation transformers applied to every server-rendered code block
  const baseTransformers: ShikiTransformer[] = [
    transformerMetaHighlight(),       // ```js {1,3} - highlight specific lines
    transformerNotationDiff(),        // // [!code ++] / // [!code --] - diff markers
    transformerNotationFocus({        // // [!code focus] - dim unfocused lines
      classActiveLine: 'has-focus',
      classActivePre: 'has-focused-lines',
    }),
    transformerNotationHighlight(),   // // [!code highlight] - highlight a line
    transformerNotationErrorLevel(),  // // [!code error] / // [!code warning]
    transformerDisableShellSymbolSelect(),
    {
      // Force LTR direction on code blocks (code is always left-to-right)
      name: 'typedown:add-dir',
      pre (node) {
        node.properties.dir = 'ltr';
      },
    },
  ];

  return async function (string_: string, language: string, attrs: string, lineNumberStart?: number): Promise<string> {
    // Parse language and separate remaining attrs from the fence info
    // e.g. "js {1,3}" -> language="js", attrs="{1,3}"
    const extracted = extractLanguage(language);

    if (extracted) {
      const original = language;

      language = extracted.toLowerCase();
      attrs = original.slice(language.length).replace(/(?<!=)\{/g, ' {') + ' ' + attrs;
      attrs = attrs.trim().replace(/\s+/g, ' ');
    }

    language ||= 'txt';

    try {
      return await highlight(string_, language, [
        ...baseTransformers,
        {
          // Empty <span class="line"> elements collapse to zero height in the browser
          // When using line number rows, inject a space so the flex row still has height
          name: 'typedown:empty-line',
          code (hast) {
            hast.children.forEach((span) => {
              if (
                span.type === 'element'
                && span.tagName === 'span'
                && Array.isArray(span.properties.class)
                && span.properties.class.includes('line')
                && span.children.length === 0
              ) {
                span.children.push(
                  lineNumberStart !== undefined
                    ? {
                      type: 'text',
                      value: ' ',
                    }
                    : {
                      type: 'element',
                      tagName: 'wbr',
                      properties: {},
                      children: [],
                    },
                );
              }
            });
          },
        },
        ...(lineNumberStart !== undefined ? [transformerLineNumbers(lineNumberStart)] : []),
      ], attrs);
    } catch {
      consoleLogger.warn(
        `\nThe language '${language}' is not loaded, falling back to 'txt' for syntax highlighting.`,
      );

      return highlight(string_, 'txt', baseTransformers, attrs);
    }
  };
}

// Prevent shell prompt symbols from being selectable
function transformerDisableShellSymbolSelect (): ShikiTransformer {
  return {
    name: 'typedown:disable-shell-symbol-select',
    tokens (tokensByLine) {
      if (!isShellLanguage(this.options.lang)) return;

      for (const tokens of tokensByLine) {
        if (tokens.length < 2) continue;

        // The first token should only be a symbol token
        const firstTokenText = tokens[0].content.trim();

        if (firstTokenText !== '$' && firstTokenText !== '>') continue;

        // The second token must have a leading space (separates the symbol)
        if (tokens[1].content[0] !== ' ') continue;

        tokens[0].content = firstTokenText + ' ';
        tokens[0].htmlStyle ??= {};
        tokens[0].htmlStyle['user-select'] = 'none';
        tokens[0].htmlStyle['-webkit-user-select'] = 'none';
        tokens[1].content = tokens[1].content.slice(1);
      }
    },
  };
}
