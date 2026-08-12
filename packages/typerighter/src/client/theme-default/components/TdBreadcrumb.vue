<script setup lang="ts">
import {
  computed,
} from 'vue';
import {
  useRoute,
} from '../../app';
import {
  unslugify,
} from '@/shared';

const route = useRoute();

const crumbs = computed(() => {
  const routePath = route.path === '/' ? '/' : route.path.replace(/\/$/, '');
  const result = [
    {
      name: 'Home',
      href: '/',
    },
  ];

  if (routePath === '/') return result;

  const parts = routePath.replace(/^\//, '').split('/');

  for (let index = 0; index < parts.length; index++) {
    result.push({
      name: unslugify(parts[index]),
      href: '/' + parts.slice(0, index + 1).join('/'),
    });
  }

  return result;
});
</script>

<template>
  <nav
    class="td-breadcrumb"
    aria-label="Breadcrumb"
  >
    <template
      v-for="(crumb, index) in crumbs"
      :key="crumb.href"
    >
      <span
        v-if="index > 0"
        class="td-breadcrumb-sep"
        aria-hidden="true"
      >/</span>
      <a
        v-if="index < crumbs.length - 1"
        :href="crumb.href"
        class="td-breadcrumb-link"
      >{{ crumb.name }}</a>
      <span
        v-else
        class="td-breadcrumb-current"
      >{{ crumb.name }}</span>
    </template>
  </nav>
</template>

<style scoped>
.td-breadcrumb {
  display: flex;
  align-items: center;
  gap: 2px;
  font-size: var(--font-size-td-body-sm);
  color: var(--color-td-neutral-border-strong);
  margin-bottom: 8px;
  min-width: 0;
}

.td-breadcrumb-sep {
  margin: 0 2px;
}

.td-breadcrumb-link,
.td-breadcrumb-current {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.td-breadcrumb-link {
  color: var(--color-td-neutral-border-strong);
  text-decoration: none;
  transition: color 0.15s;
}

.td-breadcrumb-link:hover {
  color: var(--color-td-primary-solid);
}

.td-breadcrumb-current {
  color: var(--color-td-neutral-fg);
}
</style>
