<script setup lang="ts">
import { ref, computed } from 'vue';
import {
  Content, useTdContent, useRoute, useSiteConfig, useTheme,
} from 'typerighter/client';

const route = useRoute();
const { title } = useSiteConfig();
const { isDark, toggle } = useTheme();

useTdContent();

const mobileOpen = ref(false);

const isHome = computed(() => route.path === '/' || route.path === '/index');

const sidebar = [
  {
    title: 'Guide',
    items: [
      { text: 'Introduction', link: '/guide/index' },
      { text: 'Configuration', link: '/guide/configuration' },
      { text: 'Theming', link: '/guide/theming' },
    ],
  },
  {
    title: 'API Reference',
    items: [
      { text: 'Composables', link: '/api/composables' },
      { text: 'Components', link: '/api/components' },
    ],
  },
];

function isActive (link: string): boolean {
  return route.path === link || route.path === link + '/';
}

interface Heading {
  level: number;
  title: string;
  slug: string;
  children: Heading[];
}

const tocHeadings = computed(() => {
  const result: { level: number; title: string; slug: string }[] = [];

  function walk (items: Heading[]) {
    for (const h of items) {
      if (h.level === 2 || h.level === 3) result.push(h);
      if (h.children?.length) walk(h.children);
    }
  }

  walk((route.data?.headings ?? []) as Heading[]);

  return result;
});
</script>

<template>
  <div :class="['vp-layout', { dark: isDark }]">
    <!-- Top nav -->
    <header class="vp-nav">
      <div class="vp-nav-inner">
        <div class="vp-nav-left">
          <button
            class="vp-menu-btn"
            :aria-label="mobileOpen ? 'Close menu' : 'Open menu'"
            @click="mobileOpen = !mobileOpen"
          >
            <span class="vp-icon-bar" />
            <span class="vp-icon-bar" />
            <span class="vp-icon-bar" />
          </button>
          <a
            href="/"
            class="vp-logo"
          >{{ title }}</a>
        </div>
        <nav class="vp-nav-links">
          <a href="/guide/index">Guide</a>
          <a href="/api/composables">API</a>
          <a
            href="https://github.com/typedown/typedown"
            target="_blank"
            rel="noopener"
          >GitHub</a>
        </nav>
        <button
          class="vp-theme-btn"
          :aria-label="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
          @click="toggle"
        >
          <svg
            v-if="isDark"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <circle
              cx="12"
              cy="12"
              r="5"
            /><line
              x1="12"
              y1="1"
              x2="12"
              y2="3"
            /><line
              x1="12"
              y1="21"
              x2="12"
              y2="23"
            /><line
              x1="4.22"
              y1="4.22"
              x2="5.64"
              y2="5.64"
            /><line
              x1="18.36"
              y1="18.36"
              x2="19.78"
              y2="19.78"
            /><line
              x1="1"
              y1="12"
              x2="3"
              y2="12"
            /><line
              x1="21"
              y1="12"
              x2="23"
              y2="12"
            /><line
              x1="4.22"
              y1="19.78"
              x2="5.64"
              y2="18.36"
            /><line
              x1="18.36"
              y1="5.64"
              x2="19.78"
              y2="4.22"
            />
          </svg>
          <svg
            v-else
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
          </svg>
        </button>
      </div>
    </header>

    <!-- Mobile sidebar overlay -->
    <div
      v-if="mobileOpen"
      class="vp-overlay"
      @click="mobileOpen = false"
    />
    <aside
      :class="['vp-sidebar', { 'vp-sidebar--open': mobileOpen }]"
    >
      <nav class="vp-sidebar-nav">
        <div
          v-for="group in sidebar"
          :key="group.title"
          class="vp-sidebar-group"
        >
          <div class="vp-sidebar-title">
            {{ group.title }}
          </div>
          <ul>
            <li
              v-for="item in group.items"
              :key="item.link"
            >
              <a
                :href="item.link"
                :class="['vp-sidebar-link', { 'vp-sidebar-link--active': isActive(item.link) }]"
                @click="mobileOpen = false"
              >{{ item.text }}</a>
            </li>
          </ul>
        </div>
      </nav>
    </aside>

    <!-- Home layout -->
    <main
      v-if="isHome"
      class="vp-home"
    >
      <div class="vp-hero">
        <p class="vp-hero-tagline">
          Custom Theme
        </p>
        <h1 class="vp-hero-title">
          Build your own<br>theme from scratch
        </h1>
        <p class="vp-hero-text">
          Replace the default layout entirely. Full control over every pixel - header, sidebar, typography, dark mode - using composables from <code>typerighter/client</code>.
        </p>
        <div class="vp-hero-actions">
          <a
            href="/guide/index"
            class="vp-btn vp-btn--primary"
          >Get Started</a>
          <a
            href="/api/composables"
            class="vp-btn vp-btn--secondary"
          >API Reference</a>
        </div>
      </div>
      <div class="vp-home-content td-content">
        <Content />
      </div>
    </main>

    <!-- Doc layout -->
    <div
      v-else
      class="vp-doc-layout"
    >
      <div class="vp-doc-main">
        <article class="vp-doc td-content">
          <Content />
        </article>
        <div class="vp-doc-footer">
          <a
            href="/guide/index"
            class="vp-prev-next"
          >← Back to Guide</a>
        </div>
      </div>
      <aside
        v-if="tocHeadings.length"
        class="vp-toc"
      >
        <div class="vp-toc-title">
          On this page
        </div>
        <ul>
          <li
            v-for="h in tocHeadings"
            :key="h.slug"
            :class="['vp-toc-item', `vp-toc-item--h${h.level}`]"
          >
            <a :href="`#${h.slug}`">{{ h.title }}</a>
          </li>
        </ul>
      </aside>
    </div>
  </div>
