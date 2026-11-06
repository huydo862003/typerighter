<script setup lang="ts">
defineProps<{
  /** typedown icon name without "icon." prefix */
  td: string;
  /** lucide icon name */
  lucide: string;
}>();
</script>

<template>
  <div class="icon-row">
    <span class="icon-preview"><LucideIcon :name="lucide" /></span>
    <code class="icon-td">icon.{{ td }}</code>
    <span class="icon-lucide">{{ lucide }}</span>
  </div>
</template>

<style scoped>
.icon-row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 5px 0;
  border-bottom: 1px solid var(--color-td-line);
  font-size: 0.875em;
}

.icon-preview {
  width: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--color-td-ink);
}

.icon-td {
  flex: 1;
}

.icon-lucide {
  color: var(--color-td-ink-4);
  min-width: 140px;
}
</style>
