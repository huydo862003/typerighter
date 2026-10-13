<script setup lang="ts">
import {
  nextTick, useTemplateRef, watch,
} from 'vue';
import TdContentNav from './TdContentNav.vue';
import TdSkeleton from './TdSkeleton.vue';
import {
  useRoute,
} from '@/client/app';
import {
  type ContentTree,
} from '@/shared';

const {
  tree, searchActive, ready,
} = defineProps<{
  /** Content tree to render in the nav */
  tree: ContentTree;
  /** Whether the sidebar search is active (hides nav) */
  searchActive: boolean;
  /** Whether site data has finished loading */
  ready: boolean;
}>();

const route = useRoute();
const scrollElement = useTemplateRef<HTMLElement>('scrollElement');

watch(
  () => route.path,
  () => {
    nextTick(() => {
      scrollElement.value?.querySelector<HTMLElement>('.is-active')?.scrollIntoView({
        block: 'nearest',
      });
    });
  },
  {
    immediate: true,
  },
);
</script>

<template>
  <div
    ref="scrollElement"
    class="td-sidebar-scroll"
  >
    <TdSkeleton v-if="!searchActive && !ready" />
    <TdContentNav
      v-else-if="!searchActive"
      :tree="tree"
    />
  </div>
</template>

<style scoped>
.td-sidebar-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: clip;
  overscroll-behavior: contain;
  padding: 8px 0 0;
  scrollbar-gutter: stable;
}
</style>
