import type {
  BundledLanguage, ShikiTransformer,
} from 'shiki';
import {
  createHighlighter as _createHighlighter, guessEmbeddedLanguages, isSpecialLang,
} from 'shiki';

// Display labels for language names and file extensions
export const LANGUAGE_LABEL: Record<string, string> = {
  typescript: 'TypeScript',
  ts: 'TypeScript',
  javascript: 'JavaScript',
  js: 'JavaScript',
  mjs: 'JavaScript',
  cjs: 'JavaScript',
  tsx: 'TSX',
  jsx: 'JSX',
  python: 'Python',
  py: 'Python',
  rust: 'Rust',
  rs: 'Rust',
  go: 'Go',
  java: 'Java',
  kotlin: 'Kotlin',
  kt: 'Kotlin',
  swift: 'Swift',
  ruby: 'Ruby',
  rb: 'Ruby',
  php: 'PHP',
  csharp: 'C#',
  cs: 'C#',
  cpp: 'C++',
  c: 'C',
  bash: 'Bash',
  sh: 'Bash',
  shell: 'Shell',
  zsh: 'Zsh',
  html: 'HTML',
  css: 'CSS',
  scss: 'SCSS',
  sass: 'Sass',
  json: 'JSON',
  yaml: 'YAML',
  yml: 'YAML',
  toml: 'TOML',
  xml: 'XML',
  sql: 'SQL',
  markdown: 'Markdown',
  md: 'Markdown',
  vue: 'Vue',
  svelte: 'Svelte',
  racket: 'Racket',
  haskell: 'Haskell',
  elixir: 'Elixir',
  clojure: 'Clojure',
  lua: 'Lua',
  r: 'R',
  dart: 'Dart',
  zig: 'Zig',
  nim: 'Nim',
  ocaml: 'OCaml',
  erlang: 'Erlang',
  scala: 'Scala',
  txt: 'Text',
};

export const SHIKI_THEMES = {
  light: 'github-light' as const,
  dark: 'github-dark' as const,
};

export const SHIKI_COLOR_REPLACEMENTS = {
  [SHIKI_THEMES.light]: {
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
  [SHIKI_THEMES.dark]: {
    '#586069': '#5b93a3',
    '#6a737d': '#818e99',
    '#ea4a5a': '#ef5564',
    '#2188ff': '#268bf9',
  },
};

export type ShikiHighlightFn = (
  code: string,
  language: string,
  transformers: ShikiTransformer[],
  meta?: string,
) => Promise<string>;

// Create a Shiki highlighter with dual github themes
// Returns a composable highlight function - callers supply their own transformers
export async function createHighlighter (): Promise<ShikiHighlightFn> {
  const highlighter = await _createHighlighter({
    themes: [
      SHIKI_THEMES.light,
      SHIKI_THEMES.dark,
    ],
    langs: [],
  });

  const loadedLanguages = new Set<string>(highlighter.getLoadedLanguages());

  // Auto-dispose the Shiki instance when the returned function is GC'd
  const registry = new FinalizationRegistry<() => void>((dispose) => dispose());

  async function highlight (
    code: string,
    language: string,
    transformers: ShikiTransformer[],
    meta?: string,
  ): Promise<string> {
    language ||= 'txt';

    if (!isSpecialLang(language) && !loadedLanguages.has(language)) {
      try {
        await highlighter.loadLanguage(language as BundledLanguage);
        loadedLanguages.add(language);
      } catch {
        language = 'txt';
      }
    }

    code = code.trimEnd();

    const embedded = guessEmbeddedLanguages(code, language, highlighter);

    await highlighter.loadLanguage(...(embedded as BundledLanguage[]));

    return highlighter.codeToHtml(code, {
      lang: language,
      themes: SHIKI_THEMES,
      defaultColor: false,
      colorReplacements: SHIKI_COLOR_REPLACEMENTS,
      transformers,
      meta: meta !== undefined
        ? {
          __raw: meta,
        }
        : undefined,
    });
  }

  registry.register(highlight, () => highlighter.dispose());

  return highlight;
}

// Wrap each line span in a flex row with a line number cell and a code cell
export function transformerLineNumbers (startLine: number): ShikiTransformer {
  return {
    name: 'typedown:line-numbers',
    code (hast) {
      let lineIndex = 0;

      hast.children = hast.children
        .filter((node) => node.type === 'element')
        .map((node) => {
          if (node.tagName !== 'span') return node;

          const lineNum = startLine + lineIndex++;
          const classes = Array.isArray(node.properties.class) ? node.properties.class as string[] : [];
          const extraClasses = classes.filter((cls) => cls !== 'line');

          return {
            type: 'element',
            tagName: 'div',
            properties: {
              class: [
                'td-code-row',
                ...extraClasses,
              ],
            },
            children: [
              {
                type: 'element',
                tagName: 'span',
                properties: {
                  class: ['td-code-num'],
                },
                children: [
                  {
                    type: 'text',
                    value: String(lineNum),
                  },
                ],
              },
              {
                type: 'element',
                tagName: 'span',
                properties: {
                  class: ['td-code-col'],
                },
                children: node.children,
              },
            ],
          };
        });
    },
  };
}
