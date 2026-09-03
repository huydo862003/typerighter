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
  renderInlineMarkup,
} from '../utils/renderInlineMarkup';
import {
  getPageIcon,
} from '../utils/pageIcon';
import TdTooltip from './TdTooltip.vue';
import {
  formatRelativeTime, getDirectoryUrl, getIndexUrl, getNodeIndexItem, getTdContentUrl, getTdResourceTitle, isIndexFile, isUrlAncestorOf, unslugify,
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
const indexItem = getNodeIndexItem(node);
const folderPath = indexItem ? getTdContentUrl(indexItem.filepath) : getIndexUrl(directoryUrl);
const folderHref = withBase(folderPath);
const folderLabel = indexItem?.label ?? unslugify(node.name);
const folderIcon = indexItem?.icon ? getPageIcon(indexItem.icon.name) : undefined;
const regularEntries = node.entries.filter((entry) =>
  entry.kind === 'dir' || !isIndexFile(entry.item.filepath));

const hasContent = 0 < node.entries.length;

const totalCount = computed(() => countItems(node));

function countItems (n: ContentTreeNode): number {
  let count = 0;

  for (const entry of n.entries) {
    if (entry.kind === 'file') {
      count++;
    } else {
      count += countItems(entry.node);
    }
  }

  return count;
}

const collapsed = ref(!isUrlAncestorOf(directoryUrl, route.path));
const showAll = ref(false);
const MAX_VISIBLE_FILES = 5;
const fileCount = regularEntries.filter((entry) => entry.kind === 'file').length;
const hiddenCount = computed(() => showAll.value ? 0 : Math.max(0, fileCount - MAX_VISIBLE_FILES));
const visibleEntries = computed(() => {
  if (showAll.value) return regularEntries;

  let filesShown = 0;

  return regularEntries.filter((entry) => {
    if (entry.kind === 'dir') return true;

    return ++filesShown <= MAX_VISIBLE_FILES;
  });
});

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
        <span class="td-tree-label-text">{{ folderLabel }}</span>
      </button>
      <a
        :href="folderHref"
        class="td-tree-index-btn"
        :class="{
          'is-active': isCurrent(folderPath),
        }"
      >
        <component
          :is="folderIcon"
          v-if="folderIcon"
          :size="12"
        />
        <FolderOpen
          v-else-if="isCurrent(folderPath)"
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
      <template
        v-for="entry in visibleEntries"
        :key="entry.kind === 'dir' ? entry.node.name : entry.item.filepath"
      >
        <TdTreeNode
          v-if="entry.kind === 'dir'"
          :node="entry.node"
          :url-prefix="directoryUrl"
        />
        <a
          v-else
          :href="withBase(getTdContentUrl(entry.item.filepath))"
          class="td-tree-link"
          :class="{
            'is-active': isCurrent(getTdContentUrl(entry.item.filepath)),
          }"
        >
          <component
            :is="getPageIcon(entry.item.icon.name)!"
            v-if="entry.item.icon && getPageIcon(entry.item.icon.name)"
            :size="14"
            class="td-tree-file-icon"
          />
          <File
            v-else
            :size="14"
            class="td-tree-file-icon"
          />
          <TdTooltip
            class="td-tree-link-text"
            :text="getTdResourceTitle(entry.item.filepath, entry.item.label)"
          ><span v-html="renderInlineMarkup(getTdResourceTitle(entry.item.filepath, entry.item.label))" /></TdTooltip>
          <span class="td-tree-time">{{ formatRelativeTime(entry.item.metadata.mtime) }}</span>
        </a>
      </template>
      <button
        v-if="hiddenCount > 0 && !showAll"
        class="td-tree-more"
        type="button"
        @click="expandAll"
      >
        {{ hiddenCount }} more...
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
  font-size: var(--font-size-td-2xs);
  letter-spacing: var(--tracking-td-wide);
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
  color: var(--color-td-primary-solid);
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
  font-family: var(--font-mono);
  font-size: var(--font-size-td-xs);
  color: var(--color-td-neutral-border-strong);
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
  font-size: var(--font-size-td-sm);
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
  font-size: var(--font-size-td-xs);
  color: var(--color-td-neutral-border);
}

.td-tree-link.is-active {
  background-color: var(--color-td-primary-bg-hover);
  border-left-color: var(--color-td-primary-solid);
  color: var(--color-td-primary-solid);
  font-weight: 600;
}

.td-tree-more {
  display: block;
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-td-xs);
  color: var(--color-td-primary-solid);
  padding: 5px 12px;
  transition: color 0.15s;
}

.td-tree-more:hover {
  color: var(--color-td-primary-solid-hover);
}
</style>
