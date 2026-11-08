<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  Folder, House, Search,
} from '@lucide/vue';
import {
  useRoute, useSiteConfig,
} from '../../app';
import {
  getPageIcon,
} from '../utils/pageIcon';
import TdTooltip from './TdTooltip.vue';
import {
  getDirectoryUrl,
  getIndexUrl,
  getNodeIndexItem,
  getTdContentUrl,
  isIndexFile,
  isUrlAncestorOf,
  unslugify,
  type ContentTree,
} from '@/shared';

const {
  tree,
} = defineProps<{
  /** Content tree to render icons for */
  tree: ContentTree;
}>();

const emit = defineEmits<{
  search: [];
}>();

const route = useRoute();
const {
  withBase,
} = useSiteConfig();

interface SectionIcon {
  label: string;
  href: string;
  path: string;
  icon: ReturnType<typeof getPageIcon> | undefined;
  isFallback: boolean;
}

const rootIndex = computed(() => tree.entries.find(
  (entry): entry is Extract<typeof entry, {
    kind: 'file';
  }> =>
    entry.kind === 'file' && isIndexFile(entry.item.filepath),
)?.item);

const sections = computed((): SectionIcon[] => {
  const result: SectionIcon[] = [];

  for (const entry of tree.entries) {
    if (entry.kind !== 'dir') continue;

    const node = entry.node;
    const directoryUrl = getDirectoryUrl('', node.name);
    const indexItem = getNodeIndexItem(node);
    const path = indexItem ? getTdContentUrl(indexItem.filepath) : getIndexUrl(directoryUrl);
    const label = indexItem?.label ?? unslugify(node.name);
    const pageIcon = indexItem?.icon ? getPageIcon(indexItem.icon.name) : undefined;

    result.push({
      label,
      href: withBase(path),
      path,
      icon: pageIcon,
      isFallback: !pageIcon,
    });
  }

  return result;
});

function emitSearch () {
  emit('search');
}

function isActive (path: string): boolean {
  return route.path === path || isUrlAncestorOf(path, route.path);
}
</script>

<template>
  <div class="td-sidebar-icons">
    <!-- Search -->
    <TdTooltip
      text="Search"
      always
    >
      <button
        class="td-sidebar-icon-btn"
        type="button"
        aria-label="Search"
        @click="emitSearch"
      >
        <Search :size="16" />
      </button>
    </TdTooltip>

    <!-- Root / Home -->
    <TdTooltip
      v-if="rootIndex"
      :text="rootIndex.label ?? 'Home'"
      always
    >
      <a
        :href="withBase(getTdContentUrl(rootIndex.filepath))"
        class="td-sidebar-icon-btn"
        :class="{
          'is-active': isActive(getTdContentUrl(rootIndex.filepath)),
        }"
        :aria-label="rootIndex.label ?? 'Home'"
      >
        <House :size="16" />
      </a>
    </TdTooltip>

    <div
      v-if="sections.length"
      class="td-sidebar-icon-sep"
    />

    <!-- Section icons -->
    <TdTooltip
      v-for="section in sections"
      :key="section.href"
      :text="section.label"
      always
    >
      <a
        :href="section.href"
        class="td-sidebar-icon-btn"
        :class="{
          'is-active': isActive(section.path),
        }"
        :aria-label="section.label"
      >
        <component
          :is="section.icon"
          v-if="!section.isFallback"
          :size="16"
        />
        <Folder
          v-else
          :size="16"
        />
      </a>
    </TdTooltip>
  </div>
</template>

<style scoped>
.td-sidebar-icons {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 10px 0;
  gap: 2px;
}

.td-sidebar-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  background: none;
  cursor: pointer;
  border-radius: 8px;
  color: var(--color-td-ink-3);
  text-decoration: none;
  transition:
    background var(--duration-td-fast) var(--ease-td-out-quart),
    color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-sidebar-icon-btn:hover {
  background: var(--color-td-hover);
  color: var(--color-td-ink);
}

.td-sidebar-icon-btn.is-active {
  background: var(--color-td-accent-bg);
  color: var(--color-td-accent);
}

.td-sidebar-icon-sep {
  width: 20px;
  height: 1px;
  background: var(--color-td-line);
  margin: 4px 0;
}
</style>
