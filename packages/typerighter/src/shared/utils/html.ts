import striptags from 'striptags';

// All HTML inputs come from the Rust emitter (author-provided content)
// The emitter never generates event handlers or script elements

export function escapeHtml (value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

export function stripHtml (html: string): string {
  return unescapeHtml(striptags(html))
    .replace(/\s+/g, ' ')
    .trim();
}

// Keep safe inline and MathML elements, strip everything else
const INLINE_ALLOWED_TAGS = [
  'code',
  'em',
  'strong',
  's',
  'span',
  'sub',
  'sup',
  'math',
  'mrow',
  'mi',
  'mn',
  'mo',
  'mfrac',
  'msup',
  'msub',
  'msubsup',
  'msqrt',
  'mroot',
  'mtext',
  'mspace',
  'merror',
  'semantics',
  'annotation',
  'mover',
  'munder',
  'munderover',
  'mtable',
  'mtr',
  'mtd',
];

export function sanitizeInlineHtml (html: string | undefined): string {
  if (html === undefined) return '';

  return striptags(html, INLINE_ALLOWED_TAGS);
}

export function unescapeHtml (html: string): string {
  return html
    .replaceAll('&lt;', '<')
    .replaceAll('&gt;', '>')
    .replaceAll('&quot;', '"')
    .replaceAll('&#39;', '\'')
    // &amp; decoded last to avoid double-decoding (e.g. &amp;lt; should become &lt;, not <)
    .replaceAll('&amp;', '&');
}
