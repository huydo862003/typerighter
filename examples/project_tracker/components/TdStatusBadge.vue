<template>
  <span
    class="td-status-badge"
    :data-status="status"
  >
    <span class="td-status-badge__dot" />
    <slot>{{ status }}</slot>
  </span>
</template>

<script setup lang="ts">
import {
  computed,
} from 'vue';

const {
  status = 'unknown',
} = defineProps<{
  /** One of: active, done, blocked, todo, in-progress, unknown */
  status?: string;
}>();
</script>

<style scoped>
.td-status-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 600;
  line-height: 1.5;
  white-space: nowrap;
}

.td-status-badge__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* Green: done, active */
.td-status-badge[data-status="done"],
.td-status-badge[data-status="active"] {
  background: #dcfce7;
  color: #166534;
}
.td-status-badge[data-status="done"] .td-status-badge__dot,
.td-status-badge[data-status="active"] .td-status-badge__dot {
  background: #22c55e;
}

/* Yellow: in-progress, in progress */
.td-status-badge[data-status="in-progress"],
.td-status-badge[data-status="in progress"] {
  background: #fef9c3;
  color: #854d0e;
}
.td-status-badge[data-status="in-progress"] .td-status-badge__dot,
.td-status-badge[data-status="in progress"] .td-status-badge__dot {
  background: #eab308;
}

/* Red: blocked */
.td-status-badge[data-status="blocked"] {
  background: #fee2e2;
  color: #991b1b;
}
.td-status-badge[data-status="blocked"] .td-status-badge__dot {
  background: #ef4444;
}

/* Gray: todo, unknown */
.td-status-badge[data-status="todo"],
.td-status-badge[data-status="unknown"] {
  background: #f3f4f6;
  color: #374151;
}
.td-status-badge[data-status="todo"] .td-status-badge__dot,
.td-status-badge[data-status="unknown"] .td-status-badge__dot {
  background: #9ca3af;
}

/* Dark mode */
:root.dark .td-status-badge[data-status="done"],
:root.dark .td-status-badge[data-status="active"] {
  background: #052e16;
  color: #86efac;
}
:root.dark .td-status-badge[data-status="in-progress"],
:root.dark .td-status-badge[data-status="in progress"] {
  background: #422006;
  color: #fde047;
}
:root.dark .td-status-badge[data-status="blocked"] {
  background: #450a0a;
  color: #fca5a5;
}
:root.dark .td-status-badge[data-status="todo"],
:root.dark .td-status-badge[data-status="unknown"] {
  background: #1f2937;
  color: #d1d5db;
}
</style>
