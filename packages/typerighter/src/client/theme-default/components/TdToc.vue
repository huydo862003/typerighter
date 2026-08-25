<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  useActiveTocHeading,
} from '../composables/useActiveTocHeading';
import type {
  MarkdownHeading,
} from '@/shared';

const {
  headings,
  static: isStatic = false,
} = defineProps<{
  /** Heading entries for the current page */
  headings: MarkdownHeading[];
  /** Disable active heading tracking on scroll */
  static?: boolean;
}>();

const activeTocId = isStatic
  ? computed(() => undefined)
  : useActiveTocHeading(() => headings);
</script>

<template>
  <div
    v-if="headings.length"
    class="td-toc"
  >
    <div class="td-toc-label">
      On this page
    </div>
    <ul class="td-toc-list">
      <li
        v-for="heading in headings"
        :key="heading.slug"
        :class="{
          'td-toc-indent-1': heading.level === 3,
          'td-toc-indent-2': heading.level === 4,
          'td-toc-indent-3': heading.level === 5,
        }"
      >
        <a
          :href="heading.link"
          class="td-toc-link"
          :class="{
            'is-active': activeTocId === heading.slug,
          }"
        >{{ heading.title }}</a>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.td-toc-label {
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  font-weight: 500;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-neutral-fg-muted);
  margin-bottom: 9px;
}

.td-toc-list {
  list-style: none;
  padding: 0;
  margin: 0;
  border-left: 2px solid var(--color-td-neutral-border-subtle);
}

.td-toc-list li {
  margin: 0;
}

.td-toc-link {
  display: block;
  padding: 5px 0 5px 12px;
  margin-left: -2px;
  font-size: var(--font-size-td-xs);
  color: var(--color-td-neutral-fg);
  text-decoration: none;
  border-left: 2px solid transparent;
  transition: color 0.15s;
}

.td-toc-link:hover {
  color: var(--color-td-primary-solid);
}

.td-toc-link.is-active {
  color: var(--color-td-primary-solid);
  border-left-color: var(--color-td-primary-solid);
  font-weight: 600;
}

.td-toc-indent-1 {
  padding-left: 24px;
}

.td-toc-indent-2 {
  padding-left: 36px;
}

.td-toc-indent-3 {
  padding-left: 48px;
}
</style>
