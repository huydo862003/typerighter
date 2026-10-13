---
_label: "Theming"
---

# Theming

## Dark mode

Typerighter manages dark mode state with `useTheme()`. It persists the user's preference in `localStorage` and toggles a `.dark` class on `<html>`.

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

### Theme values

```ts
setTheme('light')   // always light
setTheme('dark')    // always dark
setTheme('system')  // follow OS preference (default)
```

### Reacting to dark mode in CSS

Since `.dark` is toggled on `<html>`, you can use CSS custom properties:

```css
:root {
  --bg: #ffffff;
  --fg: #1a1a1a;
}

.dark {
  --bg: #1b1b1f;
  --fg: #e4e4e7;
}
```

Or with Tailwind v4:

```css
@theme {
  --color-bg: #ffffff;
}

.dark {
  --color-bg: #1b1b1f;
}
```

## Layout slots

If you only need to inject content into specific zones of the default theme (rather than replacing it entirely), use layout slots:

```yaml
theme:
  layout:
    slots:
      sidebar-top: MyAnnouncementBanner
      doc-before: MyPageHeader
      doc-after: MyCommentSection
```

Available slots:

| Slot | Position |
|---|---|
| `layout-top` | Above everything |
| `nav-bar-after` | After the header nav bar |
| `sidebar-top` | Top of the sidebar |
| `sidebar-bottom` | Bottom of the sidebar |
| `doc-before` | Before the doc content area |
| `doc-content-before` | Before article content |
| `doc-content-after` | After article content |
| `doc-after` | After the doc content area |
| `layout-bottom` | Below everything |

Each slot value is a component name registered in `theme.components`.

## Custom components

Register components globally for use in `.td` files:

```yaml
theme:
  components:
    - "./components"        # scan directory for .vue files
    - name: MyCard          # explicit entry
      source: "./ui/Card.vue"
```

Components registered this way are available in any `.td` file's content without importing.

## CSS

Custom layouts have no default CSS. Load what you need:

```vue
<style>
/* Global reset and tokens */
@import './tokens.css';
</style>
```

Or use Tailwind v4 with `@theme` blocks in your layout's `<style>` tag.
