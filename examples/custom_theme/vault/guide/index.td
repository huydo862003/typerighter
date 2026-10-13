---
_label: "Introduction"
---

# Introduction

Typerighter lets you replace its default theme entirely with your own Vue layout component. You own every pixel - header, sidebar, typography, dark mode, and page structure.

## When to use a custom layout

The default theme gives you a polished documentation site out of the box. Use a custom layout when you need:

- A completely different visual identity (marketing site, portfolio, dashboard)
- A different page structure (no sidebar, split-pane, full-width)
- Custom navigation patterns or routing behavior
- Integration with your own design system

## How it works

Typerighter compiles every `.td` file into a Vue SFC. Your layout wraps the rendered content via the `<Content />` component. The framework provides routing, dark mode state, and site metadata - you compose the UI around it.

## Project structure

```
my-site/
├── typedown.yaml        # site config with theme.layout
├── vault/               # markdown content files
│   ├── index.td
│   └── guide/
│       └── intro.td
└── theme/
    └── Layout.vue       # your custom layout
```

## Registering the layout

In `typedown.yaml`:

```yaml
theme:
  layout: "./theme/Layout.vue"
```

The path is relative to `typedown.yaml`. Typerighter resolves it to an absolute path and imports it as the root layout component.

## Accessing content

Your layout receives no props. Instead, use composables:

```vue
<script setup>
import { Content, useTdContent, useRoute } from 'typerighter/client'

const route = useRoute()
useTdContent() // must call - provides context to <Content />
</script>

<template>
  <main>
    <h1>{{ route.data.title }}</h1>
    <Content />
  </main>
</template>
```

> **Important:** `useTdContent()` must be called in your layout's `setup`. It sets up the provide/inject context that `<Content />` depends on.

## HMR

Editing `Layout.vue` hot-reloads in the browser immediately via Vite's Vue HMR - no special setup needed. Changing `typedown.yaml` (e.g. the layout path) triggers a full page reload.
