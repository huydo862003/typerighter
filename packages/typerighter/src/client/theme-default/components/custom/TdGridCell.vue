<script lang="ts">
// eslint-disable-next-line import/order
import {
  type InjectionKey,
} from 'vue';

export const tdGridContextKey: InjectionKey<true> = Symbol('td-grid-context');
</script>

<script setup lang="ts">
// eslint-disable-next-line import/first
import {
  inject,
} from 'vue';

const {
  // eslint-disable-next-line unicorn/name-replacements
  colSpan = 1,
  rowSpan = 1,
} = defineProps<{
  /** Number of grid columns this cell spans */
  colSpan?: number;
  /** Number of grid rows this cell spans */
  rowSpan?: number;
}>();

const inGrid = inject(tdGridContextKey, false);

if (!inGrid) {
  throw new Error('[TdGridCell] must be used inside a <Grid> component.');
}
</script>

<template>
  <div
    class="td-grid-cell"
    :style="{
      ...(colSpan
        ? {
          gridColumn: `span ${colSpan}`,
        }
        : {}),
      ...(rowSpan
        ? {
          gridRow: `span ${rowSpan}`,
        }
        : {}),
    }"
  >
    <slot />
  </div>
</template>

<style scoped>
/* Prevent wide content from blowing out a column */
.td-grid-cell {
  min-width: 0;
}
</style>
