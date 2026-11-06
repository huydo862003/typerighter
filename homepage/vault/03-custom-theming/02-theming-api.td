---
_label: "Theming"
_icon: icon.brush
description: "Custom layouts, layout slots, dark mode, and custom components."
---

Typerighter ships a default theme optimized for documentation. You can customize it incrementally with layout slots, or replace it entirely with your own layout component.

## Layout slots

The default layout has named slots where you can inject your own components. Declare slot mappings in `typedown.yaml` under `theme.layout.slots`:

```yaml
theme:
  components:
    - "./components"
  layout:
    slots:
      layout_top: "AnnouncementBar"
      doc_after: "FeedbackWidget"
```

Each key is a slot name and each value is the PascalCase name of a registered component (from `theme.components`).

### Available slots

| Slot | Position |
|------|----------|
| `layout_top` | Above the header |
| `nav_bar_after` | After the navigation links in the header |
| `sidebar_top` | Top of the sidebar, before search |
| `sidebar_bottom` | Bottom of the sidebar, after the navigation tree |
| `doc_before` | Top of the article, before the page title |
| `doc_content_before` | After title and frontmatter, before the content body |
| `doc_content_after` | After the content body, before prev/next navigation |
| `doc_after` | After prev/next and the edit link |
| `layout_bottom` | Below the footer |

Slot names use underscores in the YAML config because hyphens are reserved in Typedown's expression syntax.

## Custom layout

For full control, replace the default layout with your own Vue component:

```yaml
theme:
  layout: "./theme/MyLayout.vue"
```

A minimal custom layout:

```vue
<script setup>
import { Content, useTdContent, useRoute } from 'typerighter/client'

const route = useRoute()
useTdContent()
</script>

<template>
  <main>
    <h1 v-if="route.data?.title">{{ route.data.title }}</h1>
    <Content />
  </main>
</template>
```

`useTdContent()` must be called in your layout's `setup` before `<Content />` renders. See the [Client API reference](08-client-api) for all available composables and components.

### Reusing default components

You can import individual components from the default theme:

```ts
import { TdLayout, TdContentNav, TdThemeToggle } from 'typerighter/client/theme-default'
```

## Custom components

Register components globally for use in `.td` files and layout slots:

```yaml
theme:
  components:
    - "./components"        # scan directory for .vue files
    - name: MyCard          # explicit entry
      source: "./ui/Card.vue"
```

Components registered this way are available in any `.td` file without importing, and can be referenced by name in `layout.slots`.

## Dark mode

Typerighter manages dark mode with the `useTheme()` composable. It persists the user's preference in `localStorage` and toggles a `.dark` class on `<html>`.

```vue
<script setup>
import { useTheme } from 'typerighter/client'

const { isDark, toggle, setTheme } = useTheme()
</script>

<template>
  <button @click="toggle">
    {{ isDark ? 'Light' : 'Dark' }}
  </button>
</template>
```

To style for dark mode in CSS, target the `.dark` class:

```css
:root  { --bg: #fff; --fg: #111; }
.dark  { --bg: #111; --fg: #eee; }
```
