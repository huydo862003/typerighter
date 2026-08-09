<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  File, FolderOpen, CornerLeftUp,
} from '@lucide/vue';
import {
  getParentUrl,
} from '@/shared';
import type {
  DirectoryEntry, SubdirectoryEntry,
} from '@/shared';

const {
  url,
  subdirectories = [],
  items = [],
} = defineProps<{
  /** Absolute directory URL */
  url: string;
  /** Subdirectories with item counts */
  subdirectories?: SubdirectoryEntry[];
  /** Content items in this directory */
  items?: DirectoryEntry[];
}>();

const isRoot = computed(() => url === '/');
const parentUrl = computed(() => getParentUrl(url));
</script>

<template>
  <div>
    <a
      v-if="!isRoot"
      :href="parentUrl"
      class="td-dir-row"
    >
      <CornerLeftUp
        :size="16"
        class="text-td-gray-500 flex-shrink-0"
      />
      <span class="font-td-semibold text-td-body-sm text-td-neutral-fg-muted">..</span>
    </a>

    <div v-if="subdirectories.length > 0">
      <a
        v-for="sub in subdirectories"
        :key="sub.url"
        :href="sub.url"
        class="td-dir-row"
      >
        <FolderOpen
          :size="16"
          class="text-td-gray-500 flex-shrink-0"
        />
        <span class="font-td-semibold text-td-body-sm">{{ sub.name }}</span>
        <span class="text-td-label text-td-neutral-fg-muted ml-auto">{{ sub.count }}</span>
      </a>
    </div>

    <div v-if="items.length > 0">
      <a
        v-for="item in items"
        :key="item.url"
        :href="item.url"
        class="td-dir-row"
      >
        <File
          :size="16"
          class="text-td-gray-500 flex-shrink-0"
        />
        <span class="font-td-semibold text-td-body-sm">{{ item.name }}</span>
      </a>
    </div>
  </div>
</template>

<style scoped>
.td-dir-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 4px;
  border-bottom: 1px solid var(--color-td-neutral-border-subtle);
  text-decoration: none;
  color: var(--color-td-fg);
  transition: background-color 0.1s;
}

.td-dir-row:hover {
  background: var(--color-td-primary-bg-subtle);
}
</style>
