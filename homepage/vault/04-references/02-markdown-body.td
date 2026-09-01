The markdown body is everything after the closing `---`. It supports standard markdown with Typedown extensions.

## Headings

```markdown
# Heading 1
## Heading 2
### Heading 3
#### Heading 4
```

Headings generate anchors and appear in the table of contents.

## Text formatting

| Syntax | Result |
| ------ | ------ |
| `**bold**` | **bold** |
| `_italic_` | _italic_ |
| `***bold italic***` | ***bold italic*** |
| `~~strikethrough~~` | ~~strikethrough~~ |
| `` `inline code` `` | `inline code` |

## Code blocks

Fenced code blocks with optional language label and line ranges:

````markdown
```python
def greet(name):
    return f"Hello, {name}!"
```
````

Line highlighting with ranges:

````markdown
```js{1,3,5-8}
const a = 1;
const b = 2;
const c = 3;
```
````

The opening and closing fence must have the same number of backticks.

## Math

Inline math with single `$`:

```markdown
The formula is $E = mc^2$.
```

Renders as: The formula is $E = mc^2$.

Block math with `$$`:

```markdown
$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$
```

Renders as:

$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$

Math content is raw text. No interpolation inside math blocks.

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

Ordered lists:

```markdown
1. first
2. second
   1. nested
```

Task lists:

```markdown
- [x] done
- [ ] todo
```

## Links and images

```markdown
[Example](https://example.com)
[Alice](people/alice)
![alt text](_assets/photo.jpg)
```

Links to `.td` files are resolved relative to the vault root. The `.td` extension can be omitted.

## String interpolation

Use `${...}` to embed expressions in the body:

```markdown
This task is assigned to ${self.assignee.name}.
Total items: ${self.items.length()}.
```

Expressions are evaluated at build time.

## Callout blocks

Callout blocks use `:::` with a type label:

```markdown
::: note
This is a note.
:::
```

::: note
This is a note.
:::

Available types: `note`, `tip`, `warning`, `danger`, `details`, `info`.

The `details` type creates a collapsible section with a title:

```markdown
::: details Click to expand
Hidden content.
:::
```

::: details Click to expand
Hidden content.
:::

## Custom components

Custom components can be used with two syntaxes.

**Container syntax** wraps content in a named block:

```markdown
::: component-name {prop="value"}
Content goes here.
:::
```

**Shorthand syntax** for self-closing components:

```markdown
[[component-name]]
[[component-name {prop="value"}]]
```

### Built-in components

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

::: tip
User-registered custom components are planned for a future release.
:::

## Limitations

- HTML tags are not supported. Typedown focuses on document structure.
- Indentation-based code blocks are not supported. Use fenced code blocks.
