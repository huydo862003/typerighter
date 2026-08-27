Schemas define the shape of your content: what fields a resource has and what types those fields hold.

## Defining a schema

A schema file lives in `_types/` and sets `_type: schema`. The `properties` mapping declares each field with its type.

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

## Built-in types

| Type         | Description                     |
| ------------ | ------------------------------- |
| `string`     | Unicode text                    |
| `number`     | Floating-point number           |
| `boolean`    | `true` or `false`               |
| `date`       | ISO 8601 date (`"2024-01-15"`)  |
| `time`       | ISO 8601 time (`"14:30:00"`)    |
| `datetime`   | ISO 8601 datetime               |
| `list[T]`    | List of values of type `T`      |
| `dict[K, V]` | Mapping from `K` keys to `V` values |

## Nullable fields

Append `?` to make a field nullable. Omitted nullable fields default to `null`.

```yaml
properties:
  email:
    type: string?
  tags:
    type: list[string]?
```

## Union types and enums

A union type accepts any of its members. Write it as a YAML list:

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

Use a schema name as a type to declare a link field. The field expects a `fref` to a resource conforming to that schema.

```yaml
properties:
  assignee:
    type: Person?
  tasks:
    type: list[Task]?
```

## Nested object fields

Inline a mapping to define a nested object type without a separate schema:

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

## Schema inheritance

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

Resources conforming to the child schema must provide fields from both parent and child.

## Default values

A field can declare a default value. If the resource omits the field, the default is used.

```yaml
properties:
  status:
    type: ['todo', 'in_progress', 'done']
    default: "todo"
  count:
    type: number
    default: 0
```
