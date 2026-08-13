<script setup lang="ts">
import {
  computed, ref, watch,
} from 'vue';
import {
  ChevronDown, File, Folder, FolderOpen,
} from '@lucide/vue';
import {
  useRoute, useSiteConfig,
} from '../../app';
import {
  renderInlineMath,
} from '../composables/renderMath';
import TdTooltip from './TdTooltip.vue';
import {
  formatRelativeTime, getDirectoryUrl, getIndexUrl, getTdContentUrl, getTdResourceTitle, INDEX_FILENAME, isUrlAncestorOf, path, unslugify,
  type ContentTreeNode,
} from '@/shared';

const {
  node,
  urlPrefix = '',
} = defineProps<{
  /** Tree node to render */
  node: ContentTreeNode;
  /** Accumulated path prefix for building directory hrefs */
  urlPrefix?: string;
}>();

const route = useRoute();
const {
  withBase,
} = useSiteConfig();
const directoryUrl = getDirectoryUrl(urlPrefix, node.name);
const indexItem = node.items.find((item) => path.filestem(item.filepath) === INDEX_FILENAME);
const folderPath = indexItem ? getTdContentUrl(indexItem.filepath) : getIndexUrl(directoryUrl);
const folderHref = withBase(folderPath);
const regularItems = node.items.filter((item) => path.filestem(item.filepath) !== INDEX_FILENAME);

const hasContent = 0 < node.children.length || 0 < node.items.length;

const totalCount = computed(() => countItems(node));

function countItems (n: ContentTreeNode): number {
  return n.items.length + n.children.reduce((sum, child) => sum + countItems(child), 0);
}

const collapsed = ref(!isUrlAncestorOf(directoryUrl, route.path));
const showAll = ref(false);
const MAX_VISIBLE = 20;
const visibleItems = computed(() => showAll.value ? regularItems : regularItems.slice(0, MAX_VISIBLE));
const hiddenCount = computed(() => Math.max(0, regularItems.length - MAX_VISIBLE));

watch(() => route.path, (currentPath) => {
  if (isUrlAncestorOf(directoryUrl, currentPath)) collapsed.value = false;
});

function expandAll () {
  showAll.value = true;
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
    <div class="td-tree-label">
      <button
        type="button"
        class="td-tree-toggle"
        :aria-expanded="!collapsed"
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
      </button>
      <a
        :href="folderHref"
        class="td-tree-index-btn"
        :class="{
          'is-active': isCurrent(folderPath),
        }"
      >
        <FolderOpen
          v-if="isCurrent(folderPath)"
          :size="12"
        />
        <Folder
          v-else
          :size="12"
        />
      </a>
      <span class="td-tree-count">{{ totalCount }}</span>
    </div>
    <div
      v-if="!collapsed"
      class="td-tree-children"
    >
      <TdTreeNode
        v-for="child in node.children"
        :key="child.name"
        :node="child"
        :url-prefix="directoryUrl"
      />
      <a
        v-for="item in visibleItems"
        :key="item.filepath"
        :href="withBase(getTdContentUrl(item.filepath))"
        class="td-tree-link"
        :class="{
          'is-active': isCurrent(getTdContentUrl(item.filepath)),
        }"
      >
        <File
          :size="14"
          class="td-tree-file-icon"
        />
        <TdTooltip
          class="td-tree-link-text"
          :text="getTdResourceTitle(item.header, item.filepath)"
        ><span v-html="renderInlineMath(getTdResourceTitle(item.header, item.filepath))" /></TdTooltip>
        <span class="td-tree-time">{{ formatRelativeTime(item.metadata.mtime) }}</span>
      </a>
      <button
        v-if="hiddenCount > 0 && !showAll"
        class="td-tree-more"
        type="button"
        @click="expandAll"
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
  padding: 6px 20px;
  font-size: var(--font-size-td-label);
  letter-spacing: var(--tracking-td-label);
  text-transform: uppercase;
  color: var(--color-td-neutral-fg-muted);
}

.td-tree-toggle {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  background: none;
  border: none;
  cursor: pointer;
  font: inherit;
  letter-spacing: inherit;
  text-transform: inherit;
  color: inherit;
  padding: 0;
  overflow: hidden;
}

.td-tree-label:hover {
  color: var(--color-td-fg);
}

.td-tree-label-text {
  flex: 1;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-tree-index-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2px;
  border-radius: 4px;
  color: var(--color-td-neutral-border-strong);
  text-decoration: none;
  transition: color 0.1s;
}

.td-tree-index-btn:hover {
  color: var(--color-td-primary-solid);
}

.td-tree-index-btn.is-active {
  color: var(--color-td-primary-solid);
}

.td-tree-count {
  margin-left: auto;
  font-size: var(--font-size-td-caption);
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
  margin-left: 22px;
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
  transition: background-color 0.1s, border-left-color 0.1s, color 0.1s;
}

.td-tree-link:hover {
  background-color: var(--color-td-neutral-bg-hover);
}

.td-tree-link-text {
  flex: 1;
  min-width: 0;
}

.td-tree-time {
  flex-shrink: 0;
  margin-left: auto;
  font-size: var(--font-size-td-caption);
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
  font-size: var(--font-size-td-caption);
  color: var(--color-td-neutral-fg-muted);
  padding: 4px 12px;
  transition: color 0.15s;
}

.td-tree-more:hover {
  color: var(--color-td-fg);
}
</style>
