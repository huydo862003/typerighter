<script setup lang="ts">
import {
  ref,
} from 'vue';
import {
  getPillColor,
} from './getPillColor';

const {
  value,
} = defineProps<{
  /** Raw value (string array) */
  value: unknown;
}>();

const MAX_VISIBLE = 5;
const expanded = ref(false);

const items = Array.isArray(value) ? value : [];
const visible = () => expanded.value ? items : items.slice(0, MAX_VISIBLE);
const hiddenCount = Math.max(0, items.length - MAX_VISIBLE);

function toggle () {
  expanded.value = !expanded.value;
}
</script>

<template>
  <span
    v-for="(item, idx) in visible()"
    :key="idx"
    class="td-widget-pill"
    :style="getPillColor(item)"
  >{{ item }}</span>
  <button
    v-if="hiddenCount > 0"
    class="td-widget-more"
    type="button"
    @click="toggle"
  >
    {{ expanded ? 'show less' : `+${hiddenCount} more` }}
  </button>
</template>

<style scoped>
.td-widget-pill {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: var(--font-size-td-xs);
}

.td-widget-more {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-td-xs);
  color: var(--color-td-neutral-fg-muted);
  padding: 2px 0;
}

.td-widget-more:hover {
  color: var(--color-td-primary-solid);
}
</style>
