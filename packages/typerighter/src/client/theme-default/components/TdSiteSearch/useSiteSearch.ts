import {
  computed, createApp, markRaw, shallowRef, triggerRef, watch,
  type ShallowRef,
} from 'vue';
import MiniSearch from 'minisearch';
import {
  usePageLoader, useSearchIndex,
} from '@/client/app';
import {
  debounce,
  getAnchor, stripAnchor, stripHtml,
  SEARCH_FIELDS, SEARCH_STORE_FIELDS,
  type PageModule,
} from '@/shared';

export interface SearchResult {
  id: string;
  title: string;
  titles: string[];
  excerpt: string;
  score: number;
}

const EXCERPT_CHARS = 120;
const HEADING_RE = /^h[1-6]$/i;
const CACHE_SIZE = 16;

export function useSiteSearch () {
  const searchIndex = useSearchIndex();
  const loadPage = usePageLoader();

  let engine: MiniSearch | undefined;
  let canceled = false;

  const results = shallowRef<SearchResult[]>([]);
  const searching = shallowRef(false);
  const sectionCache = new Map<string, Map<string, string>>();

  // Invalidate engine when the search index changes (e.g. HMR)
  watch(searchIndex, () => {
    engine = undefined;
    sectionCache.clear();
  });

  // Lazily deserialize the MiniSearch index on first search
  function getEngine (): MiniSearch | undefined {
    if (engine) return engine;
    if (!searchIndex.value) return undefined;
    engine = markRaw(MiniSearch.loadJSON(searchIndex.value, {
      fields: SEARCH_FIELDS,
      storeFields: SEARCH_STORE_FIELDS,
    }));

    return engine;
  }

  // Mount a page component in a throwaway div and extract text per heading section
  function extractSections (component: PageModule['default']): Map<string, string> {
    const sections = new Map<string, string>();
    const app = createApp(component);
    const container = document.createElement('div');

    app.config.warnHandler = () => {};

    try {
      app.mount(container);
      const headings = container.querySelectorAll('h1, h2, h3, h4, h5, h6');

      for (const heading of headings) {
        const anchor = heading.querySelector('a')?.getAttribute('href')
          ?.slice(1) ?? '';
        let html = '';
        let sibling = heading.nextElementSibling;

        while (sibling && !HEADING_RE.test(sibling.tagName)) {
          html += sibling.outerHTML;
          sibling = sibling.nextElementSibling;
        }

        sections.set(anchor, stripHtml(html));
      }

      if (sections.size === 0) {
        sections.set('', stripHtml(container.innerHTML));
      } else {
        sections.set('', [...sections.values()].join(' '));
      }
    } finally {
      app.unmount();
    }

    return sections;
  }

  // Load a page and return the text for a specific section, using LRU cache
  async function getSectionText (documentId: string): Promise<string> {
    if (!loadPage) return '';

    const pageUrl = stripAnchor(documentId);
    const anchor = getAnchor(documentId);
    let sections = sectionCache.get(pageUrl);

    if (!sections) {
      try {
        const module_ = await loadPage(pageUrl);
        const component = module_?.default;

        if (!component) return '';
        sections = extractSections(component);

        if (CACHE_SIZE <= sectionCache.size) {
          const oldest = sectionCache.keys().next().value;

          if (oldest !== undefined) sectionCache.delete(oldest);
        }

        sectionCache.set(pageUrl, sections);
      } catch {
        return '';
      }
    }

    return sections.get(anchor) ?? sections.get('') ?? '';
  }

  // Build a short snippet centered on the earliest matched term
  function extractSnippet (text: string, match: Record<string, string[]>): string {
    if (!text) return '';
    const terms = Object.keys(match);

    if (terms.length === 0) return text.slice(0, EXCERPT_CHARS);

    const lower = text.toLowerCase();
    let earliest = text.length;

    for (const term of terms) {
      const index = lower.indexOf(term.toLowerCase());

      if (index !== -1 && index < earliest) earliest = index;
    }

    const start = Math.max(0, earliest - 30);
    const end = Math.min(text.length, start + EXCERPT_CHARS);
    let snippet = text.slice(start, end).trim();

    if (0 < start) snippet = '...' + snippet;
    if (end < text.length) snippet = snippet + '...';

    return snippet;
  }

  // Show results immediately with titles, then fill in excerpts progressively
  async function runSearch (trimmed: string) {
    const index = getEngine();

    if (!index) {
      searching.value = false;

      return;
    }

    const rawResults = index.search(trimmed, {
      prefix: true,
      fuzzy: (term: string) => (3 < term.length ? 0.2 : false),
      boost: {
        title: 4,
        text: 2,
        titles: 1,
      },
    });

    const top = rawResults.slice(0, 20);

    results.value = top.map((result) => ({
      id: result.id,
      title: result.title as string,
      titles: result.titles as string[],
      excerpt: '',
      score: result.score,
    }));
    searching.value = false;

    for (const [
      index_,
      result,
    ] of top.entries()) {
      if (canceled) return;

      const sectionText = await getSectionText(result.id);
      const excerpt = extractSnippet(sectionText, result.match);

      if (canceled) return;

      results.value[index_].excerpt = excerpt;
      triggerRef(results);
    }
  }

  const debouncedSearch = debounce((trimmed: string) => runSearch(trimmed), 150);

  function search (query: string) {
    canceled = false;
    const trimmed = query.trim();

    if (!trimmed) {
      results.value = [];
      searching.value = false;

      return;
    }

    searching.value = true;
    debouncedSearch(trimmed);
  }

  function cancel () {
    canceled = true;
  }

  return {
    results: results as Readonly<ShallowRef<SearchResult[]>>,
    searching: computed(() => searching.value),
    search,
    cancel,
  };
}
