<script setup lang="ts">
import TdWidgetCheckbox from './widgets/TdWidgetCheckbox.vue';
import TdWidgetList from './widgets/TdWidgetList.vue';
import TdWidgetMultiSelect from './widgets/TdWidgetMultiSelect.vue';
import TdWidgetRelation from './widgets/TdWidgetRelation.vue';
import TdWidgetSelect from './widgets/TdWidgetSelect.vue';
import TdWidgetText from './widgets/TdWidgetText.vue';
import {
  extractRef,
} from './widgets/ref';
import type {
  PropertyDescriptor,
} from '@/shared';

const {
  definition,
  value,
} = defineProps<{
  /** Property descriptor with widget hint */
  definition: PropertyDescriptor;
  /** Raw frontmatter value */
  value: unknown;
}>();
</script>

<template>
  <template v-if="value === undefined || value === null">
    <span class="td-frontmatter-empty">Empty</span>
  </template>
  <TdWidgetCheckbox
    v-else-if="definition.widget === 'checkbox'"
    :value="value"
  />
  <TdWidgetSelect
    v-else-if="definition.widget === 'select'"
    :value="value"
  />
  <TdWidgetMultiSelect
    v-else-if="definition.widget === 'multiSelect'"
    :value="value"
  />
  <TdWidgetRelation
    v-else-if="extractRef(value) || definition.widget === 'relation'"
    :value="value"
  />
  <TdWidgetList
    v-else-if="Array.isArray(value) || definition.widget === 'list'"
    :definition="definition"
    :value="value"
  />
  <TdWidgetText
    v-else
    :definition="definition"
    :value="value"
  />
</template>

<style scoped>
.td-frontmatter-empty {
  color: var(--color-td-neutral-fg-muted);
  font-style: italic;
  font-size: var(--font-size-td-xs);
}
</style>
