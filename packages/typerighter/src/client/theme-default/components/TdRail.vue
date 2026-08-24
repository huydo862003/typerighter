<script setup lang="ts">
import {
  useTdContent,
} from '@/client/app';
import {
  TdFrontmatter,
} from './TdFrontmatter';
import TdToc from './TdToc.vue';

const {
  page,
} = useTdContent();
</script>

<template>
  <aside class="td-rail">
    <TdFrontmatter
      mode="rail"
      :schema="page.schema"
      :frontmatter="page.frontmatter"
    />
    <TdToc :headings="page.headings" />
  </aside>
</template>

<style scoped>
.td-rail {
  position: sticky;
  top: var(--td-header-height);
  height: calc(100vh - var(--td-header-height));
  overflow-y: auto;
  padding: 24px 18px 40px;
  min-width: 0;
  border-left: 1px solid var(--color-td-neutral-border-subtle);
  background: var(--color-td-neutral-bg);
  display: flex;
  flex-direction: column;
  gap: 22px;
}
</style>
