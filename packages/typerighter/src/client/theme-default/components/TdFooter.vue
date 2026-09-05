<script setup lang="ts">
import {
  useSiteConfig,
} from '../../app';

const config = useSiteConfig();
const year = new Date().getFullYear();
</script>

<template>
  <footer class="td-footer">
    <span>
      Powered by
      <a
        href="https://github.com/huydo862003/typerighter"
        target="_blank"
        rel="noopener noreferrer"
        class="td-footer-link"
      >Typerighter</a>
    </span>
    <span v-if="config.author || config.license">
      <template v-if="config.license">{{ config.license }}</template>
      <template v-if="config.license && config.author"> · </template>
      <template v-if="config.author">&copy; {{ year }} {{ config.author }}</template>
    </span>
  </footer>
</template>

<style scoped>
.td-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.25rem;
  flex-wrap: wrap;
  padding: 1.5rem 24px;
  border-top: 1px solid var(--color-td-neutral-border);
  background: var(--color-td-neutral-bg);
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  color: var(--color-td-neutral-fg-muted);
}

.td-footer-link {
  font-weight: 500;
  color: var(--color-td-primary-solid);
}

.td-footer-link:hover {
  text-decoration: underline;
}
</style>
