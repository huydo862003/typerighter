Every `.td` file is a node in the vault graph. A document has two parts: a YAML frontmatter that holds structured data, and a markdown body that holds prose.

## Anatomy of a `.td` file

```yaml
---
_type: Person
_label: "Alice Chen"
name: "Alice Chen"
role: "developer"
email: "alice@example.com"
---

Alice is a backend developer focused on authentication systems.

## Skills

| Area | Level |
| ---- | ----- |
| Rust | Expert |
| TypeScript | Proficient |
```

The frontmatter is the block between `---` markers. The body is everything after the closing `---`.

## Frontmatter

The frontmatter is a YAML mapping of key-value pairs. Every value is an expression: strings, numbers, booleans, lists, and computed values all work.

```yaml
---
_type: Task
title: "Implement auth"
priority: 3
done: false
tags:
  - "backend"
  - "security"
assignee: fref("people/alice.td")
---
```

Keys starting with `_` are reserved for built-in directives:

| Key | Purpose |
| --- | --- |
| `_type` | Schema this document conforms to |
| `_label` | Display name shown in the sidebar and links |
| `_icon` | Page icon (e.g., `icon.rocket`) |
| `_extends` | Parent schema (schema files only) |
| `_imports` | File imports into scope |

The frontmatter is optional. A file with no `---` markers is treated as body-only.

## Markdown body

The body supports standard markdown plus Typedown extensions:

**Text formatting**: `**bold**`, `_italic_`, `~~strikethrough~~`, `` `inline code` ``

**Math**: inline $E = mc^2$ with `$E = mc^2$`, and block math:

$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$

with `$$..$$`.

**Code blocks** with syntax highlighting:

```python
def greet(name):
    return f"Hello, {name}!"
```

**Tables**:

| Name | Role |
| ---- | ---- |
| Alice | Developer |
| Bob | Designer |

**Callout blocks**:

::: tip
Callout blocks use `::: type` and `:::` to wrap content.
:::

**String interpolation** with `\${...}`:

```markdown
This task is assigned to \${self.assignee.name}.
```

The expression inside `\${...}` is evaluated at build time and replaced with the result.

**Container shorthands** like `[[toc]]` and `[[directory-index]]` insert built-in components.

See [Markdown reference](../references/02-markdown-body) for the full syntax.
