---
_label: "Home"
---

## Client API

Everything you need to build a custom theme is exported from `typerighter/client`.

```ts
import {
  Content,
  useRoute,
  useSiteConfig,
  useSiteData,
  useTdContent,
  useTheme,
} from 'typerighter/client'
```

## Composables

| Composable | Returns | Description |
|---|---|---|
| `useRoute()` | `Route` | Current route: `path`, `hash`, `data` (title, headings, frontmatter) |
| `useSiteConfig()` | `TypedownSiteConfig` | Site title, description, basePath, repo, nav, `withBase(path)` |
| `useSiteData()` | `Ref<TypedownSiteData>` | Content tree, schemas, directory listings |
| `useTdContent()` | `TypedownData` | Page bindings: `page`, `title`, `frontmatter`, `isDark` - **must call in layout setup** |
| `useTheme()` | Theme controls | `isDark`, `toggle()`, `setTheme('light' \| 'dark' \| 'system')` |
| `useRouter()` | `Router` | Programmatic navigation: `router.go(path)` |

## Components

| Component | Description |
|---|---|
| `<Content />` | Renders the markdown body of the current page. Mount once in your layout. |

## Route shape

```ts
interface Route {
  path: string
  hash: string
  data: PageData
}

interface PageData {
  title: string
  headings: MarkdownHeading[]
  frontmatter: Record<string, unknown>
  schema?: string
  label?: string
}
```

## TypedownData

`useTdContent()` provides and injects reactive page bindings. **You must call it once in your layout's `setup`** - it uses Vue's provide/inject to make data available to `<Content />` and child components.

```ts
interface TypedownData {
  page: Ref<PageData>
  frontmatter: Ref<Record<string, unknown>>
  title: Ref<string>
  isDark: Ref<boolean>
}
```

## Minimal layout

```vue
<script setup>
import { Content, useTdContent, useRoute, useSiteConfig } from 'typerighter/client'

const route = useRoute()
const { title } = useSiteConfig()

useTdContent() // required
</script>

<template>
  <header>{{ title }}</header>
  <main>
    <h1>{{ route.data.title }}</h1>
    <Content />
  </main>
</template>
```
