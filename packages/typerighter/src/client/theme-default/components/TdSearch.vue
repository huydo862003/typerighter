<!-- Adapted from VitePress VPLocalSearchBox.vue -->
<!-- https://github.com/vuejs/vitepress/blob/main/src/client/theme-default/components/VPLocalSearchBox.vue -->
<script setup lang="ts">
import {
  computed, markRaw, ref, shallowRef, watch,
} from 'vue';
import MiniSearch from 'minisearch';
import {
  LoaderCircle, Search, X,
} from '@lucide/vue';
import {
  usePageLoader, useRoute, useSearchIndex,
} from '../../app';
import {
  debounce,
  escapeHtml, escapeRegex,
  SEARCH_FIELDS, SEARCH_STORE_FIELDS, stripHtml,
  type PageModule,
} from '@/shared';

interface SearchResult {
  id: string;
  title: string;
  titles: string[];
  excerpt: string;
  score: number;
}

const active = defineModel<boolean>('active', {
  default: false,
});
const query = defineModel<string>('query', {
  default: '',
});

const emit = defineEmits<{
  select: [];
}>();

const route = useRoute();

const EXCERPT_CHARS = 120;

const results = shallowRef<SearchResult[]>([]);
const searching = ref(false);
const selectedIndex = ref(-1);
const isSearchActive = computed(() => 0 < query.value.trim().length);

watch(isSearchActive, (value) => {
  active.value = value;
});

const searchIndex = useSearchIndex();
const loadPage = usePageLoader();
let engine: MiniSearch | undefined;

// Rebuild engine when the search index changes (e.g. HMR update)
watch(searchIndex, () => {
  engine = undefined;
});

// Lazily deserialize the MiniSearch index on first search
function getEngine (): MiniSearch | undefined {
  if (engine) return engine;
  if (!searchIndex.value) return undefined;
  engine = markRaw(MiniSearch.loadJSON(searchIndex.value, {
    // fields/storeFields must match the build-time indexer in search.ts
    fields: SEARCH_FIELDS,
    storeFields: SEARCH_STORE_FIELDS,
  }));

  return engine;
}

let canceled = false;

const debouncedSearch = debounce((trimmed: string) => runSearch(trimmed), 150);

watch(query, (value, _old, onCleanup) => {
  onCleanup(() => {
    canceled = true;
  });
  canceled = false;

  const trimmed = value.trim();

  if (!trimmed) {
    results.value = [];
    searching.value = false;

    return;
  }
  searching.value = true;
  selectedIndex.value = -1;
  debouncedSearch(trimmed);
});

function clearQuery () {
  query.value = '';
}

// Extract text between the target heading and the next heading
function extractSection (html: string, anchor: string): string {
  // Anchor may contain regex-special chars (e.g. dots in generated ids)
  const regex = new RegExp(`id="${escapeRegex(anchor)}"[^>]*>`, 'i');
  const headingMatch = regex.exec(html);

  if (!headingMatch) return stripHtml(html);

  const afterHeading = html.slice(headingMatch.index + headingMatch[0].length);

  const nextHeading = afterHeading.search(/<h[1-6]/i);
  const section = 0 <= nextHeading ? afterHeading.slice(0, nextHeading) : afterHeading;

  return stripHtml(section);
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

// Load a page module and extract a text excerpt for the matched section
async function fetchExcerpt (
  loadPage: (path: string) => Promise<PageModule | undefined>,
  documentId: string,
  match: Record<string, string[]>,
): Promise<string> {
  // Document ids are "page-url" or "page-url#anchor"
  const hashIndex = documentId.indexOf('#');
  const pageUrl = 0 <= hashIndex ? documentId.slice(0, hashIndex) : documentId;
  const anchor = 0 <= hashIndex ? documentId.slice(hashIndex + 1) : '';

  try {
    const module_ = await loadPage(pageUrl);
    // Vue SFC pages store rendered HTML in component.data().__html
    const component = module_?.default as Record<string, unknown> | undefined;
    const data = typeof component?.data === 'function' ? (component.data as () => Record<string, string>)() : {};
    const html = data.__html ?? '';
    const sectionText = anchor ? extractSection(html, anchor) : stripHtml(html);

    return extractSnippet(sectionText, match);
  } catch {
    return '';
  }
}

// Highlight matched terms in text by wrapping in <mark> tags
function highlight (text: string, searchQuery: string): string {
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

// Check if a result id belongs to the currently viewed page
function isCurrent (resultId: string): boolean {
  const page = resultId.split('#')[0];

  return route.path === page;
}

function onKeydown (event: KeyboardEvent) {
  const count = results.value.length;

  if (count === 0) return;

  if (event.key === 'ArrowDown') {
    event.preventDefault();
    selectedIndex.value = (selectedIndex.value + 1) % count;
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    selectedIndex.value = (selectedIndex.value - 1 + count) % count;
  } else if (event.key === 'Enter' && 0 <= selectedIndex.value) {
    event.preventDefault();
    const selected = results.value[selectedIndex.value];
    const link = document.querySelector<HTMLAnchorElement>(`.td-search-result[href="${CSS.escape(selected.id)}"]`);

    link?.click();
  }
}

function onResultClick () {
  emit('select');
}

// Run search and fetch excerpts for the top results
async function runSearch (trimmed: string) {
  const index = getEngine();

  if (!index) return;

  try {
    const rawResults = index.search(trimmed, {
      prefix: true,
      fuzzy: 0.2,
      boost: {
        title: 2,
      },
    });

    // Cap results to avoid fetching too many page modules at once
    const withExcerpts = await Promise.all(
      rawResults.slice(0, 20).map(async (result) => {
        const excerpt = loadPage
          ? await fetchExcerpt(loadPage, result.id, result.match)
          : '';

        return {
          id: result.id,
          title: result.title as string,
          titles: result.titles as string[],
          excerpt,
          score: result.score,
        };
      }),
    );

    // Query changed while awaiting excerpts, discard stale results
    if (canceled) return;
    results.value = withExcerpts;
  } finally {
    if (!canceled) searching.value = false;
  }
}
</script>

<template>
  <div class="td-search">
    <div class="td-search-input-wrap">
      <Search
        :size="14"
        class="td-search-icon"
      />
      <input
        v-model="query"
        class="td-search-input"
        type="text"
        placeholder="Search..."
        @keydown="onKeydown"
      >
      <LoaderCircle
        v-if="searching"
        :size="12"
        class="td-search-spinner"
      />
      <span
        v-else-if="isSearchActive"
        class="td-search-count"
      >{{ results.length }}</span>
      <button
        v-if="query"
        class="td-search-clear"
        type="button"
        aria-label="Clear search"
        @click="clearQuery"
      >
        <X :size="12" />
      </button>
    </div>
    <div
      v-if="isSearchActive && results.length > 0"
      class="td-search-results"
    >
      <a
        v-for="(result, index) in results"
        :key="result.id"
        :href="result.id"
        class="td-search-result"
        :class="{
          'is-selected': index === selectedIndex,
        }"
        @click="onResultClick"
      >
        <span class="td-search-result-header">
          <span
            class="td-search-result-title"
            v-html="highlight(result.title, query)"
          />
          <span
            v-if="isCurrent(result.id)"
            class="td-search-result-badge"
          >current page</span>
        </span>
        <span class="td-search-result-breadcrumb">{{ result.titles.join(' / ') }}</span>
        <span
          v-if="result.excerpt"
          class="td-search-result-excerpt"
          v-html="highlight(result.excerpt, query)"
        />
      </a>
    </div>
    <div
      v-if="isSearchActive && results.length === 0"
      class="td-search-empty"
    >
      No results for "{{ query }}"
    </div>
  </div>
</template>

<style scoped>
.td-search {
  padding: 16px 16px 12px;
}

.td-search-input-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: 1px solid var(--color-td-neutral-border-subtle);
  border-radius: 6px;
  background: var(--color-td-neutral-bg);
  transition: border-color 0.15s;
}

