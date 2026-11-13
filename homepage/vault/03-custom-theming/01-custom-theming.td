---
_label: "Custom Theming"
_icon: icon.brush
description: "Extend or replace the Typedown default theme: custom Vue components, layout slots, and full layout replacement."
---

Typerighter's default theme works out of the box. When you need to go further, there are three levels of customization.

::: note
All three levels involve writing Vue single-file components (`.vue` files). Familiarity with [Vue.js](https://vuejs.org/) and its Composition API is assumed in this section.
:::

- **Custom components** - register Vue components for use inside `.td` content. Start here if you need a one-off widget like a chart or alert box.
- **Layout slots** - inject content into specific areas of the default layout. Use this to add a banner, footer widget, or analytics without changing the overall design.
- **Full layout replacement** - swap the entire layout with your own Vue component. Use this when you want complete design control.

## Custom components

Register Vue components so they can be used directly inside `.td` content files. Create a `components/` directory next to your `typedown.yaml`, add `.vue` files to it, and register the directory in the config.

Here is a complete example - an alert component that accepts a `kind` prop:

```yaml{title=typedown.yaml}
theme:
  components:
    - "./components"
```

```vue{title=components/MyAlert.vue}
<script setup>
defineProps({
  kind: { type: String, default: 'info' },
})
</script>

<template>
  <div :class="['alert', `alert-${kind}`]">
    <slot />
  </div>
</template>

<style scoped>
.alert { padding: 12px 16px; border-radius: 6px; }
.alert-info { background: #e8f4fd; color: #1a5276; }
.alert-warning { background: #fef3cd; color: #856404; }
</style>
```

Use it in any `.td` file with the container directive syntax. The component name is converted from kebab-case (`my-alert`) to PascalCase (`MyAlert`) for matching:

```td
:::my-alert{kind="warning"}
Watch out for this.
:::
```

The inner markdown is passed as the default slot, so headings, lists, and code blocks all work inside the container.

## Layout slots

If you want to add content to specific zones of the default layout without replacing it, use layout slots. Each slot name maps to a position in the default theme - see the [slot reference](02-theming-api) for the full list.

Here is a complete example that adds an announcement bar at the top of every page:

```yaml{title=typedown.yaml}
theme:
  components:
    - "./components"
  layout:
    slots:
      layout_top: "AnnouncementBar"
```

```vue{title=components/AnnouncementBar.vue}
<script setup>
import { ref } from 'vue'

const dismissed = ref(false)
</script>

<template>
  <div v-if="!dismissed" class="banner">
    <span>Typerighter v1.0 is out!</span>
    <button @click="dismissed = true">Dismiss</button>
  </div>
</template>
```

The value of each slot is the PascalCase name of a registered component (from `theme.components`).

## Custom layout

For full control over every pixel, replace the default layout with your own Vue component.

```yaml{title=typedown.yaml}
theme:
  layout: "./theme/Layout.vue"
```

Your layout component imports composables from `typerighter/client`:

```vue{title=theme/Layout.vue}
<script setup>
import {
  Content,
  useTdContent,
  useRoute,
  useSiteConfig,
  useTheme,
} from 'typerighter/client'

const route = useRoute()
const { title } = useSiteConfig()
const { isDark, toggle } = useTheme()

useTdContent()
</script>

<template>
  <header>
    <a href="/">{{ title }}</a>
    <button @click="toggle">{{ isDark ? 'Light' : 'Dark' }}</button>
  </header>
  <main>
    <h1>{{ route.data.title }}</h1>
    <Content />
  </main>
</template>
```

Key points:

- `useTdContent()` must be called in `setup` before `<Content />` renders
- `useRoute()` gives you the current page's title, headings, and frontmatter
- `useSiteConfig()` gives you the site title, nav links, and `withBase()` for subpath URLs
- `useTheme()` controls dark mode with `toggle()` and `setTheme()`
- No default CSS is loaded; you style everything yourself

See the [Client API reference](03-client-api) for all composables and their return types.

## Dark mode

Typerighter manages dark mode state via `useTheme()`, which persists the user's choice in `localStorage` and toggles a `.dark` class on `<html>`.

Your CSS can target `.dark` for dark mode styling:

```css
:root  { --bg: #fff; --fg: #1a1a1a; }
.dark  { --bg: #1b1b1f; --fg: #e4e4e7; }

body { background: var(--bg); color: var(--fg); }
```

The three modes are `'light'`, `'dark'`, and `'system'` (follows OS preference, the default).
