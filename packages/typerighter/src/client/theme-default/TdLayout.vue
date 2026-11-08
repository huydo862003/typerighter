<script setup lang="ts">
import {
  computed,
  nextTick,
  ref,
  useTemplateRef,
  watch,
  watchEffect,
} from 'vue';
import {
  ChevronsLeft, ChevronsRight, Terminal, X,
} from '@lucide/vue';
import {
  useTdContent,
  useSiteConfig,
  useSiteData,
  useRoute,
  useRouter,
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
import TdToc from './components/custom/TdToc.vue';
import TdSidebarNav from './components/TdSidebarNav.vue';
import TdSidebarIcons from './components/TdSidebarIcons.vue';
import TdSkeleton from './components/TdSkeleton.vue';
import TdCommandPalette from './components/TdCommandPalette.vue';
import TdKbdShortcut from './components/TdKbdShortcut.vue';
import TdTooltip from './components/TdTooltip.vue';
import TdShortcutsDialog from './components/TdShortcutsDialog.vue';
import TdSiteSearch from './components/TdSiteSearch/TdSiteSearch.vue';
import TdThemeToggle from './components/TdThemeToggle.vue';
import {
  useCopyCode,
} from './composables/useCopyCode';
import {
  useCopyHeadingAnchor,
} from './composables/useCopyHeadingAnchor';
import {
  useGlobalHotkey,
} from './composables/useHotkey';
import {
  usePreviousNext,
} from './composables/usePreviousNext';
import {
  TdKeyName,
} from './utils/keys';
import {
  useResizableTable,
} from './composables/useResizableTable';
import {
  useLazyEmbeds,
} from './composables/lazy-embed';
import {
  useSidebarConfig,
  SIDEBAR_MIN_WIDTH,
  SIDEBAR_MAX_WIDTH,
} from './composables/client-preferences/useSidebarConfig';
import {
  useMenu,
} from './composables/useMenu';
import {
  getPageIcon,
} from './utils/pageIcon';
import {
  formatEditTime, getIndexUrl, stripTrailingSlash,
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
const router = useRouter();
const siteData = useSiteData();
const siteDataReady = computed(() => siteData.value.ready);
const {
  isOpen, isDrawerMounted, open: openMenu, close: closeMenu,
} = useMenu();

const searchQuery = ref('');
const sidebarSearchActive = ref(false);
const menuSearchActive = ref(false);
const route = useRoute();
const shortcutsOpen = ref(false);

watch(
  () => route.path,
  () => closeMenu(),
);

useCopyCode();
useCopyHeadingAnchor();
useLazyEmbeds();

const sidebarSearch =
  useTemplateRef<InstanceType<typeof TdSiteSearch>>('sidebarSearch');
const menuSearch =
  useTemplateRef<InstanceType<typeof TdSiteSearch>>('menuSearch');
const palette =
  useTemplateRef<InstanceType<typeof TdCommandPalette>>('palette');

const {
  previous, next,
} = usePreviousNext();

function closeShortcuts () {
  shortcutsOpen.value = false;
}

function editPageUrl (filepath: string): string {
  const repo = siteConfig.repo!;
  const base = stripTrailingSlash(repo.url);
  const segments = [
    repo.root,
    siteConfig.rootDir,
    filepath,
  ].filter(Boolean);

  return `${base}/edit/main/${segments.join('/')}`;
}

function expandAndFocusSearch () {
  isSidebarCollapsed.value = false;
  nextTick(() => sidebarSearch.value?.focus());
}

function focusSearch () {
  if (sidebarSearch.value?.isVisible) {
    sidebarSearch.value.focus();
  } else {
    openMenu();
    menuSearch.value?.focus();
  }
}

function openPalette () {
  palette.value?.open();
}

function openShortcuts () {
  shortcutsOpen.value = true;
}

// ⌘K / Ctrl+K - focus site search
useGlobalHotkey([
  TdKeyName.Control,
  TdKeyName.k,
], focusSearch);
useGlobalHotkey([
  TdKeyName.Meta,
  TdKeyName.k,
], focusSearch);

// Ctrl+/ / ⌘/ - open command palette
useGlobalHotkey([
  TdKeyName.Control,
  TdKeyName.FSlash,
], openPalette);
useGlobalHotkey([
  TdKeyName.Meta,
  TdKeyName.FSlash,
], openPalette);

// Ctrl+Left / Ctrl+Right - previous / next page (ignored when typing)
useGlobalHotkey(
  [
    TdKeyName.Control,
    TdKeyName.ArrowLeft,
  ],
  () => {
    if (previous.value) router.go(previous.value.url);
  },
  {
    ignoreWhenTyping: true,
  },
);
useGlobalHotkey(
  [
    TdKeyName.Control,
    TdKeyName.ArrowRight,
  ],
  () => {
    if (next.value) router.go(next.value.url);
  },
  {
    ignoreWhenTyping: true,
  },
);

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

const {
  sidebarWidth, isSidebarCollapsed, sidebarStyle,
} = useSidebarConfig();
const isSidebarResizing = ref(false);

// ⌘\ / Ctrl+\ - toggle drawer on small screen, sidebar on large screen
function toggleNavPanel (): void {
  if (isDrawerMounted.value) {
    if (isOpen.value) {
      closeMenu();
    } else {
      openMenu();
    }
  } else {
    toggleSidebar();
  }
}

function toggleSidebar () {
  isSidebarCollapsed.value = !isSidebarCollapsed.value;
}

useGlobalHotkey([
  TdKeyName.Control,
  TdKeyName.Backslash,
], toggleNavPanel, {
  capture: true,
});
useGlobalHotkey([
  TdKeyName.Meta,
  TdKeyName.Backslash,
], toggleNavPanel, {
  capture: true,
});

function growSidebar () {
  sidebarWidth.value = Math.min(SIDEBAR_MAX_WIDTH, sidebarWidth.value + 20);
}

function onResizeStart (event: PointerEvent) {
  const startX = event.clientX;
  const startWidth = sidebarWidth.value;
  const target = event.currentTarget as HTMLElement;

  isSidebarResizing.value = true;
  target.setPointerCapture(event.pointerId);

  function onMove (event: PointerEvent) {
    const maxWidth = Math.min(SIDEBAR_MAX_WIDTH, window.innerWidth * 0.85);
    const width = Math.min(
      maxWidth,
      Math.max(SIDEBAR_MIN_WIDTH, startWidth + event.clientX - startX),
    );

    sidebarWidth.value = width;
  }

  function onUp () {
    isSidebarResizing.value = false;
    document.body.style.cursor = '';
    target.removeEventListener('pointermove', onMove);
    target.removeEventListener('pointerup', onUp);
  }

  document.body.style.cursor = 'col-resize';
  target.addEventListener('pointermove', onMove);
  target.addEventListener('pointerup', onUp);
}

function shrinkSidebar () {
  sidebarWidth.value = Math.max(SIDEBAR_MIN_WIDTH, sidebarWidth.value - 20);
}
</script>

<template>
  <div>
    <slot name="layout-top" />
    <a
      href="#td-content"
      class="td-skip-link"
    >Skip to content</a>
    <header class="td-header">
      <div class="td-header-left">
        <TdMenuButton
          class="lg:hidden"
          @click="openMenu"
        />
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
      <slot name="nav-bar-after" />
      <div class="td-header-right">
        <TdTooltip
          text="Commands"
          placement="bottom"
          :shortcut="[
            TdKeyName.Meta,
            TdKeyName.FSlash,
          ]"
          always
        >
          <button
            class="td-header-commands-btn"
            type="button"
            aria-label="Open command palette"
            @click="openPalette"
          >
            <Terminal :size="15" />
          </button>
        </TdTooltip>
        <TdThemeToggle />
        <a
          v-if="siteConfig.repo"
          :href="siteConfig.repo.url"
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

    <Transition name="td-drawer-scrim">
      <div
        v-if="isDrawerMounted"
        class="td-drawer-scrim"
        :class="{
          'is-open': isOpen,
        }"
        @click="closeMenu"
      />
    </Transition>
    <Transition name="td-drawer-panel">
      <aside
        v-if="isDrawerMounted"
        class="td-drawer"
        :class="{
          'is-open': isOpen,
        }"
      >
        <header class="td-drawer-header">
          <a
            :href="withBase(getIndexUrl('/'))"
            class="td-brand"
          >
            <TdBrandIcon />
            <span class="td-brand-name">{{
              siteConfig.title || "Typedown"
            }}</span>
          </a>
          <TdButton
            label="Close menu"
            @click="closeMenu"
          >
            <X :size="20" />
          </TdButton>
        </header>
        <TdHeaderNavMenu />
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
          <TdSkeleton v-if="!siteDataReady" />
          <TdContentNav
            v-else
            :tree="siteData.contentTree"
          />
        </nav>
      </aside>
    </Transition>

    <div class="td-page">
      <nav
        class="td-sidebar-left"
        :class="{
          'is-resizing': isSidebarResizing,
          'is-collapsed': isSidebarCollapsed,
        }"
        data-testid="sidebar"
        aria-label="Site navigation"
        :style="sidebarStyle"
      >
        <template v-if="!isSidebarCollapsed">
          <slot name="sidebar-top" />
          <TdSiteSearch
            ref="sidebarSearch"
            v-model:query="searchQuery"
            v-model:active="sidebarSearchActive"
            class="td-sidebar-search"
          />
          <TdSidebarNav
            :tree="siteData.contentTree"
            :search-active="sidebarSearchActive"
            :ready="siteDataReady"
          />
          <slot name="sidebar-bottom" />
        </template>
        <TdSidebarIcons
          v-else
          :tree="siteData.contentTree"
          @search="expandAndFocusSearch"
        />
        <button
          class="td-sidebar-collapse-btn"
          type="button"
          :aria-label="isSidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'"
          @click="toggleSidebar"
        >
          <ChevronsLeft
            v-if="!isSidebarCollapsed"
            :size="14"
          />
          <ChevronsRight
            v-else
            :size="14"
          />
          <span
            v-if="!isSidebarCollapsed"
            class="td-sidebar-collapse-label"
          >Collapse sidebar</span>
          <TdKbdShortcut
            v-if="!isSidebarCollapsed"
            :keys="[
              TdKeyName.Meta,
              TdKeyName.Backslash,
            ]"
          />
        </button>
        <div
          v-if="!isSidebarCollapsed"
          class="td-resize-handle"
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize sidebar"
          tabindex="0"
          @pointerdown.prevent="onResizeStart"
          @keydown.left.prevent="shrinkSidebar"
          @keydown.right.prevent="growSidebar"
        />
      </nav>

      <div class="td-main-and-rail">
        <main class="td-main">
          <article
            id="td-content"
            class="td-content"
            data-testid="content"
          >
            <slot name="doc-before" />
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
              v-html="page.titleHtml"
            />
            <div
              v-if="page.fileMetadata"
              class="td-page-meta"
            >
              <span>{{
                formatEditTime(page.fileMetadata.mtimeSecs, "Modified")
              }}</span>
              <span
                v-if="
                  page.fileMetadata.ctimeSecs !== page.fileMetadata.mtimeSecs
                "
              >
                ·
                {{
                  formatEditTime(page.fileMetadata.ctimeSecs, "Created")
                }}</span>
            </div>
            <TdFrontmatter
              class="td-frontmatter-inline"
              :schema="page.schema"
              :frontmatter="page.frontmatter"
            />
            <TdToc
              v-if="page.headings.length > 1"
              class="td-toc-inline"
              :headings="page.headings"
              inline
            />
            <slot name="doc-content-before" />
            <div class="td-content-body">
              <Content />
            </div>
            <slot name="doc-content-after" />
            <TdPreviousNext />
            <a
              v-if="siteConfig.repo && page.filepath"
              :href="editPageUrl(page.filepath)"
              target="_blank"
              rel="noopener noreferrer"
              class="td-page-edit"
            >Edit this page</a>
            <slot name="doc-after" />
          </article>
        </main>

        <TdRail class="td-rail" />
      </div>
    </div>

    <TdFooter />
    <slot name="layout-bottom" />

    <TdCommandPalette
      ref="palette"
      @toggle-sidebar="toggleSidebar"
      @show-shortcuts="openShortcuts"
    />
    <TdShortcutsDialog
      :open="shortcutsOpen"
      @close="closeShortcuts"
    />
  </div>
