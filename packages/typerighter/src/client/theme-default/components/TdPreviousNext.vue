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
    <div class="td-prevnext-row">
      <a
        v-if="previous"
        :href="previous.url"
        class="td-prevnext-link is-previous"
      >
        <span class="td-prevnext-label">
          <ChevronLeft
            :size="16"
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
            :size="16"
            class="td-prevnext-chevron"
          />
        </span>
        <span class="td-prevnext-title">{{ next.title }}</span>
      </a>
      <span
        v-else
        class="td-prevnext-spacer"
      />
    </div>
  </nav>
</template>

<style scoped>
.td-prevnext {
  margin-top: 38px;
  border-top: 1px solid var(--color-td-neutral-border-subtle);
  padding-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.td-prevnext-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 28px;
}

.td-prevnext-link {
  display: flex;
  flex-direction: column;
  gap: 4px;
  text-decoration: none;
  max-width: 46%;
}

.td-prevnext-link:hover {
  text-decoration: none;
}

.td-prevnext-link.is-next {
  text-align: right;
  margin-left: auto;
}

.td-prevnext-spacer {
  flex: 1;
}

.td-prevnext-label {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  font-weight: 500;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-neutral-fg-muted);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.td-prevnext-link.is-next .td-prevnext-label {
  justify-content: flex-end;
}

.td-prevnext-chevron {
  flex-shrink: 0;
}

.td-prevnext-title {
  font-size: var(--font-size-td-base);
  font-weight: 800;
  color: var(--color-td-primary-solid);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-prevnext-link:hover .td-prevnext-title {
  color: var(--color-td-primary-solid-hover);
  text-decoration: underline;
}

/* Mobile: stacked full-width cards */
@media (width < 56.25rem) {
  .td-prevnext-row {
    flex-direction: column;
    gap: 13px;
  }

  .td-prevnext-link {
    max-width: 100%;
    width: 100%;
    padding: 13px 14px;
    border: 1px solid var(--color-td-neutral-border-subtle);
    border-radius: 6px;
    background: var(--color-td-neutral-bg);
  }

  .td-prevnext-link.is-next {
    text-align: left;
  }

  .td-prevnext-link.is-next .td-prevnext-label {
    justify-content: flex-start;
  }
}
</style>
