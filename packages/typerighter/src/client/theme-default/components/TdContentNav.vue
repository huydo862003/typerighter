<script setup lang="ts">
import {
  File, House,
} from '@lucide/vue';
import {
  useRoute,
} from '../../app';
import TdTreeNode from './TdTreeNode.vue';
import {
  formatRelativeTime, getTdContentUrl, getTdResourceTitle, INDEX_FILENAME, path,
  type ContentTree,
} from '@/shared';

const {
  tree,
} = defineProps<{
  /** Content tree with root items and directory nodes */
  tree: ContentTree;
}>();

const route = useRoute();
const indexItem = tree.rootItems.find((item) => path.filestem(item.filepath) === INDEX_FILENAME);
const regularRootItems = tree.rootItems.filter((item) => path.filestem(item.filepath) !== INDEX_FILENAME);

function isCurrent (href: string): boolean {
  return route.path === href;
}
</script>

<template>
  <nav>
    <a
      :href="indexItem ? getTdContentUrl(indexItem.filepath) : '/'"
      class="td-root-link"
      :class="{
        'is-active': isCurrent(indexItem ? getTdContentUrl(indexItem.filepath) : '/'),
      }"
    >
      <House
        :size="14"
        class="td-root-link-icon"
      />
      <span class="td-root-link-text">Overview</span>
      <span
        v-if="indexItem"
        class="td-root-link-time"
      >{{ formatRelativeTime(indexItem.metadata.mtime) }}</span>
    </a>
    <a
      v-for="item in regularRootItems"
      :key="item.filepath"
      :href="getTdContentUrl(item.filepath)"
      class="td-root-link"
      :class="{
        'is-active': isCurrent(getTdContentUrl(item.filepath)),
      }"
    >
      <File
        :size="14"
        class="td-root-link-icon"
      />
      <span class="td-root-link-text">{{ getTdResourceTitle(item.header, item.filepath) }}</span>
      <span class="td-root-link-time">{{ formatRelativeTime(item.metadata.mtime) }}</span>
    </a>
    <TdTreeNode
      v-for="node in tree.children"
      :key="node.name"
      :node="node"
    />
  </nav>
</template>

<style scoped>
.td-root-link {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 20px;
  font-size: var(--font-size-td-nav);
  color: var(--color-td-neutral-fg);
  text-decoration: none;
  border-left: 3px solid transparent;
  transition: background-color 0.1s;
}

.td-root-link:hover {
  background-color: var(--color-td-neutral-bg-hover);
}

.td-root-link.is-active {
  background-color: var(--color-td-primary-bg-hover);
  border-left-color: var(--color-td-primary-solid);
  color: var(--color-td-primary-solid);
  font-weight: var(--font-weight-td-semibold);
}

.td-root-link-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-root-link-time {
  flex-shrink: 0;
  font-size: var(--font-size-td-caption);
  color: var(--color-td-neutral-border);
}

.td-root-link-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-border-strong);
}
</style>
