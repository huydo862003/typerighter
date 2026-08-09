<template>
  <div
    v-bind="$attrs"
    :id="id"
    class="flashcard"
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
    <div class="flashcard__face flashcard__face--question">
      <p class="flashcard__label">
        {{ questionLabel }}
      </p>
      <div class="flashcard__body">
        <slot />
      </div>
    </div>

    <div class="flashcard__face flashcard__face--key">
      <p class="flashcard__label">
        {{ keyLabel }}
      </p>
      <div class="flashcard__body">
        <slot name="key" />
      </div>
    </div>
  </div>
</template>

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

<style scoped>
.flashcard {
  /* Faces are stacked, so the card needs its own height to size them */
  position: relative;
  min-height: 10rem;
  user-select: none;
  cursor: pointer;
  perspective: max(100vw, 100vh); /* keep the card far enough from the screen */
}

.flashcard__face {
  /* Hide each face's reverse, so the question shows its back once rotated */
  backface-visibility: hidden;
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 1rem;
  border: 1px solid currentColor;
  border-radius: 0.5rem;
  transition-property: transform;
  transition-duration: var(--duration-slow, 300ms);
  transition-timing-function: var(--ease-default, ease);
}

.flashcard__face--key {
  position: absolute;
  inset: 0;
  transform: rotateY(180deg);
}

.flashcard.is-revealed .flashcard__face--question {
  transform: rotateY(180deg);
}

.flashcard.is-revealed .flashcard__face--key {
  transform: rotateY(0deg);
}

.flashcard__label {
  margin: 0;
  font-size: 0.75rem;
  line-height: 1rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  opacity: 0.6;
}

.flashcard__body {
  flex: 1;
  display: grid;
  place-items: center;
  text-align: center;
}

.flashcard:focus-visible {
  outline: 2px solid currentColor;
  outline-offset: 2px;
}

.flashcard.is-disabled {
  cursor: not-allowed;
  filter: brightness(0.95);
}

@media (prefers-reduced-motion: reduce) {
  .flashcard__face {
    transition-duration: 0ms;
  }
}
</style>
