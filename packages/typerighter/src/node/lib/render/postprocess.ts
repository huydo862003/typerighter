// Post-process Rust-emitted HTML: replace code/math placeholders with shiki and Temml output

import temml from 'temml';
import {
  createHighlighter,
} from './highlight';
import {
  escapeHtml, unescapeHtml,
} from '@/shared';

let highlighterPromise:
  | Promise<(code: string, language: string, attrs: string, lineNumberStart?: number) => Promise<string>>
  | undefined;

// Replace all placeholder elements with rendered output
export async function postprocessHtml (html: string): Promise<string> {
  html = await highlightCodeBlocks(html);
  html = renderMathBlocks(html);

  return html;
}

function getHighlighter () {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter();
  }

  return highlighterPromise;
}

// Code block placeholder format from Rust:
// <pre class="td-code-placeholder" data-lang="js" data-meta="js{1,3}" data-title="foo.ts" data-line-numbers="5"><code>escaped code</code></pre>
// Safe: Rust HTML-escapes the code content, so </code> cannot appear inside the placeholder
const CODE_PLACEHOLDER_RE =
  /<pre class="td-code-placeholder"([^>]*)><code>([\s\S]*?)<\/code><\/pre>/g;

interface CodeBlockOptions {
  title?: string;
  lineNumbers?: string;
}

function getAttribute (attrs: string, name: string): string | undefined {
  const match = attrs.match(new RegExp(`${name}="([^"]*)"`));

  return match?.[1];
}

async function highlightCodeBlocks (html: string): Promise<string> {
  const highlight = await getHighlighter();
  const matches = [...html.matchAll(CODE_PLACEHOLDER_RE)];

  if (matches.length === 0) return html;

  // Highlight all blocks in parallel
  const replacements = await Promise.all(
    matches.map(async (match) => {
      const attrs = match[1];
      const language = getAttribute(attrs, 'data-lang') ?? '';
      const meta = getAttribute(attrs, 'data-meta') ?? '';
      const title = getAttribute(attrs, 'data-title');
      const lineNumbers = getAttribute(attrs, 'data-line-numbers');
      const code = unescapeHtml(match[2]);

      const lineNumberStart = lineNumbers !== undefined ? (parseInt(lineNumbers) || 1) : undefined;
      const highlighted = await highlight(code, language, meta, lineNumberStart);

      return wrapCodeBlock(highlighted, language, {
        title,
        lineNumbers,
      });
    }),
  );

  let result = html;

  for (let index = matches.length - 1; 0 <= index; index--) {
    const match = matches[index];

    result =
      result.slice(0, match.index)
      + replacements[index]
      + result.slice(match.index + match[0].length);
  }

  return result;
}

// Short display labels for common languages
const LANG_ABBREV: Record<string, string> = {
  typescript: 'TS',
  javascript: 'JS',
  tsx: 'TSX',
  jsx: 'JSX',
  python: 'PY',
  rust: 'RS',
  go: 'GO',
  java: 'JAVA',
  kotlin: 'KT',
  swift: 'SWIFT',
  ruby: 'RB',
  php: 'PHP',
  csharp: 'C#',
  cpp: 'C++',
  c: 'C',
  bash: 'SH',
  sh: 'SH',
  shell: 'SH',
  zsh: 'SH',
  html: 'HTML',
  css: 'CSS',
  scss: 'SCSS',
  sass: 'SASS',
  json: 'JSON',
  yaml: 'YML',
  yml: 'YML',
  toml: 'TOML',
  xml: 'XML',
  sql: 'SQL',
  markdown: 'MD',
  vue: 'VUE',
  svelte: 'SV',
  txt: 'TXT',
};

function languageAbbrev (language: string): string {
  return (
    LANG_ABBREV[language.toLowerCase()] ?? language.slice(0, 4).toUpperCase()
  );
}

function wrapCodeBlock (
  highlighted: string,
  language: string,
  options: CodeBlockOptions,
): string {
  language ||= 'txt';
  const badge = languageAbbrev(language);

  const classes = [
    'td-code-block',
    'language-' + language,
  ];

  if (options.title) classes.push('has-title');
  if (options.lineNumbers !== undefined) classes.push('line-numbers-mode');

  // <figure> wrapper with <figcaption> header
  let result = `<figure class="${classes.join(' ')}">`;

  // Header bar: [badge + label] left, [copy button] right
  result += '<figcaption class="td-code-header">';
  result += '<span class="td-code-left">';
  result += `<span class="td-code-lang">${escapeHtml(badge)}</span>`;
  if (options.title) {
    result += `<span class="td-code-label">${escapeHtml(options.title)}</span>`;
  }
  result += '</span>';
  result += '<span class="td-code-actions">';
  result +=
    '<button type="button" aria-label="Copy code" data-copied="Copied" class="td-code-copy">';
  result +=
    '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
  result += '<span class="td-code-copy-text">Copy</span>';
  result += '</button>';
  result += '</span>';
  result += '</figcaption>';

  // Code body - line number rows are already embedded by the Shiki transformer
  result += highlighted;

  result += '</figure>';

  return result;
}

// Math placeholder formats from Rust:
// <span class="td-math-inline">escaped latex</span>
// <div class="td-math-block">escaped latex</div>
const MATH_INLINE_RE = /<span class="td-math-inline">([\s\S]*?)<\/span>/g;
const MATH_BLOCK_RE = /<div class="td-math-block">([\s\S]*?)<\/div>/g;

function renderMathBlocks (html: string): string {
  html = html.replace(MATH_INLINE_RE, (_, tex) => renderTex(tex, false));
  html = html.replace(MATH_BLOCK_RE, (_, tex) => renderTex(tex, true));

  return html;
}

// Matches `code` and $math$ in plain-text strings (e.g. page titles, labels)
const INLINE_MARKUP_RE = /`([^`]+)`|\$([^$]+)\$/g;

// Render inline code and math in a plain-text string to HTML
// Used server-side to pre-render titles so clients do not need temml
export function renderInlineTitleHtml (text: string): string {
  let lastIndex = 0;
  let result = '';

  for (const match of text.matchAll(INLINE_MARKUP_RE)) {
    result += escapeHtml(text.slice(lastIndex, match.index!));

    if (match[1] !== undefined) {
      result += `<code>${escapeHtml(match[1])}</code>`;
    } else if (match[2] !== undefined) {
      try {
        result += temml.renderToString(match[2], {
          throwOnError: false,
        });
      } catch {
        result += escapeHtml(match[0]);
      }
    }

    lastIndex = match.index! + match[0].length;
  }

  result += escapeHtml(text.slice(lastIndex));

  return result;
}

function renderTex (escapedTex: string, displayMode: boolean): string {
  const tag = displayMode ? 'div' : 'span';

  try {
    return temml.renderToString(unescapeHtml(escapedTex), {
      throwOnError: false,
      displayMode,
    });
  } catch {
    return `<${tag} class="td-math-error">${escapedTex}</${tag}>`;
  }
}
