<script setup lang="ts">
import {
  ArrowUpRight,
} from '@lucide/vue';
import {
  isUrlExternal,
} from '@/shared';
import {
  useSiteConfig, useRoute,
} from '@/client/app';

const siteConfig = useSiteConfig();
const {
  withBase,
} = siteConfig;
const route = useRoute();

function isActive (link: string): boolean {
  if (isUrlExternal(link)) return false;

  return route.path.startsWith(withBase(link));
}
</script>

<template>
  <nav
    v-if="siteConfig.nav?.length"
    class="td-header-nav"
  >
    <a
      v-for="item in siteConfig.nav"
      :key="item.link"
      :href="isUrlExternal(item.link) ? item.link : withBase(item.link)"
      :target="isUrlExternal(item.link) ? '_blank' : undefined"
      :rel="isUrlExternal(item.link) ? 'noopener noreferrer' : undefined"
      class="td-header-nav-link"
      :class="{
        'is-active': isActive(item.link),
      }"
    >{{ item.title }}<ArrowUpRight
      v-if="isUrlExternal(item.link)"
      :size="14"
    /></a>
  </nav>
</template>

<style scoped>
@reference "tailwindcss";

.td-header-nav {
  display: none;
  align-items: center;
  gap: 0.25rem;
  margin-left: auto;
  margin-right: 1rem;
}

@media (min-width: theme(--breakpoint-lg)) {
  .td-header-nav {
    display: flex;
  }
}

.td-header-nav-link {
  display: inline-flex;
  align-items: center;
  gap: 0.125rem;
  padding: 4px 10px;
  border-radius: var(--radius-td-sm);
  font-size: var(--font-size-td-sm);
  color: var(--color-td-ink-4);
  text-decoration: none;
  transition:
    color var(--duration-td-move) var(--ease-td-out-quart),
    background-color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-header-nav-link:hover {
  color: var(--color-td-accent);
  background-color: var(--color-td-hov);
}

.td-header-nav-link.is-active {
  color: var(--color-td-accent);
  background-color: var(--color-td-sel);
}
</style>
