import {
  markRaw, shallowRef, watch,
} from 'vue';
import MiniSearch from 'minisearch';
import {
  useSearchIndex,
} from '@/client/app';
import {
  debounce,
  SEARCH_FIELDS, SEARCH_STORE_FIELDS,
} from '@/shared';

export interface SearchResult {
  id: string;
  title: string;
  excerpt: string;
  score: number;
}

const EXCERPT_CHARS = 120;

export function useSiteSearch () {
  const searchIndex = useSearchIndex();

  let engine: MiniSearch | undefined;
  let canceled = false;

  const results = shallowRef<SearchResult[]>([]);
  const searching = shallowRef(false);

  // Invalidate engine when the search index changes (e.g. HMR)
  watch(searchIndex, () => {
    engine = undefined;
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
      },
    });

    const top = rawResults.slice(0, 20);

    // Build excerpts synchronously from text stored in the index
    results.value = top.map((result) => ({
      id: result.id,
      title: result.title as string,
      excerpt: extractSnippet((result.text as string) ?? '', result.match),
      score: result.score,
    }));
    searching.value = false;
  }

  const debouncedSearch = debounce(runSearch, 150);

  function search (raw: string) {
    canceled = false;
    const trimmed = raw.trim();

    if (trimmed.length === 0) {
      results.value = [];
      searching.value = false;

      return;
    }

    searching.value = true;
    debouncedSearch(trimmed);
  }

  function cancel () {
    canceled = true;
    searching.value = false;
  }

  const indexLoaded = shallowRef(false);

  watch(searchIndex, (value) => {
    if (value) indexLoaded.value = true;
  }, {
    immediate: true,
  });

  return {
    results,
    searching,
    indexLoaded,
    search,
    cancel,
  };
}