</template>

<style>
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

:root {
  --vp-c-bg: #ffffff;
  --vp-c-bg-soft: #f6f6f7;
  --vp-c-bg-alt: #f6f6f7;
  --vp-c-text-1: rgba(60, 60, 67);
  --vp-c-text-2: rgba(60, 60, 67, 0.78);
  --vp-c-text-3: rgba(60, 60, 67, 0.56);
  --vp-c-divider: rgba(60, 60, 67, 0.12);
  --vp-c-gutter: rgba(60, 60, 67, 0.05);
  --vp-c-brand-1: #5672cd;
  --vp-c-brand-2: #3451b2;
  --vp-c-brand-soft: rgba(84, 114, 235, 0.14);
  --vp-c-tip: #10b981;
  --vp-c-code-bg: #f6f6f7;

  --vp-nav-height: 64px;
  --vp-sidebar-width: 272px;
  --vp-toc-width: 224px;

  --vp-font-family-base: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Helvetica, Arial, 'Noto Sans', sans-serif;
  --vp-font-family-mono: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
}

.dark {
  --vp-c-bg: #1b1b1f;
  --vp-c-bg-soft: #202127;
  --vp-c-bg-alt: #161618;
  --vp-c-text-1: rgba(255, 255, 245, 0.86);
  --vp-c-text-2: rgba(235, 235, 245, 0.6);
  --vp-c-text-3: rgba(235, 235, 245, 0.38);
  --vp-c-divider: rgba(82, 82, 89, 0.32);
  --vp-c-gutter: rgba(255, 255, 255, 0.04);
  --vp-c-brand-1: #a8b1ff;
  --vp-c-brand-2: #5672cd;
  --vp-c-brand-soft: rgba(84, 114, 235, 0.16);
  --vp-c-code-bg: #161618;
}

.vp-layout {
  font-family: var(--vp-font-family-base);
  font-size: 16px;
  line-height: 1.7;
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  min-height: 100vh;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* Nav */

.vp-nav {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: var(--vp-nav-height);
  background: var(--vp-c-bg);
  border-bottom: 1px solid var(--vp-c-divider);
  z-index: 100;
}

.vp-nav-inner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 100%;
  max-width: 1440px;
  margin: 0 auto;
  padding: 0 32px;
}

.vp-nav-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.vp-logo {
  font-size: 16px;
  font-weight: 600;
  color: var(--vp-c-text-1);
  text-decoration: none;
}

.vp-menu-btn {
  display: none;
  flex-direction: column;
  gap: 4px;
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px;
}

.vp-icon-bar {
  display: block;
  width: 18px;
  height: 2px;
  background: var(--vp-c-text-1);
  border-radius: 2px;
}

.vp-nav-links {
  display: flex;
  gap: 32px;
}

.vp-nav-links a {
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-text-2);
  text-decoration: none;
  transition: color 0.25s;
}

