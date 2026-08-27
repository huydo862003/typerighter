The frontmatter is the YAML block between `---` markers at the top of a `.td` file. It holds the structured data for the resource.

## Structure

```
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

A display label for the resource. It is an expression that can reference other fields.

```yaml
_label: "${self.first_name} ${self.last_name}"
```

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
name: "Alice"               # string
count: 42                   # number
active: true                # boolean
author: fref("people/bob.td") # file reference
full_name: self.first_name + " " + self.last_name # computed
```

## Explicit type tags

Optional type tags override inference. Rarely needed since types are inferred from the schema.

```yaml
name: !string "Alice"
count: !number 42
birth_date: !date "1990-07-04"
```

Available tags: `!string`, `!number`, `!boolean`, `!date`, `!time`, `!datetime`, `!type`.
