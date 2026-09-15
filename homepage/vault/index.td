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

Every document starts with a schema that declares its shape, and a content file that fills in the data.

A schema defines the shape of a node:

```yaml
---
_type: schema
properties:
  name:
    type: string
  role:
    type: 'developer' | 'designer' | 'manager'
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

Here is what Typedown gives you across the language, editor, and build pipeline.

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

## How it compares

Typedown occupies a specific niche: typed structured content with a file-per-node graph model. Here is how it relates to tools you may already use.

| | Typedown | Notion | Obsidian | MDX / Contentlayer |
| --- | --- | --- | --- | --- |
| **Data model** | Typed graph (schemas + fref) | Relational databases | Link graph (untyped) | Frontmatter (unvalidated) |
| **Type checking** | Yes, inline in editor | No | No | Build-time only (Zod) |
| **Autocompletion** | Schema fields, types, frefs | Database properties | Wiki links | None |
| **Storage** | Plain `.td` files in git | Cloud (proprietary) | Plain `.md` files in git | Plain `.mdx` files in git |
| **Relations** | `fref` with type-checked traversal | Relation properties | `[[wiki links]]` (untyped) | Manual imports |
| **Site generation** | Built-in SSG | Notion pages | Requires plugin (Quartz, etc.) | Requires framework (Next, Astro) |
| **Expressions** | `self.field`, closures, interpolation | Formulas | Dataview plugin | JS/TS in MDX |
| **Editor** | Neovim, VS Code, Zed (LSP) | Web app | Obsidian app | Any text editor |

Typedown is a good fit when you want the flexibility of plain files with the structure of a database. It is not a replacement for Notion's collaboration features or Obsidian's plugin ecosystem.

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

## Roadmap

Typedown is under active development. Here is what is planned next.

- **Dark mode for the generated site** with automatic system preference detection
- **Search improvements** with fuzzy matching and field-scoped filters
- **Computed collections** to query and filter across the vault (like Dataview)
- **Publish to VS Code Marketplace** and Zed extension registry
- **Image optimization** with automatic resizing and format conversion
- **Incremental builds** for faster production builds on large vaults

See the [changelog](https://github.com/huydo862003/typerighter/blob/main/CHANGELOG.md) for what shipped recently.

## Learn more

- [Getting started](01-getting-started): install Typedown, create a project, set up your editor, and write your first file
- [Guide](02-guide/index): understand vaults, documents, schemas, expressions, and references
- [Reference](03-references/index): look up exact syntax for frontmatter, markdown, types, icons, and the CLI

## Contact

Typedown is open source under the MIT license.

- [GitHub](https://github.com/huydo862003/typerighter) for source code and releases
- [Issues](https://github.com/huydo862003/typerighter/issues) for bug reports and feature requests
- [Discussions](https://github.com/huydo862003/typerighter/discussions) for questions and ideas
