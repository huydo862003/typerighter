---
_label: "Client API"
_icon: icon.code
description: "Composables and components exported from typerighter/client for use in custom layouts."
---

When building a custom layout, you need access to page data, routing, and site configuration. The `typerighter/client` module exports composables and components for this:

```ts
import {
  Content,
  useRoute,
  useRouter,
  useSiteConfig,
  useSiteData,
  useTdContent,
  useTheme,
} from 'typerighter/client'
```

## useRoute

Returns the current route as a reactive object. Updates on every navigation.

```ts
const route = useRoute()

route.path    // "/guide/introduction"
route.hash    // "#getting-started"
route.data    // PageData for the current page
```

### PageData

```ts
interface PageData {
  title: string
  titleHtml: string              // title with inline markup, safe for v-html
  headings: MarkdownHeading[]    // nested heading tree (h1 to h6)
  frontmatter: Record<string, unknown>
  schema?: string                // schema type name if page uses a schema
  schemaLabel?: string
  label?: string                 // _label override
  filepath: string               // path relative to vault root, e.g. "guide/intro.td"
  fileMetadata?: { mtime: number; ctime: number }
}
```

### MarkdownHeading

```ts
interface MarkdownHeading {
  level: number           // 1 to 6
  title: string           // plain text
  titleHtml: string       // HTML with inline code/math, safe for v-html
  slug: string            // anchor id (deduplicated)
  link: string            // e.g. "#getting-started"
  children: MarkdownHeading[]
}
```

Headings are returned as a nested tree. To flatten for a TOC:

```ts
function flattenHeadings(items: MarkdownHeading[], depth = 2) {
  const result = []
  for (const h of items) {
    if (h.level <= depth) result.push(h)
    result.push(...flattenHeadings(h.children, depth))
  }
  return result
}
```


## useSiteConfig

Returns site-wide configuration from `typedown.yaml`. Reactive across HMR config changes.

```ts
const config = useSiteConfig()
```

```ts
interface TypedownSiteConfig {
  title: string
  description: string
  basePath: string                          // e.g. "/" or "/docs"
  origin?: string                           // e.g. "https://example.com"
  lang: string                              // HTML lang attribute
  rootDir: string                           // vault root directory name
  repo?: { url: string; root: string }
  author?: string
  license?: string
  nav: { title: string; link: string; icon?: string }[]
  withBase(path: string): string            // prepends basePath to a path
}
```

`withBase` is useful for generating hrefs that work under a subpath deployment:

```ts
const { withBase } = useSiteConfig()

withBase('/guide/index')  // returns "/docs/guide/index" if basePath is "/docs"
```


## useSiteData

Returns a reactive `Ref` containing the full vault graph. Updates via HMR when content changes. Use this to query pages, schemas, and directory contents from a custom layout or component.

```ts
const siteData = useSiteData()

// wait for initial load before accessing content
if (!siteData.value.ready) return
```

```ts
interface TypedownSiteData {
  ready: boolean
  contentTree: ContentTree
  schemas: Record<string, SchemaInfo>
  directoryListings: Record<string, DirectoryListing>
}
```

### ContentTree

The full set of published pages, keyed by route path.

```ts
type ContentTree = Record<string, ContentNode>

interface ContentNode {
  route: string                        // e.g. "/people/alice"
  filepath: string                     // path relative to vault root
  title: string
  label?: string                       // _label override
  icon?: string                        // _icon value
  schema?: string                      // schema type name
  frontmatter: Record<string, unknown>
  children: string[]                   // child routes for directory nodes
}
```

Example - list all pages that use a given schema:

```ts
const nodes = Object.values(siteData.value.contentTree)
const articles = nodes.filter(node => node.schema === 'Article')
```

### SchemaInfo

Metadata about a schema defined in `_types/`.

```ts
interface SchemaInfo {
  name: string
  label?: string
  fields: SchemaField[]
}

interface SchemaField {
  name: string
  type: string        // e.g. "string", "number", "boolean", "fref", schema name
  required: boolean
  description?: string
}
```

Example - render a table of all fields for the current page's schema:

```ts
const { page } = useTdContent()
const schema = computed(() =>
  page.value.schema
    ? siteData.value.schemas[page.value.schema]
    : undefined
)
```

### DirectoryListing

The ordered list of child pages for a directory node. Keyed by directory route.

```ts
type DirectoryListing = Record<string, DirectoryEntry[]>

interface DirectoryEntry {
  route: string
  title: string
  label?: string
  icon?: string
  description?: string    // frontmatter.description
  schema?: string
}
```

Example - render a card grid of sibling pages in a custom layout:

```ts
const { page } = useTdContent()
const siblings = computed(() => {
  const dir = page.value.filepath.split('/').slice(0, -1).join('/')
  return siteData.value.directoryListings[`/${dir}`] ?? []
})
```


## useTdContent

Sets up the provide/inject context required by `<Content />` and returns reactive page bindings. Must be called once in your layout's `setup` function.

```ts
const { page, title, frontmatter, isDark } = useTdContent()
```

```ts
interface TypedownData {
  page: Ref<PageData>
  frontmatter: Ref<Record<string, unknown>>
  title: Ref<string>
  isDark: Ref<boolean>
}
```

`isDark` reflects the current theme state, derived from `useTheme()` internally.


## useTheme

Controls the dark mode state. Persists the user's preference in `localStorage` and toggles the `.dark` class on `<html>`.

```ts
const { isDark, toggle, setTheme, effectiveTheme, theme } = useTheme()
```

```ts
{
  theme: Readonly<Ref<'light' | 'dark' | 'system'>>
  effectiveTheme: ComputedRef<'light' | 'dark'>   // resolved value when mode is 'system'
  isDark: ComputedRef<boolean>
  toggle(): void
  setTheme(value: 'light' | 'dark' | 'system'): void
}
```

`'system'` is the default. It reads `prefers-color-scheme` and updates reactively when the OS setting changes.

```vue
<button @click="toggle">Toggle dark mode</button>
```

To style for dark mode in CSS, target the `.dark` class on `<html>`:

```css
:root       { --bg: #fff; --fg: #111; }
.dark       { --bg: #111; --fg: #eee; }
```


## useRouter

Returns the router for programmatic navigation.

```ts
const router = useRouter()

await router.go('/guide/introduction')
await router.go('/guide/introduction#section', { replace: true })
```

```ts
interface Router {
  route: Route
  go(to: string, options?: { replace?: boolean }): Promise<void>
  onBeforeRouteChange?: (to: string) => Promise<boolean | undefined> | boolean | undefined
  onAfterRouteChange?: (to: string) => Promise<void> | void
}
```

## Content

The `<Content />` component renders the compiled markdown body of the current page. It has no props.

```vue
<template>
  <article class="prose">
    <Content />
  </article>
</template>
```

`useTdContent()` must be called before `<Content />` renders.

Rendered output consists of standard HTML elements (`h1`-`h6`, `p`, `ul`, `pre`, `table`, etc.). Style them by wrapping with a class:

```css
.prose h1 { font-size: 2rem; font-weight: 800; }
.prose p  { line-height: 1.7; color: #555; }
.prose pre { background: #f5f5f5; border-radius: 8px; padding: 16px; }
```
