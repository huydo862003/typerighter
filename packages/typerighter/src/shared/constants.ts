export const INDEX_FILENAME = 'index';
export const CONTENT_EXTENSIONS: readonly string[] = [
  '.td',
  '.md',
];
export const CONTENT_GLOB = '**/*.{td,md}';

// Fields indexed by MiniSearch for full-text matching
export const SEARCH_FIELDS: string[] = [
  'title',
  'text',
];

// Fields stored in the index and retrievable from search results without re-loading the page
export const SEARCH_STORE_FIELDS: string[] = ['title'];
