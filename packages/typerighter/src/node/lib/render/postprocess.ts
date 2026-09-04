// Post-process Rust-emitted HTML: replace code/math placeholders with shiki and Temml output

import temml from 'temml';
import {
  createHighlighter,
} from './highlight';
import {
  unescapeHtml,
} from '@/shared';

let highlighterPromise: Promise<(code: string, language: string, attrs: string) => Promise<string>> | undefined;

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
// <pre class="td-code-placeholder" data-lang="js" data-meta="js{1,3}"><code>escaped code</code></pre>
// Safe: Rust HTML-escapes the code content, so </code> cannot appear inside the placeholder
const CODE_PLACEHOLDER_RE = /<pre class="td-code-placeholder"(?: data-lang="([^"]*)")?(?: data-meta="([^"]*)")?><code>([\s\S]*?)<\/code><\/pre>/g;

async function highlightCodeBlocks (html: string): Promise<string> {
  const highlight = await getHighlighter();
  const matches = [...html.matchAll(CODE_PLACEHOLDER_RE)];

  if (matches.length === 0) return html;

  // Highlight all blocks in parallel
  const replacements = await Promise.all(
    matches.map(async (match) => {
      const language = match[1] || '';
      const meta = match[2] || '';
      const code = unescapeHtml(match[3]);

      const highlighted = await highlight(code, language, meta);

      return wrapCodeBlock(highlighted, language, meta);
    }),
  );

  let result = html;

  for (let index = matches.length - 1; 0 <= index; index--) {
    const match = matches[index];

    result = result.slice(0, match.index) + replacements[index] + result.slice(match.index + match[0].length);
  }

  return result;
}

// Wrap shiki output with language div, copy button, and language label
function wrapCodeBlock (highlighted: string, language: string, meta: string): string {
  language ||= 'txt';
  const label = language.replace(/_/g, ' ');

  let result =
    `<div class="language-${language}">`
    + '<button title="Copy code" data-copied="Copied" class="copy"></button>'
    + `<span class="lang">${label}</span>`
    + highlighted
    + '</div>';

  // Line numbers if :line-numbers is in the meta
  if (/:line-numbers\b/.test(meta)) {
    let startLineNumber = 1;
    const startMatch = meta.match(/=(\d+)/);

    if (startMatch?.[1]) {
      startLineNumber = parseInt(startMatch[1]);
    }

    const codeSection = result.slice(result.indexOf('<code>'), result.indexOf('</code>'));
    const lineCount = codeSection.split('\n').length;

    const lineNumbersHtml = [...Array(lineCount)]
      .map((_, index) => `<span class="line-number">${index + startLineNumber}</span><br>`)
      .join('');

    result = result
      .replace(/<\/div>$/, `<div class="line-numbers-wrapper" aria-hidden="true">${lineNumbersHtml}</div></div>`)
      .replace(/"(language-[^"]*?)"/, '"$1 line-numbers-mode"');
  }

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
