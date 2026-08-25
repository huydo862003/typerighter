<script setup lang="ts">
import {
  formatRelativeTime,
  type DirectoryEntry,
} from '@/shared';

const {
  item,
  href,
  showTags = false,
} = defineProps<{
  /** Directory entry to render */
  item: DirectoryEntry;
  /** Resolved link href */
  href: string;
  /** Whether to show tags */
  showTags?: boolean;
}>();
</script>

<template>
  <a
    :href="href"
    class="td-glossary-row"
  >
    <span class="td-glossary-row-header">
      <span class="td-glossary-row-title">{{ item.name }}</span>
      <span
        v-if="item.mtime"
        class="td-glossary-row-time"
      >{{ formatRelativeTime(item.mtime) }}</span>
    </span>
    <span
      v-if="item.description"
      class="td-glossary-row-desc"
    >{{ item.description }}</span>
    <span
      v-else
      class="td-glossary-row-desc td-glossary-row-stub"
    >No description yet</span>
    <span
      v-if="showTags && item.tags && item.tags.length > 0"
      class="td-glossary-row-tags"
    >
      <span
        v-for="tag in item.tags"
        :key="tag"
        class="td-glossary-row-tag"
      >{{ tag }}</span>
    </span>
  </a>
</template>

<style scoped>
.td-glossary-row,
.td-glossary-row:hover {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 13px 4px;
  border-bottom: 1px solid var(--color-td-neutral-border-subtle);
  text-decoration: none;
  color: inherit;
  transition: background-color 0.1s;
}

.td-glossary-row:last-child {
  border-bottom: none;
}

.td-glossary-row:hover {
  background: var(--color-td-primary-bg-subtle);
}

.td-glossary-row:hover .td-glossary-row-title {
  text-decoration: underline;
}

.td-glossary-row-header {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

.td-glossary-row-title {
  font-size: var(--font-size-td-base);
  font-weight: 800;
  color: var(--color-td-primary-solid);
}

.td-glossary-row-time {
  margin-left: auto;
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  color: var(--color-td-neutral-border-strong);
  flex-shrink: 0;
}

.td-glossary-row-desc {
  font-size: var(--font-size-td-xs);
  line-height: var(--leading-td-normal);
  color: var(--color-td-neutral-fg-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-glossary-row-stub {
  font-style: italic;
  color: var(--color-td-neutral-border-strong);
}

.td-glossary-row-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.td-glossary-row-tag {
  border: 1px solid var(--color-td-neutral-border-subtle);
  background: var(--color-td-neutral-bg);
  padding: 1px 8px;
  border-radius: 999px;
  font-size: var(--font-size-td-2xs);
  color: var(--color-td-neutral-fg-muted);
}
</style>
