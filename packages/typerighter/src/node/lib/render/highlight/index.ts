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
  BundledLanguage, ShikiTransformer,
} from 'shiki';
import {
  createHighlighter as _createHighlighter, guessEmbeddedLanguages, isSpecialLang,
} from 'shiki';
import {
  extractLanguage,
  isShellLanguage,
} from './language';
import {
  consoleLogger,
} from '@/node/lib/logger';

// Auto-dispose the Shiki highlighter when it is GC'd
const highlighterRegistry = new FinalizationRegistry<() => void>((dispose) => {
  dispose();
});

const THEME = {
  light: 'github-light' as const,
  dark: 'github-dark' as const,
};

// Create a Shiki-based syntax highlighter with all features enabled
export async function createHighlighter (): Promise<(string_: string, language: string, attrs: string) => Promise<string>> {
  const highlighter = await _createHighlighter({
    themes: [
      THEME.light,
      THEME.dark,
    ],
    langs: [],
  });

  // Built-in transformers applied to every code block
  const transformers: ShikiTransformer[] = [
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

  const loadedLanguages = new Set<string>(highlighter.getLoadedLanguages());

  async function highlight (string_: string, language: string, attrs: string): Promise<string> {
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

    // Lazy-load the language grammar if not already available
    try {
      // https://github.com/shikijs/shiki/issues/952
      if (
        !isSpecialLang(language)
        && !loadedLanguages.has(language)
      ) {
        await highlighter.loadLanguage(language as BundledLanguage);
        loadedLanguages.add(language);
      }
    } catch {
      consoleLogger.warn(
        `\nThe language '${language}' is not loaded, falling back to 'txt' for syntax highlighting.`,
      );
      language = 'txt';
    }

    string_ = string_.trimEnd();

    // Load any languages embedded in the code (e.g. CSS inside HTML)
    const embeddedLanguage = guessEmbeddedLanguages(string_, language, highlighter);

    await highlighter.loadLanguage(...(embeddedLanguage as BundledLanguage[]));

    // Convert code to highlighted HTML with post-processing transformers
    const highlighted = highlighter.codeToHtml(string_, {
      lang: language,
      transformers: [
        ...transformers,
        {
          // Empty <span class="line"> elements collapse to zero height in the browser
          // Inject a <wbr> to preserve blank line spacing
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
                span.children.push({
                  type: 'element',
                  tagName: 'wbr',
                  properties: {},
                  children: [],
                });
              }
            });
          },
        },
      ],
      // Pass raw fence attributes (e.g. line highlight ranges `{1,3-5}`) to transformers
      meta: {
        __raw: attrs,
      },
      themes: THEME,
      defaultColor: false,
      // Improve contrast for GitHub themes
      colorReplacements: {
        'github-light': {
          '#959da5': '#6c676f',
          '#28a745': '#0e790b',
          '#b08800': '#846312',
          '#e36209': '#c13617',
          '#3192aa': '#05728b',
          '#d73a49': '#c62739',
          '#22863a': '#11782a',
          '#6a737d': '#62687b',
          '#1b7c83': '#06747a',
          '#0366d6': '#0663d0',
          '#cb2431': '#c82430',
        },
        'github-dark': {
          '#586069': '#5b93a3',
          '#6a737d': '#818e99',
          '#ea4a5a': '#ef5564',
          '#2188ff': '#268bf9',
        },
      },
    });

    return highlighted;
  }

  highlighterRegistry.register(highlight, () => highlighter.dispose());

  return highlight;
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
