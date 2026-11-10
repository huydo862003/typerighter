---
_label: "Markdown body"
_icon: icon.edit
description: "Headings, text formatting, code blocks, diagrams, math, callouts, tables, links, embeds, and string interpolation."
---

Everything after the closing `---` is the markdown body. It supports standard markdown plus Typedown extensions:

- Fenced code blocks with line highlighting, titles, and line numbers
- Mermaid and D2 diagrams rendered inline
- LaTeX math with `$` and `$$`
- Callout blocks (`note`, `tip`, `warning`, `danger`, `details`)
- Embeds for YouTube, Spotify, Figma, GitHub, and more
- String interpolation with `${self.field}` to render frontmatter values

## Headings

Headings structure your page into sections and automatically appear in the right-hand table of contents, making long documents easy to navigate.

```markdown
# Heading 1
## Heading 2
### Heading 3
#### Heading 4
```

Each heading also generates an anchor link that readers can share or bookmark.

## Text formatting

Standard inline formatting for emphasis, deletion, and code spans:

| Syntax | Result |
| ------ | ------ |
| `**bold**` | **bold** |
| `_italic_` | _italic_ |
| `***bold italic***` | ***bold italic*** |
| `~~strikethrough~~` | ~~strikethrough~~ |
| `` `inline code` `` | `inline code` |

## Code blocks

Fenced code blocks render syntax-highlighted source code with optional line highlighting, titles, and line numbers. Specify a language label after the opening backticks for highlighting:

````markdown
```python
def greet(name):
    return f"Hello, {name}!"
```
````

Renders as:

```python
def greet(name):
    return f"Hello, {name}!"
```

The opening and closing fence must have the same number of backticks. Typedown extends basic code blocks with line highlighting, titles, and line numbers via `{...}` annotations after the language.

### Line highlighting

To draw attention to specific lines, add `{line-ranges}` after the language:

````markdown
```js{1,3}
const a = 1;
const b = 2;
const c = 3;
```
````

Renders as:

```js{1,3}
const a = 1;
const b = 2;
const c = 3;
```

Supports single lines (`1,3`), ranges (`5-8`), and combinations (`1,3,5-8`).

### Title

Display a filename or title above the code block with `title=` inside `{...}`:

````markdown
```ts{title=vite.config.ts}
import { defineConfig } from 'vite'
export default defineConfig({})
```
````

Renders as:

```ts{title=vite.config.ts}
import { defineConfig } from 'vite'
export default defineConfig({})
```

Titles can be combined with line highlighting:

```ts{1 title=vite.config.ts}
import { defineConfig } from 'vite'
export default defineConfig({})
```

### Line numbers

For longer snippets where readers need to reference specific lines, add `line-numbers` inside `{...}`:

````markdown
```js{line-numbers}
const a = 1;
const b = 2;
const c = 3;
```
````

Renders as:

```js{line-numbers}
const a = 1;
const b = 2;
const c = 3;
```

You can also start numbering from a specific line with `line-numbers=N`:

```js{1,3 line-numbers=5}
const a = 1;
const b = 2;
const c = 3;
```


## Diagrams

When you need to illustrate architecture, workflows, or relationships, Typedown renders diagrams directly from text-based descriptions inside fenced code blocks. Diagrams are lazy-loaded from CDN, so they add no weight to pages that don't use them.

### Mermaid

Use a fenced code block with the `mermaid` language to create flowcharts, sequence diagrams, Gantt charts, and more:

````markdown
```mermaid
graph LR
    A[Start] --> B{Decision}
    B -->|Yes| C[Action]
    B -->|No| D[End]
```
````

Renders as:

```mermaid
graph LR
    A[Start] --> B{Decision}
    B -->|Yes| C[Action]
    B -->|No| D[End]
```

