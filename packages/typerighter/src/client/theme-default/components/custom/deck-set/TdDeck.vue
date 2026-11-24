<script setup lang="ts">
import {
  computed, onMounted, ref, watch,
} from 'vue';
import {
  ChevronLeft, ChevronRight, ChevronsRight, Check,
} from '@lucide/vue';
import {
  useFlashcardStore,
} from '@/client/theme-default/composables/flashcard';
import type {
  CardState,
} from '@/client/theme-default/composables/flashcard';
import type {
  Resource,
} from '@/shared';

const {
  name,
  cards,
} = defineProps<{
  /** Deck display name */
  name: string;
  /** Flashcard content items in this deck */
  cards: Resource[];
}>();

const {
  getCardStates, isCardDue, reviewCard,
} = useFlashcardStore();

const GRADE_INDEX: Record<string, number> = {
  again: 0,
  hard: 1,
  good: 2,
  easy: 3,
};

type Mode = 'browse' | 'review' | 'done';
interface Tally {
  again: number;
  hard: number;
  good: number;
  easy: number;
}

// State

const expanded = ref(false);
const mode = ref<Mode>('browse');
const queue = ref<string[]>([]);
const position = ref(0);
const revealed = ref(false);
const expandedId = ref<string | null>(null);
const tally = ref<Tally>({
  again: 0,
  hard: 0,
  good: 0,
  easy: 0,
});
const cardStates = ref<Map<string, CardState>>(new Map());

// Derived

const dueCards = computed(() =>
  cards.filter((card) => isCardDue(cardStates.value.get(card.filepath))));

const currentCard = computed(() => {
  const filepath = queue.value[position.value];

  return cards.find((card) => card.filepath === filepath);
});

const remaining = computed(() => queue.value.length - position.value);

const progressPct = computed(() =>
  0 < queue.value.length
    ? Math.round((position.value / queue.value.length) * 100)
    : 0);

const actionLabel = computed(() => {
  if (mode.value === 'browse') return 'Review';
  if (mode.value === 'done') return 'Close';

  return 'End';
});

const doneSummary = computed(() =>
  0 < dueCards.value.length
    ? `${dueCards.value.length} card${dueCards.value.length === 1 ? '' : 's'} still due`
    : 'All cards reviewed. Nothing due right now.');

// Helpers

function endReview (): void {
  mode.value = 'browse';
  revealed.value = false;
}

function getCardGrade (card: Resource): string {
  const state = cardStates.value.get(card.filepath);

  if (!state) return 'due';

  return state.lastGrade ?? 'due';
}

// Actions

async function grade (level: 'again' | 'hard' | 'good' | 'easy'): Promise<void> {
  const filepath = queue.value[position.value];

  if (!filepath) return;

  const next = await reviewCard(filepath, GRADE_INDEX[level]);

  cardStates.value.set(filepath, next);
  tally.value[level]++;

  if (level === 'again') queue.value.push(filepath);

  const nextPosition = position.value + 1;

  if (queue.value.length <= nextPosition) {
    mode.value = 'done';
  } else {
    position.value = nextPosition;
  }

  revealed.value = false;
}

function gradeAgain (): void {
  grade('again');
}

function gradeEasy (): void {
  grade('easy');
}

function gradeGood (): void {
  grade('good');
}

function gradeHard (): void {
  grade('hard');
}

function handleAction (): void {
  if (mode.value === 'browse') startReview();
  else endReview();
}

async function loadCardStates (): Promise<void> {
  cardStates.value = await getCardStates(cards.map((card) => card.filepath));
}

function previousCard (): void {
  if (0 < position.value) {
    position.value--;
    revealed.value = false;
  }
}

function revealAnswer (): void {
  revealed.value = true;
}

function skipCard (): void {
  if (remaining.value <= 1) return;
  const current = queue.value.splice(position.value, 1)[0];

  queue.value.push(current);
  revealed.value = false;
}

