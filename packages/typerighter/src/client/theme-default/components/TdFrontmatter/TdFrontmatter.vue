<script setup lang="ts">
import {
  computed, ref,
} from 'vue';
import {
  ArrowUpRight, ChevronDown, ChevronRight, CircleDot, List, Tag, Tags, Type,
} from '@lucide/vue';
import type {
  Component,
} from 'vue';
import TdFrontmatterValue from './TdFrontmatterValue.vue';
import {
  useSiteData,
} from '@/client/app';
import {
  isBuiltinField, unslugify,
  type PropertyDescriptor, type SchemaDefinition,
} from '@/shared';

const {
  mode = 'inline',
  schema = undefined,
  frontmatter,
} = defineProps<{
  /** Display mode: inline box or rail column */
  mode?: 'inline' | 'rail';
  /** Schema type name */
  schema?: string;
  /** Frontmatter header fields */
  frontmatter: Record<string, unknown>;
}>();

const siteData = useSiteData();

const schemaDefinition = computed((): SchemaDefinition | undefined => {
  if (!schema) return undefined;

  return siteData.value.schemas[schema];
});

const entries = computed(() => {
  const schemaDef = schemaDefinition.value;
  const result: Array<[string, PropertyDescriptor]> = [];

  const keys = new Set<string>();

  if (schemaDef) {
    Object.keys(schemaDef).forEach((keyName) => keys.add(keyName));
  }
  Object.keys(frontmatter).forEach((keyName) => keys.add(keyName));

  for (const key of keys) {
    if (isBuiltinField(key)) continue;

    const value = frontmatter[key];

    if (value === undefined || value === null) continue;
    if (Array.isArray(value) && value.length === 0) continue;

    const descriptor: PropertyDescriptor = schemaDef?.[key] ?? {
      widget: 'text',
    };

    result.push([
      key,
      descriptor,
    ]);
  }

  return result;
});

const widgetIcons: Record<string, Component> = {
  text: Type,
  checkbox: CircleDot,
  select: Tag,
  multiSelect: Tags,
  relation: ArrowUpRight,
  list: List,
};

function getWidgetIcon (widget: string): Component {
  return widgetIcons[widget] ?? Type;
}

const collapsed = ref(false);
const railCollapsed = ref(false);

function toggleInline () {
  collapsed.value = !collapsed.value;
}

function toggleRail () {
  railCollapsed.value = !railCollapsed.value;
}
</script>

<template>
  <!-- Rail mode: collapsible vertical stacked key/value pairs -->
  <div
    v-if="mode === 'rail' && entries.length > 0"
    class="td-fm-rail"
  >
    <button
      type="button"
      class="td-fm-rail-header"
      @click="toggleRail"
    >
      <span class="td-fm-rail-label">Properties</span>
      <ChevronDown
        :size="14"
        class="td-fm-rail-caret"
        :class="{
          'is-collapsed': railCollapsed,
        }"
      />
    </button>
    <div v-if="!railCollapsed">
      <div
        v-for="[
          key,
          definition,
        ] in entries"
        :key="key"
        class="td-fm-rail-row"
      >
        <span class="td-fm-rail-key">
          <component
            :is="getWidgetIcon(definition.widget)"
            :size="12"
            class="td-fm-icon"
          />
          {{ unslugify(key) }}
        </span>
        <span class="td-fm-rail-value">
          <TdFrontmatterValue
            :definition="definition"
            :value="frontmatter[key]"
          />
        </span>
      </div>
    </div>
  </div>

  <!-- Inline mode: collapsible bordered box -->
  <div
    v-else-if="entries.length > 0"
    class="td-fm-box"
  >
    <button
      type="button"
      class="td-fm-box-header"
      @click="toggleInline"
    >
      <span class="td-fm-box-label">Properties <span class="td-fm-box-count">{{ entries.length }}</span></span>
      <span class="td-fm-box-toggle">
        {{ collapsed ? 'Show' : 'Hide' }}
        <ChevronDown
          v-if="!collapsed"
          :size="14"
        />
        <ChevronRight
          v-else
          :size="14"
        />
      </span>
    </button>
    <div
      v-if="!collapsed"
      class="td-fm-box-body"
    >
      <template
        v-for="[
          key,
          definition,
        ] in entries"
        :key="key"
      >
        <span class="td-fm-box-key">
          <component
            :is="getWidgetIcon(definition.widget)"
            :size="12"
            class="td-fm-icon"
          />
          {{ unslugify(key) }}
        </span>
        <span class="td-fm-box-value">
          <TdFrontmatterValue
            :definition="definition"
            :value="frontmatter[key]"
          />
        </span>
      </template>
    </div>
  </div>
</template>

<style scoped>
/* Rail mode (desktop right column) */

.td-fm-rail {
  display: flex;
  flex-direction: column;
}

.td-fm-rail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 0;
  margin-bottom: 6px;
  background: none;
  border: none;
  cursor: pointer;
}

.td-fm-rail-label {
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  font-weight: 500;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-neutral-fg-muted);
}

.td-fm-rail-caret {
  color: var(--color-td-neutral-fg-muted);
  transition: transform 0.15s;
}

.td-fm-rail-caret.is-collapsed {
  transform: rotate(-90deg);
}

.td-fm-rail-row {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 9px 0;
}

.td-fm-rail-key {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: var(--font-size-td-2xs);
  color: var(--color-td-neutral-fg-muted);
}

.td-fm-rail-value {
  font-size: var(--font-size-td-xs);
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  align-items: center;
}

/* Inline box mode (medium and mobile) */

.td-fm-box {
  margin: 14px 0 0;
  border: 1px solid var(--color-td-neutral-border-subtle);
  border-radius: 6px;
  background: var(--color-td-neutral-bg);
  overflow: hidden;
}

.td-fm-box-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 10px 14px;
  border: none;
  background: none;
  cursor: pointer;
}

.td-fm-box-label {
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  font-weight: 500;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-neutral-fg-muted);
}

.td-fm-box-count {
  color: var(--color-td-neutral-border-strong);
}

.td-fm-box-toggle {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  font-size: var(--font-size-td-2xs);
  font-weight: 500;
  color: var(--color-td-primary-solid);
}

.td-fm-box-body {
  display: grid;
  grid-template-columns: 120px 1fr;
  row-gap: 10px;
  column-gap: 24px;
  align-items: start;
  padding: 14px;
  border-top: 1px solid var(--color-td-neutral-border-subtle);
  font-size: var(--font-size-td-xs);
}

.td-fm-box-key {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--color-td-neutral-fg-muted);
}

.td-fm-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-border-strong);
}

.td-fm-box-value {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  align-items: center;
}
</style>
