<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  LucideFileText,
} from '@lucide/vue';
import {
  extractRef,
  isImageRef,
} from './ref';
import {
  useSiteConfig,
} from '@/client/app';
import {
  formatDateString,
  type PropertyDescriptor,
} from '@/shared';

const {
  definition,
  value,
} = defineProps<{
  /** Property descriptor */
  definition: PropertyDescriptor;
  /** Raw value */
  value: unknown;
}>();

const {
  withBase,
} = useSiteConfig();

const stringValue = typeof value === 'string' ? value.trim() : '';
const refObject = extractRef(value);
const isImage = refObject
  ? refObject.isImage
  : (0 < stringValue.length && isImageRef({
    url: stringValue,
  }));

const rawUrl = refObject ? refObject.url : stringValue;
const targetUrl = withBase(rawUrl);

const isFileDownload = computed(() => {
  if (!stringValue || !stringValue.includes('.') || stringValue.includes(' ')) return false;

  return stringValue.endsWith('.pdf')
    || stringValue.endsWith('.zip')
    || stringValue.endsWith('.doc')
    || stringValue.endsWith('.docx');
});

function format (value_: unknown): string {
  if (value_ === undefined || value_ === null) return '';
  if (definition.widget === 'date') return formatDateString(String(value_));

  return String(value_);
}
</script>

<template>
  <!-- Image preview: fixed size, clickable opening in a new tab -->
  <a
    v-if="isImage"
    :href="targetUrl"
    target="_blank"
    rel="noopener noreferrer"
    class="td-widget-image-preview"
    :title="refObject ? refObject.name : stringValue"
  >
    <img
      :src="targetUrl"
      :alt="refObject ? refObject.name : stringValue"
      class="td-widget-image-img"
      loading="lazy"
    >
  </a>

  <!-- Non-image file link if string contains file extension -->
  <a
    v-else-if="isFileDownload"
    :href="targetUrl"
    target="_blank"
    rel="noopener noreferrer"
    class="td-widget-file-link"
  >
    <LucideFileText class="td-widget-file-icon" />
    <span class="td-widget-file-name">{{ stringValue.split('/').pop() ?? stringValue }}</span>
  </a>

  <!-- Normal formatted text -->
  <template v-else>
    {{ format(value) }}
  </template>
</template>

<style scoped>
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
