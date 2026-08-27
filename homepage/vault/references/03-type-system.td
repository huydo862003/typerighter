Type expressions appear in schema property definitions under the `type` key. Types are inferred for resource files, so explicit type annotations are only needed in schemas.

## Primitive types

| Type       | Example value        |
| ---------- | -------------------- |
| `string`   | `"hello"`            |
| `number`   | `42`, `3.14`         |
| `boolean`  | `true`, `false`      |
| `date`     | `"2024-01-15"`       |
| `time`     | `"14:30:00"`         |
| `datetime` | `"2024-01-15T14:30"` |

```yaml
properties:
  name:
    type: string
  birth_date:
    type: date
  active:
    type: boolean
```

## Nullable types

Append `?` to any type to make it nullable. The field accepts either the declared type or `null`. Omitted nullable fields default to `null`.

```yaml
type: string?
type: date?
type: list[string]?
```

## List types

`list[T]` is a list where every element has type `T`.

```yaml
type: list[string]
type: list[number]
type: list[Task]
```

## Dict types

`dict[K, V]` is a homogeneous mapping from keys of type `K` to values of type `V`.

```yaml
type: dict[string, number]
```

## Fixed-key object types

Inline a mapping to define a nested object type:

```yaml
type:
  street:
    type: string
  city:
    type: string
  zip:
    type: number
```

## Union types

A YAML list of types or literals. Accepts any member.

```yaml
type: [string, number]
```

## Enum types

A union of string or number literals:

```yaml
type: ['todo', 'in_progress', 'done']
type: [1, 2, 3]
```

String literals must be quoted to distinguish them from type names.

## Literal types

A type with exactly one valid value:

```yaml
type: 1           # must be the number 1
type: "draft"     # must be the string "draft"
```

## Schema references

Use a schema name as a type to declare a link field. The field expects a `fref` to a resource conforming to that schema.

```yaml
type: Person           # link to a Person resource
type: Person?          # optional link
type: list[Task]       # list of links
```
