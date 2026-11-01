<script setup lang="ts">
import {
  inject,
} from 'vue';
import {
  tdGridContextKey,
} from '../../utils/injectionKeys';

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
  console.warn('[TdGridCell] should be used inside a <TdGrid> component.');
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
