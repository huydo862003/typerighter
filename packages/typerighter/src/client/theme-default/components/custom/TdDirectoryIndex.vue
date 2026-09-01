<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  Folder, CornerLeftUp,
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
  const directory = siteData.value.directoryListings[path];

  return {
    url: path,
    entries: directory?.entries ?? [],
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

    <template
      v-for="entry in listing.entries"
      :key="entry.kind === 'dir' ? entry.sub.url : entry.item.url"
    >
      <a
        v-if="entry.kind === 'dir'"
        :href="withBase(entry.sub.url)"
        class="td-dir-row"
      >
        <Folder
          :size="16"
          class="td-dir-icon"
        />
        <span class="td-dir-name">{{ entry.sub.name }}</span>
        <span class="td-dir-count">{{ entry.sub.count }}</span>
      </a>
      <a
        v-else
        :href="withBase(entry.item.url)"
        class="td-dir-row"
      >
        <span class="td-dir-name">{{ entry.item.name }}</span>
      </a>
    </template>
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