</template>

<style scoped>
/* Skip link */

.td-skip-link {
  position: absolute;
  top: -100%;
  left: 16px;
  z-index: var(--layer-td-base);
  padding: 8px 16px;
  background: var(--color-td-accent);
  color: white;
  border-radius: 4px;
  font-size: var(--font-size-td-sm);
  text-decoration: none;
}

.td-skip-link:focus {
  top: 8px;
}

/* Header */

.td-header {
  height: var(--td-header-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  border-bottom: 1px solid var(--color-td-line);
  background: var(--color-td-bg);
  position: sticky;
  top: 0;
  z-index: var(--layer-td-base);
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

.td-header-commands-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  background: none;
  cursor: pointer;
  border-radius: 6px;
  color: var(--color-td-ink-4);
  transition: color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-header-commands-btn:hover {
  color: var(--color-td-accent);
}

.td-header-icon-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 6px;
  color: var(--color-td-ink-4);
  transition: color var(--duration-td-move) var(--ease-td-out-quart);
}

.td-header-icon-link:hover {
  color: var(--color-td-accent);
}

.td-header-breadcrumb {
  font-size: var(--font-size-td-xs);
}

/* Brand */

.td-brand {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--color-td-accent);
  text-decoration: none;
  transition: color var(--duration-td-move) var(--ease-td-out-quart);
}

