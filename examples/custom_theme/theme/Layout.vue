<script setup lang="ts">
import {
  Content, useTdContent, useRoute, useSiteConfig,
} from 'typerighter/client';

const route = useRoute();
const { title } = useSiteConfig();

useTdContent(route);
</script>

<template>
  <div class="layout">
    <header class="header">
      <a
        href="/"
        class="logo"
      >
        {{ title }}
      </a>
      <nav class="nav">
        <a href="/getting-started">Getting started</a>
        <a href="/components">Components</a>
        <a href="/about">About</a>
      </nav>
    </header>

    <main class="main">
      <h1 v-if="route.data?.title">
        {{ route.data.title }}
      </h1>
      <Content />
    </main>

    <footer class="footer">
      Built with Typerighter using a custom layout.
    </footer>
  </div>
</template>

<style>
:root {
  --bg: #fafaf9;
  --fg: #1c1917;
  --accent: #2563eb;
  --border: #e7e5e4;
  --radius: 8px;
}

:root.dark {
  --bg: #0c0a09;
  --fg: #fafaf9;
  --accent: #60a5fa;
  --border: #292524;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: system-ui, -apple-system, sans-serif;
  background: var(--bg);
  color: var(--fg);
  line-height: 1.6;
}

.layout {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 32px;
  border-bottom: 1px solid var(--border);
}

.logo {
  font-size: 18px;
  font-weight: 700;
  color: var(--fg);
  text-decoration: none;
}

.nav {
  display: flex;
  gap: 24px;
}

.nav a {
  font-size: 14px;
  color: var(--fg);
  text-decoration: none;
  opacity: 0.7;
  transition: opacity 0.15s;
}

.nav a:hover {
  opacity: 1;
}

.main {
  flex: 1;
  max-width: 720px;
  width: 100%;
  margin: 0 auto;
  padding: 48px 32px;
}

.main h1 {
  font-size: 32px;
  font-weight: 700;
  margin-bottom: 24px;
  letter-spacing: -0.02em;
}

.footer {
  padding: 24px 32px;
  border-top: 1px solid var(--border);
  font-size: 13px;
  color: var(--fg);
  opacity: 0.5;
  text-align: center;
}
</style>
