<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  useActiveTocHeading,
} from '../composables/useActiveTocHeading';
import TdToc from './custom/TdToc.vue';
import {
  useTdContent,
} from '@/client/app';

const {
  page,
} = useTdContent();

const flatHeadings = computed(() => {
  const result = [];

  for (const heading of page.value.headings) {
    result.push(heading);
    for (const child of heading.children) {
      result.push(child);
    }
  }

  return result;
});

const activeTocId = useActiveTocHeading(() => flatHeadings.value);
</script>

<template>
  <aside
    v-if="page.headings.length > 1"
    class="td-rail"
  >
    <div class="td-rail-inner">
      <!-- collapsed: centered dash indicators -->
      <div
        class="td-rail-indicators"
        aria-hidden="true"
      >
        <div
          v-for="heading in flatHeadings"
          :key="heading.slug"
          class="td-rail-indicator"
          :class="{
            'is-h3': heading.level === 3,
            'is-active': activeTocId === heading.slug,
          }"
        />
      </div>

      <!-- expanded: floating TOC card, vertically centered, Notion-style borders -->
      <div class="td-rail-panel">
        <TdToc
          :headings="page.headings"
        />
      </div>
    </div>
  </aside>
</template>

<style scoped>
.td-rail {
  position: sticky;
  top: var(--td-header-height);
  height: calc(100vh - var(--td-header-height));
  overflow: visible;
  z-index: 0;
}

.td-rail-inner {
  position: relative;
  height: 100%;
}

/* Indicators - vertically centered */

.td-rail-indicators {
  position: absolute;
  right: 12px;
  top: 48px;
  bottom: 0;
  display: flex;
  flex-direction: column;
  gap: 7px;
  opacity: 1;
  overflow: hidden;
  transition: opacity var(--duration-td-fast) var(--ease-td-out-quart);
  pointer-events: none;
}

.td-rail:hover .td-rail-indicators {
  opacity: 0;
}

.td-rail-indicator {
  height: 2px;
  width: 20px;
  border-radius: 1px;
  background: var(--color-td-line);
  transition: background var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-rail-indicator.is-h3 {
  width: 13px;
  margin-left: 7px;
}

.td-rail-indicator.is-active {
  background: var(--color-td-accent);
}

/* Floating TOC panel - top-anchored, borders on top/bottom/left only */

.td-rail-panel {
  position: absolute;
  right: 0;
  top: 32px;
  transform: translateX(6px);
  width: 216px;
  min-height: unset;
  max-height: calc(100vh - var(--td-header-height) - 120px);
  overflow-y: auto;
  overscroll-behavior: contain;
  display: flex;
  flex-direction: column;
  background: var(--color-td-bg);
  border-top: 1px solid var(--color-td-line-soft);
  border-bottom: 1px solid var(--color-td-line-soft);
  border-left: 1px solid var(--color-td-line-soft);
  border-radius: 8px 0 0 8px;
  box-shadow: -4px 4px 20px rgba(0, 0, 0, 0.07);
  padding: 16px 16px 16px 16px;
  opacity: 0;
  pointer-events: none;
  transition:
    opacity var(--duration-td-fast) var(--ease-td-out-quart),
    transform var(--duration-td-fast) var(--ease-td-out-quart);
}

@media (hover: hover) and (pointer: fine) {
  .td-rail:hover .td-rail-panel {
    opacity: 1;
    pointer-events: auto;
    transform: translateX(0);
  }
}
</style>
