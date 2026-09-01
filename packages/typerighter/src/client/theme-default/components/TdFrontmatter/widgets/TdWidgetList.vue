<script setup lang="ts">
import {
  ref,
} from 'vue';
import {
  File,
} from '@lucide/vue';
import TdWidgetRelation from './TdWidgetRelation.vue';
import {
  extractRef, type ResolvedRef,
} from './ref';
import {
  getPageIcon,
} from '@/client/theme-default/composables/pageIcon';
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
  <!-- List of relations / files / images -->
  <template v-if="isRelationList">
    <div>
      <ul class="td-widget-list td-widget-list-icons">
        <li
          v-for="(resolvedItem, idx) in visible(resolvedRefs)"
          :key="idx"
        >
          <component
            :is="resolvedItem.icon ? getPageIcon(resolvedItem.icon.name) ?? File : File"
            :size="14"
            class="td-widget-list-icon"
          />
          <TdWidgetRelation
            :value="{
              $ref: resolvedItem,
            }"
          />
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
    </div>
  </template>

  <!-- Other lists -->
  <template v-else>
    <div>
      <ul class="td-widget-list">
        <li
          v-for="(item, idx) in visible(items)"
          :key="idx"
        >
          <TdWidgetRelation
            v-if="extractRef(item)"
            :value="item"
          />
          <template v-else>
            {{ item }}
          </template>
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
    </div>
  </template>
</template>

<style scoped>
.td-widget-list {
  list-style-type: disc;
  padding-left: 1.25rem;
  margin: 0;
}

.td-widget-list-icons {
  list-style-type: none;
  padding-left: 0;
}

.td-widget-list-icons li {
  display: flex;
  align-items: center;
  gap: 6px;
}

.td-widget-list-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-fg-muted);
}

.td-widget-list li {
  margin: 0;
  font-size: var(--font-size-td-sm);
}

.td-widget-more {
  border: none;
  background: none;
  color: var(--color-td-neutral-fg-muted);
  font-size: var(--font-size-td-sm);
  cursor: pointer;
  padding: 2px 4px;
  margin-top: 4px;
  border-radius: var(--border-radius-td-sm);
}

.td-widget-more:hover {
  background-color: var(--color-td-neutral-muted);
  color: var(--color-td-neutral-fg);
}
</style>
