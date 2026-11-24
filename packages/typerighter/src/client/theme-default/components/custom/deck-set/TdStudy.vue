<script setup lang="ts">
import {
  computed, onMounted, onUnmounted, ref, watch,
} from 'vue';
import {
  ChevronLeft,
} from '@lucide/vue';
import {
  useFlashcardStore,
} from '@/client/theme-default/composables/flashcard';
import type {
  CardState,
  StudyMode,
} from '@/client/theme-default/composables/flashcard';
import {
  previewIntervals,
} from '@/client/theme-default/composables/flashcard/sm2';
import type {
  Resource,
} from '@/shared';

const {
  deckName,
  cards,
  mode = 'review',
} = defineProps<{
  /** Deck display name */
  deckName: string;
  /** Cards to study */
  cards: Resource[];
  /** Study mode: review (graded + persisted), preview (browse only), cram (graded but not persisted) */
  mode?: StudyMode;
}>();

const emit = defineEmits<{
  close: [];
}>();

const {
  reviewCard, getCardStates, isCardDue,
} = useFlashcardStore();

// Grade index matches SM-2: 0=again 1=hard 2=good 3=easy
const GRADE_INDEX: Record<string, number> = {
  again: 0,
  hard: 1,
  good: 2,
  easy: 3,
};

// Queue entry tracks which pool the card came from
interface QueueEntry {
  filepath: string;
  source: 'new' | 'learning' | 'due';
}

const queue = ref<QueueEntry[]>([]);
const history = ref<QueueEntry[]>([]);
const revealed = ref(false);
const studied = ref(0);
const done = ref(false);
const cardStates = ref<Map<string, CardState>>(new Map());

const currentEntry = computed(() => queue.value[0]);
const currentCard = computed(() =>
  cards.find((card) => card.filepath === currentEntry.value?.filepath));

const countNew = computed(() =>
  queue.value.filter((entry) => entry.source === 'new').length);
const countLearning = computed(() =>
  queue.value.filter((entry) => entry.source === 'learning').length);
const countDue = computed(() =>
  queue.value.filter((entry) => entry.source === 'due').length);

// Interval preview labels for each grade button
function formatInterval (days: number): string {
  if (days < 1) return '<1m';
  if (days === 1) return '1d';

  return `${days}d`;
}

const intervals = computed(() => {
  const state = currentEntry.value
    ? cardStates.value.get(currentEntry.value.filepath)
    : undefined;
  const preview = previewIntervals(state);

  return {
    again: formatInterval(preview.again),
    hard: formatInterval(preview.hard),
    good: formatInterval(preview.good),
    easy: formatInterval(preview.easy),
  };
});

function close () {
  emit('close');
}

async function grade (level: 'again' | 'hard' | 'good' | 'easy') {
  if (!revealed.value) return;
  if (queue.value.length === 0) return;

  const entry = queue.value.shift()!;

  history.value.push(entry);

  // Review mode persists to scheduler; cram grades but doesn't persist; preview never grades
  if (mode === 'review') {
    const next = await reviewCard(entry.filepath, GRADE_INDEX[level]);

    cardStates.value.set(entry.filepath, next);
  }

  if (level === 'again') {
    queue.value.push({
      filepath: entry.filepath,
      source: 'learning',
    });
  }

  studied.value++;
  revealed.value = false;

  if (queue.value.length === 0) {
    done.value = true;
  }
}

function gradeAgain () {
  grade('again');
}

function gradeEasy () {
  grade('easy');
}

function gradeGood () {
  grade('good');
}

function gradeHard () {
  grade('hard');
}

function handleKeydown (event: KeyboardEvent) {
  // Block all other keyboard shortcuts while studying
  event.stopPropagation();

  if (event.metaKey || event.ctrlKey || event.altKey) return;

  if (event.key === 'Escape') {
    event.preventDefault();
    close();

    return;
  }

  if (done.value || queue.value.length === 0) return;

  if (!revealed.value && (event.key === ' ' || event.key === 'Enter')) {
    event.preventDefault();
    reveal();

    return;
  }

  if (revealed.value) {
    if (mode === 'preview') {
      if (event.key === ' ' || event.key === 'Enter' || event.key === 'ArrowRight') {
        event.preventDefault();
        nextCard();
      } else if (event.key === 'ArrowLeft') {
        event.preventDefault();
        previousCard();
      }

      return;
    }

    const map: Record<string, 'again' | 'hard' | 'good' | 'easy'> = {
      1: 'again',
      2: 'hard',
      3: 'good',
      4: 'easy',
      ' ': 'good',
    };
    const action = map[event.key];

    if (action) {
      event.preventDefault();
      grade(action);
    }
  }
}

