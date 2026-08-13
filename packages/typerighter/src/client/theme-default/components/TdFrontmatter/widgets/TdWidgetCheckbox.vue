<script setup lang="ts">
import {
  CheckSquare, Square,
} from '@lucide/vue';

const {
  value,
} = defineProps<{
  /** Raw value */
  value: unknown;
}>();
</script>

<template>
  <span class="td-widget-checkbox">
    <CheckSquare
      v-if="value"
      :size="16"
      class="td-widget-checkbox-icon is-checked"
    />
    <Square
      v-else
      :size="16"
      class="td-widget-checkbox-icon"
    />
  </span>
</template>

<style scoped>
.td-widget-checkbox {
  display: inline-flex;
  align-items: center;
}

.td-widget-checkbox-icon {
  color: var(--color-td-neutral-fg-muted);
}

.td-widget-checkbox-icon.is-checked {
  color: var(--color-td-primary-solid);
}
</style>
