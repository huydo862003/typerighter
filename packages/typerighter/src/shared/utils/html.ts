export function escapeHtml (value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

export function stripHtml (html: string): string {
  return html
    // Strip angle brackets directly to avoid incomplete multi-character tag sanitization
    .replace(/[<>]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}
