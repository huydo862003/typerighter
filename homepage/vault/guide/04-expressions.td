Every value in the frontmatter is an expression. Most of the time you write simple literals and the type is inferred from the schema. But you can also write computed values.

## Literals

Strings must be quoted. Unquoted values are identifiers, numbers, or booleans.

```yaml
name: "Alice"           # string
count: 42               # number
active: true            # boolean
birth_date: "1990-07-04" # date (inferred from schema)
```

## Property access

`self` refers to the current resource. Use dot access to read other fields.

```yaml
full_name: self.first_name + " " + self.last_name
```

Dot access works across links too:

```yaml
assignee_name: self.assignee.name
```

## Operators

| Operator | Description          |
| -------- | -------------------- |
| `+`      | Add or concatenate   |
| `-`      | Subtract             |
| `*`      | Multiply             |
| `/`      | Divide               |
| `**`     | Exponentiate         |
| `%`      | Remainder            |
| `==`     | Equal                |
| `!=`     | Not equal            |
| `<`      | Less than            |
| `>`      | Greater than         |
| `<=`     | Less or equal        |
| `>=`     | Greater or equal     |
| `&&`     | Logical AND          |
| `\|\|`   | Logical OR           |
| `~`      | Logical NOT (unary)  |
| `.`      | Property access      |
| `[n]`    | List/string indexing |
| `[key]`  | Dict indexing        |

## Indexing

```yaml
first_tag: self.tags[0]
city: self.address["city"]
initial: self.first_name[0]
```

Out-of-bounds access and missing keys evaluate to `null`.

## String interpolation

Use `${...}` inside quoted strings to embed expressions:

```yaml
greeting: "Hello, ${self.name}!"
summary: "Written by ${self.author.name}"
label: "Result: ${"value is ${self.compute()}"}"
```

Interpolation works in the markdown body too:

```markdown
This task is assigned to ${self.assignee.name}.
```

## Closures

Anonymous functions use `(params) -> body` syntax:

```yaml
double: (x) -> x * 2
greet: (name) -> "Hello, ${name}!"
add: (a, b) -> a + b
```

## File references

`fref` creates a link to another file. The path is relative to the vault root.

```yaml
author: fref("people/alice.td")
tasks:
  - fref("tasks/setup-ci.td")
  - fref("tasks/write-tests.td")
```
