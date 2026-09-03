import {
  escapeHtml, escapeRegex,
} from '@/shared';

export function highlight (text: string, searchQuery: string): string {
  if (!text || !searchQuery) return escapeHtml(text);
  const terms = searchQuery.trim().split(/\s+/)
    .filter(Boolean);
  let result = escapeHtml(text);

  for (const term of terms) {
    const regex = new RegExp(`(${escapeRegex(term)})`, 'gi');

    result = result.replace(regex, '<mark>$1</mark>');
  }

  return result;
}
