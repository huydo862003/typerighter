<script setup lang="ts">
import {
  computed,
} from 'vue';
import TdFrontmatterValue from './TdFrontmatterValue.vue';
import {
  useSiteData,
} from '@/client/app';
import {
  unslugify,
  type SchemaDefinition,
} from '@/shared';

const {
  schema = undefined,
  frontmatter,
} = defineProps<{
  /** Schema type name */
  schema?: string;
  /** Frontmatter header fields */
  frontmatter: Record<string, unknown>;
}>();

const siteData = useSiteData();

const schemaDefinition = computed((): SchemaDefinition | undefined => {
  if (!schema) return undefined;

  return siteData.schemas[schema];
});

const entries = computed(() => {
  const schemaDef = schemaDefinition.value;

  if (!schemaDef) return [];

  return Object.entries(schemaDef)
    .filter(([key]) => !key.startsWith('_'));
});
</script>

<template>
  <div
    v-if="entries.length > 0"
    class="td-frontmatter"
  >
    <div
      v-for="[
        key,
        definition,
      ] in entries"
      :key="key"
      class="td-frontmatter-row"
    >
      <span class="td-frontmatter-label">{{ unslugify(key) }}</span>
      <span class="td-frontmatter-value">
        <TdFrontmatterValue
          :definition="definition"
          :value="frontmatter[key]"
        />
      </span>
    </div>
  </div>
</template>

<style scoped>
.td-frontmatter {
  margin: 16px 0 24px;
  border: 1px solid var(--color-td-neutral-border-subtle);
  border-radius: 8px;
  overflow: hidden;
}

.td-frontmatter-row {
  display: flex;
  align-items: start;
  gap: 12px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--color-td-neutral-border-subtle);
}

.td-frontmatter-row:last-child {
  border-bottom: none;
}

.td-frontmatter-label {
  flex-shrink: 0;
  width: 120px;
  max-width: 40%;
  font-size: var(--font-size-td-nav);
  color: var(--color-td-neutral-fg-muted);
}

.td-frontmatter-value {
  flex: 1;
  font-size: var(--font-size-td-ui);
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
}
</style>
