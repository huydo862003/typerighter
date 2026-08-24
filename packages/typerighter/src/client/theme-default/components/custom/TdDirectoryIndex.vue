<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  File, Folder, CornerLeftUp,
} from '@lucide/vue';
import {
  useSiteData, useRoute, useSiteConfig,
} from '@/client/app';
import {
  getIndexUrl, getParentUrl,
} from '@/shared';

const route = useRoute();
const siteData = useSiteData();
const {
  withBase,
} = useSiteConfig();

const listing = computed(() => {
  const path = getParentUrl(route.path);
  const directory = siteData.directoryListings[path];

  return {
    url: path,
    subdirectories: directory?.subdirectories ?? [],
    items: directory?.items ?? [],
  };
});

const isRoot = computed(() => listing.value.url === '/');
const parentUrl = computed(() => withBase(getIndexUrl(getParentUrl(listing.value.url))));
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

    <div v-if="listing.subdirectories.length > 0">
      <a
        v-for="sub in listing.subdirectories"
        :key="sub.url"
        :href="withBase(sub.url)"
        class="td-dir-row"
      >
        <Folder
          :size="16"
          class="td-dir-icon"
        />
        <span class="td-dir-name">{{ sub.name }}</span>
        <span class="td-dir-count">{{ sub.count }}</span>
      </a>
    </div>

    <div v-if="listing.items.length > 0">
      <a
        v-for="item in listing.items"
        :key="item.url"
        :href="withBase(item.url)"
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
  font-weight: 600;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-neutral-fg-muted);
}

.td-dir-name {
  font-weight: 600;
  font-size: var(--font-size-td-sm);
}

.td-dir-count {
  margin-left: auto;
  font-size: var(--font-size-td-2xs);
  color: var(--color-td-neutral-fg-muted);
}
</style>
