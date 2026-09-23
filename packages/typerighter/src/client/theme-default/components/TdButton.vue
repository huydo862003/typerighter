<script setup lang="ts">
const {
  label,
} = defineProps<{
  /** Accessible label */
  label: string;
}>();
</script>

<template>
  <button
    type="button"
    class="td-icon-btn"
    :title="label"
    :aria-label="label"
  >
    <slot />
  </button>
</template>

<style scoped>
.td-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 6px;
  background: none;
  cursor: pointer;
  color: var(--color-td-ink-4);
  transition: color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-icon-btn:hover {
  color: var(--color-td-ink);
}

.td-icon-btn:active {
  transform: scale(0.94);
}
</style>
