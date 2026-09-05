<!-- Adapted from VitePress VPLocalSearchBox.vue -->
<!-- https://github.com/vuejs/vitepress/blob/main/src/client/theme-default/components/VPLocalSearchBox.vue -->
<script setup lang="ts">
import {
  computed, ref, useTemplateRef, watch,
} from 'vue';
import {
  LoaderCircle, Search, X,
} from '@lucide/vue';
import {
  TdKeyName,
} from '../../utils/keys';
import TdKbdShortcut from '../TdKbdShortcut.vue';
import {
  useSiteSearch,
} from './useSiteSearch';
import type {
  SearchResult,
} from './useSiteSearch';
import {
  highlight,
} from './highlight';
import {
  useRoute, useSiteConfig,
} from '@/client/app';
import {
  stripAnchor, unslugify,
} from '@/shared';

// States and emits

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
const {
  withBase,
} = useSiteConfig();
const {
  results, searching, search, cancel,
} = useSiteSearch();
const selectedIndex = ref(-1);
const isSearchActive = computed(() => 0 < query.value.trim().length);

// Result grouping

interface SearchResultGroup {
  pageUrl: string;
  pageTitle: string;
  results: SearchResult[];
}

const groupedResults = computed((): SearchResultGroup[] => {
  const groups: SearchResultGroup[] = [];
  const groupMap = new Map<string, SearchResultGroup>();

  for (const result of results.value) {
    const pageUrl = stripAnchor(result.id);
    let group = groupMap.get(pageUrl);

    if (!group) {
      const filename = pageUrl.split('/').pop() || 'index';

      group = {
        pageUrl,
        pageTitle: unslugify(filename),
        results: [],
      };
      groupMap.set(pageUrl, group);
      groups.push(group);
    }
    group.results.push(result);
  }

  return groups;
});

// Query watcher

watch(isSearchActive, (value) => {
  active.value = value;
});

watch(query, (value, _old, onCleanup) => {
  onCleanup(() => cancel());
  selectedIndex.value = -1;
  search(value);
});

function clearQuery () {
  query.value = '';
}

// Keyboard navigation

const flatResults = computed(() => groupedResults.value.flatMap((group) => group.results));

const flatIndexMap = computed(() => {
  const map = new Map<string, number>();

  for (let index = 0; index < flatResults.value.length; index++) {
    map.set(flatResults.value[index].id, index);
  }

  return map;
});

function onKeydown (event: KeyboardEvent) {
  const count = flatIndexMap.value.size;

  if (count === 0) return;

  if (event.key === 'ArrowDown') {
    event.preventDefault();
    selectedIndex.value = (selectedIndex.value + 1) % count;
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    selectedIndex.value = (selectedIndex.value - 1 + count) % count;
  } else if (event.key === 'Enter' && 0 <= selectedIndex.value) {
    event.preventDefault();
    const selected = flatResults.value[selectedIndex.value];

    if (!selected) return;
    const link = document.querySelector<HTMLAnchorElement>(`.td-search-result[href="${CSS.escape(withBase(selected.id))}"]`);

    link?.click();
  }
}

function onResultClick () {
  emit('select');
}

const searchInput = useTemplateRef<HTMLInputElement>('searchInput');

defineExpose({
  focus () {
    searchInput.value?.focus();
  },
  blur () {
    searchInput.value?.blur();
  },
  get isVisible () {
    return searchInput.value?.offsetParent !== null;
  },
  get isFocused () {
    return document.activeElement === searchInput.value;
  },
});
</script>

<template>
  <!-- Input -->
  <div class="td-search">
    <div class="td-search-input-wrap">
      <Search
        :size="14"
        class="td-search-icon"
      />
      <input
        ref="searchInput"
        v-model="query"
        class="td-search-input"
        type="text"
        size="1"
        placeholder="Search..."
        @keydown="onKeydown"
      >
      <TdKbdShortcut
        v-if="!isSearchActive && !searching"
        :keys="[
          TdKeyName.Meta,
          TdKeyName.k,
        ]"
        class="td-search-kbd"
      />
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

    <!-- Results -->
    <div
      v-if="isSearchActive && results.length > 0"
      class="td-search-results"
    >
      <div
        v-for="group in groupedResults"
        :key="group.pageUrl"
        class="td-search-group"
      >
        <div class="td-search-group-label">
          {{ group.pageTitle }}
        </div>
        <a
          v-for="result in group.results"
          :key="result.id"
          :href="withBase(result.id)"
          class="td-search-result"
          :class="{
            'is-selected': flatIndexMap.get(result.id) === selectedIndex,
            'is-current': withBase(result.id) === route.path,
          }"
          @click="onResultClick"
        >
          <span
            class="td-search-result-title"
            v-html="highlight(result.title, query)"
          />
          <span
            v-if="result.excerpt"
            class="td-search-result-excerpt"
            v-html="highlight(result.excerpt, query)"
          />
        </a>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="isSearchActive && results.length === 0"
      class="td-search-empty"
    >
      No results for "{{ query }}"
    </div>
  </div>
</template>

<style scoped>
/* Input */

.td-search {
  padding: 16px 16px 12px;
}

.td-search-input-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: 1px solid color-mix(in srgb, var(--color-td-primary-solid) 12%, transparent);
  border-radius: 6px;
  background: color-mix(in srgb, var(--color-td-primary-solid) 5%, transparent);
  transition: border-color 0.15s;
}

.td-search-input-wrap:focus-within {
  border-color: color-mix(in srgb, var(--color-td-primary-solid) 30%, transparent);
}

.td-search-input-wrap:focus-within .td-search-kbd {
  display: none;
}

.td-search-kbd {
  flex-shrink: 0;
  opacity: 0.5;
}

.td-search-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-fg-muted);
}

.td-search-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: none;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-fg);
  font-family: inherit;
}

.td-search-input::placeholder {
  color: var(--color-td-neutral-fg-muted);
}

.td-search-count {
  flex-shrink: 0;
  font-size: var(--font-size-td-2xs);
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

/* Results */

.td-search-results {
  margin-top: 8px;
  display: flex;
  flex-direction: column;
}

.td-search-group-label {
  padding: 6px 20px;
  font-size: var(--font-size-td-2xs);
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-neutral-fg-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-search-result {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 5px 12px 5px 20px;
  border-left: 2px solid var(--color-td-neutral-border-subtle);
  text-decoration: none;
  transition: background-color 0.1s;
}

.td-search-result:hover,
.td-search-result.is-selected {
  background: var(--color-td-neutral-bg-hover);
}

.td-search-result.is-current {
  border-left-color: var(--color-td-primary-solid);
  background: color-mix(in srgb, var(--color-td-primary-solid) 6%, transparent);
}

.td-search-result.is-current .td-search-result-title {
  color: var(--color-td-primary-solid);
  font-weight: 600;
}

.td-search-result-title {
  font-size: var(--font-size-td-sm);
  color: var(--color-td-fg);
}

.td-search-result-title :deep(mark) {
  background: none;
  color: var(--color-td-primary-solid);
}

.td-search-result-excerpt {
  font-size: var(--font-size-td-xs);
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
  font-weight: 600;
}

/* Empty state */

.td-search-empty {
  margin-top: 8px;
  padding: 8px 10px;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-neutral-fg-muted);
}
</style>
