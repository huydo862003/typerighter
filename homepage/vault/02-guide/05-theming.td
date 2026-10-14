---
_label: "Theming"
_icon: icon.brush
---

# Theming

Typerighter's default theme works out of the box. This guide walks through the three levels of customization: custom components, layout slots, and full layout replacement.

## Custom components

Register Vue components so they can be used directly inside `.td` content files.

1. Create a `components/` directory next to your `typedown.yaml`
2. Add `.vue` files to it
3. Register the directory in `typedown.yaml`:

```yaml
theme:
  components:
    - "./components"
```

Now use them in any `.td` file with the container directive syntax:

```td
:::my-alert{kind="warning"}
Watch out for this.
:::
```

The component name is converted from kebab-case (`my-alert`) to PascalCase (`MyAlert`) for matching.

## Layout slots

If you want to add content to specific zones of the default layout without replacing it, use layout slots.

```yaml
theme:
  components:
    - "./components"
  layout:
    slots:
      layout_top: "AnnouncementBar"
      doc_after: "FeedbackWidget"
```

Each slot name maps to a position in the default theme (see the [slot reference](../03-references/06-theming)). The value is the name of a registered component.

## Custom layout

For full control over every pixel, replace the default layout with your own Vue component.

```yaml
theme:
  layout: "./theme/Layout.vue"
```

Your layout component imports composables from `typerighter/client`:

```vue
<script setup>
import { Content, useTdContent, useRoute, useSiteConfig, useTheme } from 'typerighter/client'

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

See the [Client API reference](../03-references/07-client-api) for all composables and their return types.

## Dark mode

Typerighter manages dark mode state via `useTheme()`. It persists the user's choice in `localStorage` and toggles a `.dark` class on `<html>`.

Your CSS can target `.dark` for dark mode styling:

```css
:root  { --bg: #fff; --fg: #1a1a1a; }
.dark  { --bg: #1b1b1f; --fg: #e4e4e7; }

body { background: var(--bg); color: var(--fg); }
```

The three modes are `'light'`, `'dark'`, and `'system'` (follows OS preference, the default).
