<script setup lang="ts">
import {
  ExternalLink,
  LucideFileText,
} from '@lucide/vue';
import {
  extractRef,
} from './ref';
import {
  getPageIcon,
} from '@/client/theme-default/composables/pageIcon';
import {
  useSiteConfig,
} from '@/client/app';
import {
  isUrlExternal,
} from '@/shared';

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
const refIcon = resolved?.icon ? getPageIcon(resolved.icon.name) : undefined;
const isExternal = resolved && isUrlExternal(resolved.url);
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

    <!-- External link -->
    <a
      v-else-if="isExternal"
      :href="resolved.url"
      target="_blank"
      rel="noopener noreferrer"
      class="td-widget-ref td-widget-ref-external"
    >{{ resolved.name }}<ExternalLink
      :size="12"
      class="td-widget-external-icon"
    /></a>

    <!-- Normal relation link with icon -->
    <a
      v-else
      :href="withBase(resolved.url)"
      class="td-widget-ref"
    ><component
      :is="refIcon"
      v-if="refIcon"
      :size="14"
      class="td-widget-ref-icon"
    />{{ resolved.name }}</a>
  </span>
  <span v-else>{{ value }}</span>
</template>

<style scoped>
.td-widget-relation-wrapper {
  display: inline-flex;
  align-items: center;
}

.td-widget-ref {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--color-td-link);
  text-decoration: none;
}

.td-widget-ref:hover {
  color: var(--color-td-link-hover);
  text-decoration: underline;
}

.td-widget-ref-icon {
  flex-shrink: 0;
  opacity: 0.7;
}

.td-widget-external-icon {
  flex-shrink: 0;
  margin-left: 3px;
  opacity: 0.5;
}

.td-widget-image-preview {
  display: inline-block;
  width: 180px;
  height: 260px;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--color-td-neutral-border-subtle);
  box-shadow: 0 2px 6px color-mix(in srgb, var(--color-td-fg) 8%, transparent);
  transition: transform 0.15s, box-shadow 0.15s;
  vertical-align: middle;
  background-color: var(--color-td-neutral-bg-subtle);
}

.td-widget-image-preview:hover {
  transform: scale(1.02);
  box-shadow: 0 4px 12px color-mix(in srgb, var(--color-td-fg) 12%, transparent);
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
  color: var(--color-td-neutral-fg);
  text-decoration: none;
  font-size: var(--font-size-td-base);
  line-height: 1.4;
  padding: 4px 8px;
  border-radius: 4px;
  background-color: var(--color-td-neutral-bg-subtle);
  border: 1px solid var(--color-td-neutral-border-subtle);
  transition: background-color 0.15s, border-color 0.15s;
}

.td-widget-file-link:hover {
  background-color: var(--color-td-neutral-bg-hover);
  text-decoration: none;
}

.td-widget-file-icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  color: var(--color-td-neutral-fg-muted);
}

.td-widget-file-name {
  word-break: break-all;
}
</style>
