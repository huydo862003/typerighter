<script setup lang="ts">
const {
  lines = 5,
} = defineProps<{
  /** Number of skeleton lines */
  lines?: number;
}>();

const widths = [
  87,
  64,
  75,
  58,
  68,
  80,
  55,
  72,
];
</script>

<template>
  <div
    class="td-skeleton"
    role="status"
    aria-label="Loading"
  >
    <div
      v-for="i in lines"
      :key="i"
      class="td-skeleton-line"
      :style="{
        width: `${widths[(i - 1) % widths.length]}%`,
      }"
    />
  </div>
</template>

<style scoped>
.td-skeleton {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 8px 16px;
}

.td-skeleton-line {
  height: 12px;
  border-radius: 4px;
  background: var(--color-td-neutral-bg-subtle);
  animation: td-skeleton-pulse 1.5s ease-in-out infinite;
}

@media (prefers-reduced-motion: reduce) {
  .td-skeleton-line {
    animation: none;
    opacity: 0.6;
  }
}

@keyframes td-skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}
</style>