.td-brand:hover {
  color: var(--color-td-accent-hi);
}

.td-brand-name {
  font-family: var(--font-sans);
  font-weight: 800;
  font-size: var(--font-size-td-base);
  letter-spacing: var(--tracking-td-tight);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Page grid */

.td-page {
  display: flex;
  min-height: calc(100vh - var(--td-header-height));
}

.td-main-and-rail {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 48px;
  flex: 1;
  min-width: 0;
}

@media (hover: hover) and (pointer: fine) {
  .td-main {
    padding-left: 48px;
  }
}

/* Main content */

.td-main {
  min-width: 0;
  position: relative;
  z-index: 0;
  isolation: isolate;
}

.td-content {
  padding: 44px 32px 48px;
  max-width: 860px;
  margin: 0 auto;
  overflow-x: hidden;
}

.td-page-eyebrow {
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  font-weight: 500;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-accent);
  margin-bottom: 6px;
}

.td-page-icon {
  margin-bottom: 8px;
  color: var(--color-td-accent);
}

.td-page-title {
  font-family: var(--font-sans);
  font-weight: 600;
  font-size: var(--font-size-td-2xl);
  line-height: var(--leading-td-tight);
  letter-spacing: var(--tracking-td-tight);
  font-variation-settings: "opsz" 28;
  color: var(--color-td-ink);
  margin: 0 0 8px 0;
}

