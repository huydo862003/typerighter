# Typedown Abstract Model

This document specifies the abstract model of Typedown, independent of any serialization format.

The model is inspired by the [property graph model](../research/graph-database/graph-databases-book/2-concept-property-graph-model.md) from graph databases and [RDF](../research/web-3/technologies/rdf.md) from the semantic web.

For a complete, opinionated serialization of the resource graph, see [Typedown](./typedown.md) (how individual resources are serialized as markdown with YAML frontmatter) and [Typedown Vault](./typedown-vault.md) (how a collection of Typedown files is organized on disk). Note that the vault is purely an organization convention for structuring Typedown files on disk and has no meaning in the abstract model itself.

## Resource Graph

A Typedown project is a graph of [resources](#resources), which act as nodes. The graph model makes it natural to traverse connections, follow references, and query by structure rather than by table shape.

The graph is directed with **named, typed edges**: every [link](#links) has a source resource, a target resource, a name (what the relationship means), and a type. Nodes are also typed via [schemas](#schemas).

## Resources

A resource is the fundamental unit in Typedown. It represents any entity: a note, a person, a book, a tag, a project. Every resource has a unique identifier (its file path within the vault) that remains stable regardless of how the resource is displayed or serialized.

Everything attached to a resource is a [**property**](#properties). Properties differ only in what their value is:

- A scalar value (e.g. a name, a date, a number).
- A [**link**](#links) to another resource, forming an edge in the graph.
- A [**schema**](#schemas) reference, typing the resource itself.

## Schemas

A schema is a resource that describes other resources. Where a regular resource represents content (a note, a person, a book), a schema represents structure: it governs what properties a resource has and what types those properties hold.

A resource declares its schema using `_type`. A resource conforms to exactly one schema (not multiple). Schema inheritance is supported via `_extends`: a child schema inherits all properties from its parent and can add its own. The effective shape of a resource is the union of all fields from the schema's `_extends` chain.

`schema` is itself a built-in type. Schema files are identified by `_type: schema`.

## Properties

A property is a named value attached to a resource. Every property has:

- A **name**: identifies the property on the resource.
- A **value**: one or more values of a supported type.

Supported value types are:

- `string`: a Unicode text value.
- `number`: a floating-point number.
- `boolean`: `true` or `false`.
- `date`: an ISO 8601 date.
- `time`: an ISO 8601 time.
- `datetime`: an ISO 8601 datetime.
- `link`: a reference to another resource, forming an edge in the graph (see [Links](#links)).
- `list[T]`: a list of values of type `T`.
- `dict[K, V]`: a homogeneous mapping from keys of type `K` to values of type `V`.
- A fixed-key dict: a mapping where each named key has its own independently typed value.

A union type combines multiple types: `[string, number]` accepts either. A union of string or number literals serves as an enum: `['draft', 'published']`.

The `?` postfix marks a type as nullable: `string?` accepts `string` or `null`.

Whether a property is required or optional, and any constraints on its values, are enforced by the resource's [schema](#schemas).

A property value can be a static value or an **expression**. The two are interchangeable: any property that holds a value can hold an expression instead. Expressions are evaluated lazily on read, and can reference:

- Other properties on the same resource.
- Properties on linked resources, traversing the graph.
- Built-in functions.

## Labels

A label is a human-friendly name for a resource. It is itself a property (`_label`), but one with special semantics: it exists purely for display and identification in the UI rather than for data storage or querying.

A label is typically a string expression that interpolates other properties:

```yaml
_label: "${self.first_name} ${self.last_name}"
```

The label changes whenever the underlying properties change.

## Links

A link is a property whose value is a reference to another resource. It forms a directed edge in the resource graph, connecting exactly two resources. Links are expressed using `fref("path/to/file.td")`.

A link is defined by two things:

- A **name**: the property name on the source resource (e.g. `author`, `assignee`).
- A **target schema**: the schema that the target resource must conform to. This constrains what kind of resource can be linked to.

Links do not carry properties of their own. If a relationship needs its own data (e.g. a role, a date, a weight), the recommended approach is to model it as a new resource with its own schema, and link both parties to that resource.

### Forward Links

A forward link is a link as seen from the source resource: it is a named property whose value is the target resource.

### Back Links

A back link is the same link as seen from the target resource: it exposes the incoming references to a resource. Back links are implicit: they are derived by traversing the graph in reverse. No explicit declaration is needed on the target resource.
