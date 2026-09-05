<script setup lang="ts">
import {
  computed, nextTick, ref, useTemplateRef, watch, watchEffect,
} from 'vue';
import {
  X,
} from '@lucide/vue';
import {
  useTdContent, useSiteConfig, useSiteData, useRoute,
} from '../app';
import {
  Content,
} from '../app/components/Content';
import TdBrandIcon from './components/TdBrandIcon.vue';
import TdButton from './components/TdButton.vue';
import TdContentNav from './components/TdContentNav.vue';
import TdFooter from './components/TdFooter.vue';
import TdBreadcrumb from './components/TdBreadcrumb.vue';
import {
  TdHeaderNavBar, TdHeaderNavMenu,
} from './components/TdHeaderNav';
import {
  TdFrontmatter,
} from './components/TdFrontmatter';
import TdMenuButton from './components/TdMenuButton.vue';
import TdPreviousNext from './components/TdPreviousNext.vue';
import TdRail from './components/TdRail.vue';
import TdToc from './components/TdToc.vue';
import TdSiteSearch from './components/TdSiteSearch/TdSiteSearch.vue';
import TdThemeToggle from './components/TdThemeToggle.vue';
import {
  useCopyCode,
} from './composables/useCopyCode';
import {
  useGlobalHotkey,
} from './composables/useHotkey';
import {
  TdKeyName,
} from './utils/keys';
import {
  useResizableTable,
} from './composables/useResizableTable';
import {
  useMenu,
} from './composables/useMenu';
import {
  renderInlineMarkup,
} from './utils/renderInlineMarkup';
import {
  getPageIcon,
} from './utils/pageIcon';
import {
  formatEditTime, getIndexUrl,
} from '@/shared';
import './styles/main.css';
import './styles/markdown/content.css';
import './styles/markdown/code.css';
import './styles/markdown/containers.css';

const {
  title, page,
} = useTdContent();
const pageIcon = computed(() => {
  const icon = page.value.icon;

  return icon ? getPageIcon(icon.name) : undefined;
});
const siteConfig = useSiteConfig();
const {
  withBase,
} = siteConfig;
const siteData = useSiteData();
const {
  isOpen, open: openMenu, close: closeMenu,
} = useMenu();

const searchQuery = ref('');
const sidebarSearchActive = ref(false);
const menuSearchActive = ref(false);
const route = useRoute();

watch(
  () => route.path,
  () => closeMenu(),
);

useCopyCode();

const sidebarSearch =
  useTemplateRef<InstanceType<typeof TdSiteSearch>>('sidebarSearch');
const menuSearch = useTemplateRef<InstanceType<typeof TdSiteSearch>>('menuSearch');

function focusSearch () {
  // Toggle: if search is already focused, blur and close
  if (document.activeElement?.closest('.td-search-input-wrap')) {
    (document.activeElement as HTMLElement).blur();
    if (isOpen.value) closeMenu();
    return;
  }

  if (isOpen.value) {
    menuSearch.value?.focus();
  } else if (sidebarSearch.value?.isVisible) {
    sidebarSearch.value.focus();
  } else {
    openMenu();
    nextTick(() => menuSearch.value?.focus());
  }
}

useGlobalHotkey([
  TdKeyName.Control,
  TdKeyName.k,
], focusSearch);
useGlobalHotkey([
  TdKeyName.Meta,
  TdKeyName.k,
], focusSearch);

// Update document title and meta description per page
watchEffect(() => {
  if (typeof document === 'undefined') return;
  const pageTitle = title.value;
  const siteName = siteConfig.title;

  document.title =
    pageTitle && pageTitle !== siteName
      ? `${pageTitle} - ${siteName}`
      : siteName;

  const description =
    page.value.frontmatter?.description !== undefined
      ? String(page.value.frontmatter.description)
      : (siteConfig.description ?? '');
  const metaDescription = document.querySelector('meta[name="description"]');

  if (metaDescription) {
    metaDescription.setAttribute('content', description);
  }
});
useResizableTable();

const SIDEBAR_MIN = 200;
const SIDEBAR_MAX = 500;
const SIDEBAR_DEFAULT = 272;
const SIDEBAR_STORAGE_KEY = 'td-sidebar-width';

const sidebarWidth = ref(
  typeof localStorage !== 'undefined'
    ? Number(localStorage.getItem(SIDEBAR_STORAGE_KEY)) || SIDEBAR_DEFAULT
    : SIDEBAR_DEFAULT,
);

function onResizeStart (event: PointerEvent) {
  const startX = event.clientX;
  const startWidth = sidebarWidth.value;
  const target = event.currentTarget as HTMLElement;

  target.setPointerCapture(event.pointerId);

  function onMove (event: PointerEvent) {
    const width = Math.min(
      SIDEBAR_MAX,
      Math.max(SIDEBAR_MIN, startWidth + event.clientX - startX),
    );

    sidebarWidth.value = width;
  }

  function onUp () {
    document.body.style.cursor = '';
    target.removeEventListener('pointermove', onMove);
    target.removeEventListener('pointerup', onUp);
    localStorage.setItem(SIDEBAR_STORAGE_KEY, String(sidebarWidth.value));
  }

  document.body.style.cursor = 'col-resize';
  target.addEventListener('pointermove', onMove);
  target.addEventListener('pointerup', onUp);
}
</script>

