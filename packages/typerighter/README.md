# typerighter

See [README.npm.md](README.npm.md) for the package documentation, or visit the [homepage](https://huydo862003.github.io/typerighter/).

## Technical Debts

- Currently, fake index page for directories are handled a bit adhocly, causing them to be easily missed... However, I expect the core logic of the plugin to changes slowly so this can be accepted for now.

## Learnt Lessons

### Hydration Flash (Flash of Incorrect State)

For SSR pages, the full initial render is prerendered to HTML and sent to the client, then hydrated by Vue.

The prerendered HTML lacks client-only state (local storage, cookies, etc.), so the initial UI can differ from what the client should show on first render. Vue detects this as a hydration mismatch and corrects it immediately after mount, causing visible jitters and layout shifts.

I encountered this in this project: ThemeToggle getting flipped right after hydration, Sidebar getting its width shifted after hydration.

My approach:
- Generate a blocking `<script>` in `<head>` that reads from local storage and sets CSS variables before paint.
- Components use those CSS variables instead of inlining state, so SSR emits no value and the script-set CSS takes effect immediately on first paint.

This completely eliminates the problem.

See: https://github.com/huydo862003/typerighter/commit/5139adc8cf2b87f6ef15eb11a9c1c3a7fc88e7e5

Next step: Can we abstract this?

### Skipping SSR for Collapsed Subtrees

Vue SSR renders the full component tree to an HTML string regardless of CSS-driven visibility. If you use a CSS-only collapse (e.g. `grid-template-rows: 0fr`) to hide children, those children still appear in the SSR output and inflate HTML size and render time.

The fix is to guard the children with `v-if` using `import.meta.env.SSR`, a Vite compile-time constant that is `true` in the SSR bundle and `false` in the client bundle:

```vue
<script setup lang="ts">
// Replaced by Vite at build time: true in SSR bundle, false in client bundle
const isSSR = import.meta.env.SSR;
</script>

<template>
  <!-- Skip rendering collapsed children during SSR; always render on client for CSS animation -->
  <div
    v-if="!isSSR || !collapsed"
    class="td-collapse td-tree-children"
    :class="{ 'is-collapsed': collapsed }"
  >
    ...
  </div>
</template>
```

In the SSR bundle this becomes `v-if="!collapsed"`, skipping collapsed subtrees entirely. In the client bundle this becomes `v-if="true"`, always rendering so CSS transitions work normally. Because the initially expanded subtrees are identical between SSR and client, there is no hydration mismatch.