.td-page-meta {
  font-size: var(--font-size-td-xs);
  color: var(--color-td-ink-4);
  margin-bottom: 8px;
}

/* Edit link */

.td-page-edit {
  display: inline-block;
  margin-top: 16px;
  font-size: var(--font-size-td-xs);
  color: var(--color-td-ink-3);
  text-decoration: none;
  transition: color var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-page-edit:hover {
  color: var(--color-td-accent);
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
  flex-shrink: 0;
  overflow: clip;
  display: flex;
  flex-direction: column;
  background: var(--color-td-surface);
  border-right: 1px solid var(--color-td-line);
  transition: width var(--duration-td-drawer) var(--ease-td-out-quart);
}

.td-sidebar-left.is-resizing {
  transition: none;
}

/* Collapsed icon-only strip: button is centered */
.td-sidebar-left.is-collapsed .td-sidebar-collapse-btn {
  justify-content: center;
  padding: 8px 0;
  margin: 8px auto;
  width: 36px;
}

.td-sidebar-search {
  flex-shrink: 0;
  padding: 12px 10px;
}

.td-sidebar-collapse-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  margin: 4px 8px;
  border: none;
  background: none;
  cursor: pointer;
  border-radius: 6px;
  color: var(--color-td-ink-4);
  font-size: var(--font-size-td-xs);
  font-family: inherit;
  flex-shrink: 0;
  transition:
    color var(--duration-td-fast) var(--ease-td-out-quart),
    background var(--duration-td-fast) var(--ease-td-out-quart);
  white-space: nowrap;
  overflow: hidden;
}

