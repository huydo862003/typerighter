import katex from 'katex';
import {
  escapeHtml,
} from '@/shared';

const INLINE_MATH_RE = /\$([^$]+)\$/g;

// Render inline math and escape non-math segments for v-html
export function renderInlineMath (text: string): string {
  let lastIndex = 0;
  let result = '';

  for (const match of text.matchAll(INLINE_MATH_RE)) {
    result += escapeHtml(text.slice(lastIndex, match.index));

    try {
      result += katex.renderToString(match[1], {
        throwOnError: false,
        output: 'htmlAndMathml',
      });
    } catch {
      result += escapeHtml(match[0]);
    }

    lastIndex = match.index + match[0].length;
  }

  result += escapeHtml(text.slice(lastIndex));

  return result;
}