// Preview/cram: advance without grading
function nextCard () {
  if (queue.value.length === 0) return;

  const entry = queue.value.shift()!;

  history.value.push(entry);
  studied.value++;
  revealed.value = false;

  if (queue.value.length === 0) {
    done.value = true;
  }
}

function previousCard () {
  if (history.value.length === 0) return;

  const entry = history.value.pop()!;

  queue.value.unshift(entry);
  studied.value = Math.max(0, studied.value - 1);
  revealed.value = false;
  done.value = false;
}

function reveal () {
  if (!revealed.value && 0 < queue.value.length) {
    revealed.value = true;
  }
}

async function start () {
  const states = await getCardStates(cards.map((card) => card.filepath));

  cardStates.value = states;

  let entries: QueueEntry[];

  if (mode === 'preview' || mode === 'cram') {
    // Preview and cram include all cards regardless of schedule
    entries = cards.map((card) => ({
      filepath: card.filepath,
      source: 'due' as const,
    }));
  } else {
    // Review mode: only due and new cards
    entries = [];

    for (const card of cards) {
      const state = states.get(card.filepath);

      if (!state) {
        entries.push({
          filepath: card.filepath,
          source: 'new',
        });
      } else if (isCardDue(state)) {
        entries.push({
          filepath: card.filepath,
          source: 'due',
        });
      }
    }

    // If nothing due, study all cards
    if (entries.length === 0) {
      entries = cards.map((card) => ({
        filepath: card.filepath,
        source: 'due' as const,
      }));
    }
  }

  queue.value = entries;
  history.value = [];
  revealed.value = false;
  studied.value = 0;
  done.value = false;
}

onMounted(() => {
  start();
  document.addEventListener('keydown', handleKeydown, true);
  document.body.style.overflow = 'hidden';
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown, true);
  document.body.style.overflow = '';
});

watch(() => cards, start);
</script>

<template>
  <Teleport to="body">
    <div class="td-study">
      <!-- Top bar -->
      <header class="td-study-top">
        <button
          type="button"
          class="td-study-back"
          @click="close"
        >
          <ChevronLeft
            :size="16"
            aria-hidden="true"
          />
          Back to note
          <kbd>Esc</kbd>
        </button>
        <span class="td-study-where">Flashcards · {{ deckName }}</span>
        <span
          class="td-study-spacer"
          aria-hidden="true"
        />
      </header>

      <!-- Main: card or done -->
      <main class="td-study-main">
        <!-- Card -->
        <div
          v-if="!done && currentCard"
          class="td-study-card"
        >
          <div
            class="td-study-question"
            v-html="currentCard.fields.question || currentCard.filepath"
          />
          <div
            v-if="revealed"
            class="td-study-answer-area"
          >
            <hr class="td-study-divider">
            <div
              class="td-study-answer"
              v-html="currentCard.fields.answer"
            />
          </div>
        </div>

        <!-- Done -->
        <div
          v-if="done"
          class="td-study-done"
        >
          <h1 class="td-study-done-title">
            All caught up
          </h1>
          <p class="td-study-done-text">
            You've finished this deck for now. Studied {{ studied }} card{{ studied === 1 ? '' : 's' }}.
          </p>
          <button
            type="button"
            class="td-study-done-btn"
            @click="close"
          >
            Back to note
          </button>
        </div>
      </main>

      <!-- Bottom: counter + actions -->
      <footer
        v-if="!done"
        class="td-study-bottom"
      >
        <!-- Counter (review and cram only) -->
        <p
          v-if="mode !== 'preview'"
          class="td-study-counts"
          aria-label="New, learning and due counts"
        >
          <span
            class="td-study-count-new"
            :class="{
              'is-current': currentEntry?.source === 'new',
            }"
          >{{ countNew }}</span>
          +
          <span
            class="td-study-count-learning"
            :class="{
              'is-current': currentEntry?.source === 'learning',
            }"
          >{{ countLearning }}</span>
          +
          <span
            class="td-study-count-due"
            :class="{
              'is-current': currentEntry?.source === 'due',
            }"
          >{{ countDue }}</span>
        </p>

        <!-- Preview: position counter -->
        <p
          v-if="mode === 'preview'"
          class="td-study-counts"
        >
          {{ history.length + 1 }} / {{ history.length + queue.length }}
        </p>

        <!-- Show answer (all modes) -->
        <button
          v-if="!revealed"
          type="button"
          class="td-study-show-btn"
          @click="reveal"
        >
          Show answer
          <kbd>Space</kbd>
        </button>

        <!-- Preview: Next button -->
        <template v-else-if="mode === 'preview'">
          <div class="td-study-preview-nav">
            <button
              type="button"
              class="td-study-show-btn"
              :disabled="history.length === 0"
              @click="previousCard"
            >
              <kbd>&#x2190;</kbd>
              Previous
            </button>
            <button
              type="button"
              class="td-study-show-btn"
              @click="nextCard"
            >
              Next
              <kbd>&#x2192;</kbd>
            </button>
          </div>
        </template>

        <!-- Grade buttons (review and cram) -->
        <div
          v-else
          class="td-study-eases"
          role="group"
          aria-label="How well did you remember?"
        >
          <button
            type="button"
            class="td-study-ease td-study-ease-again"
            @click="gradeAgain"
          >
            <span class="td-study-ease-ivl">{{ intervals.again }}</span>
            <span class="td-study-ease-label">Again</span>
          </button>
          <button
            type="button"
            class="td-study-ease td-study-ease-hard"
            @click="gradeHard"
          >
            <span class="td-study-ease-ivl">{{ intervals.hard }}</span>
            <span class="td-study-ease-label">Hard</span>
          </button>
          <button
            type="button"
            class="td-study-ease td-study-ease-good"
            @click="gradeGood"
          >
            <span class="td-study-ease-ivl">{{ intervals.good }}</span>
            <span class="td-study-ease-label">Good</span>
          </button>
          <button
            type="button"
            class="td-study-ease td-study-ease-easy"
            @click="gradeEasy"
          >
            <span class="td-study-ease-ivl">{{ intervals.easy }}</span>
            <span class="td-study-ease-label">Easy</span>
          </button>
        </div>
      </footer>
    </div>
  </Teleport>
