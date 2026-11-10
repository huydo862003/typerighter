---
_label: "Custom components"
_icon: icon.puzzle
description: "Built-in and user-defined Vue components in the markdown body: grid layouts, registration, contract files, and prop types."
---

When built-in markdown features are not enough, you can use Vue components directly inside your content. This is useful for interactive widgets, styled cards, data tables, or any UI that goes beyond static text. Custom components support two syntaxes: a container syntax that wraps content in a named block:

```markdown
::: component-name {prop="value"}
Content goes here.
:::
```

And a shorthand syntax for self-closing components:

```markdown
[[component-name]]
[[component-name {prop="value"}]]
```

## Built-in components

These components are available without any configuration:

| Component | Description | Syntax |
| --------- | ----------- | ------ |
| `toc` | Table of contents for the current page | `[[toc]]` |
| `directory-index` | Index listing of the current directory's children | `[[directory-index]]` |
| `glossary-index` | Alphabetical glossary of all pages | `[[glossary-index]]` |
| `note` | Note callout | `::: note ... :::` |
| `tip` | Tip callout | `::: tip ... :::` |
| `warning` | Warning callout | `::: warning ... :::` |
| `danger` | Danger callout | `::: danger ... :::` |
| `info` | Info callout | `::: info ... :::` |
| `details` | Collapsible section with title | `::: details Title ... :::` |
| `grid` | Grid layout container | `::: grid{columns=3} ... :::` |
| `grid-cell` | Cell within a grid | `::: grid-cell ... :::` |

### Grid layout

The `grid` component creates a CSS grid layout. Nest `grid-cell` components inside to define each cell:

```markdown
:::grid{columns=3}
:::grid-cell
First column
:::
:::grid-cell
Second column
:::
:::grid-cell
Third column
:::
:::
```

:::grid{columns=3}
:::grid-cell
First column
:::
:::grid-cell
Second column
:::
:::grid-cell
Third column
:::
:::

Props:

| Prop | Type | Description |
| ---- | ---- | ----------- |
| `columns` | number | Number of equal columns |

`grid-cell` accepts `col-span` and `row-span` props to span multiple columns or rows:

```markdown
:::grid{columns=3}
:::grid-cell{col-span=2}
This cell spans two columns
:::
:::grid-cell
Normal cell
:::
:::
```

:::grid{columns=3}
:::grid-cell{col-span=2}
This cell spans two columns
:::
:::grid-cell
Normal cell
:::
:::

## User-defined components

Beyond the built-in set, you can register your own Vue components for use in the markdown body. Registered components are available by their PascalCase name or its kebab-case equivalent.

::: note
Writing Vue components requires familiarity with [Vue.js](https://vuejs.org/). See [Custom Theming](../03-custom-theming/01-custom-theming) for the full theming guide.
:::

### Registering components

Declare components in `typedown.yaml` under `theme.components`. Each entry is either a folder path (scanned recursively for `.vue` files) or an explicit name-to-source mapping:

```yaml{title=typedown.yaml}
theme:
  components:
    - "./src/components"
    - { "Chart": "./src/Chart.vue" }
```

When multiple entries define the same name, the last one wins.

### Contract file

Typerighter auto-generates `.typedown/components.json` during each build, declaring each component's props so the LSP can warn on unknown components, missing required props, and unknown props in the markdown body.

```json{title=.typedown/components.json}
{
  "TdAlert": {
    "source": "./src/components/TdAlert.vue",
    "props": {
      "kind": { "type": "string", "required": false }
    }
  }
}
```

This file should be committed alongside your vault so the LSP can validate component usage without running a build first.

### Example

A `TdAlert` component registered in `typedown.yaml` can be used as a container block:

```markdown
:::td-alert{kind="info"}
This is an informational alert.
:::
```

The component receives `kind` as a prop and the inner markdown as a slot. Any markdown syntax works inside the container, including headings, lists, code blocks, and links.

### Prop types

The `type` field in the contract uses Typedown type names:

| Type | Example |
| ---- | ------- |
| `string` | `"type": "string"` |
| `number` | `"type": "number"` |
| `boolean` | `"type": "boolean"` |
| `date` | `"type": "date"` |
| list | `"type": "list[string]"` |
| dict | `"type": "dict[string, number]"` |
| union | `"type": "string | number"` |
| unknown | `"type": "any"` |

Required props are enforced: the LSP warns when a required prop is absent at a usage site.
