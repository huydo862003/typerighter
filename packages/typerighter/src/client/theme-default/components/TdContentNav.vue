<script setup lang="ts">
import {
  File, House,
} from '@lucide/vue';
import {
  useRoute, useSiteConfig,
} from '../../app';
import TdTreeNode from './TdTreeNode.vue';
import {
  formatRelativeTime, getIndexUrl, getTdContentUrl, getTdResourceTitle, isIndexFile,
  type ContentTree,
} from '@/shared';

const {
  tree,
} = defineProps<{
  /** Content tree with interleaved entries */
  tree: ContentTree;
}>();

const route = useRoute();
const {
  withBase,
} = useSiteConfig();

const indexItem = tree.entries
  .find((entry): entry is Extract<typeof entry, {
    kind: 'file';
  }> => entry.kind === 'file' && isIndexFile(entry.item.filepath))
  ?.item;
const regularEntries = tree.entries.filter((entry) =>
  entry.kind === 'dir' || !isIndexFile(entry.item.filepath));

function isCurrent (href: string): boolean {
  return route.path === href;
}
</script>

<template>
  <nav>
    <a
      :href="withBase(indexItem ? getTdContentUrl(indexItem.filepath) : getIndexUrl('/'))"
      class="td-root-link"
      :class="{
        'is-active': isCurrent(indexItem ? getTdContentUrl(indexItem.filepath) : getIndexUrl('/')),
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
    <template
      v-for="entry in regularEntries"
      :key="entry.kind === 'dir' ? entry.node.name : entry.item.filepath"
    >
      <TdTreeNode
        v-if="entry.kind === 'dir'"
        :node="entry.node"
      />
      <a
        v-else
        :href="withBase(getTdContentUrl(entry.item.filepath))"
        class="td-root-link"
        :class="{
          'is-active': isCurrent(getTdContentUrl(entry.item.filepath)),
        }"
      >
        <File
          :size="14"
          class="td-root-link-icon"
        />
        <span class="td-root-link-text">{{ getTdResourceTitle(entry.item.filepath, entry.item.label) }}</span>
        <span class="td-root-link-time">{{ formatRelativeTime(entry.item.metadata.mtime) }}</span>
      </a>
    </template>
  </nav>
</template>

<style scoped>
.td-root-link {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 20px;
  font-size: var(--font-size-td-sm);
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
  font-weight: 600;
}

.td-root-link-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-root-link-time {
  flex-shrink: 0;
  margin-left: auto;
  font-size: var(--font-size-td-xs);
  color: var(--color-td-neutral-border);
}

.td-root-link-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-border-strong);
}
</style>
