<script setup lang="ts">
import {
  computed, onMounted, ref,
} from 'vue';
import {
  resources,
} from '@typedown/resources?type=flashcard';
import TdStudy from './TdStudy.vue';
import type {
  StudyMode,
  CardState,
} from '@/client/theme-default/composables/flashcard';
import {
  useFlashcardStore,
} from '@/client/theme-default/composables/flashcard';
import {
  useRoute,
} from '@/client/app';
import {
  ensureTrailingSlash, getParentUrl, parseNumericPrefix, path, stripLeadingSlash, unslugify,
} from '@/shared';
import type {
  Resource,
} from '@/shared';

const {
  name, folder = undefined,
} = defineProps<{
  /** Display name shown above the deck set */
  name: string;
  /** Folder to scan for flashcards. Same format as fref() paths */
  folder?: string;
}>();

const route = useRoute();
const {
  pruneStaleCards, getCardStates, isCardDue,
} = useFlashcardStore();

const cardStates = ref<Map<string, CardState>>(new Map());

const studying = ref(false);
const studyDeckId = ref<string | undefined>();
const studyMode = ref<StudyMode>('review');

interface Deck {
  id: string;
  name: string;
  cards: Resource[];
  dueCount: number;
}

// Parse deck field: string -> single deck, JSON array -> multiple decks
function parseDeckField (raw: string | undefined): string[] {
  if (!raw) return [''];

  if (raw.startsWith('[')) {
    try {
      const parsed: unknown = JSON.parse(raw);

      if (Array.isArray(parsed)) {
        return parsed.filter((item): item is string => typeof item === 'string');
      }
    } catch {
      // Not valid JSON, treat as plain string
    }
  }

  return [raw];
}

function resolveFolder (): string {
  const pageDirectory = stripLeadingSlash(getParentUrl(route.path));

  return folder
    ? path.resolveFrefPath(folder, pageDirectory)
    : pageDirectory;
}

const decks = computed((): Deck[] => {
  const prefix = ensureTrailingSlash(resolveFolder());

  const flashcards = resources.value.filter((resource) =>
    resource.filepath.startsWith(prefix));

  const groups = new Map<string, Resource[]>();

  for (const card of flashcards) {
    const deckNames = parseDeckField(card.fields.deck);

    for (const deckName of deckNames) {
      const list = groups.get(deckName) ?? [];

      list.push(card);
      groups.set(deckName, list);
    }
  }

  return [...groups.entries()]
    .sort(([nameA], [nameB]) => {
      if (!nameA) return 1;
      if (!nameB) return -1;

      const prefixA = parseNumericPrefix(nameA);
      const prefixB = parseNumericPrefix(nameB);

      if (prefixA.order !== prefixB.order) return prefixA.order - prefixB.order;

      return prefixA.rest.localeCompare(prefixB.rest);
    })
    .map(([
      rawName,
      cards,
    ]) => ({
      id: rawName,
      name: rawName ? unslugify(rawName) : name,
      cards,
      dueCount: cards.filter((card) => isCardDue(cardStates.value.get(card.filepath))).length,
    }));
});

const totalDue = computed(() =>
  decks.value.reduce((sum, deck) => sum + deck.dueCount, 0));

const allCards = computed(() =>
  decks.value.flatMap((deck) => deck.cards));

// Study targets
const studyCards = computed((): Resource[] => {
  if (studyDeckId.value === undefined) return allCards.value;

  const deck = decks.value.find((deck) => deck.id === studyDeckId.value);

  return deck?.cards ?? [];
});

const studyDeckName = computed((): string => {
  if (studyDeckId.value === undefined) return 'All decks';

  const deck = decks.value.find((deck) => deck.id === studyDeckId.value);

  return deck?.name ?? name;
});

async function closeStudy () {
  studying.value = false;
  // Refresh card states after study session
  cardStates.value = await getCardStates(allCards.value.map((card) => card.filepath));
}

function handleDeckClick (event: Event) {
  const deckId = (event.currentTarget as HTMLElement).dataset.deck ?? '';

  studyDeck(deckId);
}

async function loadStates () {
  cardStates.value = await getCardStates(allCards.value.map((card) => card.filepath));
}

function previewAll () {
  startStudy(undefined, 'preview');
}

function startStudy (deckId: string | undefined, selectedMode: StudyMode) {
  studyDeckId.value = deckId;
  studyMode.value = selectedMode;
  studying.value = true;
}

function studyAll () {
  startStudy(undefined, 'review');
}

function studyDeck (deckId: string) {
  startStudy(deckId, 'review');
}