<template>
  <div>
    <header class="td-header">
      <div class="td-header-left">
        <TdMenuButton />
        <a
          :href="withBase(getIndexUrl('/'))"
          class="td-brand"
        >
          <TdBrandIcon />
          <span class="td-brand-name">{{
            siteConfig.title || "Typedown"
          }}</span>
        </a>
        <TdBreadcrumb class="td-header-breadcrumb" />
      </div>
      <TdHeaderNavBar />
      <div class="td-header-right">
        <TdThemeToggle />
        <a
          v-if="siteConfig.repo"
          :href="siteConfig.repo"
          target="_blank"
          rel="noopener noreferrer"
          class="td-header-icon-link"
          aria-label="GitHub repository"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 98 96"
            xmlns="http://www.w3.org/2000/svg"
            aria-hidden="true"
            fill="currentColor"
          >
            <path
              fill-rule="evenodd"
              clip-rule="evenodd"
              d="M48.854 0C21.839 0 0 22 0 49.217c0 21.756 13.993 40.172 33.405 46.69 2.427.49 3.316-1.059 3.316-2.362 0-1.141-.08-5.052-.08-9.127-13.59
              2.934-16.42-5.867-16.42-5.867-2.184-5.704-5.42-7.17-5.42-7.17-4.448-3.015.324-3.015.324-3.015 4.934.326 7.523 5.052 7.523 5.052 4.367
              7.496 11.404 5.378 14.235 4.074.404-3.178 1.699-5.378 3.074-6.6-10.839-1.141-22.243-5.378-22.243-24.283 0-5.378 1.94-9.778
              5.014-13.2-.485-1.222-2.184-6.275.486-13.038 0 0 4.125-1.304 13.426 5.052a46.97 46.97 0 0 1 12.214-1.63c4.125
              0 8.33.571 12.213 1.63 9.302-6.356 13.427-5.052 13.427-5.052 2.67 6.763.97 11.816.485 13.038 3.155 3.422 5.015 7.822 5.015 13.2
              0 18.905-11.404 23.06-22.324 24.283 1.78 1.548 3.316 4.481 3.316 9.126 0 6.6-.08 11.897-.08 13.526 0 1.304.89 2.853 3.316 2.364
              19.412-6.52 33.405-24.935 33.405-46.691C97.707 22 75.788 0 48.854 0z"
            />
          </svg>
        </a>
      </div>
    </header>

    <div
      class="td-menu-overlay"
      :class="{
        'is-open': isOpen,
      }"
    >
      <header class="td-menu-header">
        <div class="td-header-left">
          <TdButton
            class="w-9 h-9 text-td-neutral-fg-muted hover:text-td-primary-solid"
            label="Close menu"
            @click="closeMenu"
          >
            <X :size="20" />
          </TdButton>
          <a
            :href="withBase(getIndexUrl('/'))"
            class="td-brand"
          >
            <TdBrandIcon />
            <span class="td-brand-name">{{
              siteConfig.title || "Typedown"
            }}</span>
          </a>
        </div>
      </header>
      <TdHeaderNavMenu v-if="!menuSearchActive" />
      <TdSiteSearch
        ref="menuSearch"
        v-model:query="searchQuery"
        v-model:active="menuSearchActive"
        @select="closeMenu"
      />
      <nav
        v-if="!menuSearchActive"
        class="pt-4"
        aria-label="Site navigation"
      >
        <TdContentNav :tree="siteData.contentTree" />
      </nav>
    </div>

    <div class="td-page">
      <nav
        class="td-sidebar-left"
        aria-label="Site navigation"
        :style="{
          width: `${sidebarWidth}px`,
        }"
      >
        <TdSiteSearch
          ref="sidebarSearch"
          v-model:query="searchQuery"
          v-model:active="sidebarSearchActive"
        />
        <TdContentNav
          v-if="!sidebarSearchActive"
          :tree="siteData.contentTree"
        />
        <div
          class="td-sidebar-resize"
          @pointerdown.prevent="onResizeStart"
        />
      </nav>

      <div class="td-main-and-rail">
        <main class="td-main">
          <article class="td-content">
            <div
              v-if="page.schema"
              class="td-page-eyebrow"
            >
              {{ page.schemaLabel ?? page.schema }}
            </div>
            <div
              v-if="pageIcon"
              class="td-page-icon"
            >
              <component
                :is="pageIcon"
                :size="32"
              />
            </div>
            <h1
              v-if="title"
              class="td-page-title"
              v-html="renderInlineMarkup(title)"
            />
            <div
              v-if="page.metadata"
              class="td-page-meta"
            >
              <span>{{ formatEditTime(page.metadata.mtime, "Modified") }}</span>
              <span v-if="page.metadata.ctime !== page.metadata.mtime">
                · {{ formatEditTime(page.metadata.ctime, "Created") }}</span>
            </div>
            <TdFrontmatter
              class="td-frontmatter-inline"
              :schema="page.schema"
              :frontmatter="page.frontmatter"
            />
            <TdToc
              v-if="page.headings.length > 0"
              class="td-toc-inline"
              :headings="page.headings"
              static
            />
            <div class="td-content-body">
              <Content />
            </div>
            <TdPreviousNext />
          </article>
        </main>

        <TdRail class="td-rail" />
      </div>
    </div>

    <TdFooter />
  </div>
