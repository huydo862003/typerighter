import striptags from 'striptags';

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

export function unescapeHtml (html: string): string {
  return html
    .replaceAll('&lt;', '<')
    .replaceAll('&gt;', '>')
    .replaceAll('&quot;', '"')
    .replaceAll('&#39;', '\'')
    // &amp; decoded last to avoid double-decoding (e.g. &amp;lt; should become &lt;, not <)
    .replaceAll('&amp;', '&');
}
