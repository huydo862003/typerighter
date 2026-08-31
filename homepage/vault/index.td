Your documents are a typed graph. Typedown is the language for it.

A vault is a collection of `.td` files. Each file is a node. Schemas define node types. File references (`fref`) are edges. The result is a knowledge graph with type safety, autocompletion, and a static site generator built in.

```
                      Vault (graph)
          ┌──────────────┴──────────────┐
          │                             │
     ┌────┴────┐                   ┌────┴────┐
     │  alice   │───assignee───▶   │  task-1  │
     │ (Person) │                  │  (Task)  │
     └─────────┘                   └────┬────┘
                                        │
                                   milestone
                                        │
                                   ┌────▼────┐
                                   │ launch  │
                                   │(Mstone) │
                                   └─────────┘

     Nodes  = .td files
     Types  = schemas in _types/
     Edges  = fref("path/to/file.td")
```

Typedown draws from Notion's relational databases, Obsidian's linked vault model, and TypeScript's structural type system. If you have used any of these, the mental model will feel familiar.

## What it looks like

A schema defines the shape of a node:

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
---
```

A content file conforms to a schema and has a markdown body:

```yaml
---
_type: Person
name: "Alice Chen"
role: "developer"
email: "alice@example.com"
---

Alice is a backend developer focused on authentication systems.

## Skills

| Area | Level |
| ---- | ----- |
| Rust | Expert |
| TypeScript | Proficient |
```

Link files together with `fref` and traverse the graph:

```yaml
---
_type: Task
title: "Implement auth"
assignee: fref("people/alice.td")
assignee_name: self.assignee.name
---

Assigned to ${self.assignee.name}.
```

## What you get

**Language**

- Typed frontmatter with schemas, inheritance, and nullable fields
- Computed fields with expressions, dot access, and closures
- File references that link documents into a traversable graph

**Editor**

- Autocompletion for schema fields, types, and file references
- Type checking with inline error diagnostics
- Go-to-definition, semantic renaming, and hover info
- Neovim, VS Code, and Zed extensions via LSP

**Build**

- Static site generation with search and sidebar navigation
- Syntax highlighting, math rendering, and callout blocks
- Per-page icons, breadcrumbs, and prev/next navigation

## Quick start

Install and scaffold a new project:

```sh
npm install -g typerighter
typerighter init
```

Start the dev server:

```sh
typerighter dev
```

Build for production:

```sh
typerighter build
```

## Learn more

- [Getting started](getting-started): install Typedown, create a project, and write your first file
- [Editors](editors): set up Neovim, VS Code, or Zed
- [Concepts](concepts/index): understand the vault, documents, schemas, expressions, and references
- [Reference](references/index): look up exact syntax for frontmatter, markdown, types, and the CLI
