<script setup lang="ts">
import {
  computed, ref,
} from 'vue';
import MiniSearch from 'minisearch';
import {
  Search,
} from '@lucide/vue';
import TdGlossaryRow from './TdGlossaryRow.vue';
import {
  useSiteData, useRoute, useSiteConfig,
} from '@/client/app';
import {
  getParentUrl,
  type DirectoryEntry,
} from '@/shared';

const {
  mode = 'informative',
} = defineProps<{
  /** Informative (rows with excerpt/tags/time) or dense (multi-column links) */
  mode?: 'informative' | 'dense';
}>();

const route = useRoute();
const siteData = useSiteData();
const {
  withBase,
} = useSiteConfig();

interface AlphaGroup {
  letter: string;
  items: DirectoryEntry[];
}

const filterQuery = ref('');

const allItems = computed((): DirectoryEntry[] => {
  const path = getParentUrl(route.path);
  const directory = siteData.value.directoryListings[path];

  return directory?.items ?? [];
});

const totalCount = computed(() => allItems.value.length);

const itemsByUrl = computed(() => {
  const map = new Map<string, DirectoryEntry>();

  for (const item of allItems.value) {
    map.set(item.url, item);
  }

  return map;
});

const searchIndex = computed(() => {
  const index = new MiniSearch<DirectoryEntry & {
    id: number;
  }>({
    fields: [
      'name',
      'description',
    ],
    storeFields: [
      'name',
      'url',
      'description',
      'tags',
      'mtime',
      'schema',
    ],
    searchOptions: {
      boost: {
        name: 2,
      },
      fuzzy: 0.2,
      prefix: true,
    },
  });

  index.addAll(allItems.value.map((item, index) => ({
    ...item,
    id: index,
  })));

  return index;
});

const isFiltering = computed(() => 0 < filterQuery.value.trim().length);

const filteredItems = computed((): DirectoryEntry[] => {
  const query = filterQuery.value.trim();

  if (query.length === 0) return allItems.value;

  const results = searchIndex.value.search(query);

  if (results.length === 0) return [];

  const lookup = itemsByUrl.value;

  return results.map((result) => lookup.get(result.url as string)).filter((item): item is DirectoryEntry => item !== undefined);
});

const groups = computed((): AlphaGroup[] => {
  if (isFiltering.value) return [];

  return groupAlphabetically(allItems.value);
});

function groupAlphabetically (items: DirectoryEntry[]): AlphaGroup[] {
  const grouped = new Map<string, DirectoryEntry[]>();

  for (const item of items) {
    const first = (item.name[0] ?? '').toUpperCase();
    const letter = /[A-Z]/.test(first) ? first : '#';

    const existing = grouped.get(letter);

    if (existing !== undefined) {
      existing.push(item);
    } else {
      grouped.set(letter, [item]);
    }
  }

  return Array.from(grouped.entries())
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([
      letter,
      entries,
    ]) => ({
      letter,
      items: entries,
    }));
}
</script>

<template>
  <div>
    <div class="td-glossary-filter">
      <Search
        :size="14"
        class="td-glossary-filter-icon"
      />
      <input
        v-model="filterQuery"
        type="text"
        class="td-glossary-filter-input"
        :placeholder="`Filter ${totalCount} entries`"
      >
    </div>

    <!-- Filtered results (flat, ranked by relevance) -->
    <template v-if="isFiltering">
      <div
        v-if="filteredItems.length === 0"
        class="td-glossary-empty"
      >
        No entries matching "{{ filterQuery.trim() }}"
      </div>

      <div
        v-else-if="mode === 'dense'"
        class="td-glossary-dense-group"
      >
        <a
          v-for="item in filteredItems"
          :key="item.url"
          :href="withBase(item.url)"
          class="td-glossary-dense-link"
        >{{ item.name }}</a>
      </div>

      <div v-else>
        <TdGlossaryRow
          v-for="item in filteredItems"
          :key="item.url"
          :item="item"
          :href="withBase(item.url)"
        />
      </div>
    </template>

    <!-- Unfiltered: alphabetical groups -->
    <template v-else>
      <div
        v-if="mode === 'dense'"
        class="td-glossary-columns"
      >
        <div
          v-for="group in groups"
          :key="group.letter"
          class="td-glossary-dense-group"
        >
          <div class="td-glossary-letter-row">
            <span class="td-glossary-letter">{{ group.letter }}</span>
            <span class="td-glossary-letter-line" />
          </div>
          <a
            v-for="item in group.items"
            :key="item.url"
            :href="withBase(item.url)"
            class="td-glossary-dense-link"
          >{{ item.name }}</a>
        </div>
      </div>

      <div v-else>
        <div
          v-for="group in groups"
          :key="group.letter"
        >
          <div class="td-glossary-letter-row">
            <span class="td-glossary-letter">{{ group.letter }}</span>
            <span class="td-glossary-letter-line" />
            <span class="td-glossary-letter-count">{{ group.items.length }}</span>
          </div>
          <TdGlossaryRow
            v-for="item in group.items"
            :key="item.url"
            :item="item"
            :href="withBase(item.url)"
            show-tags
          />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* Filter input */

.td-glossary-filter {
  display: flex;
  align-items: center;
  gap: 8px;
  border: 1px solid var(--color-td-neutral-border-subtle);
  border-radius: 7px;
  background: var(--color-td-neutral-bg);
  padding: 8px 12px;
  margin-bottom: 20px;
}

.td-glossary-filter-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-border-strong);
}

.td-glossary-filter-input {
  flex: 1;
  border: none;
  background: none;
  outline: none;
  font-size: var(--font-size-td-xs);
  color: var(--color-td-fg);
}

.td-glossary-filter-input::placeholder {
  color: var(--color-td-neutral-border-strong);
}

.td-glossary-empty {
  font-size: var(--font-size-td-sm);
  color: var(--color-td-neutral-fg-muted);
  font-style: italic;
  padding: 16px 4px;
}

/* Letter heading */

.td-glossary-letter-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 24px;
  padding-bottom: 4px;
}

.td-glossary-letter-row:first-child {
  margin-top: 0;
}

.td-glossary-letter {
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  font-weight: 500;
  letter-spacing: var(--tracking-td-wide);
  color: var(--color-td-primary-solid);
}

.td-glossary-letter-line {
  flex: 1;
  height: 1px;
  background: var(--color-td-neutral-border-subtle);
}

.td-glossary-letter-count {
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  color: var(--color-td-neutral-border-strong);
}

/* Dense mode */

.td-glossary-columns {
  column-count: 3;
  column-gap: 32px;
}

.td-glossary-dense-group {
  break-inside: avoid;
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-bottom: 18px;
}

.td-glossary-dense-link {
  font-size: var(--font-size-td-sm);
  padding: 4px 0;
  color: var(--color-td-primary-solid);
  text-decoration: none;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: block;
}

.td-glossary-dense-link:hover {
  color: var(--color-td-primary-solid-hover);
}

@media (width < 75rem) {
  .td-glossary-columns {
    column-count: 2;
  }
}

@media (width < 56.25rem) {
  .td-glossary-columns {
    column-count: 1;
  }
}
</style>
