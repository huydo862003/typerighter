---
_label: "Vite Plugin"
_icon: icon.document
---

References: [vite.dev/guide/api-plugin](https://vite.dev/guide/api-plugin).

## Overview

Vite's plugin interface is just Rolldown's interface with a few extra hooks on top. So if you can write a Rolldown plugin, you can write a Vite plugin.

The extra hooks are mostly about dev server integration: HMR, configuring the dev server, etc. The core hooks (`resolveId`, `load`, `transform`) are all inherited from Rolldown.

It is recommended to read the [Rolldown plugin page](./rolldown-plugin.md) first, as the fundamentals are defined there.

## Plugin Config

Users will typically do this to register a plugin to Vite:

1. Add plugins to `devDependencies`.
2. Configure them via the `plugins` array in `vite.config.js`:

```js
import vitePlugin from "vite-plugin-feature";
import rollupPlugin from "rollup-plugin-feature";

export default defineConfig({
  plugins: [vitePlugin(), rollupPlugin()],
});
```

> Remark: Rollup plugins work in Vite too, since Vite's plugin interface is a superset of Rolldown's (which is Rollup-compatible).

> Convention: It is common to author a plugin as a factory function that returns the actual plugin object. The function accepts options, allowing users to customize the plugin's behavior. This is why plugins are called as `vitePlugin()` rather than passed as `vitePlugin`.

### Falsy Plugins

Falsy values in the `plugins` array are silently ignored. This is useful for conditionally enabling plugins:

```js
plugins: [isDev && myDevPlugin()];
```

### Presets

> Definition: Preset is like an opinionated bundle of plugins.

`plugins` also accepts **presets**: A single element that is itself an array of plugins. Vite flattens the array internally, so the user just calls one function. This is the recommended pattern for complex integrations like framework plugins:

```js
// framework-plugin package
export default function framework(config) {
  return [frameworkRefresh(config), frameworkDevtools(config)];
}
```

```js
// vite.config.js
import framework from "vite-plugin-framework";

export default defineConfig({
  plugins: [framework()],
});
```

> Remark: From the user's perspective, this is identical to a single plugin. The fact that it expands to multiple plugins is an implementation detail of the preset.

## Universal Hooks

During dev, the Vite dev server creates a **plugin container** (?) that invokes Rolldown build hooks the same way Rolldown does.

> A plugin container is an internal Vite abstraction that implements the same plugin runner interface as Rolldown.
> Since Vite doesn't bundle during dev (it serves modules individually on demand), the plugin container is what lets Rolldown-compatible plugins still work: it intercepts each module request and runs the relevant hooks (`resolveId`, `load`, `transform`) as if Rolldown were processing the module.

The following hooks are called once on **server start**:

- `options`
- `buildStart`

The following hooks are called on each **incoming module request**:

- `resolveId`
- `load`
- `transform`

> These hooks also have an extended options parameter with additional Vite-specific properties.

> Remark: Regarding `importer`...
>
> - Context, `importer` as in `resolveId(source, importer)`: the file containing the `import` statement being resolved.
> - In Rolldown, this is always known since the full graph is analyzed upfront.
> - In Vite's dev server, it is sometimes unknown (modules are processed on demand), so Vite falls back to the root `index.html` path.
> - For imports going through Vite's resolve pipeline, the real importer is available via import analysis.
>   Takeaways: Plugin authors should not assume `importer` is always accurate during dev.

The following hooks are called when **the server is closed**:

- `buildEnd`
- `closeBundle`

Some notes:

- `moduleParsed` is not called during dev, because Vite avoids full AST parses for better performance.
- Output generation hooks (except `closeBundle`) are not called during dev.

## Vite-Specific Hooks

These hooks are only meaningful to Vite and are ignored by Rollup.

| Hook                     | Kind                                                                                                       | Purpose                                                                                                                       |
| ------------------------ | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `config`                 | [`async`](./rolldown-plugin.md#hook-kind-async), [`sequential`](./rolldown-plugin.md#hook-kind-sequential) | Modify the raw user config before it is resolved. Return a partial config to deep-merge, or mutate directly.                  |
| `configResolved`         | [`async`](./rolldown-plugin.md#hook-kind-async), [`parallel`](./rolldown-plugin.md#hook-kind-parallel)     | Called after config is fully resolved. Use this to read and store the final config for use in other hooks.                    |
| `configureServer`        | [`async`](./rolldown-plugin.md#hook-kind-async), [`sequential`](./rolldown-plugin.md#hook-kind-sequential) | Configure the dev server (e.g. add custom middlewares). Return a function to inject middleware after Vite's internal ones.    |
| `configurePreviewServer` | [`async`](./rolldown-plugin.md#hook-kind-async), [`sequential`](./rolldown-plugin.md#hook-kind-sequential) | Same as `configureServer` but for the preview server (`vite preview`).                                                        |
| `transformIndexHtml`     | [`async`](./rolldown-plugin.md#hook-kind-async), [`sequential`](./rolldown-plugin.md#hook-kind-sequential) | Transform HTML entry files (e.g. `index.html`). Can return a new HTML string, an array of tag descriptors to inject, or both. |
| `handleHotUpdate`        | [`async`](./rolldown-plugin.md#hook-kind-async), [`sequential`](./rolldown-plugin.md#hook-kind-sequential) | Custom HMR update handling. Can filter the affected module list, trigger a full reload, or send custom events to the client.  |

## Plugin Context Meta

`this.meta` is the plugin context's metadata object. Vite adds two extra fields to it:

- `this.meta.viteVersion`: Current Vite version string (e.g. `"8.0.0"`).
- `this.meta.rolldownVersion`: Only set on Rolldown-powered Vite (Vite 8+).

Use `this.meta.rolldownVersion` to branch behavior between Rolldown and Rollup backends.

## Output Bundle Metadata

`viteMetadata` is a Vite-specific field added to Rolldown's output objects (`RenderedChunk`, `OutputChunk`, `OutputAsset`) during build.

It exposes which CSS files and static assets a chunk imports:

- `viteMetadata.importedCss`: `Set<string>`
- `viteMetadata.importedAssets`: `Set<string>`

Use this when your plugin needs to inspect emitted assets per chunk without parsing `build.manifest`.

Access it inside output hooks like `generateBundle` or `renderChunk`.

## Plugin Ordering

User plugins can set `enforce: 'pre'` or `enforce: 'post'` to control where they run relative to Vite's internal plugins. The full resolved order is:

1. Alias resolution (Vite internal).
2. User plugins with `enforce: 'pre'`.
3. Vite core plugins (Vite internal).
4. User plugins with no `enforce` value.
5. Vite build plugins (Vite internal).
6. User plugins with `enforce: 'post'`.
7. Vite post-build plugins: minify, manifest, reporting (Vite internal).

> Remark: `enforce` and hook-level `order` are independent:
>
> - `enforce` affects all hooks of a plugin at once, placing the whole plugin into a bucket relative to Vite's internals.
> - Hook `order` is a per-hook override. Two plugins in the same `enforce` bucket can still use `order` to sequence a specific hook between themselves.

> Remark: Within each bucket, plugins run in the order they appear in the `plugins` array. `enforce` only determines which bucket a plugin is placed into.

## Conditional Application

`apply` is a **top-level field on the plugin object** that controls **which mode the plugin runs in**.

By default it runs in both `serve` and `build`.

- Set to `'build'` or `'serve'` to restrict to one mode.
- Set to a function returning a boolean for finer control:

```ts
apply(config, { command }) {
  return command === 'build' && !config.build.ssr
}
```

## Rolldown Plugin Compatibility

Most Rolldown/Rollup plugins work as Vite plugins. A plugin is compatible as long as:

- It does not use `moduleParsed`.
- It does not rely on Rolldown-specific options like `transform.inject`.
- It does not have strong coupling between bundle-phase and output-phase hooks.

For build-only Rolldown/Rollup plugins, place them under `build.rolldownOptions.plugins`. This is equivalent to `enforce: 'post'` + `apply: 'build'`.

You can also spread an existing plugin and add Vite-only properties:

```js
plugins: [{ ...example(), enforce: "post", apply: "build" }];
```

## Path Normalization

Vite normalizes resolved IDs to POSIX separators (`/`), including on Windows. Rollup/Rolldown does not: resolved IDs use `\` on Windows by default.

When comparing paths against resolved IDs in a Vite plugin, normalize first using `normalizePath` from `vite`:

```js
import { normalizePath } from "vite";

normalizePath("foo\\bar"); // 'foo/bar'
normalizePath("foo/bar"); // 'foo/bar'
```

> Remark:
>
> - Rollup plugins that use `createFilter` from `@rollup/pluginutils` already normalize paths before comparing, so their `include`/`exclude` patterns work correctly in both Rollup and Vite without changes.
> - If you are writing a Vite plugin from scratch and doing your own path comparisons, call `normalizePath` yourself.

## Filtering

- `createFilter` is a utility from `@rollup/pluginutils`, re-exported by Vite.
- It builds a filter function from `include`/`exclude` glob patterns, the same way Vite core filters files internally.
- Use it in plugins to implement standard file filtering.

### Hook Filters

Hook filters are a Rolldown feature. They let a plugin declare patterns directly on a hook so Rolldown skips invoking the JS handler entirely when the pattern does not match, avoiding unnecessary Rust-to-JS calls.

- A `filter` object on a hook's `ObjectHook` form.
- It can be specified on any hook that supports `ObjectHook` (e.g. `transform`).
- It was introduced to reduce overhead between Rust and JS runtimes.

Hook filters are supported in Rollup 4.38.0+ and Vite 6.3.0+. For backward compatibility, also check the filter inside the handler:

```ts
transform: {
  filter: { id: /\.js$/ },
  handler(code, id) {
    if (!/\.js$/.test(id)) return null // backward compat
    return { code: transformCode(code), map: null }
  },
}
```

> Remark: `@rolldown/pluginutils` exports filter utilities like `exactRegex` and `prefixRegex`, also re-exported from `rolldown/filter`.

## Client-Server Communication

Vite provides utilities for plugins to communicate between the dev server and the client (browser).

### Server to Client

Use `server.ws.send` to broadcast events from the plugin:

```js
configureServer(server) {
  server.ws.on('connection', () => {
    server.ws.send('my:greetings', { msg: 'hello' })
  })
}
```

On the client side, use `import.meta.hot.on` to listen:

```js
if (import.meta.hot) {
  import.meta.hot.on("my:greetings", (data) => {
    console.log(data.msg); // hello
  });
}
```

> Convention: Always prefix event names (e.g. `my:`) to avoid collisions with other plugins.

### Client to Server

Use `import.meta.hot.send` to send events from the client:

```js
if (import.meta.hot) {
  import.meta.hot.send("my:from-client", { msg: "Hey!" });
}
```

Listen on the server with `server.ws.on`:

```js
configureServer(server) {
  server.ws.on('my:from-client', (data, client) => {
    console.log(data.msg) // Hey!
    client.send('my:ack', { msg: 'Hi! I got your message!' })
  })
}
```

### TypeScript for Custom Events

Extend the `CustomEventMap` interface from `vite/types/customEvent.d.ts` to type custom event payloads:

```ts
// events.d.ts
import "vite/types/customEvent.d.ts";

declare module "vite/types/customEvent.d.ts" {
  interface CustomEventMap {
    "custom:foo": { msg: string };
  }
}
```

Vite uses this interface with `InferCustomEventPayload<T>` to infer the payload type when calling `hot.on`:

```ts
import.meta.hot?.on("custom:foo", (payload) => {
  // payload is { msg: string }
});
```
