A step-by-step introduction to writing content with Typedown.

## What is a vault?

A vault is a directory of `.td` files managed by a `typedown.yaml` config file. The config tells Typedown where to find your content.

```
my-project/
  typedown.yaml
  vault/
    _types/
      Person.td
    people/
      alice.td
      bob.td
    index.td
```

The `vault/` directory is the vault root. Everything inside it is your content. The `_types/` directory holds schema definitions. Everything else is content files.

## Writing a file

A `.td` file has two parts: a YAML frontmatter and a markdown body.

```
---
_type: Person
name: "Alice"
role: "developer"
---

Alice is a backend developer.
```

The frontmatter is between the `---` markers. It holds structured data. The body is everything after and is regular markdown.

The frontmatter is optional. A file with no `---` markers is treated as a body-only file.

## Defining a schema

Schemas live in `_types/` and define the shape of your content. A schema declares what properties a resource has and their types.

```yaml
---
_type: schema
properties:
  name:
    type: string
  role:
    type: ['developer', 'designer', 'manager']
  email:
    type: string?
  tags:
    type: list[string]?
---
```

The `?` postfix makes a field nullable. Omitted nullable fields default to `null`.

Schema names match the file name (without extension). A file at `_types/Person.td` creates a schema named `Person`. Resources reference it with `_type: Person`.

## Linking files

Use `fref` to link one file to another. The path is relative to the vault root.

```yaml
---
_type: Task
title: "Implement auth"
assignee: fref("people/alice.td")
---
```

You can traverse links with dot access in expressions:

```yaml
assignee_name: self.assignee.name
```

And in the markdown body:

```markdown
Assigned to ${self.assignee.name}.
```