</template>

<style scoped>
/* Header */

.td-header {
  height: var(--td-header-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  border-bottom: 1px solid var(--color-td-neutral-border);
  background: var(--color-td-neutral-bg);
  position: sticky;
  top: 0;
  z-index: 40;
}

.td-header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.td-header-right {
  display: flex;
  align-items: center;
  gap: 2px;
}

.td-header-icon-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 6px;
  color: var(--color-td-neutral-fg-muted);
  transition: color 0.2s;
}

.td-header-icon-link:hover {
  color: var(--color-td-primary-solid);
}

.td-header-breadcrumb {
  font-size: var(--font-size-td-xs);
}

/* Brand */

.td-brand {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--color-td-primary-solid);
  text-decoration: none;
  transition: color 0.2s;
}

.td-brand:hover {
  color: var(--color-td-primary-solid-hover);
}

.td-brand-name {
  font-family: var(--font-sans);
  font-weight: 800;
  font-size: var(--font-size-td-base);
  letter-spacing: var(--tracking-td-tight);
}

/* Page grid */

.td-page {
  display: flex;
  min-height: calc(100vh - var(--td-header-height));
}

.td-main-and-rail {
  display: grid;
  grid-template-columns: minmax(0, 1fr) var(--td-toc-width);
  flex: 1;
  min-width: 0;
}

/* Main content */

.td-main {
  overflow-y: auto;
  min-width: 0;
}

.td-content {
  max-width: var(--td-content-max);
  margin: 0 auto;
  padding: 44px 56px 120px;
}

.td-page-eyebrow {
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  font-weight: 500;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-primary-solid);
  margin-bottom: 6px;
}

.td-page-icon {
  margin-bottom: 8px;
  color: var(--color-td-primary-solid);
}

.td-page-title {
  font-family: var(--font-sans);
  font-weight: 800;
  font-size: var(--font-size-td-3xl);
  line-height: var(--leading-td-tight);
  letter-spacing: var(--tracking-td-tight);
  margin: 0 0 8px 0;
}

.td-page-meta {
  font-size: var(--font-size-td-xs);
  color: var(--color-td-neutral-border-strong);
  margin-bottom: 8px;
}

/* Menu button: hidden above 900px */

.td-menu-button {
  display: none;
}

/* Sidebar: static column above 900px */

.td-sidebar-left {
  position: sticky;
  top: var(--td-header-height);
  height: calc(100vh - var(--td-header-height));
  overflow-y: auto;
  overflow-x: hidden;
  padding: 22px 0 60px;
  flex-shrink: 0;
}

.td-sidebar-resize {
  position: absolute;
  top: 0;
  right: -2px;
  width: 5px;
  height: 100%;
  cursor: col-resize;
  z-index: 10;
}

.td-sidebar-resize:hover,
.td-sidebar-resize:active {
  background: var(--color-td-primary-solid);
  opacity: 0.3;
}

/* Menu overlay: full-screen with own header */

.td-menu-overlay {
  display: none;
  position: fixed;
  inset: 0;
  z-index: 50;
  background: var(--color-td-neutral-bg);
  flex-direction: column;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.td-menu-overlay.is-open {
  display: flex;
}

.td-menu-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--td-header-height);
  padding: 0 16px;
  border-bottom: 1px solid var(--color-td-neutral-border);
  flex-shrink: 0;
}

/* Inline frontmatter and TOC hidden at desktop */
.td-frontmatter-inline,
.td-toc-inline {
  display: none;
}

.td-toc-inline {
  margin: 16px 0 0;
}

/* Below 1200px: hide rail, show inline frontmatter */

@media (width < 75rem) {
  .td-rail {
    display: none;
  }

  .td-main-and-rail {
    grid-template-columns: 1fr;
  }

  .td-frontmatter-inline {
    display: block;
    margin-bottom: 16px;
  }

  .td-toc-inline {
    display: block;
  }
}

/* Below 900px: hide sidebar, show menu overlay */

@media (width < 56.25rem) {
  .td-header {
    padding: 0 16px;
  }

  .td-menu-button {
    display: inline-flex;
  }

  .td-sidebar-left {
    display: none;
  }

  .td-header-breadcrumb {
    display: none;
  }

  .td-content {
    padding: 24px 16px 60px;
  }
}
</style>
