<script setup lang="ts">
import {
  ref, watch,
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
import TdBreadcrumb from './components/TdBreadcrumb.vue';
import {
  TdFrontmatter,
} from './components/TdFrontmatter';
import TdMenuButton from './components/TdMenuButton.vue';
import TdPreviousNext from './components/TdPreviousNext.vue';
import TdSearch from './components/TdSearch.vue';
import TdThemeToggle from './components/TdThemeToggle.vue';
import {
  useCopyCode,
} from './composables/useCopyCode';
import {
  useResizableTable,
} from './composables/useResizableTable';
import {
  useMenu,
} from './composables/useMenu';
import TdToc from './components/TdToc.vue';
import {
  formatEditTime,
} from '@/shared';
import './styles/main.css';
import './styles/markdown/content.css';
import './styles/markdown/code.css';
import './styles/markdown/containers.css';

const {
  title, page,
} = useTdContent();
const siteConfig = useSiteConfig();
const siteData = useSiteData();
const {
  isOpen, close: closeMenu,
} = useMenu();

const searchQuery = ref('');
const sidebarSearchActive = ref(false);
const menuSearchActive = ref(false);
const route = useRoute();

watch(() => route.path, () => closeMenu());

useCopyCode();
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
    const width = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, startWidth + event.clientX - startX));

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
          href="/"
          class="td-brand"
        >
          <TdBrandIcon />
          <span class="td-brand-name">{{ siteConfig.title || 'Typedown' }}</span>
        </a>
      </div>
      <div class="td-header-right">
        <TdThemeToggle />
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
            href="/"
            class="td-brand"
          >
            <TdBrandIcon />
            <span class="td-brand-name">{{ siteConfig.title || 'Typedown' }}</span>
          </a>
        </div>
      </header>
      <TdSearch
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
        <TdSearch
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

      <div class="td-main-and-toc">
        <main class="td-main">
          <article class="td-content">
            <TdBreadcrumb />
            <h1
              v-if="title"
              class="td-page-title"
            >
              {{ title }}
            </h1>
            <div
              v-if="page.metadata"
              class="td-page-meta"
            >
              <span>{{ formatEditTime(page.metadata.mtime, 'Modified') }}</span>
              <span v-if="page.metadata.ctime !== page.metadata.mtime"> · {{ formatEditTime(page.metadata.ctime, 'Created') }}</span>
            </div>
            <TdFrontmatter
              :schema="page.schema"
              :frontmatter="page.frontmatter"
            />
            <Content />
            <TdPreviousNext />
          </article>
        </main>

        <nav
          class="td-sidebar-right"
          aria-label="Table of contents"
        >
          <TdToc :headings="page.headings" />
        </nav>
      </div>
    </div>
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
  gap: 8px;
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
  font-weight: var(--font-weight-td-heading);
  font-size: var(--font-size-td-site-title);
  letter-spacing: var(--tracking-td-title);
}

/* Page grid */

.td-page {
  display: flex;
  min-height: calc(100vh - var(--td-header-height));
}

.td-main-and-toc {
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

.td-page-title {
  font-family: var(--font-sans);
  font-weight: var(--font-weight-td-heading);
  font-size: var(--font-size-td-h1);
  line-height: var(--leading-td-heading);
  letter-spacing: var(--tracking-td-heading);
  margin: 0 0 8px 0;
}

.td-page-meta {
  font-size: var(--font-size-td-caption);
  color: var(--color-td-neutral-border-strong);
  margin-bottom: 24px;
}

/* Menu button: hidden above lg */

.td-menu-button {
  display: none;
}

/* Sidebar: static column above lg */

.td-sidebar-left {
  position: sticky;
  top: var(--td-header-height);
  height: calc(100vh - var(--td-header-height));
  overflow-y: auto;
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

/* Right sidebar (TOC) */

.td-sidebar-right {
  position: sticky;
  top: var(--td-header-height);
  height: calc(100vh - var(--td-header-height));
  overflow-y: auto;
  padding: 44px 22px 60px;
  min-width: 0;
}

/* Below lg breakpoint */

@media (width < 64rem) {
  .td-header {
    padding: 0 16px;
  }

  .td-menu-button {
    display: inline-flex;
  }

  .td-sidebar-left,
  .td-sidebar-right {
    display: none;
  }

  .td-main-and-toc {
    grid-template-columns: 1fr;
  }

  .td-content {
    padding: 24px 16px 60px;
  }
}
</style>
