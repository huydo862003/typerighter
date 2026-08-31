The markdown body is everything after the closing `---`. It supports standard markdown with some Typedown extensions.

## Headings

```markdown
# Heading 1
## Heading 2
### Heading 3
```

## Text formatting

```markdown
**bold**
_italic_
***bold italic***
~~strikethrough~~
`inline code`
```

## Code blocks

Fenced code blocks with optional language label:

````markdown
```python
print("hello")
```
````

The opening and closing fence must have the same number of backticks. Use more backticks to include backticks in code.

## Math

Inline math with single `$`, block math with `$$`:

```markdown
The formula is $E = mc^2$.

$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$
```

Math content is raw text. No interpolation inside math.

## Blockquotes

```markdown
> This is a blockquote.
```

## Tables

```markdown
| Name  | Age |
| ----- | --- |
| Alice | 30  |
| Bob   | 25  |
```

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

## Links

Standard markdown links:

```markdown
[Example](https://example.com)
[Alice](people/alice)
```

Links to `.td` files are resolved relative to the vault root. The `.td` extension can be omitted.

## Images

```markdown
![alt text](image.png)
![photo](_assets/photo.jpg)
```

## String interpolation

Use `${...}` to embed expressions in the body:

```markdown
This task is assigned to ${self.assignee.name}.
Total items: ${self.items.length()}.
```

## Callout blocks

Callout blocks use `:::` with a type label:

```markdown
::: note
This is a note.
:::

::: warning
Be careful.
:::

::: details Click to expand
Hidden content.
:::
```

The type label supports kebab-case identifiers.

## Container shorthands

Self-closing containers use `[[label]]`:

```markdown
[[directory-index]]
[[toc]]
```

## Limitations

- HTML tags are not supported. Typedown focuses on document structure.
- Indentation-based code blocks are not supported. Use fenced code blocks instead.