onMounted(async () => {
  await pruneStaleCards(
    new Set(resources.value.map((resource) => resource.filepath)),
  );
  await loadStates();
});
</script>

<template>
  <div class="td-decks">
    <!-- Table -->
    <div
      class="td-decks-table"
      role="table"
      aria-label="Decks"
    >
      <div
        class="td-decks-row td-decks-head"
        role="row"
      >
        <span role="columnheader">Deck</span>
        <span
          role="columnheader"
          class="td-decks-num-header"
        >Due</span>
        <span
          role="columnheader"
          class="td-decks-num-header"
        >Cards</span>
      </div>
      <button
        v-for="deck in decks"
        :key="deck.id"
        type="button"
        class="td-decks-row td-decks-link"
        role="row"
        :data-deck="deck.id"
        @click="handleDeckClick"
      >
        <span role="cell">{{ deck.name }}</span>
        <span
          role="cell"
          class="td-decks-num"
          :class="deck.dueCount > 0 ? 'td-decks-num-due' : 'td-decks-num-zero'"
        >{{ deck.dueCount }}</span>
        <span
          role="cell"
          class="td-decks-num"
        >{{ deck.cards.length }}</span>
      </button>
    </div>

    <!-- Footer -->
    <div class="td-decks-foot">
      <div class="td-decks-foot-actions">
        <button
          type="button"
          class="td-decks-study-btn"
          :aria-disabled="totalDue === 0"
          :disabled="totalDue === 0"
          @click="studyAll"
        >
          Study all due ({{ totalDue }})
        </button>
        <button
          type="button"
          class="td-decks-preview-btn"
          @click="previewAll"
        >
          Preview
        </button>
      </div>
    </div>

    <p
      v-if="decks.length === 0"
      class="td-decks-empty"
    >
      No flashcards found in this directory.
    </p>

    <!-- Study overlay -->
    <TdStudy
      v-if="studying"
      :deck-name="studyDeckName"
      :cards="studyCards"
      :mode="studyMode"
      @close="closeStudy"
    />
  </div>
</template>

<style scoped>
.td-decks {
  display: flex;
  flex-direction: column;
  gap: 20px;
  margin: 16px 0;
}

.td-decks-table {
  display: flex;
  flex-direction: column;
}

.td-decks-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 80px 80px;
  gap: 16px;
  align-items: center;
  padding: 0 12px;
}

.td-decks-head {
  padding-bottom: 10px;
  border-bottom: 1px solid var(--color-td-line);
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-td-ink-4);
}

.td-decks-link {
  min-height: 48px;
  border: none;
  border-bottom: 1px solid var(--color-td-line-soft, var(--color-td-line));
  background: none;
  font: inherit;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-ink);
  text-align: left;
  cursor: pointer;
}

.td-decks-link:hover {
  background: var(--color-td-hover);
}

.td-decks-num-header {
  text-align: right;
}

.td-decks-num {
  text-align: right;
  font-size: var(--font-size-td-sm);
  font-variant-numeric: tabular-nums;
  color: var(--color-td-ink-4);
}

.td-decks-num-due {
  font-weight: 500;
  color: var(--color-td-tip);
}

.td-decks-num-zero {
  font-weight: 400;
  color: var(--color-td-ink-4);
}

.td-decks-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 12px;
}

.td-decks-study-btn {
  display: inline-flex;
  align-items: center;
  min-height: 40px;
  padding: 0 18px;
  border-radius: var(--radius-td-md);
  border: none;
  background: var(--color-td-accent);
  color: #fff;
  font: inherit;
  font-size: var(--font-size-td-sm);
  font-weight: 500;
  cursor: pointer;
}

.td-decks-study-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.td-decks-study-btn:disabled {
  background: var(--color-td-line);
  color: var(--color-td-ink-4);
  cursor: default;
}

.td-decks-foot-actions {
  display: flex;
  gap: 8px;
}

.td-decks-preview-btn {
  display: inline-flex;
  align-items: center;
  min-height: 40px;
  padding: 0 18px;
  border-radius: var(--radius-td-md);
  border: 1px solid var(--color-td-line);
  background: var(--color-td-surface);
  color: var(--color-td-ink);
  font: inherit;
  font-size: var(--font-size-td-sm);
  font-weight: 500;
  cursor: pointer;
}

.td-decks-preview-btn:hover {
  background: var(--color-td-hover);
}

.td-decks-empty {
  color: var(--color-td-ink-4);
  font-size: var(--font-size-td-sm);
  padding: 0 12px;
}
</style>
