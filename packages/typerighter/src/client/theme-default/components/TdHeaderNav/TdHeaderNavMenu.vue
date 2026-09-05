<script setup lang="ts">
import {
  ArrowUpRight,
} from '@lucide/vue';
import TdLucideIcon from '../TdLucideIcon.vue';
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
    class="td-menu-nav"
  >
    <a
      v-for="item in siteConfig.nav"
      :key="item.link"
      :href="isUrlExternal(item.link) ? item.link : withBase(item.link)"
      :target="isUrlExternal(item.link) ? '_blank' : undefined"
      :rel="isUrlExternal(item.link) ? 'noopener noreferrer' : undefined"
      class="td-menu-nav-item"
      :class="{
        'is-active': isActive(item.link),
      }"
    >
      <TdLucideIcon
        v-if="item.icon"
        :name="item.icon"
        :size="16"
      />
      <span>{{ item.title }}</span>
      <ArrowUpRight
        v-if="isUrlExternal(item.link)"
        :size="14"
      />
    </a>
  </nav>
</template>

<style scoped>
.td-menu-nav {
  display: flex;
  flex-direction: column;
  padding: 0.5rem 0;
}

.td-menu-nav::after {
  content: "";
  display: block;
  height: 1px;
  margin: 0.5rem 1rem 0;
  background: color-mix(in srgb, var(--color-td-neutral-border) 50%, transparent);
}

.td-menu-nav-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 1rem;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-neutral-fg);
  text-decoration: none;
  border-left: 3px solid transparent;
  transition: background-color 0.15s;
}

.td-menu-nav-item:hover {
  background-color: var(--color-td-neutral-bg-hover);
}

.td-menu-nav-item.is-active {
  background-color: var(--color-td-primary-bg-hover);
  border-left-color: var(--color-td-primary-solid);
  color: var(--color-td-primary-solid);
  font-weight: 600;
}
</style>
