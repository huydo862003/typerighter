---
_label: "Vault query API"
_icon: icon.database
description: "Query the full vault graph at runtime: pages, schemas, and directory listings."
---

The vault graph is available at runtime through the `useSiteData` composable exported from `typerighter/client`. It gives components and custom layouts access to the full set of published pages, their frontmatter, schema definitions, and directory contents.

This is useful for building custom index pages, related-content widgets, schema browsers, or any component that needs to aggregate across multiple documents.

```ts
import { useSiteData } from 'typerighter/client'

const siteData = useSiteData()

// wait for initial data before accessing content
watchEffect(() => {
  if (!siteData.value.ready) return
  // safe to query here
})
```

## ContentTree

The full set of published pages, keyed by route path.

```ts
type ContentTree = Record<string, ContentNode>

interface ContentNode {
  route: string                        // e.g. "/people/alice"
  filepath: string                     // path relative to vault root
  title: string
  label?: string                       // _label override
  icon?: string                        // _icon value
  schema?: string                      // schema type name, if set
  frontmatter: Record<string, unknown>
  children: string[]                   // child routes for directory nodes
}
```

Example - list all pages that use a given schema:

```ts
const nodes = Object.values(siteData.value.contentTree)
const people = nodes.filter(node => node.schema === 'Person')
```

Example - find all pages tagged with a keyword (assumes `tags: list[string]` in the schema):

```ts
const tagged = nodes.filter(node =>
  Array.isArray(node.frontmatter.tags) &&
  node.frontmatter.tags.includes('backend')
)
```

## SchemaInfo

Metadata about a schema defined in `_types/`, keyed by schema name.

```ts
type Schemas = Record<string, SchemaInfo>

interface SchemaInfo {
  name: string
  label?: string
  fields: SchemaField[]
}

interface SchemaField {
  name: string
  type: string        // "string", "number", "boolean", "fref", or a schema name
  required: boolean
  description?: string
}
```

Example - render all fields for the current page's schema:

```ts
const { page } = useTdContent()
const schema = computed(() =>
  page.value.schema
    ? siteData.value.schemas[page.value.schema]
    : undefined
)
```

## DirectoryListings

The ordered list of child pages for each directory, keyed by directory route. The order reflects the numeric prefix ordering used in the vault (e.g. `01-`, `02-`).

```ts
type DirectoryListings = Record<string, DirectoryEntry[]>

interface DirectoryEntry {
  route: string
  title: string
  label?: string
  icon?: string
  description?: string    // frontmatter.description if present
  schema?: string
}
```

Example - render a card grid of sibling pages:

```ts
const { page } = useTdContent()

const siblings = computed(() => {
  const dirRoute = '/' + page.value.filepath.split('/').slice(0, -1).join('/')
  return siteData.value.directoryListings[dirRoute] ?? []
})
```

Example - build a custom index for a directory:

```vue
<script setup lang="ts">
import { useSiteData, useTdContent } from 'typerighter/client'

const siteData = useSiteData()
const { page } = useTdContent()

const entries = computed(() =>
  siteData.value.directoryListings['/' + page.value.filepath.replace(/\/index\.td$/, '')] ?? []
)
</script>

<template>
  <div class="card-grid">
    <a v-for="entry in entries" :key="entry.route" :href="entry.route" class="card">
      <span v-if="entry.icon" class="card-icon">
        <LucideIcon :name="entry.icon.replace('icon.', '')" />
      </span>
      <strong>{{ entry.label ?? entry.title }}</strong>
      <p v-if="entry.description">{{ entry.description }}</p>
    </a>
  </div>
</template>
```
