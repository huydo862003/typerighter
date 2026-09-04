import temml from 'temml';
import {
  escapeHtml,
} from '@/shared';

// Matches inline code (`...`) and inline math ($...$)
const INLINE_RE = /`([^`]+)`|\$([^$]+)\$/g;

// Render inline code and math, escape everything else for v-html
export function renderInlineMarkup (text: string): string {
  let lastIndex = 0;
  let result = '';

  for (const match of text.matchAll(INLINE_RE)) {
    result += escapeHtml(text.slice(lastIndex, match.index));

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

    lastIndex = match.index + match[0].length;
  }

  result += escapeHtml(text.slice(lastIndex));

  return result;
}
