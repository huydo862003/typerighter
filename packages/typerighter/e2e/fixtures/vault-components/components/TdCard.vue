<template>
  <div
    class="td-card"
    :data-has-footer="Boolean(footer)"
  >
    <header class="td-card__header">
      <span
        v-if="icon"
        class="td-card__icon"
        aria-hidden="true"
      >{{ icon }}</span>
      <h3 class="td-card__title">
        {{ title }}
      </h3>
    </header>
    <div class="td-card__body">
      <slot />
    </div>
    <footer
      v-if="footer"
      class="td-card__footer"
    >
      {{ footer }}
    </footer>
  </div>
</template>

<script setup lang="ts">
const {
  title,
  icon = undefined,
  footer = undefined,
} = defineProps<{
  /** Card heading */
  title: string;
  /** Emoji or icon text */
  icon?: string;
  /** Footer text */
  footer?: string;
}>();
</script>
