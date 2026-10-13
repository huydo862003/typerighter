---
_label: "Components"
---

# Components

## Content

Renders the markdown body of the current page. Import from `typerighter/client`.

```vue
<script setup>
import { Content, useTdContent } from 'typerighter/client'

useTdContent() // required before Content renders
</script>

<template>
  <Content />
</template>
```

`<Content />` uses Vue's `<component :is="..." />` to mount the compiled SFC for the current route. It has no props.

### Requirements

`useTdContent()` must be called before `<Content />` is rendered. It sets up the provide/inject context (`tdContentKey`) that the content component reads from. If you call `<Content />` without `useTdContent()`, it throws at runtime.

### Applying content styles

`<Content />` renders raw HTML elements (`h1`, `p`, `code`, `table`, etc.). Style them by targeting a wrapper class:

```vue
<template>
  <article class="prose">
    <Content />
  </article>
</template>

<style>
.prose h1 { font-size: 2rem; }
.prose p  { line-height: 1.7; }
.prose code { background: #f3f4f6; }
</style>
```

Typerighter adds a `td-content` class to the wrapper element in the default theme. You can use the same class in your own layout for consistency.

---

## Global components

Components registered in `typedown.yaml` under `theme.components` are available directly inside `.td` content files without importing.

```yaml
theme:
  components:
    - "./components"
```

Any `.vue` file in `./components/` is registered globally as its filename (without extension).

```td
## My page

<MyCard title="Hello">
  Content here.
</MyCard>
```

### Explicit registration

```yaml
theme:
  components:
    - name: FeatureCard
      source: "./ui/FeatureCard.vue"
```

### Built-in components

The following components are registered automatically by Typerighter:

| Component | Description |
|---|---|
| `TdDirectoryIndex` | Renders a directory listing page |
| `TdGlossaryIndex` | Renders a glossary from schema instances |
| `TdFlashcard` | Flashcard component for review content |
| `TdLucideIcon` | Renders any Lucide icon by name |
