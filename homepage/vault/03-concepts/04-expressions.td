Every frontmatter value is an expression. Most of the time you write simple literals and the type is inferred. But you can also write computed values that reference other fields, traverse links, and transform data.

## Literals

Strings must be quoted. Unquoted values are identifiers, numbers, or booleans.

```yaml
name: "Alice"              # string
count: 42                  # number
active: true               # boolean
birth_date: "1990-07-04"  # date (inferred from schema)
```

## Property access

`self` refers to the current document. Use dot access to read fields.

```yaml
full_name: self.first_name + " " + self.last_name
```

Dot access works across links. If `assignee` is a `fref` to a Person, you can reach into it:

```yaml
assignee_name: self.assignee.name
```

## Operators

| Operator | Description |
| -------- | ----------- |
| `+` | Add or concatenate |
| `-`, `*`, `/` | Arithmetic |
| `**` | Exponentiate |
| `%` | Remainder |
| `==`, `!=` | Equality |
| `<`, `>`, `<=`, `>=` | Comparison |
| `&&`, `\|\|` | Logical AND, OR |
| `~` | Logical NOT (unary) |
| `.` | Property access |
| `[n]` | List/string indexing |
| `[key]` | Dict indexing |

## Indexing

```yaml
first_tag: self.tags[0]
city: self.address["city"]
initial: self.first_name[0]
```

Out-of-bounds access and missing keys evaluate to `null`.

## String interpolation

Use `${...}` to embed expressions inside strings:

```yaml
greeting: "Hello, ${self.name}!"
summary: "Written by ${self.author.name}"
```

Interpolation works in the markdown body too:

```markdown
This task is assigned to ${self.assignee.name}.
```

The expression is evaluated at build time. The rendered page shows the result, not the expression.

## Closures

Anonymous functions use `(params) -> body` syntax:

```yaml
double: (x) -> x * 2
greet: (name) -> "Hello, ${name}!"
add: (a, b) -> a + b
```

## File references

`fref` creates an edge to another document. The path is relative to the vault root.

```yaml
author: fref("people/alice.td")
tasks:
  - fref("tasks/setup-ci.td")
  - fref("tasks/write-tests.td")
```

See [References](05-references) for the full story on how edges work.
