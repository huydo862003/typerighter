<script setup lang="ts">
import {
  computed, ref, watch,
} from 'vue';
import {
  Ellipsis,
} from '@lucide/vue';
import {
  useRoute, useSiteConfig,
  useSiteData,
} from '../../app';
import TdDropdown from './TdDropdown.vue';
import {
  findCrumbLabel, getIndexUrl, isIndexUrl, stripTrailingSlash,
} from '@/shared';

const MAX_VISIBLE = 3;

const route = useRoute();
const {
  withBase,
} = useSiteConfig();
const siteData = useSiteData();
const ellipsisOpen = ref(false);

interface Crumb {
  name: string;
  href: string;
}

const crumbs = computed((): Crumb[] => {
  const routePath = stripTrailingSlash(route.path);

  if (routePath === '/') return [];

  const parts = routePath.replace(/^\//, '').split('/');

  const visibleParts = isIndexUrl(routePath)
    ? parts.slice(0, -1)
    : parts;

  const tree = siteData.value.fullContentTree ?? siteData.value.contentTree;

  return visibleParts.map((part, index) => {
    const basePath = '/' + visibleParts.slice(0, index + 1).join('/');
    const isLast = index === visibleParts.length - 1;

    return {
      name: findCrumbLabel(tree, visibleParts, index),
      href: withBase(isLast ? basePath : getIndexUrl(basePath)),
    };
  });
});

const needsCollapse = computed(() => MAX_VISIBLE < crumbs.value.length);
const collapsedCrumbs = computed(() => crumbs.value.slice(0, -2));

watch(() => route.path, () => {
  ellipsisOpen.value = false;
});
</script>

<template>
  <nav
    v-if="crumbs.length > 0"
    class="td-breadcrumb"
    aria-label="Breadcrumb"
  >
    <!-- Leading "/" acts as the home separator (brand already shows "Home") -->
    <span
      class="td-breadcrumb-sep"
      aria-hidden="true"
    >/</span>

    <!-- When collapsed: ellipsis dropdown for middle crumbs, then last two -->
    <template v-if="needsCollapse">
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

    <!-- When not collapsed: all crumbs with separators between them -->
    <template v-else>
      <template
        v-for="(crumb, index) in crumbs"
        :key="crumb.href"
      >
        <a
          v-if="index < crumbs.length - 1"
          :href="crumb.href"
          class="td-breadcrumb-link"
        >{{ crumb.name }}</a>
        <span
          v-else
          class="td-breadcrumb-current"
        >{{ crumb.name }}</span>
        <span
          v-if="index < crumbs.length - 1"
          class="td-breadcrumb-sep"
          aria-hidden="true"
        >/</span>
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
  color: var(--color-td-ink-4);
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
  max-width: 20ch;
}

.td-breadcrumb-link {
  color: var(--color-td-ink-4);
  text-decoration: none;
  transition: color var(--duration-td-fast) var(--ease-td-out-quart);
}

@media (width < 64rem) {
  .td-breadcrumb-link,
  .td-breadcrumb-current {
    max-width: 12ch;
  }
}

.td-breadcrumb-link:hover {
  color: var(--color-td-accent);
}

.td-breadcrumb-current {
  color: var(--color-td-ink-3);
}

.td-breadcrumb-ellipsis {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  vertical-align: middle;
  background: none;
  border: none;
  cursor: pointer;
  color: var(--color-td-ink-4);
  padding: 2px 4px;
  border-radius: 4px;
  transition: color var(--duration-td-fast) var(--ease-td-out-quart), background-color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-breadcrumb-ellipsis:hover {
  color: var(--color-td-ink);
  background: var(--color-td-hover);
}

.td-breadcrumb-dropdown-item {
  display: block;
  padding: 6px 10px;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-ink-3);
  text-decoration: none;
  border-radius: 4px;
  transition: background-color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-breadcrumb-dropdown-item:hover {
  background: var(--color-td-hover);
  color: var(--color-td-accent);
}
</style>
