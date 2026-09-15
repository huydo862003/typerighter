---
_label: "Schemas and types"
_icon: icon.database
description: "Define node types with schemas, use the type system, write expressions, and link files with references."
---

Schemas give structure to your documents. This page covers schema definitions, the type system, computed expressions, and file references that link documents into a graph. These are the features that set Typedown apart from plain markdown.

## Defining a schema

A schema file lives in `_types/` and sets `_type: schema`. The `properties` mapping declares each field and its type.

```yaml
# _types/Person.td
---
_type: schema
properties:
  name:
    type: string
  birth_date:
    type: date?
  active:
    type: boolean?
---
```

The file name becomes the schema name: `_types/Person.td` creates a schema named `Person`. Content files reference it with `_type: Person`.

Without a schema, a document is untyped. With one, the editor gives you autocompletion, type checking, and inline diagnostics for every field.

## Built-in types

Typedown supports these primitive and compound types out of the box.

| Type | Description | Example |
| ---- | ----------- | ------- |
| `string` | Unicode text | `"hello"` |
| `number` | Floating-point number | `42`, `3.14` |
| `boolean` | True or false | `true`, `false` |
| `date` | ISO 8601 date | `"2024-01-15"` |
| `time` | ISO 8601 time | `"14:30:00"` |
| `datetime` | ISO 8601 datetime | `"2024-01-15T14:30"` |
| `list[T]` | List of values | `list[string]` |
| `dict[K, V]` | Key-value mapping | `dict[string, number]` |

## Nullable fields

Append `?` to make a field optional. Omitted nullable fields default to `null`.

```yaml
properties:
  email:
    type: string?
  tags:
    type: list[string]?
```

## Enums and union types

Use `|` to combine types or string literals. The field accepts any member of the union.

```yaml
properties:
  status:
    type: 'todo' | 'in_progress' | 'done'
  priority:
    type: 'low' | 'medium' | 'high'
```

String literals must be quoted. Unquoted values are type names:

```yaml
type: string | number    # accepts a string or a number
type: 'draft' | 'live'  # accepts "draft" or "live"
```

## Nested objects

Inline a mapping to define a nested object without a separate schema file:

```yaml
properties:
  address:
    type:
      street:
        type: string
      city:
        type: string
      zip:
        type: number
```

## Inheritance

A schema can extend another using `_extends`. The child inherits all parent fields and can add its own.

```yaml
# _types/Contractor.td
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

Resources conforming to the child must provide fields from both parent and child.

## Default values

Fields can declare defaults. If a resource omits the field, the default is used.

```yaml
properties:
  status:
    type: 'todo' | 'in_progress' | 'done'
    default: "todo"
  count:
    type: number
    default: 0
```

## Expressions

Every frontmatter value is an expression. Most of the time you write simple literals and the type is inferred. But you can also write computed values that reference other fields, traverse links, and transform data.

`self` refers to the current document. Dot access reads fields:

```yaml
full_name: self.first_name + " " + self.last_name
```

Use `${...}` to embed expressions inside strings, both in frontmatter and in the markdown body:

```yaml
greeting: "Hello, ${self.name}!"
```

### Operators

| Operator | Description |
| -------- | ----------- |
| `+` | Add or concatenate |
| `-`, `*`, `/` | Arithmetic |
| `==`, `!=` | Equality |
| `<`, `>`, `<=`, `>=` | Comparison |
| `&&`, `\|\|` | Logical AND, OR |
| `.` | Property access |
| `[n]` | List/string indexing |

### Closures

Anonymous functions use `(params) -> body` syntax:

```yaml
double: (x) -> x * 2
greet: (name) -> "Hello, ${name}!"
```

## File references

File references are edges in the vault graph. They connect one document to another and let you traverse the connection to read fields from linked resources.

Use `fref("path")` to link to another file. The path is relative to the vault root.

```yaml
---
_type: Task
title: "Implement auth"
assignee: fref("people/alice.td")
---
```

This creates an edge from the Task node to the Person node. The editor resolves the link, checks that the target exists, and verifies that the target's type matches the schema's declared type.

### Typed edges

When a schema field has a schema type (like `Person`), the `fref` target must conform to that schema. The type checker rejects `assignee: fref("tasks/setup-ci.td")` because a Task is not a Person.

```yaml
# _types/Task.td
---
_type: schema
properties:
  title:
    type: string
  assignee:
    type: Person?
  subtasks:
    type: list[Task]?
---
```

### Traversing edges

Dot access crosses edges. If `assignee` is a `fref` to a Person, you can read the Person's fields directly:

```yaml
assignee_name: self.assignee.name
assignee_email: self.assignee.email
```

Use string interpolation to render linked data in the markdown body:

```markdown
This task is assigned to ${self.assignee.name}.
```

The expressions are evaluated at build time. The rendered page shows the resolved values.

### The content graph

Every `fref` is an edge. Every `.td` file is a node. Together they form a directed graph. The type system ensures that edges connect compatible node types, and the evaluator resolves data across edges at build time.

```
    alice.td ◄────── task-1.td ──────► launch.td
    (Person)         (Task)            (Milestone)
                       │
                       ▼
                  task-2.td
                   (Task)
```

This graph is what powers autocompletion across links, type checking of `fref` targets, and the rendered relation lists in the site.

## Putting it together

Here is a schema that uses most features: primitives, enums, nullable fields, links, nested objects, and defaults.

```yaml
# _types/Project.td
---
_type: schema
properties:
  name:
    type: string
  description:
    type: string?
  status:
    type: 'planning' | 'active' | 'done'
    default: "planning"
  started:
    type: date?
  tags:
    type: list[string]?
  members:
    type: list[Person]?
  lead:
    type: Person?
  metadata:
    type:
      priority:
        type: 'low' | 'medium' | 'high'
      budget:
        type: number?
---
```

And a content file conforming to it:

```yaml
# projects/website-redesign.td
---
_type: Project
_label: "Website Redesign"
_icon: icon.globe
name: "Website Redesign"
description: "Overhaul the public site."
status: "active"
started: "2026-03-01"
tags:
  - "frontend"
  - "design"
members:
  - fref("people/alice.td")
  - fref("people/bob.td")
lead: fref("people/alice.td")
metadata:
  priority: "high"
  budget: 50000
---

## Goals

Rebuild the public site with a consistent design system.
```

Every field here is type-checked against the schema. The editor autocompletes field names, validates values, and resolves `fref` links.

For the full syntax of every feature covered here, see the [Reference](../03-references/index) section.
