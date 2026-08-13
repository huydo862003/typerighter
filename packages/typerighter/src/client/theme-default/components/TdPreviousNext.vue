<script setup lang="ts">
import {
  ChevronLeft, ChevronRight,
} from '@lucide/vue';
import {
  usePreviousNext,
} from '../composables/usePreviousNext';

const {
  previous, next,
} = usePreviousNext();
</script>

<template>
  <nav
    v-if="previous || next"
    class="td-previous-next"
    aria-label="Page navigation"
  >
    <a
      v-if="previous"
      :href="previous.url"
      class="td-previous-next-link is-previous"
    >
      <ChevronLeft
        :size="16"
        class="td-previous-next-icon"
      />
      <span class="td-previous-next-content">
        <span class="td-previous-next-label">Previous</span>
        <span class="td-previous-next-title">{{ previous.title }}</span>
      </span>
    </a>
    <span
      v-else
      class="td-previous-next-spacer"
    />
    <a
      v-if="next"
      :href="next.url"
      class="td-previous-next-link is-next"
    >
      <span class="td-previous-next-content">
        <span class="td-previous-next-label">Next</span>
        <span class="td-previous-next-title">{{ next.title }}</span>
      </span>
      <ChevronRight
        :size="16"
        class="td-previous-next-icon"
      />
    </a>
    <span
      v-else
      class="td-previous-next-spacer"
    />
  </nav>
</template>

<style scoped>
.td-previous-next {
  display: flex;
  gap: 16px;
  margin-top: 48px;
  padding-top: 24px;
  border-top: 1px solid var(--color-td-neutral-border-subtle);
}

.td-previous-next-link {
  display: flex;
  align-items: start;
  gap: 8px;
  flex: 1;
  min-width: 0;
  padding: 12px 16px;
  border: 1px solid var(--color-td-neutral-border-subtle);
  border-radius: 8px;
  text-decoration: none;
  color: var(--color-td-fg);
  transition: border-color 0.15s;
}

.td-previous-next-link:hover {
  border-color: var(--color-td-primary-solid);
}

.td-previous-next-link.is-next {
  justify-content: flex-end;
  text-align: right;
}

.td-previous-next-spacer {
  flex: 1;
}

.td-previous-next-icon {
  flex-shrink: 0;
  color: var(--color-td-neutral-fg-muted);
}

.td-previous-next-link:hover .td-previous-next-icon {
  color: var(--color-td-primary-solid);
}

.td-previous-next-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.td-previous-next-label {
  font-size: var(--font-size-td-label);
  letter-spacing: var(--tracking-td-label);
  text-transform: uppercase;
  color: var(--color-td-neutral-fg-muted);
}

.td-previous-next-title {
  font-size: var(--font-size-td-nav);
  font-weight: var(--font-weight-td-semibold);
  color: var(--color-td-primary-solid);
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
}
</style>
