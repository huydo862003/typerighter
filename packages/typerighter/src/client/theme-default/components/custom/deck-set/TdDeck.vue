<script setup lang="ts">
import {
  computed, onMounted, ref,
} from 'vue';
import {
  getCardStates,
  isDue,
  reviewCard,
} from '@/client/theme-default/composables/useFlashcardStore';
import type {
  CardState,
} from '@/client/theme-default/composables/useFlashcardStore';
import {
  useSiteConfig,
} from '@/client/app';
import type {
  ContentSummary,
} from '@/shared';

const {
  name, cards,
} = defineProps<{
  /** Deck display name */
  name: string;
  /** Flashcard content items in this deck */
  cards: ContentSummary[];
}>();

const {
  withBase,
} = useSiteConfig();

const mode = ref<'idle' | 'review'>('idle');
const reviewQueue = ref<string[]>([]);
const reviewIndex = ref(0);
const showAnswer = ref(false);
const cardStates = ref<Map<string, CardState>>(new Map());

onMounted(async () => {
  const ids = cards.map((card) => card.filepath);

  cardStates.value = await getCardStates(ids);
});

const dueCards = computed(() =>
  cards.filter((card) => isDue(cardStates.value.get(card.filepath))));

const currentCard = computed(() => {
  const filepath = reviewQueue.value[reviewIndex.value];

  return cards.find((card) => card.filepath === filepath);
});

const progress = computed(() =>
  0 < reviewQueue.value.length
    ? `${reviewIndex.value + 1} / ${reviewQueue.value.length}`
    : '');

async function answer (quality: number): Promise<void> {
  const filepath = reviewQueue.value[reviewIndex.value];

  if (filepath) {
    const next = await reviewCard(filepath, quality);

    cardStates.value.set(filepath, next);
  }

  showAnswer.value = false;
  if (reviewIndex.value < reviewQueue.value.length - 1) {
    reviewIndex.value++;
  } else {
    mode.value = 'idle';
  }
}

function answerAgain (): void {
  answer(0);
}

function answerEasy (): void {
  answer(3);
}

function answerGood (): void {
  answer(2);
}

function answerHard (): void {
  answer(1);
}

function cardUrl (card: ContentSummary): string {
  const path = card.filepath.replace(/\.td$/, '');

  return withBase('/' + path);
}

function endReview (): void {
  mode.value = 'idle';
}

function reveal (): void {
  showAnswer.value = true;
}

function startReview (): void {
  reviewQueue.value = dueCards.value.map((card) => card.filepath);
  reviewIndex.value = 0;
  showAnswer.value = false;
  mode.value = 'review';
}
</script>

<template>
  <div class="td-deck">
    <div class="td-deck-header">
      <span class="td-deck-name">{{ name }}</span>
      <span class="td-deck-count">
        <span
          v-if="dueCards.length > 0"
          class="td-deck-due"
        >{{ dueCards.length }} due</span>
        <span
          v-else
          class="td-deck-done"
        >caught up</span>
        <span class="td-deck-total">/ {{ cards.length }}</span>
      </span>
      <button
        v-if="mode === 'idle' && dueCards.length > 0"
        type="button"
        class="td-deck-btn td-deck-btn-primary"
        @click="startReview"
      >
        Review
      </button>
      <button
        v-if="mode === 'review'"
        type="button"
        class="td-deck-btn td-deck-btn-ghost"
        @click="endReview"
      >
        End
      </button>
    </div>

    <div
      v-if="mode === 'review' && currentCard"
      class="td-deck-review"
    >
      <div class="td-deck-progress">
        {{ progress }}
      </div>
      <div class="td-deck-card">
        <a
          :href="cardUrl(currentCard)"
          class="td-deck-card-label"
        >
          {{ currentCard.label || currentCard.filepath }}
        </a>
        <div
          v-if="showAnswer && currentCard.excerpt"
          class="td-deck-card-excerpt"
        >
          {{ currentCard.excerpt }}
        </div>
      </div>
      <div class="td-deck-controls">
        <button
          v-if="!showAnswer"
          type="button"
          class="td-deck-btn td-deck-btn-primary"
          @click="reveal"
        >
          Show answer
        </button>
        <template v-else>
          <button
            type="button"
            class="td-deck-btn td-deck-btn-again"
            @click="answerAgain"
          >
            Again
          </button>
          <button
            type="button"
            class="td-deck-btn td-deck-btn-hard"
            @click="answerHard"
          >
            Hard
          </button>
          <button
            type="button"
            class="td-deck-btn td-deck-btn-good"
            @click="answerGood"
          >
            Good
          </button>
          <button
            type="button"
            class="td-deck-btn td-deck-btn-easy"
            @click="answerEasy"
          >
            Easy
          </button>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.td-deck {
  border: 1px solid var(--color-td-line-soft);
  border-radius: var(--radius-td-lg);
  overflow: hidden;
}

.td-deck-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: var(--color-td-surface);
}

.td-deck-name {
  font-weight: 600;
  font-size: var(--font-size-td-sm);
}

.td-deck-count {
  margin-left: auto;
  font-size: var(--font-size-td-xs);
  color: var(--color-td-ink-4);
  display: flex;
  gap: 4px;
}

.td-deck-due {
  color: var(--color-td-accent);
  font-weight: 600;
}

.td-deck-done {
  color: var(--color-td-ink-4);
}

.td-deck-total {
  color: var(--color-td-ink-4);
}

.td-deck-review {
  padding: 16px;
  border-top: 1px solid var(--color-td-line-soft);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.td-deck-progress {
  font-size: var(--font-size-td-xs);
  color: var(--color-td-ink-4);
  font-variant-numeric: tabular-nums;
}

.td-deck-card {
  min-height: 6rem;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 8px;
}

.td-deck-card-label {
  font-size: var(--font-size-td-base);
  font-weight: 500;
  color: var(--color-td-ink);
  text-decoration: none;
}

.td-deck-card-label:hover {
  color: var(--color-td-accent);
}

.td-deck-card-excerpt {
  font-size: var(--font-size-td-sm);
  color: var(--color-td-ink-3);
  line-height: var(--leading-td-relaxed);
}

.td-deck-controls {
  display: flex;
  gap: 8px;
}

.td-deck-btn {
  padding: 6px 14px;
  border: none;
  border-radius: var(--radius-td-md);
  font-size: var(--font-size-td-xs);
  font-weight: 500;
  cursor: pointer;
  transition:
    background-color var(--duration-td-fast) var(--ease-td-out-quart),
    opacity var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-deck-btn-primary {
  background: var(--color-td-accent);
  color: white;
}

.td-deck-btn-primary:hover {
  opacity: 0.9;
}

.td-deck-btn-ghost {
  background: transparent;
  color: var(--color-td-ink-3);
  border: 1px solid var(--color-td-line-soft);
}

.td-deck-btn-ghost:hover {
  background: var(--color-td-hover);
}

.td-deck-btn-again {
  background: var(--color-td-danger-bg);
  color: var(--color-td-danger);
}

.td-deck-btn-hard {
  background: var(--color-td-warning-bg);
  color: var(--color-td-warning-text);
}

.td-deck-btn-good {
  background: var(--color-td-info-bg);
  color: var(--color-td-info);
}

.td-deck-btn-easy {
  background: var(--color-td-tip-bg);
  color: var(--color-td-tip);
}

.td-deck-btn-again:hover,
.td-deck-btn-hard:hover,
.td-deck-btn-good:hover,
.td-deck-btn-easy:hover {
  opacity: 0.85;
}
</style>
