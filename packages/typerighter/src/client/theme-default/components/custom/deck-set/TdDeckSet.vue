<script setup lang="ts">
import {
  computed, onMounted,
} from 'vue';
import TdDeck from './TdDeck.vue';
import {
  pruneStaleCards,
} from '@/client/theme-default/composables/useFlashcardStore';
import {
  useSiteData, useRoute,
} from '@/client/app';
import {
  ensureTrailingSlash, flattenContentTree, getParentUrl, stripLeadingSlash,
} from '@/shared';
import type {
  ContentSummary,
} from '@/shared';

const {
  schema = undefined, folder = undefined,
} = defineProps<{
  /** Filter by schema name (e.g. "PltFlashcard"). If omitted, matches any schema containing "Flashcard" */
  schema?: string;
  /** Folder to scan for flashcards. Same format as fref() paths: "/abs", "./rel", "../rel". Defaults to current directory */
  folder?: string;
}>();

const route = useRoute();
const siteData = useSiteData();

function isFlashcardSchema (schemaName: string | undefined): boolean {
  if (!schemaName) return false;
  if (schema) return schemaName === schema;

  return schemaName.includes('Flashcard');
}

function resolveFolder (): string {
  const pageDirectory = stripLeadingSlash(getParentUrl(route.path));

  if (!folder) return pageDirectory;
  if (folder.startsWith('/')) return folder.slice(1);

  const parts = pageDirectory.split('/').filter(Boolean);

  for (const segment of folder.split('/')) {
    if (segment === '..') parts.pop();
    else if (segment !== '.') parts.push(segment);
  }

  return parts.join('/');
}

const decks = computed(() => {
  const allItems = flattenContentTree(siteData.value.contentTree.entries);
  const prefix = ensureTrailingSlash(resolveFolder());

  const flashcards = allItems.filter(
    (item) => isFlashcardSchema(item.schema) && item.filepath.startsWith(prefix),
  );

  const groups = new Map<string, ContentSummary[]>();

  for (const card of flashcards) {
    const deckName = (card.header?.deck as string) ?? '';
    const list = groups.get(deckName) ?? [];

    list.push(card);
    groups.set(deckName, list);
  }

  // Named decks sorted alphabetically, unnamed deck last
  return [...groups.entries()].sort(([nameA], [nameB]) => {
    if (!nameA) return 1;
    if (!nameB) return -1;

    return nameA.localeCompare(nameB);
  });
});

onMounted(async () => {
  const allFlashcards = flattenContentTree(siteData.value.contentTree.entries)
    .filter((item) => isFlashcardSchema(item.schema));

  await pruneStaleCards(new Set(allFlashcards.map((item) => item.filepath)));
});
</script>

<template>
  <div class="td-deck-set">
    <TdDeck
      v-for="[
        deckName,
        cards,
      ] in decks"
      :key="deckName"
      :name="deckName"
      :cards="cards"
    />
    <p
      v-if="decks.length === 0"
      class="td-deck-set-empty"
    >
      No flashcards found in this directory.
    </p>
  </div>
</template>

<style scoped>
.td-deck-set {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin: 16px 0;
}

.td-deck-set-empty {
  color: var(--color-td-ink-4);
  font-size: var(--font-size-td-sm);
}
</style>
