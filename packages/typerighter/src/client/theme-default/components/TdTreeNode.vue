<script setup lang="ts">
import {
  computed, ref, watch,
} from 'vue';
import {
  ChevronDown, File, House,
} from '@lucide/vue';
import {
  useRoute,
} from '../../app';
import {
  formatRelativeTime, getDirectoryUrl, getTdContentUrl, getTdResourceTitle, INDEX_FILENAME, isUrlAncestorOf, path, unslugify,
  type ContentTreeNode,
} from '@/shared';

const {
  node,
  depth = 0,
  urlPrefix = '',
} = defineProps<{
  /** Tree node to render */
  node: ContentTreeNode;
  /** Nesting depth for indentation */
  depth?: number;
  /** Accumulated path prefix for building directory hrefs */
  urlPrefix?: string;
}>();

const route = useRoute();
const directoryUrl = getDirectoryUrl(urlPrefix, node.name);
const indexItem = node.items.find((item) => path.filestem(item.filepath) === INDEX_FILENAME);
const regularItems = node.items.filter((item) => path.filestem(item.filepath) !== INDEX_FILENAME);

const hasContent = 0 < node.children.length || 0 < node.items.length;

const totalCount = computed(() => countItems(node));

function countItems (n: ContentTreeNode): number {
  return n.items.length + n.children.reduce((sum, child) => sum + countItems(child), 0);
}

const collapsed = ref(!isUrlAncestorOf(directoryUrl, route.path));
const expanded = ref(false);
const MAX_VISIBLE = 4;
const visibleItems = computed(() => expanded.value ? regularItems : regularItems.slice(0, MAX_VISIBLE));
const hiddenCount = computed(() => Math.max(0, regularItems.length - MAX_VISIBLE));

watch(() => route.path, (currentPath) => {
  if (isUrlAncestorOf(directoryUrl, currentPath)) collapsed.value = false;
});

function expand () {
  expanded.value = true;
}

function isCurrent (href: string): boolean {
  return route.path === href;
}

function toggle () {
  collapsed.value = !collapsed.value;
}
</script>

<template>
  <div
    v-if="hasContent"
    class="td-tree-node"
  >
    <button
      type="button"
      class="td-tree-label"
      :aria-expanded="!collapsed"
      :style="{
        paddingLeft: `${12 + depth * 12}px`,
      }"
      @click="toggle"
    >
      <ChevronDown
        :size="14"
        class="td-tree-caret"
        :class="{
          'is-collapsed': collapsed,
        }"
      />
      <span class="td-tree-label-text">{{ unslugify(node.name) }}</span>
      <span class="td-tree-count">{{ totalCount }}</span>
    </button>
    <div
      v-if="!collapsed"
      class="td-tree-children"
      :style="{
        marginLeft: `${14 + depth * 10}px`,
      }"
    >
      <a
        :href="indexItem ? getTdContentUrl(indexItem.filepath) : directoryUrl"
        class="td-tree-link"
        :class="{
          'is-active': isCurrent(indexItem ? getTdContentUrl(indexItem.filepath) : directoryUrl),
        }"
        :style="{
          paddingLeft: `${32 + depth * 12}px`,
        }"
      >
        <House
          :size="14"
          class="td-tree-file-icon"
        />
        <span class="td-tree-link-text">{{ unslugify(node.name) }}</span>
        <span
          v-if="indexItem"
          class="td-tree-time"
        >{{ formatRelativeTime(indexItem.metadata.mtime) }}</span>
      </a>
      <TdTreeNode
        v-for="child in node.children"
        :key="child.name"
        :node="child"
        :depth="depth + 1"
        :url-prefix="directoryUrl"
      />
      <a
        v-for="item in visibleItems"
        :key="item.filepath"
        :href="getTdContentUrl(item.filepath)"
        class="td-tree-link"
        :class="{
          'is-active': isCurrent(getTdContentUrl(item.filepath)),
        }"
        :style="{
          paddingLeft: `${32 + depth * 12}px`,
        }"
      >
        <File
          :size="14"
          class="td-tree-file-icon"
        />
        <span class="td-tree-link-text">{{ getTdResourceTitle(item.header, item.filepath) }}</span>
        <span class="td-tree-time">{{ formatRelativeTime(item.metadata.mtime) }}</span>
      </a>
      <button
        v-if="hiddenCount > 0 && !expanded"
        class="td-tree-more"
        type="button"
        :style="{
          paddingLeft: `${32 + depth * 12}px`,
        }"
        @click="expand"
      >
        {{ hiddenCount }} more
      </button>
    </div>
  </div>
</template>

<style scoped>
.td-tree-label {
  display: flex;
  align-items: center;
  gap: 4px;
  width: 100%;
  padding: 6px 20px;
  font-size: var(--font-size-td-label);
  letter-spacing: var(--tracking-td-label);
  text-transform: uppercase;
  color: var(--color-td-neutral-fg-muted);
  background: none;
  border: none;
  cursor: pointer;
}

.td-tree-label:hover {
  color: var(--color-td-fg);
}

.td-tree-label-text {
  flex: 1;
  text-align: left;
}

.td-tree-count {
  font-size: 0.75rem;
  color: var(--color-td-neutral-border);
  letter-spacing: normal;
  text-transform: none;
}

.td-tree-caret {
  flex-shrink: 0;
  transition: transform 0.15s;
}

.td-tree-caret.is-collapsed {
  transform: rotate(-90deg);
}

.td-tree-children {
  border-left: 1px solid var(--color-td-neutral-border-subtle);
}

.td-tree-file-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-border-strong);
}

.td-tree-link {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  font-size: var(--font-size-td-nav);
  color: var(--color-td-neutral-fg);
  text-decoration: none;
  transition: background-color 0.1s;
}

.td-tree-link:hover {
  background-color: var(--color-td-neutral-bg-hover);
}

.td-tree-link-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-tree-time {
  flex-shrink: 0;
  font-size: 0.75rem;
  color: var(--color-td-neutral-border);
}

.td-tree-link.is-active {
  background-color: var(--color-td-primary-bg-hover);
  border-left-color: var(--color-td-primary-solid);
  color: var(--color-td-primary-solid);
  font-weight: var(--font-weight-td-semibold);
}

.td-tree-more {
  display: block;
  background: none;
  border: none;
  cursor: pointer;
  font-size: 0.75rem;
  color: var(--color-td-neutral-fg-muted);
  padding: 4px 12px;
}

.td-tree-more:hover {
  color: var(--color-td-fg);
}
</style>
