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
        class="td-dir-icon"
      />
      <span class="td-dir-parent">..</span>
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
          class="td-dir-icon"
        />
        <span class="td-dir-name">{{ sub.name }}</span>
        <span class="td-dir-count">{{ sub.count }}</span>
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
          class="td-dir-icon"
        />
        <span class="td-dir-name">{{ item.name }}</span>
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

.td-dir-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-border-strong);
}

.td-dir-parent {
  font-weight: var(--font-weight-td-semibold);
  font-size: var(--font-size-td-body-sm);
  color: var(--color-td-neutral-fg-muted);
}

.td-dir-name {
  font-weight: var(--font-weight-td-semibold);
  font-size: var(--font-size-td-body-sm);
}

.td-dir-count {
  margin-left: auto;
  font-size: var(--font-size-td-label);
  color: var(--color-td-neutral-fg-muted);
}
</style>