.vp-nav-links a:hover {
  color: var(--vp-c-text-1);
}

.vp-theme-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: none;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  color: var(--vp-c-text-2);
  transition: color 0.25s;
}

.vp-theme-btn:hover {
  color: var(--vp-c-text-1);
}

/* Sidebar */

.vp-sidebar {
  position: fixed;
  top: var(--vp-nav-height);
  left: 0;
  bottom: 0;
  width: var(--vp-sidebar-width);
  overflow-y: auto;
  padding: 32px 32px 32px 32px;
  background: var(--vp-c-bg);
  z-index: 50;
}

.vp-sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.vp-sidebar-group {
  padding-bottom: 16px;
}

.vp-sidebar-group ul {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0;
  margin-top: 4px;
  border-left: 1px solid var(--vp-c-divider);
  padding-left: 0;
}

.vp-sidebar-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--vp-c-text-1);
  padding: 4px 0;
}

.vp-sidebar-link {
  display: block;
  padding: 4px 0 4px 16px;
  margin-left: -1px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--vp-c-text-2);
  text-decoration: none;
  border-left: 1px solid transparent;
  transition: color 0.25s, border-color 0.25s;
}

.vp-sidebar-link:hover {
  color: var(--vp-c-text-1);
}

.vp-sidebar-link--active {
  color: var(--vp-c-brand-1);
  border-left-color: var(--vp-c-brand-1);
  font-weight: 600;
}

/* Overlay for mobile */

.vp-overlay {
  display: none;
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  z-index: 40;
}

/* Home */

.vp-home {
  padding-top: var(--vp-nav-height);
  min-height: 100vh;
}

.vp-hero {
  max-width: 960px;
  margin: 0 auto;
  padding: 76px 48px 56px;
  text-align: center;
}

.vp-hero-tagline {
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-brand-1);
  margin-bottom: 8px;
}

.vp-hero-title {
  font-size: 48px;
  font-weight: 700;
  line-height: 1.15;
  letter-spacing: -0.02em;
  color: var(--vp-c-text-1);
  margin-bottom: 24px;
}

.vp-hero-text {
  font-size: 20px;
  color: var(--vp-c-text-2);
  max-width: 576px;
  margin: 0 auto 32px;
  line-height: 1.6;
}

.vp-hero-text code {
  font-family: var(--vp-font-family-mono);
  font-size: 0.85em;
  background: var(--vp-c-code-bg);
  padding: 3px 6px;
  border-radius: 4px;
  color: var(--vp-c-brand-1);
}

.vp-hero-actions {
  display: flex;
  gap: 16px;
  justify-content: center;
  flex-wrap: wrap;
}

.vp-btn {
  display: inline-flex;
  align-items: center;
  height: 48px;
  padding: 0 20px;
  border-radius: 24px;
  font-size: 14px;
  font-weight: 600;
  text-decoration: none;
  transition: background 0.25s, color 0.25s, border-color 0.25s;
}

.vp-btn--primary {
  background: var(--vp-c-brand-1);
  color: #fff;
  border: 1px solid transparent;
}

.vp-btn--primary:hover {
  background: var(--vp-c-brand-2);
}

.vp-btn--secondary {
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-1);
  border: 1px solid var(--vp-c-divider);
}

.vp-btn--secondary:hover {
  border-color: var(--vp-c-text-2);
}

.vp-home-content {
  max-width: 960px;
  margin: 0 auto;
  padding: 0 48px 80px;
  border-top: 1px solid var(--vp-c-divider);
}

/* Doc layout */

.vp-doc-layout {
  display: flex;
  padding-top: var(--vp-nav-height);
  padding-left: var(--vp-sidebar-width);
  min-height: 100vh;
}

.vp-doc-main {
  flex: 1;
  min-width: 0;
  padding: 48px 32px 48px 48px;
  max-width: calc(100% - var(--vp-toc-width));
}

.vp-doc-footer {
  margin-top: 48px;
  padding-top: 16px;
  border-top: 1px solid var(--vp-c-divider);
}

.vp-prev-next {
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-brand-1);
  text-decoration: none;
  transition: color 0.25s;
}

.vp-prev-next:hover {
  color: var(--vp-c-brand-2);
}

/* TOC */

