<script setup lang="ts">
import {
  LucideFileText,
} from '@lucide/vue';
import {
  extractRef,
} from './ref';
import {
  useSiteConfig,
} from '@/client/app';

const {
  value,
} = defineProps<{
  /** Raw value ({ $ref: { url, name, format? } }) */
  value: unknown;
}>();

const {
  withBase,
} = useSiteConfig();
const resolved = extractRef(value);
</script>

<template>
  <span
    v-if="resolved"
    class="td-widget-relation-wrapper"
  >
    <!-- Image preview: fixed size, clickable, opens in a new tab -->
    <a
      v-if="resolved.isImage"
      :href="withBase(resolved.url)"
      target="_blank"
      rel="noopener noreferrer"
      class="td-widget-image-preview"
      :title="resolved.name"
    >
      <img
        :src="withBase(resolved.url)"
        :alt="resolved.name"
        class="td-widget-image-img"
        loading="lazy"
      >
    </a>

    <!-- Non-image file: file icon with file name, opens in a new tab -->
    <a
      v-else-if="resolved.format || resolved.url.includes('.')"
      :href="withBase(resolved.url)"
      target="_blank"
      rel="noopener noreferrer"
      class="td-widget-file-link"
    >
      <LucideFileText class="td-widget-file-icon" />
      <span class="td-widget-file-name">{{ resolved.name }}</span>
    </a>

    <!-- Normal relation link -->
    <a
      v-else
      :href="withBase(resolved.url)"
      class="td-widget-ref"
    >{{ resolved.name }}</a>
  </span>
  <span v-else>{{ value }}</span>
</template>

<style scoped>
.td-widget-relation-wrapper {
  display: inline-flex;
  align-items: center;
}

.td-widget-ref {
  color: var(--color-td-link);
  text-decoration: none;
}

.td-widget-ref:hover {
  color: var(--color-td-link-hover);
  text-decoration: underline;
}

.td-widget-image-preview {
  display: inline-block;
  width: 180px;
  height: 260px;
  border-radius: var(--border-radius-td-lg, 8px);
  overflow: hidden;
  border: 1px solid var(--color-td-border, rgba(55, 53, 47, 0.16));
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08);
  transition: transform 0.15s ease, box-shadow 0.15s ease;
  vertical-align: middle;
  background-color: var(--color-td-bg-secondary, #f6f8fa);
}

.td-widget-image-preview:hover {
  transform: scale(1.02);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
}

.td-widget-image-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.td-widget-file-link {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--color-td-neutral-fg, #37352f);
  text-decoration: none;
  font-size: var(--font-size-td-body, 14px);
  line-height: 1.4;
  padding: 4px 8px;
  border-radius: 4px;
  background-color: var(--color-td-bg-secondary, rgba(55, 53, 47, 0.08));
  border: 1px solid var(--color-td-border, rgba(55, 53, 47, 0.12));
  transition: background-color 0.15s ease, border-color 0.15s ease;
}

.td-widget-file-link:hover {
  background-color: var(--color-td-bg-hover, rgba(55, 53, 47, 0.12));
  text-decoration: none;
}

.td-widget-file-icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  color: var(--color-td-neutral-fg-muted, #787774);
}

.td-widget-file-name {
  word-break: break-all;
}
</style>
