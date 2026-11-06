<script setup lang="ts">
import {
  computed,
  nextTick,
  useTemplateRef,
  watch,
} from 'vue';
import {
  useActiveTocHeading,
} from '../../composables/useActiveTocHeading';
import {
  sanitizeInlineHtml,
  type MarkdownHeading,
} from '@/shared';

const {
  headings,
  static: isStatic = false,
  inline = false,
  scrollable = false,
} = defineProps<{
  /** Heading entries for the current page */
  headings: MarkdownHeading[];
  /** Disable active heading tracking */
  static?: boolean;
  /** Render as inline hierarchical list instead of sidebar list */
  inline?: boolean;
  /** Enable scroll-to-active behavior within a constrained container */
  scrollable?: boolean;
}>();

const flatHeadings = computed(() => {
  const result: MarkdownHeading[] = [];

  for (const heading of headings) {
    result.push(heading);
    for (const child of heading.children) {
      result.push(child);
    }
  }

  return result;
});

const activeTocId = (isStatic || inline)
  ? computed(() => undefined)
  : useActiveTocHeading(() => flatHeadings.value);

const listRef = useTemplateRef<HTMLElement>('list');

// Scroll the TOC list so the active heading is centered
// scroll-behavior is controlled by CSS: instant when hidden, smooth when hovered
watch(activeTocId, () => {
  nextTick(() => {
    listRef.value?.querySelector('a.is-active')?.scrollIntoView({
      block: 'center',
    });
  });
});
</script>

<template>
  <!-- Inline: label above hierarchical list -->
  <div
    v-if="inline"
    class="td-toc-inline-wrap"
  >
    <div class="td-toc-inline-label">
      On this page
    </div>
    <ul class="td-toc-inline-list">
      <li
        v-for="heading in headings"
        :key="heading.slug"
        class="td-toc-inline-item"
      >
        <a
          :href="heading.link"
          class="td-toc-inline-link is-h2"
          v-html="sanitizeInlineHtml(heading.titleHtml)"
        />
        <ul
          v-if="heading.children.length"
          class="td-toc-inline-children"
        >
          <li
            v-for="child in heading.children"
            :key="child.slug"
          >
            <a
              :href="child.link"
              class="td-toc-inline-link is-h3"
              v-html="sanitizeInlineHtml(child.titleHtml)"
            />
          </li>
        </ul>
      </li>
    </ul>
  </div>

  <!-- Standard: left-bordered list (rail/sidebar) -->
  <div
    v-else-if="headings.length"
    class="td-toc"
    data-testid="toc"
  >
    <div
      ref="list"
      class="td-toc-list-wrap"
      :class="{
        'is-scrollable': scrollable,
      }"
    >
      <ul class="td-toc-list">
        <li
          v-for="heading in flatHeadings"
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
            v-html="sanitizeInlineHtml(heading.titleHtml)"
          />
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
/* Inline hierarchical list */

.td-toc-inline-wrap {
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--color-td-surface);
  border: 1px solid var(--color-td-line-soft);
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.td-toc-inline-label {
  font-size: var(--font-size-td-2xs);
  font-weight: 600;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-ink-4);
}

.td-toc-inline-list {
  list-style: decimal;
  margin: 0;
  padding: 0 0 0 18px;
}

.td-toc-inline-list > li {
  margin-top: 5px;
}

.td-toc-inline-list > li:first-child {
  margin-top: 0;
}

.td-toc-inline-list::marker,
.td-toc-inline-list > li::marker {
  font-size: var(--font-size-td-xs);
  font-weight: 400;
  color: var(--color-td-ink-4);
}

.td-toc-inline-children {
  list-style: disc;
  margin: 0;
  padding: 0 0 0 16px;
}

.td-toc-inline-item {
  margin: 0;
}

.td-toc-inline-link {
  display: block;
  padding: 0;
  text-decoration: none;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-toc-inline-link.is-h2 {
  font-size: var(--font-size-td-xs);
  font-weight: 400;
  color: var(--color-td-ink-2);
}

.td-toc-inline-link.is-h3 {
  font-size: var(--font-size-td-xs);
  color: var(--color-td-ink-3);
}

.td-toc-inline-link:hover {
  color: var(--color-td-accent);
}

/* Standard list layout */

.td-toc-list-wrap.is-scrollable {
  max-height: calc(100vh - var(--td-header-height) - 120px);
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-width: thin;
}

.td-toc-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.td-toc-list li {
  margin: 0;
}

.td-toc-link {
  display: block;
  padding: 4px 0 4px 10px;
  font-size: 13px;
  color: var(--color-td-ink-3);
  text-decoration: none;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  border-left: 2px solid transparent;
  transition:
    color var(--duration-td-fast) var(--ease-td-out-quart),
    border-left-color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-toc-link:hover {
  color: var(--color-td-ink);
  border-left-color: var(--color-td-ink-4);
}

.td-toc-link.is-active {
  color: var(--color-td-accent);
  border-left-color: var(--color-td-accent);
  font-weight: 500;
}

.td-toc-indent-1 {
  padding-left: 12px;
}

.td-toc-indent-2 {
  padding-left: 20px;
}

.td-toc-indent-3 {
  padding-left: 28px;
}
</style>
