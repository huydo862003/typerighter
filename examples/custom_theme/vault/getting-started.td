---
_label: "Getting started"
---

## Create a custom layout

1. Create a Vue component that serves as your layout
2. Import composables from `typerighter/client`
3. Use `<Content />` to render the markdown body
4. Set `theme.layout` in your `typedown.yaml`

## Available composables

| Composable | Returns |
|------------|---------|
| `useRoute()` | Current route with `path`, `data` (title, headings, frontmatter) |
| `useSiteConfig()` | Site config: title, description, repo, nav |
| `useSiteData()` | Content tree, schemas, directory listings |
| `useTdContent(route)` | Provides content rendering context (call in setup) |

## Minimal layout

```vue
<script setup>
import { Content, useTdContent, useRoute } from 'typerighter/client'
const route = useRoute()
useTdContent(route)
</script>

<template>
  <main>
    <Content />
  </main>
</template>
```