</template>

<style scoped>
.td-study {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  background: var(--color-td-bg);
}

/* Top bar */
.td-study-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 20px 32px;
  flex-shrink: 0;
}

.td-study-back {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 44px;
  padding: 0;
  border: none;
  background: none;
  font: inherit;
  font-size: 14px;
  color: var(--color-td-ink-4);
  cursor: pointer;
}

.td-study-back:hover {
  color: var(--color-td-ink);
}

.td-study-back kbd {
  font-family: inherit;
  font-size: 11px;
  color: var(--color-td-ink-4);
  border: 1px solid var(--color-td-line);
  border-radius: 4px;
  padding: 1px 6px;
}

.td-study-where {
  font-size: 13px;
  color: var(--color-td-ink-4);
}

.td-study-spacer {
  width: 130px;
}

/* Main */
.td-study-main {
  flex: 1;
  display: flex;
  justify-content: center;
  padding: 72px 24px 48px;
  overflow-y: auto;
}

/* Card */
.td-study-card {
  width: 100%;
  max-width: 640px;
  text-align: center;
}

.td-study-question {
  font-family: var(--font-serif, Georgia, serif);
  font-weight: 400;
  font-size: 34px;
  line-height: 1.3;
  letter-spacing: -0.01em;
}

.td-study-question :deep(p) {
  margin: 0;
  font-size: inherit;
}

.td-study-question :deep(code) {
  font-family: var(--font-mono);
  font-size: 0.82em;
  background: var(--color-td-hover);
  padding: 2px 8px;
  border-radius: 6px;
}

.td-study-divider {
  border: 0;
  border-top: 1px solid var(--color-td-line);
  margin: 40px 0;
}

.td-study-answer {
  font-family: var(--font-serif, Georgia, serif);
  font-size: 22px;
  line-height: 1.6;
  color: var(--color-td-ink-3);
  text-align: left;
}

.td-study-answer :deep(p) {
  margin: 8px 0;
  font-size: inherit;
}

.td-study-answer :deep(p:first-child) {
  margin-top: 0;
}

.td-study-answer :deep(strong) {
  font-weight: 600;
}

.td-study-answer :deep(ul),
.td-study-answer :deep(ol) {
  margin: 8px 0;
  padding-left: 24px;
  text-align: left;
}

.td-study-answer :deep(li) {
  margin: 4px 0;
}

