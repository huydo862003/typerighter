<script setup lang="ts">
import {
  ref,
} from 'vue';
import {
  extractRef, type ResolvedRef,
} from './ref';
import type {
  PropertyDescriptor,
} from '@/shared';

const {
  definition,
  value,
} = defineProps<{
  /** Property descriptor */
  definition: PropertyDescriptor;
  /** Raw value (array) */
  value: unknown;
}>();

const MAX_VISIBLE = 5;
const expanded = ref(false);

const items = Array.isArray(value) ? value : [];
const isRelationList = definition.items?.widget === 'relation';

const resolvedRefs = isRelationList
  ? items.map((item) => extractRef(item)).filter((ref_): ref_ is ResolvedRef => ref_ !== undefined)
  : [];

const hiddenCount = isRelationList
  ? Math.max(0, resolvedRefs.length - MAX_VISIBLE)
  : Math.max(0, items.length - MAX_VISIBLE);

function toggle () {
  expanded.value = !expanded.value;
}

function visible<T> (list: T[]): T[] {
  return expanded.value ? list : list.slice(0, MAX_VISIBLE);
}
</script>

<template>
  <!-- List of relations: links -->
  <template v-if="isRelationList">
    <ul class="td-widget-list">
      <li
        v-for="(resolvedItem, idx) in visible(resolvedRefs)"
        :key="idx"
      >
        <a
          :href="resolvedItem.url"
          class="td-widget-ref"
        >{{ resolvedItem.name }}</a>
      </li>
    </ul>
    <button
      v-if="hiddenCount > 0"
      class="td-widget-more"
      type="button"
      @click="toggle"
    >
      {{ expanded ? 'show less' : `+${hiddenCount} more` }}
    </button>
  </template>

  <!-- Other lists: bullet list -->
  <template v-else>
    <ul class="td-widget-list">
      <li
        v-for="(item, idx) in visible(items)"
        :key="idx"
      >
        {{ item }}
      </li>
    </ul>
    <button
      v-if="hiddenCount > 0"
      class="td-widget-more"
      type="button"
      @click="toggle"
    >
      {{ expanded ? 'show less' : `+${hiddenCount} more` }}
    </button>
  </template>
</template>

<style scoped>
.td-widget-ref {
  color: var(--color-td-link);
  text-decoration: none;
}

.td-widget-ref:hover {
  color: var(--color-td-link-hover);
  text-decoration: underline;
}

.td-widget-list {
  margin: 0;
  padding-left: 20px;
  list-style: disc;
}

.td-widget-list li {
  padding: 1px 0;
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
