<script setup lang="ts">
import {
  computed, onMounted,
} from 'vue';
import {
  resources,
} from '@typedown/resources?type=flashcard';
import TdDeck from './TdDeck.vue';
import {
  useFlashcardStore,
} from '@/client/theme-default/composables/flashcard';
import {
  useRoute,
} from '@/client/app';
import {
  ensureTrailingSlash, getParentUrl, stripLeadingSlash,
} from '@/shared';
import type {
  Resource,
} from '@/shared';

const {
  name, folder = undefined,
} = defineProps<{
  /** Display name shown above the deck set */
  name: string;
  /** Folder to scan for flashcards. Same format as fref() paths: "/abs", "./rel", "../rel". Defaults to current directory */
  folder?: string;
}>();

const route = useRoute();
const {
  pruneStaleCards,
} = useFlashcardStore();

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
  const prefix = ensureTrailingSlash(resolveFolder());

  const flashcards = resources.value.filter((resource) =>
    resource.filepath.startsWith(prefix));

  const groups = new Map<string, Resource[]>();

  for (const card of flashcards) {
    const deckName = card.fields.deck ?? '';
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
  await pruneStaleCards(
    new Set(resources.value.map((resource) => resource.filepath)),
  );
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
      :name="deckName || name"
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
