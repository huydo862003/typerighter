# Typedown Syntax

- [Basic Structure](#basic-structure)
- [Modes](#modes)
- [Escape Sequences](#escape-sequences)
- [String Interpolation](#string-interpolation)
- [Typedown Frontmatter (YAML Mode)](#typedown-frontmatter-yaml-mode)
  - [Comments](#comments)
  - [Top-level Frontmatter Value](#top-level-frontmatter-value)
  - [Type Declaration](#type-declaration)
  - [Schema Inheritance](#schema-inheritance)
  - [Imports](#imports)
  - [Label](#label)
  - [Properties](#properties)
  - [Links](#links)
- [Typedown Expression](#typedown-expression)
  - [Scalars](#scalars)
  - [Operators](#operators)
  - [Lists](#lists)
  - [Dicts](#dicts)
  - [Closures](#closures)
  - [Type Expressions](#type-expressions)
- [Typedown Explicit Type Tags](#typedown-explicit-type-tags)
- [Typedown Schema](#typedown-schema)
- [Typedown Body (Markdown Mode)](#typedown-body-markdown-mode)
  - [Headings](#headings)
  - [Code](#code)
  - [Blockquotes](#blockquotes)
  - [Math](#math)
  - [Tables](#tables)
  - [Lists](#lists-1)
  - [Callout Blocks](#callout-blocks)
  - [Multimedia](#multimedia)
  - [Links](#links-1)

## Basic Structure

A `.td` file consists of two sections:

1. A YAML-like frontmatter block containing the resource's structured data (the **Typedown frontmatter**, or **frontmatter**), followed by
2. A [Typedown Markdown](#typedown-body-markdown-mode) body for free-form content (the **Typedown body**, or **body**).

```
---
<frontmatter>
---

<body>
```

- The opening `---` is the frontmatter start marker.
- The closing `---` is the frontmatter end marker.
- Everything after belongs to the body.

The frontmatter is optional. A file with no `---` delimiters is treated as a body-only file.

The syntaxes will be familiar to anyone who has worked with YAML and Markdown. Typedown is case-sensitive throughout: identifiers, property names, type names, and reserved keys like `_type` and `_label` must match exactly.

## Modes

Like Typst, Typedown has four distinct modes that determine how content is interpreted. Each mode has its own syntax and semantics.

### YAML Mode

Active inside the frontmatter (between the opening and closing `---`). Content is interpreted as structured data: key-value mappings, sequences, expressions, and type annotations. Indentation is significant. Comments start with `#`.

```yaml
---
_type: Person
first_name: "Bob"
tags:
  - "research"
  - "rdf"
---
```

Values are expressions. Strings must be quoted. Unquoted values are identifiers, numbers, or compound expressions. Inside quoted strings, `${...}` enters Formula mode and `$...$` enters Math mode:

```yaml
greeting: "Hello, ${self.first_name}!"
description: "The formula is $E = mc^2$ and it works"
```

### Markdown Mode

Active after the closing `---`. Content is interpreted as rich text with formatting, headings, lists, code blocks, tables, and other document elements.

```markdown
# Introduction

This is a paragraph with **bold** and _italic_ text.
```

`$` without `{` enters Math mode. `${...}` enters Formula mode.

### Formula Mode

Entered with `${` in Markdown mode or inside quoted strings in YAML mode. Content is interpreted as a Typedown expression: identifiers, operators, numbers, function calls, and property access. The mode exits when the matching closing `}` is found.

```markdown
This note was written by ${self.author.first_name}.
Total: ${self.items.length()}.
Result: ${"value is ${self.compute()}"}
```

```yaml
greeting: "Hello, ${self.first_name}!"
```

### Math Mode

Entered with `$` (inline) or `$$` (block) inside Markdown mode. Content is treated as a math formula (e.g. LaTeX). No interpolation or Typedown expressions are supported inside math. The mode exits when the matching closing delimiter is found.

```markdown
The formula is $E = mc^2$.

$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$
```

## Escape Sequences & HTML Entities

Backslash `\` is the escape character. It works in quoted strings (both `"..."` and `'...'`).

In quoted strings:

| Sequence      | Result                                 |
| ------------- | -------------------------------------- |
| `\\`          | Literal `\`                            |
| `\"`          | Literal `"` (in double-quoted strings) |
| `\'`          | Literal `'` (in single-quoted strings) |
| `\/`          | Literal `/`                            |
| `\n`          | Newline                                |
| `\t`          | Tab                                    |
| `\r`          | Carriage return                        |
| `\b`          | Backspace                              |
| `\f`          | Form feed                              |
| `\v`          | Vertical tab                           |
| `\$`          | Literal `$` (prevents interpolation)   |
| `\uXXXX`      | Unicode escape (4 hex digits)          |
| `\xXX`        | Hex byte escape (2 hex digits)         |
| `\NNN`        | Octal escape (1-3 octal digits)        |

In markdown body, we use HTML entities.

## String Interpolation

Both YAML mode and Markdown mode support string interpolation with `${...}`. Any expression can appear inside the braces:

```yaml
greeting: "Hello, ${self.first_name}!"
```

```markdown
This note was written by ${self.author.first_name} ${self.author.last_name}.
```

`$` without `{` does not trigger interpolation. In Markdown mode, it enters Math mode. In YAML mode, it is a literal `$`.

Interpolations can be nested. A string inside an interpolation can itself contain interpolations:

```yaml
label: "Result: ${"value is ${self.compute()}"}"
```

## Typedown Frontmatter (YAML Mode)

Every `.td` file is a **Typedown resource file**. It contains a frontmatter and a body. The body is free-form content written in [Typedown Markdown](#typedown-body-markdown-mode). The frontmatter is where the resource's structured data lives.

### Comments

The frontmatter supports YAML line comments using `#`:

```yaml
---
first_name: "Bob" # this is a comment
---
```

### Top-level Frontmatter Value

The top-level frontmatter must be a **YAML mapping** with identifier keys. A YAML mapping is a set of key-value pairs, similar to a JSON object. Keys must be single-word identifiers (alphanumeric and underscores, no spaces or special characters):

```yaml
first_name: "Bob" # valid
birth_date: "1990-07-04" # valid
my_key_2: 42 # valid
```

Keys starting with `_` are reserved for built-in directives (`_type`, `_label`, `_extends`, `_imports`). User-defined properties should not start with `_` to avoid conflicts with current or future reserved keys.

Values are **expressions**, not raw strings. Unquoted values are parsed as expressions (identifiers, numbers, booleans, operators). To write a string value, use double quotes or single quotes:

```yaml
first_name: "Bob" # string literal
status: "draft" # string literal
count: 42 # number expression
active: true # boolean identifier
author: fref("people/bob.td") # file reference (link)
full_name: self.first_name + " " + self.last_name # expression
```

### Type Declaration

A resource file declares its type using `_type`. The value is the name of a [Typedown Schema](#typedown-schema) that the resource conforms to. Schema names use PascalCase. The schema enforces what properties the resource is expected to have.

For example, given a `Person` schema defined as:

```yaml
---
_type: schema
properties:
  first_name:
    type: string
  last_name:
    type: string
  birth_date:
    type: date
---
```

A resource conforming to it declares `_type: Person` and must provide the required fields:

```yaml
---
_type: Person
first_name: "Bob"
last_name: "Smith"
birth_date: "1990-07-04"
---
```

Property values do not need explicit type tags when the type can be inferred from the schema. `"Bob"` above is inferred as a `string` because the schema declares `first_name` as `string`.

A resource can also declare additional fields not defined in its schema. These are stored as-is and are not validated by the schema.

### Schema Inheritance

A schema can extend another schema using `_extends`. The child schema inherits all properties from the parent and can add its own:

```yaml
---
_type: schema
_extends: Person
properties:
  agency:
    type: string?
  rate:
    type: number
---
```

Resources conforming to the child schema must provide fields from both the parent and the child.

### Imports

A file can import other files from the vault using `_imports`. Each import maps an alias to a vault-relative file path:

```yaml
---
_imports:
  colors: "_partials/colors.td"
brand: colors.primary
---
```

The alias (`colors`) becomes available as an identifier in the file's scope. Dot access on the alias resolves fields from the imported file. Imported files can live in `_`-prefixed directories (internal modules) that are excluded from content discovery.

### Label

A resource file can declare a human-readable label using `_label`. The label is a [Typedown Expression](#typedown-expression) and can reference other properties:

```yaml
---
_type: Person
_label: "${self.first_name} ${self.last_name}"
---
```

### Properties

All frontmatter keys other than reserved `_` keys are properties of the resource. Property values are [Typedown Expressions](#typedown-expression).

```yaml
---
_type: Person
_label: "${self.first_name} ${self.last_name}"
first_name: "Bob"
birth_date: "1990-07-04"
author: fref("people/mona_lisa.td")
tags:
  - "research"
  - "rdf"
---
Free-form markdown body content.
```

### Links

A link is a property pointing to another `.td` file by filename. You can refer to another file by using `fref`. The path is relative to the vault root. Links form directed edges in the resource graph.

```yaml
author: fref("people/bob.td")
```

A link can also reference a property that resolves to the target:

```yaml
author: self.default_author
```

Multi-valued links are expressed as a YAML sequence:

```yaml
tags:
  - fref("tags/research.td")
  - fref("tags/rdf.td")
```

## Typedown Expression

Every value in Typedown frontmatter is an expression. Each expression has a type. In most cases the type is inferred from the schema, so it does not need to be stated explicitly.

Whether a value is an identifier or a literal is inferred from context in most cases. In ambiguous contexts, identifiers are preferred. To force a literal interpretation, wrap the value in single or double quotes (e.g. `'draft'`, `"published"`).

### Scalars

A scalar is a single primitive value. Every value in frontmatter is an expression. The scalar types are: `string`, `number`, `boolean`, `date`, `time`, `datetime`:

```yaml
first_name: "Bob" # string (quoted)
birth_date: "1990-07-04" # date (quoted)
count: 42 # number
active: true # boolean (identifier)
author: fref("people/bob.td") # link
```

Unquoted values are identifiers or expressions, not strings. Strings must always be quoted with double quotes (`"..."`) or single quotes (`'...'`):

```yaml
name: "Bob"              # string
name: Bob                # identifier, NOT a string
full_name: "${self.first_name} ${self.last_name}"  # expression
```

String values support interpolation with `${}`. Any expression can appear inside the braces:

```yaml
greeting: "Hello, ${self.first_name}!"
summary: "Written by ${self.author.first_name}"
```

Interpolation is **not** supported inside `$` math expressions. `$` enters math mode, where the content is treated as a math formula, not as a Typedown expression.

Interpolations can be nested. A string inside an interpolation can itself contain interpolations:

```yaml
label: "Result: ${"value is ${self.compute()}"}"
```

### Operators

The following operators are available in Typedown expressions:

| Operator | Description                                     |
| -------- | ----------------------------------------------- |
| `+`      | Addition (numbers) or concatenation (strings)   |
| `-`      | Subtraction                                     |
| `*`      | Multiplication                                  |
| `/`      | Division                                        |
| `**`     | Exponentiation                                  |
| `%`      | Remainder                                       |
| `==`     | Equality                                        |
| `!=`     | Inequality                                      |
| `<`      | Less than                                       |
| `>`      | Greater than                                    |
| `<=`     | Less than or equal                              |
| `>=`     | Greater than or equal                           |
| `&&`     | Logical AND                                     |
| `\|\|`   | Logical OR                                      |
| `~`      | Logical NOT (unary)                             |
| `.`      | Property access on an object or linked resource |
| `[n]`    | Index by zero-based integer (lists and strings) |
| `[key]`  | Dict indexing by string key                     |
| `?`      | Nullable postfix (in type expressions)          |

```yaml
first_tag: self.tags[0] # list indexing
city: self.address["city"] # dict indexing
initial: self.first_name[0] # string indexing
author_name: self.author.name # property access on linked resource
```

Out-of-bounds index access and missing dict keys evaluate to `null`.

### Lists

A list is a YAML sequence. Its type is `list[T]`, where `T` is the element type. Each element is itself an expression:

```yaml
tags: # list[string]
  - "research"
  - "rdf"
authors: # list of file references
  - fref("people/bob.td")
  - fref("people/alice.td")
```

### Dicts

A dict is a YAML mapping nested under a property key. Each value is itself an expression. Records come in two forms:

`dict[K, V]` is a homogeneous mapping where all keys share the same key type `K` and all values share the same value type `V`:

```yaml
scores: # dict[string, number]
  alice: 95
  bob: 87
```

A fixed-key mapping assigns a specific type to each named key independently:

```yaml
address: # { street: string, city: string, zip: number }
  street: "Main St"
  city: "Springfield"
  zip: 12345
```

### Closures

A closure is an anonymous function. The syntax is `(params) -> body`:

```yaml
double: (x) -> x * 2
greet: (name) -> "Hello, ${name}!"
add: (a, b) -> a + b
```

Closures capture their enclosing scope.

### Type Expressions

A type expression appears in schema property definitions under the `type` key. The `!type` tag is optional; bare type names are preferred:

The built-in types are:

| Type         | Description                                         |
| ------------ | --------------------------------------------------- |
| `string`     | A Unicode text value                                |
| `number`     | A floating-point number                             |
| `boolean`    | `true` or `false`                                   |
| `date`       | An ISO 8601 date (e.g. `"2024-01-15"`)              |
| `time`       | An ISO 8601 time (e.g. `"14:30:00"`)                |
| `datetime`   | An ISO 8601 datetime (e.g. `"2024-01-15T14:30:00"`) |
| `list[T]`    | A list of values of type `T`                        |
| `dict[K, V]` | A homogeneous mapping from keys of type `K` to `V`  |

```yaml
type: string
type: number
type: date
type: list[string]
type: dict[string, number]
```

A fixed-key dict type is expressed as a YAML mapping:

```yaml
type:
  street:
    type: string
  city:
    type: string
  zip:
    type: number
```

A union type is expressed as a YAML sequence. Each element is a type name or a literal value:

```yaml
type: [string, number]             # union of string and number
type: ['draft', 'published']       # string enum (union of string literals)
type: [1, 2, 3]                    # number enum (union of number literals)
```

String literals in a union must be quoted to distinguish them from type name identifiers.

The `?` postfix marks a type as nullable (accepts the declared type or `null`):

```yaml
type: string?           # string or null
type: list[string]?     # list or null
type: date?             # date or null
```

A schema name can be used as a type to declare a link field. The field expects a file reference (`fref`) to a resource conforming to that schema:

```yaml
type: Person            # link to a Person resource
type: Person?           # optional link
type: list[Task]        # list of links to Task resources
```

A literal type is a type whose only valid value is a specific literal:

```yaml
# schema
properties:
  version:
    type: 1 # version must always be 1
  status:
    type: "draft" # status must always be "draft"
```

An enum type is therefore shorthand for a union of literal types.

## Typedown Explicit Type Tags

A value can carry an explicit type tag to override inference or disambiguate. The available tags are: `!string`, `!number`, `!boolean`, `!date`, `!time`, `!datetime`, `!type`. These are optional and rarely needed since types are inferred from the schema:

```yaml
first_name: !string "Bob"
birth_date: !date "1990-07-04"
count: !number 42
active: !boolean true
```

## Typedown Schema

A schema file self-identifies by setting `_type: schema`. It defines the shape of resources that reference it: what properties they have and their types. Schema files live under `_types/` in the vault.

Use the `?` postfix to mark a property as nullable. A nullable property accepts either its declared type or `null`. Omitted nullable fields default to `null`.

A schema can extend another schema using `_extends`:

```yaml
---
_type: schema
properties:
  first_name:
    type: string
  birth_date:
    type: date?
  tags:
    type: list[string]?
  status:
    type: ["draft", "published", "archived"]
---
```

```yaml
---
_type: schema
_extends: Person
properties:
  agency:
    type: string?
---
```

A property can declare a default value using `default`:

```yaml
properties:
  status:
    type: ['todo', 'in_progress', 'done']
    default: "todo"
  count:
    type: number
    default: 0
```

## Typedown Body (Markdown Mode)

The body of a `.td` file is written in Typedown Markdown, an extension of standard Markdown with Typedown-specific syntax.

There are some limitations though:

- Typedown does not support HTML tags.
  - Context: HTML allows a wide range of semantic structures to be expressed. Moreover, HTML also allows the `style` element, which can be used to style the doc.
  - The decision: Typedown mostly concerns itself with the document structure and relationship, which is believed to be adequately covered by the built-in primitive. Typedown does not really care about the presentation. Typedown processor will allow the generation of static sites, which can integrate with most frontend frameworks for flexible presentation.
- Typedown does not implicit code blocks via indentation: I think code fences are more flexible and explicit while still being concise.

### Headings

Headings use the standard `#` syntax:

```markdown
# Heading 1

## Heading 2

### Heading 3
```

### Code

Code spans use backticks. The number of backticks in the opening fence must match the closing fence. A code span is inline by default. A code block is a code span where the content starts and ends with a newline (i.e. the opening fence is followed by a newline, and the closing fence is preceded by a newline):

````markdown
`inline code`

``code with ` inside``

```
code block
multiple lines
```

```python
print("hello")
```
````

Since the delimiter is matched by count, backticks inside code can be used freely as long as the count differs from the fence. For example, `` `a`` ` `` contains a literal double backtick.

### Blockquotes

```markdown
> This is a blockquote.
```

### Math

Math spans use `$`. Like code spans, the number of `$` in the opening delimiter must match the closing delimiter. A math span is inline by default. A math block is a math span where the content starts and ends with a newline:

```markdown
The formula is $E = mc^2$.

$$E = mc^2$$

$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$
```

Since the delimiter is matched by count, `$` inside `$$` is treated as a literal character:

```markdown
$$x = $100$$
```

Math content is raw text. No interpolation (`${...}`) is supported inside math spans. To embed a computed value in a math context, close the math span, interpolate, then reopen:

```markdown
$x = $ ${self.value} $ + 1$
```

### Tables

Tables use the standard Markdown pipe syntax:

```markdown
| Name  | Age |
| ----- | --- |
| Alice | 30  |
| Bob   | 25  |
```

### Lists

Bullet lists use `-` or `*`:

```markdown
- item one
- item two
  - nested item
```

Ordered lists use a number followed by `.`:

```markdown
1. first
2. second
   1. nested
```

Toggle lists use `>-`. The item is collapsed by default and can be expanded:

```markdown
>- Summary line
>  Content shown when expanded.
```

### Callout Blocks

Callout blocks use `:::` with an optional type label. The label supports kebab-case identifiers:

```markdown
::: note
This is a note.
:::

::: warning
This is a warning.
:::
```

A self-closing shorthand uses `[[label]]`:

```markdown
[[directory-index]]
```

### Multimedia

Multimedia embeds images, video, audio, and iframes using the standard Markdown image syntax. The type is inferred from the URL or file extension:

```markdown
![alt text](image.png)
![demo](video.mp4)
![podcast](audio.mp3)
![embed](https://www.youtube.com/embed/dQw4w9WgXcQ)
```

### Links

Standard Markdown links are supported. Links can point to external URLs or to other `.td` files by filename:

```markdown
[Anthropic](https://anthropic.com)
[Bob](people/bob)
```
