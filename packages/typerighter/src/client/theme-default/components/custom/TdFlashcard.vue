<script setup lang="ts">
import {
  readonly,
  ref,
  watch,
} from 'vue';

defineOptions({
  inheritAttrs: false,
});

const {
  id = undefined,
  revealed: initialState = false,
  disabled = false,
  questionLabel = 'Question',
  keyLabel = 'Answer',
} = defineProps<{
  /** HTML id attribute */
  id?: string;
  /** Start with the key side showing */
  revealed?: boolean;
  /** Disable flipping interaction */
  disabled?: boolean;
  /** Caption shown above the question */
  questionLabel?: string;
  /** Caption shown above the key */
  keyLabel?: string;
}>();

const isRevealed = ref(initialState);

watch(() => initialState, () => {
  isRevealed.value = initialState;
});

function flip (): boolean {
  if (disabled) return isRevealed.value;

  const oldValue = isRevealed.value;

  isRevealed.value = !isRevealed.value;

  return oldValue;
}

function showKey (): boolean {
  const oldValue = isRevealed.value;

  isRevealed.value = true;

  return oldValue;
}

function showQuestion (): boolean {
  const oldValue = isRevealed.value;

  isRevealed.value = false;

  return oldValue;
}

defineExpose({
  flip,
  showQuestion,
  showKey,
  state: readonly(isRevealed),
});
</script>

<template>
  <div
    v-bind="$attrs"
    :id="id"
    class="td-flashcard"
    :class="{
      'is-revealed': isRevealed,
      'is-disabled': disabled,
    }"
    role="button"
    :tabindex="disabled ? -1 : 0"
    :aria-pressed="isRevealed"
    :aria-label="isRevealed ? keyLabel : questionLabel"
    @click="flip"
    @keydown.enter.prevent="flip"
    @keydown.space.prevent="flip"
  >
    <div class="td-flashcard-face td-flashcard-face-question">
      <p class="td-flashcard-label">
        {{ questionLabel }}
      </p>
      <div class="td-flashcard-body">
        <slot />
      </div>
    </div>

    <div class="td-flashcard-face td-flashcard-face-key">
      <p class="td-flashcard-label">
        {{ keyLabel }}
      </p>
      <div class="td-flashcard-body">
        <slot name="key" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.td-flashcard {
  /* Both faces share one grid cell, so the card grows to fit the taller of
     the two instead of being sized by the question alone */
  display: grid;
  grid-template: 1fr / 1fr;
  min-height: 8rem;
  user-select: none;
  cursor: pointer;
  perspective: max(100vw, 100vh);
}

.td-flashcard-face {
  grid-area: 1 / 1;
  /* Hide each face's reverse, so the question shows its back once rotated */
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1.25rem 1.5rem;
  border-radius: 0.75rem;
  /* `neutral-border-strong` rather than `-subtle`: the subtle token measures
     1.3:1 against the page and fails the 3:1 floor for a component boundary */
  border: 1px solid var(--color-td-neutral-border-strong);
  background: var(--color-td-neutral-bg-subtle);
  color: var(--color-td-fg);
  font-size: var(--font-size-td-base);
  line-height: var(--leading-td-relaxed);
  transition-property: transform;
  transition-duration: var(--duration-slow, 300ms);
  transition-timing-function: var(--ease-default, ease);
}

/* Tint the revealed side so a flipped card reads as flipped at a glance */
.td-flashcard-face-key {
  background: var(--color-td-primary-bg-subtle);
  border-color: var(--color-td-primary-solid);
  transform: rotateY(180deg);
}

.td-flashcard.is-revealed .td-flashcard-face-question {
  transform: rotateY(180deg);
}

.td-flashcard.is-revealed .td-flashcard-face-key {
  transform: rotateY(0deg);
}

/* The face turned away must not swallow selections or clicks */
.td-flashcard-face-key,
.td-flashcard.is-revealed .td-flashcard-face-question {
  pointer-events: none;
}

.td-flashcard.is-revealed .td-flashcard-face-key {
  pointer-events: auto;
}

.td-flashcard-label {
  margin: 0;
  font-size: var(--font-size-td-2xs);
  font-weight: 600;
  letter-spacing: var(--tracking-td-wide);
  line-height: 1;
  text-transform: uppercase;
  /* `neutral-fg` not `-fg-muted`: at 11px the muted token measures 3.6:1 in
     light mode, under the 4.5:1 needed for small text */
  color: var(--color-td-neutral-fg);
}

.td-flashcard-face-key .td-flashcard-label {
  color: var(--color-td-primary-fg);
}

.td-flashcard-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: left;
  overflow-wrap: anywhere;
}

/* Slot content belongs to the page, not this component, so it needs :slotted */
.td-flashcard-body :slotted(:first-child) {
  margin-top: 0;
}

.td-flashcard-body :slotted(:last-child) {
  margin-bottom: 0;
}

.td-flashcard:focus-visible {
  outline: 2px solid var(--color-td-link);
  outline-offset: 3px;
}

.td-flashcard.is-disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

@media (prefers-reduced-motion: reduce) {
  .td-flashcard-face {
    transition-duration: 0ms;
  }
}
</style>