See [Mermaid documentation](https://mermaid.js.org/) for the full syntax reference.

### D2

Use a fenced code block with the `d2` language for D2 diagrams. D2 is a modern diagram language with a clean syntax:

````markdown
```d2
Web Client -> API Gateway: HTTPS
API Gateway -> Auth Service: validate
Auth Service -> API Gateway: token
API Gateway -> App Server: request
App Server -> Database: query
```
````

Renders as:

```d2
Web Client -> API Gateway: HTTPS
API Gateway -> Auth Service: validate
Auth Service -> API Gateway: token
API Gateway -> App Server: request
App Server -> Database: query
```

D2 supports shapes, connections, containers, and more. See [D2 documentation](https://d2lang.com/) for the full syntax reference.

### Diagram captions

Add a title to any diagram block using `{title=...}` to render a caption below the diagram:

````markdown
```mermaid{title="Authentication Flow"}
graph LR
    A[Client] --> B[Auth Server]
```
````

Both Mermaid and D2 are loaded lazily from CDN only when a diagram block is present on the page. They respect the current light/dark theme.

## Math

For technical and academic writing, Typedown supports LaTeX math notation rendered via Temml. Use single `$` for inline math:

```markdown
The formula is $E = mc^2$.
```

Renders as: The formula is $E = mc^2$.

For display equations, use `$$` for a centered block:

```markdown
$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$
```

Renders as:

$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$

Math content is raw text, so `${}` interpolation does not apply inside math blocks.

## Blockquotes

```markdown
> This is a blockquote.
```

> This is a blockquote.

## Tables

```markdown
| Name  | Age |
| ----- | --- |
| Alice | 30  |
| Bob   | 25  |
```

| Name | Age |
| ---- | --- |
| Alice | 30 |
| Bob | 25 |

## Lists

Bullet lists:

```markdown
- item one
- item two
  - nested item
```

Renders as:

- item one
- item two
  - nested item

Ordered lists:

```markdown
1. first
2. second
   1. nested
```

Renders as:

1. first
2. second
   1. nested

Task lists:

```markdown
- [x] done
- [ ] todo
```

Renders as:

- [x] done
- [ ] todo

## Links and images

Link to other pages in your vault, external sites, or embed images and media. Typedown resolves vault-relative paths automatically:

```markdown
[Example](https://example.com)
[Alice](people/alice)
![alt text](_assets/photo.jpg)
```

Links to `.td` files are resolved relative to the vault root. The `.td` extension can be omitted.

### Link titles

Add a quoted title after the URL for tooltips:

```markdown
[Example](https://example.com "Visit Example")
```

### Image captions

Add a quoted title to an image to render it as a figure with a caption:

```markdown
![A sunset](_assets/sunset.jpg "Golden hour at the beach")
```

The image is wrapped in a `<figure>` element with the title rendered as a `<figcaption>` below it.

### Embeds

Instead of linking to external content, you can embed it directly on the page using image syntax with a URL. Typerighter detects the service and renders the appropriate embed:

| Service | Example URL |
|---------|------------|
| YouTube | `https://www.youtube.com/watch?v=ID` or `https://youtu.be/ID` |
| Spotify | `https://open.spotify.com/track/ID` |
| Apple Music | `https://music.apple.com/...` |
| Vimeo | `https://vimeo.com/ID` |
| CodePen | `https://codepen.io/user/pen/ID` |
| CodeSandbox | `https://codesandbox.io/s/ID` |
| Figma | `https://www.figma.com/file/ID` |
| Excalidraw | `https://excalidraw.com/...` |
| GitHub | `https://github.com/user/repo/blob/ref/path#L1-L10` |
| GitHub Gist | `https://gist.github.com/user/ID` |

```markdown
![](https://www.youtube.com/watch?v=dQw4w9WgXcQ)

![server.ts](https://github.com/user/repo/blob/main/src/server.ts#L10-L20)
```

Here is a YouTube embed:

![](https://www.youtube.com/watch?v=dQw4w9WgXcQ)

Any other `https://` URL without an image extension is rendered as a generic `<iframe>`, with a fallback link shown if the site blocks framing. URLs with image extensions (`.png`, `.jpg`, `.svg`, `.webp`, etc.) and relative paths are always rendered as images.

## String interpolation

Frontmatter values can be referenced directly in the body text using `${...}` expressions, which are evaluated at build time. This keeps your prose in sync with structured data without manual duplication:

```yaml{title=vault/tasks/setup-ci.td}
---
_type: Task
title: "Set up CI"
assignee: fref("people/alice.td")
tags:
  - "infra"
  - "devops"
---

This task is assigned to ${self.assignee.name}.
It is tagged with ${self.tags.length()} labels.
```

Here `self` refers to the current file's frontmatter, so `self.assignee.name` resolves to Alice's name from the referenced Person document.

## Callout blocks

Callout blocks draw attention to important information like warnings, tips, or supplementary notes. They use the `:::` fence with a type label:

```markdown
::: note
This is a note.
:::
```

::: note
This is a note.
:::

The available types are `note`, `tip`, `warning`, `danger`, `details`, and `info`. The `details` type creates a collapsible section with a title:

```markdown
::: details Click to expand
Hidden content.
:::
```

::: details Click to expand
Hidden content.
:::

## Limitations

- HTML tags are not supported because Typedown focuses on document structure rather than raw markup.
- Indentation-based code blocks are not supported; use fenced code blocks instead.

## Custom components

When built-in markdown features are not enough, you can embed Vue components directly inside your content using `:::` containers and `[[shorthand]]` syntax. See [Custom components](05-custom-components) for the full reference on built-in components, grid layouts, registering your own components, and the component contract file.
