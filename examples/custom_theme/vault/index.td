---
_label: "Home"
---

Welcome to the custom theme example. This site uses a completely custom layout that replaces the default Typerighter theme.

## What this demonstrates

- A custom `Layout.vue` component that controls the entire page structure
- Using `useRoute()`, `useSiteConfig()`, and `useTdContent()` composables from the client API
- The `<Content />` component renders the markdown body
- Full control over header, footer, and page layout

## How it works

In `typedown.yaml`:

```yaml
theme:
  layout: "./theme/Layout.vue"
```

The layout component imports composables from `typerighter/client`:

```ts
import { Content, useTdContent, useRoute, useSiteConfig } from 'typerighter/client'
```

No default theme CSS is loaded. You own every pixel.