.td-sidebar-collapse-btn:hover {
  color: var(--color-td-accent);
  background: var(--color-td-hover);
}

.td-sidebar-collapse-label {
  flex: 1;
  text-align: left;
}

.td-resize-handle {
  position: absolute;
  top: 0;
  right: -2px;
  width: 5px;
  height: 100%;
  cursor: col-resize;
  z-index: 2;
}

.td-resize-handle:hover,
.td-resize-handle:active {
  background: var(--color-td-accent);
  opacity: 0.3;
}

/* Drawer: sliding sidebar panel with backdrop scrim */

.td-drawer-scrim {
  position: fixed;
  inset: 0;
  z-index: var(--layer-td-drawer);
  background: var(--color-td-scrim);
  opacity: 0;
  pointer-events: none;
  transition: opacity 220ms var(--ease-td-out-quart);
}

.td-drawer-scrim.is-open {
  opacity: 1;
  pointer-events: auto;
}

.td-drawer {
  position: fixed;
  width: min(350px, 100vw);
  top: 0;
  left: 0;
  bottom: 0;
  z-index: var(--layer-td-drawer);
  max-width: min(85vw, 500px);
  background: var(--color-td-bg);
  border-right: 1px solid var(--color-td-line);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  overflow-x: clip;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  transform: translateX(-100%);
  transition: transform var(--duration-td-drawer) var(--ease-td-out-quart);
}

.td-drawer.is-open {
  transform: translateX(0);
}

/* leave-to overrides is-open so an open drawer slides out when transitioning to desktop */
.td-drawer-scrim-leave-active {
  transition: opacity 220ms var(--ease-td-out-quart);
}

.td-drawer-scrim-leave-to {
  opacity: 0 !important;
}

.td-drawer-panel-leave-active {
  transition: transform var(--duration-td-drawer) var(--ease-td-out-quart);
}

.td-drawer-panel-leave-to {
  transform: translateX(-100%) !important;
}

.td-drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--td-header-height);
  padding: 0 16px;
  border-bottom: 1px solid var(--color-td-line);
  background: var(--color-td-surface);
  flex-shrink: 0;
  position: sticky;
  top: 0;
  z-index: 1;
}

.td-frontmatter-inline {
  display: block;
  margin-bottom: 16px;
}

/* Inline TOC: only on touch/non-mouse devices */
.td-toc-inline {
  display: none;
  margin: 16px 0 0;
}

@media not ((hover: hover) and (pointer: fine)) {
  .td-toc-inline {
    display: block;
  }
}

/* Non-mouse devices: hide rail entirely */

@media not ((hover: hover) and (pointer: fine)) {
  .td-rail {
    display: none;
  }

  .td-main-and-rail {
    grid-template-columns: 1fr;
  }

  .td-main {
    padding-left: 0;
  }
}

/* Below 900px: hide sidebar, show menu overlay */

@media (width < 64rem) {
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