.vp-toc {
  width: var(--vp-toc-width);
  flex-shrink: 0;
  padding: 48px 32px 48px 16px;
  position: sticky;
  top: var(--vp-nav-height);
  height: calc(100vh - var(--vp-nav-height));
  overflow-y: auto;
  border-left: 1px solid var(--vp-c-divider);
}

.vp-toc-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--vp-c-text-1);
  margin-bottom: 8px;
  padding-left: 12px;
}

.vp-toc ul {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0;
  border-left: 1px solid var(--vp-c-divider);
}

.vp-toc-item a {
  font-size: 13px;
  color: var(--vp-c-text-3);
  text-decoration: none;
  line-height: 1.5;
  display: block;
  padding: 2px 0 2px 12px;
  transition: color 0.25s;
}

.vp-toc-item a:hover {
  color: var(--vp-c-text-1);
}

.vp-toc-item--h3 a {
  padding-left: 24px;
}

/* Content styles */

.td-content h1 {
  font-size: 28px;
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1.28;
  margin-bottom: 16px;
  margin-top: 0;
}

.vp-home-content .td-content h1 {
  margin-top: 48px;
}

.td-content h2 {
  font-size: 24px;
  font-weight: 600;
  letter-spacing: -0.02em;
  margin-top: 40px;
  margin-bottom: 16px;
  padding-top: 24px;
  border-top: 1px solid var(--vp-c-divider);
}

.td-content h3 {
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.01em;
  margin-top: 32px;
  margin-bottom: 12px;
}

.td-content p {
  margin-bottom: 16px;
  line-height: 1.7;
}

.td-content a {
  color: var(--vp-c-brand-1);
  text-decoration: underline;
  text-underline-offset: 2px;
  text-decoration-thickness: 1px;
  transition: color 0.25s;
}

.td-content a:hover {
  color: var(--vp-c-brand-2);
}

.td-content code {
  font-family: var(--vp-font-family-mono);
  font-size: 0.875em;
  background: var(--vp-c-code-bg);
  padding: 3px 6px;
  border-radius: 4px;
}

.td-content pre {
  background: var(--vp-c-code-bg);
  border-radius: 8px;
  padding: 20px 24px;
  overflow-x: auto;
  margin: 16px 0;
  line-height: 1.7;
}

.td-content pre code {
  background: none;
  padding: 0;
  font-size: 13px;
  line-height: 1.7;
}

.td-content table {
  width: 100%;
  border-collapse: collapse;
  margin: 16px 0;
  font-size: 14px;
  display: block;
  overflow-x: auto;
}

.td-content th {
  text-align: left;
  padding: 8px 16px;
  background: var(--vp-c-bg-soft);
  border-bottom: 1px solid var(--vp-c-divider);
  font-weight: 600;
  font-size: 14px;
}

.td-content td {
  padding: 8px 16px;
  border-bottom: 1px solid var(--vp-c-divider);
}

.td-content ul, .td-content ol {
  padding-left: 20px;
  margin-bottom: 16px;
}

.td-content li {
  margin-bottom: 4px;
  line-height: 1.7;
}

.td-content li + li {
  margin-top: 4px;
}

.td-content blockquote {
  border-left: 2px solid var(--vp-c-tip);
  margin: 16px 0;
  padding: 8px 16px;
  color: var(--vp-c-text-2);
}

.td-content blockquote p {
  margin: 0;
}

.td-content hr {
  border: none;
  border-top: 1px solid var(--vp-c-divider);
  margin: 32px 0;
}

/* Responsive */

@media (max-width: 960px) {
  .vp-toc {
    display: none;
  }

  .vp-doc-main {
    max-width: 100%;
    padding: 32px 40px;
  }
}

@media (max-width: 768px) {
  .vp-menu-btn {
    display: flex;
  }

  .vp-nav-links {
    display: none;
  }

  .vp-sidebar {
    transform: translateX(-100%);
    transition: transform 0.25s ease;
    border-right: 1px solid var(--vp-c-divider);
  }

  .vp-sidebar--open {
    transform: translateX(0);
  }

  .vp-overlay {
    display: block;
  }

  .vp-doc-layout {
    padding-left: 0;
  }

  .vp-doc-main {
    padding: 24px 24px;
  }

  .vp-hero {
    padding: 48px 24px 40px;
  }

  .vp-hero-title {
    font-size: 36px;
  }

  .vp-hero-text {
    font-size: 16px;
  }

  .vp-home-content {
    padding: 0 24px 48px;
  }
}
</style>
