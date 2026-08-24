<script setup lang="ts">
import {
  getPillColor,
} from './getPillColor';

const {
  value,
} = defineProps<{
  /** Raw value */
  value: unknown;
}>();
</script>

<template>
  <span
    class="td-widget-pill"
    :style="getPillColor(value)"
  >{{ value }}</span>
</template>

<style scoped>
.td-widget-pill {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: var(--font-size-td-xs);
}
</style>
