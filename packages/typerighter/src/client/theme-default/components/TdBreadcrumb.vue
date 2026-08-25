<script setup lang="ts">
import {
  computed, ref, watch,
} from 'vue';
import {
  Ellipsis,
} from '@lucide/vue';
import {
  useRoute, useSiteConfig,
} from '../../app';
import TdDropdown from './TdDropdown.vue';
import {
  getIndexUrl, isIndexUrl, stripTrailingSlash, unslugify,
} from '@/shared';

const MAX_VISIBLE = 3;

const route = useRoute();
const {
  withBase,
} = useSiteConfig();
const ellipsisOpen = ref(false);

interface Crumb {
  name: string;
  href: string;
}

const crumbs = computed((): Crumb[] => {
  const routePath = stripTrailingSlash(route.path);
  const result: Crumb[] = [
    {
      name: 'Home',
      href: withBase(getIndexUrl('/')),
    },
  ];

  if (routePath === '/') return result;

  const parts = routePath.replace(/^\//, '').split('/');

  const visibleParts = isIndexUrl(routePath)
    ? parts.slice(0, -1)
    : parts;

  for (let index = 0; index < visibleParts.length; index++) {
    const basePath = '/' + visibleParts.slice(0, index + 1).join('/');
    const isLast = index === visibleParts.length - 1;

    result.push({
      name: unslugify(visibleParts[index]),
      href: withBase(isLast ? basePath : getIndexUrl(basePath)),
    });
  }

  return result;
});

const needsCollapse = computed(() => MAX_VISIBLE < crumbs.value.length);
const collapsedCrumbs = computed(() => crumbs.value.slice(1, -2));

watch(() => route.path, () => {
  ellipsisOpen.value = false;
});
</script>

<template>
  <nav
    class="td-breadcrumb"
    aria-label="Breadcrumb"
  >
    <a
      :href="crumbs[0].href"
      class="td-breadcrumb-link td-breadcrumb-home"
    >{{ crumbs[0].name }}</a>

    <!-- When collapsed: ellipsis dropdown for middle crumbs -->
    <template v-if="needsCollapse">
      <span
        class="td-breadcrumb-sep"
        aria-hidden="true"
      >/</span>
      <TdDropdown v-model:open="ellipsisOpen">
        <template #trigger>
          <button
            type="button"
            class="td-breadcrumb-ellipsis"
          >
            <Ellipsis :size="14" />
          </button>
        </template>
        <a
          v-for="crumb in collapsedCrumbs"
          :key="crumb.href"
          :href="crumb.href"
          class="td-breadcrumb-dropdown-item"
        >{{ crumb.name }}</a>
      </TdDropdown>

      <!-- Last two crumbs -->
      <template
        v-for="crumb in crumbs.slice(-2)"
        :key="crumb.href"
      >
        <span
          class="td-breadcrumb-sep"
          aria-hidden="true"
        >/</span>
        <a
          v-if="crumb !== crumbs[crumbs.length - 1]"
          :href="crumb.href"
          class="td-breadcrumb-link"
        >{{ crumb.name }}</a>
        <span
          v-else
          class="td-breadcrumb-current"
        >{{ crumb.name }}</span>
      </template>
    </template>

    <!-- When not collapsed: all crumbs normally -->
    <template v-else>
      <template
        v-for="(crumb, index) in crumbs.slice(1)"
        :key="crumb.href"
      >
        <span
          class="td-breadcrumb-sep"
          aria-hidden="true"
        >/</span>
        <a
          v-if="index < crumbs.length - 2"
          :href="crumb.href"
          class="td-breadcrumb-link"
        >{{ crumb.name }}</a>
        <span
          v-else
          class="td-breadcrumb-current"
        >{{ crumb.name }}</span>
      </template>
    </template>
  </nav>
</template>

<style scoped>
.td-breadcrumb {
  display: flex;
  align-items: center;
  gap: 2px;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-neutral-border-strong);
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

.td-breadcrumb-home {
  flex-shrink: 0;
}

.td-breadcrumb-link:hover {
  color: var(--color-td-primary-solid);
}

.td-breadcrumb-current {
  color: var(--color-td-neutral-fg);
}

.td-breadcrumb-ellipsis {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  vertical-align: middle;
  background: none;
  border: none;
  cursor: pointer;
  color: var(--color-td-neutral-fg-muted);
  padding: 2px 4px;
  border-radius: 4px;
  transition: color 0.15s, background-color 0.15s;
}

.td-breadcrumb-ellipsis:hover {
  color: var(--color-td-fg);
  background: var(--color-td-neutral-bg-hover);
}

.td-breadcrumb-dropdown-item {
  display: block;
  padding: 6px 10px;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-neutral-fg);
  text-decoration: none;
  border-radius: 4px;
  transition: background-color 0.1s;
}

.td-breadcrumb-dropdown-item:hover {
  background: var(--color-td-neutral-bg-hover);
  color: var(--color-td-primary-solid);
}
</style>
