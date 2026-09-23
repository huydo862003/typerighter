<script setup lang="ts">
import {
  TdFrontmatter,
} from './TdFrontmatter';
import TdToc from './TdToc.vue';
import {
  useTdContent,
} from '@/client/app';

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
  overscroll-behavior: contain;
  padding: 24px 18px 40px;
  min-width: 0;
  border-left: 1px solid var(--color-td-line-soft);
  background: var(--color-td-pg);
  display: flex;
  flex-direction: column;
  gap: 22px;
}
</style>
