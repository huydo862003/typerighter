---
_label: "Frontmatter"
_icon: icon.document
description: "YAML frontmatter syntax, reserved keys, type declarations, and built-in fields."
---

Every `.td` file starts with a frontmatter block between `---` markers. This is where you declare the document's type, set field values, and write computed expressions.

```yaml
---
_type: Person
_label: "${self.first_name} ${self.last_name}"
_icon: icon.user
first_name: "Alice"
last_name: "Chen"
---
```

The frontmatter is the structured half of the document. The markdown body below it is the prose half.

## Structure

```yaml
---
_type: Person
name: "Alice"
role: "developer"
---
```

The top-level value must be a YAML mapping. Keys are identifiers (alphanumeric and underscores). Values are expressions.

## Reserved keys

Keys starting with `_` are reserved for built-in directives.

| Key | Required | Description |
| --- | --- | --- |
| `_type` | yes | Schema name this resource conforms to |
| `_label` | no | Human-readable display label (expression) |
| `_icon` | no | Page icon from the `icon` module (e.g., `icon.rocket`) |
| `_extends` | no | Parent schema name (schema files only) |
| `_imports` | no | Map of alias to vault-relative file path |
| `_content` | implicit | The markdown body (available as `self._content`) |

## Type declaration: `_type`

Declares the schema. The value is a PascalCase schema name matching a file in `_types/`.

```yaml
_type: Person
```

For schema files, the value is `schema`:

```yaml
_type: schema
```

## Display label: `_label`

A display label for the resource. Shown in the sidebar, breadcrumbs, and link text. It is an expression that can reference other fields.

```yaml
_label: "Alice Chen"
_label: "${self.first_name} ${self.last_name}"
```

## Page icon: `_icon`

An icon displayed in the sidebar and page header. Uses the built-in `icon` module with dot access syntax.

```yaml
_icon: icon.rocket
_icon: icon.book
_icon: icon.lightbulb
```

See [Icons reference](05-icons) for the full list of available icons.

## Schema inheritance: `_extends`

Schema inheritance. The child schema inherits all properties from the parent.

```yaml
_type: schema
_extends: Person
properties:
  agency:
    type: string?
```

## Module imports: `_imports`

Import files from the vault into the current file's scope. Each key is an alias, each value is a vault-relative file path.

```yaml
_imports:
  colors: "_partials/colors.td"
brand: colors.primary
```

## Comments

Line comments use `#`:

```yaml
name: "Alice" # this is a comment
```

## Values

All values are expressions. Strings must be quoted. Unquoted values are identifiers, numbers, or compound expressions.

```yaml
name: "Alice"                             # string
count: 42                                 # number
active: true                              # boolean
author: fref("people/bob.td")            # file reference
full_name: self.first_name + " " + self.last_name  # computed
```

## Explicit type tags

Optional type tags override inference. Rarely needed since types are inferred from the schema.

```yaml
name: !string "Alice"
count: !number 42
birth_date: !date "1990-07-04"
```

Available tags: `!string`, `!number`, `!boolean`, `!date`, `!time`, `!datetime`, `!type`.

## Reference

This section provides a quick lookup for every frontmatter key, value form, and type tag.

### Reserved keys

Keys starting with `_` are reserved for built-in directives:

| Key | Required | Description |
| --- | --- | --- |
| `_type` | yes | Schema name this resource conforms to |
| `_label` | no | Display label (expression, supports interpolation) |
| `_icon` | no | Page icon from the `icon` module |
| `_extends` | no | Parent schema name (schema files only) |
| `_imports` | no | Map of alias to vault-relative file path |
| `_content` | implicit | The markdown body, available as `self._content` |

### Value syntax

Every frontmatter value is an expression. These are the supported forms:

| Form | Description | Example |
| --- | --- | --- |
| `"text"` | String literal | `"Alice"` |
| `42` | Number literal | `42`, `3.14` |
| `true` / `false` | Boolean literal | `true` |
| `null` | Null value | `null` |
| `fref("path")` | File reference | `fref("people/alice.td")` |
| `self.field` | Self-reference | `self.name` |
| `expr + expr` | Binary expression | `self.x + 1` |
| `"${expr}"` | String interpolation | `"Hello, ${self.name}"` |
| `(p) -> body` | Closure | `(x) -> x * 2` |

### Type tags

Type tags override inference when the schema cannot determine the type automatically:

| Tag | Description |
| --- | --- |
| `!string` | Force string type |
| `!number` | Force number type |
| `!boolean` | Force boolean type |
| `!date` | Force date type |
| `!time` | Force time type |
| `!datetime` | Force datetime type |
| `!type` | Force type expression |

### List and mapping syntax

```yaml
# list
tags:
  - "frontend"
  - "design"

# imports mapping
_imports:
  colors: "_partials/colors.td"
```
