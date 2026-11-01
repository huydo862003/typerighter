<script setup lang="ts">
import {
  computed, provide,
} from 'vue';
import {
  tdGridContextKey,
} from '../../utils/injectionKeys';

const {
  columns, gap = 1,
} = defineProps<{
  /** Number of equal columns */
  columns: number;
  /** Gap between cells in rem (default 1) */
  gap?: number;
}>();

provide(tdGridContextKey, true);

const gridTemplateColumns = computed(() => `repeat(${columns}, 1fr)`);
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
