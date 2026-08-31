Schemas are node types. They define what fields a document has and what types those fields hold. Without a schema, a document is untyped. With one, the editor gives you autocompletion, type checking, and diagnostics.

## Defining a schema

A schema file lives in `_types/` and sets `_type: schema`. The `properties` mapping declares each field.

```yaml
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

## Built-in types

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

Append `?` to make a field nullable. Omitted nullable fields default to `null`.

```yaml
properties:
  email:
    type: string?
  tags:
    type: list[string]?
```

## Enums and union types

A YAML list of types or literals. The field accepts any member.

```yaml
properties:
  status:
    type: ['todo', 'in_progress', 'done']
  priority:
    type: ['low', 'medium', 'high']
```

String literals must be quoted. Unquoted values are type names:

```yaml
type: [string, number]   # accepts a string or a number
type: ['draft', 'live']  # accepts "draft" or "live"
```

## Link fields

Use a schema name as a type to declare a link. The field expects a `fref` to a resource of that type. This is how you create edges in the graph.

```yaml
properties:
  assignee:
    type: Person?
  tasks:
    type: list[Task]?
```

## Nested objects

Inline a mapping to define a nested object without a separate schema:

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

A schema can extend another using `_extends`. The child inherits all fields from the parent and can add its own.

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

Resources conforming to the child must provide fields from both parent and child.

## Default values

Fields can declare defaults. If a resource omits the field, the default is used.

```yaml
properties:
  status:
    type: ['todo', 'in_progress', 'done']
    default: "todo"
  count:
    type: number
    default: 0
```

## Putting it together

Here is a schema that uses most type features:

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
    type: ['planning', 'active', 'done']
    default: "planning"
  started:
    type: date?
  archived:
    type: boolean?
  tags:
    type: list[string]?
  members:
    type: list[Person]?
  tasks:
    type: list[Task]?
  lead:
    type: Person?
  metadata:
    type:
      priority:
        type: ['low', 'medium', 'high']
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
archived: false
tags:
  - "frontend"
  - "design"
members:
  - fref("people/alice.td")
  - fref("people/bob.td")
tasks:
  - fref("tasks/design-mockups.td")
  - fref("tasks/implement-auth.td")
lead: fref("people/alice.td")
metadata:
  priority: "high"
  budget: 50000
---

## Goals

Rebuild the public site with a consistent design system.
```

Every field here is type-checked against the schema. The editor autocompletes field names, validates values, and resolves `fref` links.
