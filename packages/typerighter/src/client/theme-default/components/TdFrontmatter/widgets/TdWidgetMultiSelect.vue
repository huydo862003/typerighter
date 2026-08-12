<script setup lang="ts">
import {
  ref,
} from 'vue';

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

function toggleExpand () {
  expanded.value = !expanded.value;
}
</script>

<template>
  <span
    v-for="(item, idx) in visible()"
    :key="idx"
    class="td-widget-pill"
  >{{ item }}</span>
  <button
    v-if="hiddenCount > 0"
    class="td-widget-more"
    type="button"
    @click="toggleExpand"
  >
    {{ expanded ? 'show less' : `+${hiddenCount} more` }}
  </button>
</template>

<style scoped>
.td-widget-pill {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: var(--font-size-td-caption);
  background: var(--color-td-primary-bg-subtle);
  color: var(--color-td-primary-solid);
}

.td-widget-more {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-td-caption);
  color: var(--color-td-neutral-fg-muted);
  padding: 2px 0;
}

.td-widget-more:hover {
  color: var(--color-td-primary-solid);
}
</style>
