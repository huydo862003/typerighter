---
_label: "Composables"
---

# Composables

All composables are imported from `typerighter/client`.

## useRoute

Returns the current route as a reactive object.

```ts
import { useRoute } from 'typerighter/client'

const route = useRoute()
```

### Route shape

```ts
interface Route {
  path: string             // current URL path, e.g. "/guide/introduction"
  hash: string             // URL hash, e.g. "#getting-started"
  query: string            // URL query string
  data: PageData           // current page data
}
```

### PageData shape

```ts
interface PageData {
  title: string                       // page title (from first h1 or _label)
  titleHtml: string                   // HTML-rendered title (safe for v-html)
  headings: MarkdownHeading[]         // nested heading tree
  frontmatter: Record<string, unknown>
  schema?: string                     // schema type name
  schemaLabel?: string                // human-readable schema label
  label?: string                      // display label from _label
  filepath: string                    // source file path relative to vault root
}
```

### MarkdownHeading shape

```ts
interface MarkdownHeading {
  level: number           // 1 to 6
  title: string           // plain text
  titleHtml: string       // HTML with inline code/math
  slug: string            // anchor id
  link: string            // e.g. "#my-heading"
  children: MarkdownHeading[]
}
```

---

## useSiteConfig

Returns site-wide configuration from `typedown.yaml`.

```ts
import { useSiteConfig } from 'typerighter/client'

const config = useSiteConfig()
```

### Return shape

```ts
interface TypedownSiteConfig {
  title: string
  description: string
  basePath: string
  origin?: string
  lang: string
  rootDir: string
  repo?: { url: string; root: string }
  author?: string
  license?: string
  nav: NavItem[]
  withBase(path: string): string     // prepend basePath to a path
}
```

---

## useSiteData

Returns reactive site data including the content tree and directory listings.

```ts
import { useSiteData } from 'typerighter/client'

const siteData = useSiteData()
```

`siteData` is a `Ref<TypedownSiteData>`. Access with `siteData.value`.

```ts
interface TypedownSiteData {
  ready: boolean
  contentTree: ContentTree
  schemas: Record<string, SchemaInfo>
  directoryListings: Record<string, DirectoryListing>
}
```

---

## useTdContent

Provides page-level reactive bindings. **Must be called once in your layout's setup function.**

```ts
import { useTdContent } from 'typerighter/client'

const { page, title, frontmatter, isDark } = useTdContent()
```

### Return shape

```ts
interface TypedownData {
  page: Ref<PageData>
  frontmatter: Ref<Record<string, unknown>>
  title: Ref<string>
  isDark: Ref<boolean>
}
```

---

## useTheme

Controls dark mode state. Persists to `localStorage` and toggles `.dark` on `<html>`.

```ts
import { useTheme } from 'typerighter/client'

const { isDark, toggle, setTheme, effectiveTheme } = useTheme()
```

### Return shape

```ts
{
  theme: Readonly<Ref<'light' | 'dark' | 'system'>>
  effectiveTheme: ComputedRef<'light' | 'dark'>  // resolved value when system
  isDark: ComputedRef<boolean>
  toggle(): void
  setTheme(theme: 'light' | 'dark' | 'system'): void
}
```

---

## useRouter

Returns the router for programmatic navigation.

```ts
import { useRouter } from 'typerighter/client'

const router = useRouter()
await router.go('/guide/introduction')
```