.td-study-answer :deep(code) {
  font-family: var(--font-mono);
  font-size: 0.82em;
  background: var(--color-td-hover);
  padding: 2px 8px;
  border-radius: 6px;
}

.td-study-answer :deep(blockquote) {
  margin: 12px 0;
  padding-left: 16px;
  border-left: 3px solid var(--color-td-line);
  color: var(--color-td-ink-4);
  font-style: italic;
}

/* Answer reveal animation */
.td-study-answer-area {
  animation: td-study-fade 160ms ease-out;
}

@keyframes td-study-fade {
  from { opacity: 0; }
  to { opacity: 1; }
}

@media (prefers-reduced-motion: reduce) {
  .td-study-answer-area {
    animation: none;
  }
}

/* Bottom */
.td-study-bottom {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 20px 32px 32px;
  flex-shrink: 0;
}

.td-study-counts {
  margin: 0;
  font-size: 14px;
  font-variant-numeric: tabular-nums;
  color: var(--color-td-ink-4);
}

.td-study-counts span {
  text-underline-offset: 4px;
}

.td-study-counts .is-current {
  text-decoration: underline;
}

.td-study-count-new {
  color: var(--color-td-info);
}

.td-study-count-learning {
  color: var(--color-td-danger);
}

.td-study-count-due {
  color: var(--color-td-tip);
}

/* Show answer button */
.td-study-show-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  min-width: 240px;
  min-height: 52px;
  padding: 0 24px;
  border-radius: var(--radius-td-lg);
  border: 1px solid var(--color-td-line);
  background: var(--color-td-surface);
  color: var(--color-td-ink);
  font: 500 15px var(--font-sans);
  cursor: pointer;
}

.td-study-show-btn:hover {
  background: var(--color-td-hover);
}

.td-study-show-btn kbd {
  font-family: inherit;
  font-size: 11px;
  color: var(--color-td-ink-4);
  border: 1px solid var(--color-td-line);
  border-radius: 4px;
  padding: 1px 6px;
}

.td-study-show-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

/* Preview navigation */
.td-study-preview-nav {
  display: flex;
  gap: 12px;
}

/* Ease buttons */
.td-study-eases {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 12px;
}

.td-study-ease {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  width: 128px;
  min-height: 64px;
  border-radius: var(--radius-td-lg);
  border: 1px solid var(--color-td-line);
  background: var(--color-td-surface);
  font-family: var(--font-sans);
  cursor: pointer;
}

.td-study-ease:hover {
  background: var(--color-td-hover);
  border-color: var(--color-td-ink-4);
}

.td-study-ease-ivl {
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--color-td-ink-4);
}

.td-study-ease-label {
  font-size: 15px;
  font-weight: 500;
}

.td-study-ease-again .td-study-ease-label { color: var(--color-td-danger); }
.td-study-ease-hard .td-study-ease-label { color: var(--color-td-ink-4); }
.td-study-ease-good .td-study-ease-label { color: var(--color-td-tip); }
.td-study-ease-easy .td-study-ease-label { color: var(--color-td-info); }

/* Done */
.td-study-done {
  max-width: 520px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  text-align: center;
  padding-top: 48px;
}

.td-study-done-title {
  margin: 0;
  font-family: var(--font-serif, Georgia, serif);
  font-weight: 400;
  font-size: 32px;
}

.td-study-done-text {
  margin: 0;
  font-size: 16px;
  line-height: 1.6;
  color: var(--color-td-ink-4);
}

.td-study-done-btn {
  margin-top: 16px;
  display: inline-flex;
  align-items: center;
  min-height: 48px;
  padding: 0 24px;
  border-radius: 999px;
  border: 1px solid var(--color-td-line);
  background: none;
  color: var(--color-td-ink);
  font: inherit;
  font-size: 15px;
  cursor: pointer;
  text-decoration: none;
}

.td-study-done-btn:hover {
  background: var(--color-td-hover);
}

/* Touch: hide kbd hints */
@media (hover: none) and (pointer: coarse) {
  .td-study-back kbd,
  .td-study-show-btn kbd {
    display: none;
  }
}

/* Mobile */
@media (width < 40rem) {
  .td-study-main {
    padding: 40px 20px 24px;
  }

  .td-study-question {
    font-size: 28px;
  }

  .td-study-answer {
    font-size: 19px;
  }

  .td-study-spacer,
  .td-study-where {
    display: none;
  }

  .td-study-ease {
    width: calc(50% - 6px);
  }

  .td-study-show-btn {
    width: 100%;
  }
}
</style>
