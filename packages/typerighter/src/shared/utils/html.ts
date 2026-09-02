export function escapeHtml (value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

export function stripHtml (html: string): string {
  return html
    // Remove style/script blocks entirely (including MathJax injected styles)
    .replace(/<(style|script)[^>]*>[\s\S]*?<\/\1>/gi, '')
    // Strip angle brackets directly to avoid incomplete multi-character tag sanitization
    .replace(/[<>]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}
