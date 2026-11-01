<script setup lang="ts">
import {
  computed, provide,
} from 'vue';
import {
  tdGridContextKey,
} from './TdGridCell.vue';

const {
  columns, gap = 1,
} = defineProps<{
  /** Number of equal columns, or 'auto' to fill based on intrinsic content width */
  columns: number | 'auto';
  /** Gap between cells in rem (default 1) */
  gap?: number;
}>();

provide(tdGridContextKey, true);

const gridTemplateColumns = computed(() =>
  columns === 'auto'
    ? 'repeat(auto-fill, minmax(auto, 1fr))'
    : `repeat(${columns}, 1fr)`);
</script>

<template>
  <div
    class="td-grid"
    :style="{
      gridTemplateColumns,
      gap: `${gap}rem`,
    }"
  >
    <slot />
  </div>
</template>

<style scoped>
.td-grid {
  display: grid;
}
</style>
