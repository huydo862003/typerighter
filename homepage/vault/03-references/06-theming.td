---
_label: "Theming"
_icon: icon.brush
description: "Custom layouts, layout slots, and the theme API."
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
useTdContent() // no argument - call once in layout setup
</script>

<template>
  <main>
    <h1 v-if="route.data?.title">{{ route.data.title }}</h1>
    <Content />
  </main>
</template>
```

### Client API

All composables and the `<Content />` component are imported from `typerighter/client`. See the [Client API reference](07-client-api) for full details.

| Composable | Notes |
|---|---|
| `useRoute()` | Reactive current route with `path`, `hash`, `data` |
| `useSiteConfig()` | Site title, nav, repo, `withBase(path)` |
| `useSiteData()` | Content tree, schemas, directory listings |
| `useTdContent()` | Must call in layout setup |
| `useTheme()` | Dark mode: `isDark`, `toggle()`, `setTheme()` |
| `useRouter()` | Programmatic navigation with `router.go(path)` |

`useTdContent()` must be called before `<Content />` renders. It establishes the provide/inject context the content component depends on.

### Reusing default components

Import individual components from the default theme:

```ts
import { TdLayout, TdContentNav, TdThemeToggle } from 'typerighter/client/theme-default'
```

## Theme API

Both layout forms use the `TypedownTheme` interface:

```ts
interface TypedownTheme {
  Layout: Component
  NotFound?: Component
  enhanceApp?: (ctx: {
    app: App
    router: Router
    siteConfig: TypedownSiteConfig
    siteData: Ref<TypedownSiteData>
  }) => void | Promise<void>
}
```

`enhanceApp` runs after the Vue app is created but before it mounts. Use it to register global plugins, directives, or additional components.

## Repository config

The `repo` field links the edit page button to your repository. It accepts a string shorthand or an object:

```yaml
# String shorthand (typedown.yaml at repo root)
repo: "https://github.com/user/repo"

# Object form (typedown.yaml nested in a subdirectory)
repo:
  url: "https://github.com/user/repo"
  root: "docs"
```

The `root` field is the path from the git root to the directory containing `typedown.yaml`. The edit link is constructed as `{repo.url}/edit/main/{repo.root}/{vault.root_dir}/{filepath}`.
