<script setup lang="ts">
import {
  ChevronLeft, ChevronRight,
} from '@lucide/vue';
import {
  usePreviousNext,
} from '../composables/usePreviousNext';

const {
  previous, next, groupName,
} = usePreviousNext();
</script>

<template>
  <nav
    v-if="previous || next"
    class="td-prevnext"
    aria-label="Page navigation"
  >
    <a
      v-if="previous"
      :href="previous.url"
      class="td-prevnext-link is-previous"
    >
      <span class="td-prevnext-label">
        <ChevronLeft
          :size="14"
          class="td-prevnext-chevron"
        />
        {{ groupName ? `Previous in ${groupName}` : 'Previous' }}
      </span>
      <span class="td-prevnext-title">{{ previous.title }}</span>
    </a>
    <span
      v-else
      class="td-prevnext-spacer"
    />
    <a
      v-if="next"
      :href="next.url"
      class="td-prevnext-link is-next"
    >
      <span class="td-prevnext-label">
        {{ groupName ? `Next in ${groupName}` : 'Next' }}
        <ChevronRight
          :size="14"
          class="td-prevnext-chevron"
        />
      </span>
      <span class="td-prevnext-title">{{ next.title }}</span>
    </a>
    <span
      v-else
      class="td-prevnext-spacer"
    />
  </nav>
</template>

<style scoped>
@reference "tailwindcss";

.td-prevnext {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-top: 64px;
  padding-top: 28px;
  border-top: 1px solid var(--color-td-line);
}

.td-prevnext-link {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 14px 16px;
  border: 1px solid var(--color-td-line);
  border-radius: theme(--radius-td-lg);
  text-decoration: none;
  transition:
    border-color var(--duration-td-move) var(--ease-td-out-quart),
    background var(--duration-td-move) var(--ease-td-out-quart),
    transform var(--duration-td-move) var(--ease-td-out-quart);
}

.td-prevnext-link:hover {
  border-color: var(--color-td-accent);
  background: var(--color-td-sel);
  transform: translateY(-1px);
}

.td-prevnext-link.is-next {
  text-align: right;
}

.td-prevnext-spacer {
  /* Empty cell keeps grid columns even */
}

.td-prevnext-label {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-ink-4);
}

.td-prevnext-link.is-next .td-prevnext-label {
  justify-content: flex-end;
}

.td-prevnext-chevron {
  flex-shrink: 0;
}

.td-prevnext-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-td-accent);
}

/* Mobile: stacked full-width */
@media (width < 56.25rem) {
  .td-prevnext {
    grid-template-columns: 1fr;
  }

  .td-prevnext-link.is-next {
    text-align: left;
  }

  .td-prevnext-link.is-next .td-prevnext-label {
    justify-content: flex-start;
  }
}
</style>