function startReview (): void {
  let ids = dueCards.value.map((card) => card.filepath);

  if (ids.length === 0) ids = cards.map((card) => card.filepath);
  queue.value = ids;
  position.value = 0;
  revealed.value = false;
  tally.value = {
    again: 0,
    hard: 0,
    good: 0,
    easy: 0,
  };
  mode.value = 'review';
  expanded.value = true;
}

function toggleCard (filepath: string): void {
  expandedId.value = expandedId.value === filepath ? null : filepath;
}

function toggleExpanded (): void {
  expanded.value = !expanded.value;
}

// Lifecycle

onMounted(loadCardStates);
watch(() => cards, loadCardStates);
</script>

<template>
  <div class="td-deck">
    <!-- Header: click to expand/collapse, always visible -->
    <header
      class="td-deck-header"
      role="button"
      tabindex="0"
      :aria-expanded="expanded"
      @click="toggleExpanded"
      @keydown.enter.prevent="toggleExpanded"
      @keydown.space.prevent="toggleExpanded"
    >
      <ChevronRight
        aria-hidden="true"
        :size="16"
        class="td-deck-header-chevron"
        :class="{
          'is-open': expanded,
        }"
      />
      <div class="td-deck-header-left">
        <h3 class="td-deck-name">
          {{ name }}
        </h3>
      </div>
      <span class="td-deck-due-counter">
        <strong class="td-deck-due-num">{{ dueCards.length }} due</strong> /
        {{ cards.length }}
      </span>
      <button
        type="button"
        class="td-deck-action"
        :class="
          mode === 'browse'
            ? 'td-deck-action-primary'
            : 'td-deck-action-secondary'
        "
        @click.stop="handleAction"
      >
        {{ actionLabel }}
      </button>
    </header>

    <!-- Body: expands/collapses with grid transition -->
    <div
      class="td-collapse"
      :class="{
        'is-collapsed': !expanded,
      }"
    >
      <div>
        <div class="td-deck-body">
          <!-- Browse mode: scrollable card list -->
          <ul
            v-if="mode === 'browse'"
            class="td-deck-list"
          >
            <li
              v-for="(card, index) in cards"
              :key="card.filepath"
              class="td-deck-list-item"
            >
              <button
                type="button"
                class="td-deck-list-row"
                :aria-expanded="expandedId === card.filepath"
                @click="() => toggleCard(card.filepath)"
              >
                <span class="td-deck-list-num">{{
                  String(index + 1).padStart(2, "0")
                }}</span>
                <span class="td-deck-list-text">
                  <span class="td-deck-list-q">{{
                    card.fields.question || card.filepath
                  }}</span>
                  <span
                    v-if="card.fields.answer"
                    class="td-deck-list-hint"
                  >{{
                    card.fields.answer
                  }}</span>
                </span>
                <span
                  class="td-deck-chip"
                  :class="`td-deck-chip-${getCardGrade(card)}`"
                >{{ getCardGrade(card) }}</span>
                <ChevronRight
                  aria-hidden="true"
                  :size="18"
                  class="td-deck-list-chevron"
                  :class="{
                    'is-open': expandedId === card.filepath,
                  }"
                />
              </button>
              <div
                class="td-collapse"
                :class="{
                  'is-collapsed': expandedId !== card.filepath,
                }"
              >
                <div>
                  <p class="td-deck-list-answer">
                    {{ card.fields.answer }}
                  </p>
                </div>
              </div>
            </li>
          </ul>

          <!-- Review mode -->
          <div
            v-else-if="mode === 'review' && currentCard"
            class="td-deck-review"
          >
            <div class="td-deck-review-top">
              <span class="td-deck-position">{{ position + 1 }} / {{ queue.length }}</span>
              <div
                class="td-deck-progress-bar"
                aria-hidden="true"
              >
                <div
                  class="td-deck-progress-fill"
                  :style="{
                    width: `${progressPct}%`,
                  }"
                />
              </div>
              <button
                type="button"
                class="td-deck-icon-btn"
                :disabled="position === 0"
                aria-label="Previous card"
                @click="previousCard"
              >
                <ChevronLeft
                  aria-hidden="true"
                  :size="16"
                />
              </button>
              <button
                type="button"
                class="td-deck-skip-btn"
                :disabled="remaining <= 1"
                @click="skipCard"
              >
                Skip
                <ChevronsRight
                  aria-hidden="true"
                  :size="16"
                />
              </button>
            </div>

            <div class="td-deck-question">
              <h3 class="td-deck-question-text">
                {{ currentCard.fields.question || currentCard.filepath }}
              </h3>
            </div>

            <Transition
              name="td-deck-reveal"
              mode="out-in"
            >
              <div
                v-if="revealed"
                key="answer"
                class="td-deck-answer-panel"
              >
                <p class="td-deck-answer-eyebrow">
                  Answer
                </p>
                <p class="td-deck-answer-text">
                  {{ currentCard.fields.answer }}
                </p>
              </div>
              <div
                v-else
                key="placeholder"
                class="td-deck-answer-placeholder"
              >
                Try to recall the answer, then reveal it.
              </div>
            </Transition>

            <div class="td-deck-bottom-row">
              <button
                v-if="!revealed"
                type="button"
                class="td-deck-show-btn"
                @click="revealAnswer"
              >
                Show answer
              </button>
              <template v-else>
                <button
                  type="button"
                  class="td-deck-grade-btn td-deck-grade-again"
                  @click="gradeAgain"
                >
                  Again
                </button>
                <button
                  type="button"
                  class="td-deck-grade-btn td-deck-grade-hard"
                  @click="gradeHard"
                >
                  Hard
                </button>
                <button
                  type="button"
                  class="td-deck-grade-btn td-deck-grade-good"
                  @click="gradeGood"
                >
                  Good
                </button>
                <button
                  type="button"
                  class="td-deck-grade-btn td-deck-grade-easy"
                  @click="gradeEasy"
                >
                  Easy
                </button>
              </template>
            </div>
          </div>

          <!-- Done mode -->
          <div
            v-else-if="mode === 'done'"
            class="td-deck-done"
          >
            <div class="td-deck-done-icon">
              <Check
                aria-hidden="true"
                :size="26"
              />
            </div>
            <h3 class="td-deck-done-title">
              Session complete
            </h3>
            <p class="td-deck-done-summary">
              {{ doneSummary }}
            </p>
            <div class="td-deck-tally">
              <span class="td-deck-tally-pill td-deck-chip-again">Again {{ tally.again }}</span>
              <span class="td-deck-tally-pill td-deck-chip-hard">Hard {{ tally.hard }}</span>
              <span class="td-deck-tally-pill td-deck-chip-good">Good {{ tally.good }}</span>
              <span class="td-deck-tally-pill td-deck-chip-easy">Easy {{ tally.easy }}</span>
            </div>
            <div class="td-deck-done-actions">
              <button
                type="button"
                class="td-deck-action-secondary"
                @click="endReview"
              >
                Back to deck
              </button>
              <button
                v-if="dueCards.length > 0"
                type="button"
                class="td-deck-action-primary"
                @click="startReview"
              >
                Review {{ dueCards.length }} again
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.td-deck {
  border: 1px solid var(--color-td-line-soft);
  border-radius: 14px;
  background: var(--color-td-surface);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* Header: clickable toggle + controls */
.td-deck-header {
  box-sizing: border-box;
  padding: 10px 16px 10px 14px;
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--color-td-surface-muted, #f4f1ec);
  border-bottom: 1px solid var(--color-td-line-soft);
  flex-shrink: 0;
  cursor: pointer;
  user-select: none;
}

.td-deck-header:hover {
  background: var(--color-td-hover);
}

.td-deck-header-chevron {
  flex-shrink: 0;
  color: var(--color-td-ink-4);
  transition: transform var(--duration-td-open) var(--ease-td-out-quart);
}

.td-deck-header-chevron.is-open {
  transform: rotate(90deg);
}

.td-deck-header-left {
  display: flex;
  flex-direction: column;
  gap: 3px;
  flex: 1;
  min-width: 0;
}

.td-deck-name {
  margin: 0;
  font-size: 14px;
  line-height: 20px;
  font-weight: 600;
}

.td-deck-due-counter {
  font-size: 13px;
  color: var(--color-td-ink-4);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.td-deck-due-num {
  font-weight: 600;
  color: var(--color-td-accent);
}

/* Action button: fixed width to prevent shift */
.td-deck-action,
.td-deck-action-primary,
.td-deck-action-secondary {
  width: 80px;
  height: 32px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
}

.td-deck-action-primary {
  border: 1px solid transparent;
  background: var(--color-td-accent);
  color: white;
}

.td-deck-action-primary:hover {
  opacity: 0.9;
}

.td-deck-action-secondary {
  border: 1px solid var(--color-td-line-soft);
  background: var(--color-td-surface);
  color: var(--color-td-ink);
}

.td-deck-action-secondary:hover {
  background: var(--color-td-hover);
}

/* Body: fixed height */
.td-deck-body {
  height: 442px;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* Browse: card list */
.td-deck-list {
  list-style: none;
  margin: 0;
  padding: 0;
  flex: 1;
  overflow-y: auto;
}

.td-deck-list-item {
  border-bottom: 1px solid var(--color-td-line-soft);
}

.td-deck-list-row {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 14px 24px;
  background: none;
  border: 0;
  text-align: left;
  cursor: pointer;
  color: inherit;
  font: inherit;
}

.td-deck-list-row:hover {
  background: var(--color-td-hover);
}

.td-deck-list-num {
  width: 24px;
  font-size: 13px;
  color: var(--color-td-ink-4);
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
}

.td-deck-list-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.td-deck-list-q {
  font-size: 15px;
  font-weight: 500;
}

.td-deck-list-hint {
  font-size: 13px;
  color: var(--color-td-ink-4);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-deck-list-chevron {
  flex-shrink: 0;
  color: var(--color-td-ink-4);
  transition: transform 150ms;
}

.td-deck-list-chevron.is-open {
  transform: rotate(90deg);
}

.td-deck-list-answer {
  margin: 0;
  min-height: 0;
  overflow: hidden;
  padding: 8px 56px 16px 64px;
  font-size: 14px;
  line-height: 1.55;
  color: var(--color-td-ink-3);
}

.td-deck-list-answer a {
  color: var(--color-td-accent);
  text-decoration: none;
}

.td-deck-list-answer a:hover {
  text-decoration: underline;
}

/* Status chips */
.td-deck-chip {
  flex-shrink: 0;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  text-transform: capitalize;
}

.td-deck-chip-due {
  background: var(--color-td-danger-bg);
  color: var(--color-td-danger);
}

.td-deck-chip-again {
  background: var(--color-td-danger-bg);
  color: var(--color-td-danger);
}

.td-deck-chip-hard {
  background: var(--color-td-warning-bg);
  color: var(--color-td-warning-text);
}

.td-deck-chip-good {
  background: var(--color-td-info-bg);
  color: var(--color-td-info);
}

.td-deck-chip-easy {
  background: var(--color-td-tip-bg);
  color: var(--color-td-tip);
}

/* Review mode */
.td-deck-review {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 20px 24px 24px;
}

.td-deck-review-top {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 36px;
}

.td-deck-position {
  font-size: 13px;
  color: var(--color-td-ink-4);
  font-variant-numeric: tabular-nums;
  width: 56px;
}

.td-deck-progress-bar {
  flex: 1;
  height: 4px;
  border-radius: 2px;
  background: var(--color-td-line-soft);
  overflow: hidden;
}

.td-deck-progress-fill {
  height: 100%;
  background: var(--color-td-accent);
  transition: width 200ms;
}

.td-deck-icon-btn {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  border: 1px solid var(--color-td-line-soft);
  background: var(--color-td-surface);
  cursor: pointer;
  color: var(--color-td-ink);
}

.td-deck-icon-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.td-deck-skip-btn {
  height: 36px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  border-radius: 8px;
  border: 1px solid var(--color-td-line-soft);
  background: var(--color-td-surface);
  color: var(--color-td-ink);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
}

.td-deck-skip-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.td-deck-question {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.td-deck-question-text {
  margin: 0;
  font-size: 22px;
  line-height: 30px;
  font-weight: 600;
}

.td-deck-question-hint {
  margin: 0;
  font-size: 15px;
  line-height: 22px;
  color: var(--color-td-ink-4);
}

.td-deck-answer-panel {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  box-sizing: border-box;
  padding: 18px 20px;
  border-radius: 10px;
  background: var(--color-td-surface-muted, #f7f5f1);
  border: 1px solid var(--color-td-line-soft);
}

.td-deck-answer-eyebrow {
  margin: 0 0 6px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-td-ink-4);
}

.td-deck-answer-text {
  margin: 0;
  font-size: 16px;
  line-height: 1.6;
}

.td-deck-answer-text a {
  color: var(--color-td-accent);
  text-decoration: none;
}

.td-deck-answer-text a:hover {
  text-decoration: underline;
}

.td-deck-answer-placeholder {
  flex: 1;
  min-height: 0;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  border: 1.5px dashed var(--color-td-line-soft);
  color: var(--color-td-ink-4);
  font-size: 14px;
}

.td-deck-bottom-row {
  height: 44px;
  display: flex;
  gap: 10px;
  flex-shrink: 0;
}

.td-deck-show-btn {
  flex: 1;
  height: 44px;
  border-radius: 8px;
  border: 0;
  background: var(--color-td-ink);
  color: var(--color-td-surface);
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
}

.td-deck-show-btn:hover {
  opacity: 0.9;
}

.td-deck-grade-btn {
  flex: 1;
  height: 44px;
  border-radius: 8px;
  border: 0;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
}

.td-deck-grade-btn:hover {
  opacity: 0.85;
}

.td-deck-grade-again {
  background: var(--color-td-danger-bg);
  color: var(--color-td-danger);
}

.td-deck-grade-hard {
  background: var(--color-td-warning-bg);
  color: var(--color-td-warning-text);
}

.td-deck-grade-good {
  background: var(--color-td-info-bg);
  color: var(--color-td-info);
}

.td-deck-grade-easy {
  background: var(--color-td-tip-bg);
  color: var(--color-td-tip);
}

/* Done mode */
.td-deck-done {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 24px;
  text-align: center;
}

.td-deck-done-icon {
  width: 56px;
  height: 56px;
  border-radius: 28px;
  background: var(--color-td-tip-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-td-tip);
}

.td-deck-done-title {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
}

.td-deck-done-summary {
  margin: 0;
  font-size: 15px;
  color: var(--color-td-ink-4);
}

.td-deck-tally {
  display: flex;
  gap: 8px;
}

.td-deck-tally-pill {
  padding: 6px 12px;
  border-radius: 999px;
  font-size: 14px;
  font-weight: 600;
}

.td-deck-done-actions {
  display: flex;
  gap: 10px;
  margin-top: 8px;
}

.td-deck-done-actions button {
  height: 44px;
  padding: 0 18px;
  border-radius: 8px;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
}

/* Answer reveal transition */
.td-deck-reveal-enter-active,
.td-deck-reveal-leave-active {
  transition: opacity 200ms ease, transform 200ms ease;
}

.td-deck-reveal-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.td-deck-reveal-leave-to {
  opacity: 0;
}
</style>