.td-search-input-wrap:focus-within {
  border-color: var(--color-td-primary-solid);
}

.td-search-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-fg-muted);
}

.td-search-input {
  flex: 1;
  border: none;
  outline: none;
  background: none;
  font-size: var(--font-size-td-nav);
  color: var(--color-td-fg);
  font-family: inherit;
}

.td-search-input::placeholder {
  color: var(--color-td-neutral-fg-muted);
}

.td-search-count {
  flex-shrink: 0;
  font-size: var(--font-size-td-label);
  color: var(--color-td-neutral-fg-muted);
}

.td-search-spinner {
  flex-shrink: 0;
  color: var(--color-td-neutral-fg-muted);
  animation: td-spin 0.8s linear infinite;
}

@keyframes td-spin {
  to { transform: rotate(360deg); }
}

.td-search-clear {
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  cursor: pointer;
  color: var(--color-td-neutral-fg-muted);
  padding: 2px;
  transition: color 0.15s;
}

.td-search-clear:hover {
  color: var(--color-td-fg);
}

.td-search-results {
  margin-top: 8px;
  display: flex;
  flex-direction: column;
}

.td-search-result {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 10px;
  text-decoration: none;
  transition: background-color 0.1s;
}

.td-search-result:hover,
.td-search-result.is-selected {
  background: var(--color-td-neutral-bg-hover);
}

.td-search-result-header {
  display: flex;
  align-items: center;
  gap: 6px;
}

.td-search-result-badge {
  flex-shrink: 0;
  font-size: var(--font-size-td-label);
  color: var(--color-td-primary-solid);
  border: 1px solid var(--color-td-primary-solid);
  border-radius: 4px;
  padding: 1px 4px;
  line-height: 1;
}

.td-search-result-title {
  font-size: var(--font-size-td-nav);
  font-weight: var(--font-weight-td-semibold);
  color: var(--color-td-fg);
}

.td-search-result-title :deep(mark) {
  background: none;
  color: var(--color-td-primary-solid);
}

.td-search-result-breadcrumb {
  font-size: var(--font-size-td-label);
  color: var(--color-td-neutral-fg-muted);
}

.td-search-result-excerpt {
  font-size: var(--font-size-td-caption);
  color: var(--color-td-neutral-fg);
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
}

.td-search-result-excerpt :deep(mark) {
  background: none;
  color: var(--color-td-primary-solid);
  font-weight: var(--font-weight-td-semibold);
}

.td-search-empty {
  margin-top: 8px;
  padding: 8px 10px;
  font-size: var(--font-size-td-nav);
  color: var(--color-td-neutral-fg-muted);
}
</style>
