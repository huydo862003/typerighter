---
_meta:
  title: "Typerighter & Typedown: Typed Knowledge Graph for Markdown Vaults"
  description: "Build a typed knowledge graph from plain .td files. Schema validation, file references, computed fields, LSP support, and a built-in static site generator."
---

Typerighter is a suite of plugins/add-ons that integrate with existing plain text editors to facilitate authoring structured notes, linking notes into a graph and thereby enabling building your knowledge graph.

Typerighter is not a centralized solution or introduces new platform: You still use your favorite editors and your notes are plain markdown files with YAML frontmatters in git. Therefore, it doesn't require you to change your workflow or impose any intrusive changes to your setup. Furthermore, you can choose to quit using Typerighter later without having to make heavy migrations.

::: note
Typerighter's philosophy is to enhance and be supplementary.
:::

Typerighter currently provides:

- Editor plugins that gives you autocompletion, type checking, and go-to-definition. (See ${fref("./01-getting-started/01-editor-setup.td")})
- An SSG system with a default theming so you can export a website out of the knowledge base.

## How it works

In Typerighter, we use `.td` files as an acronym for "typedown" (Typed markdown). However, `.md` files are equally accepted.

A vault is a collection of `.td` files, interlinked into a knowledge network:

- Each file in the vault is a node in the network.
- Each file can optionally have a schemas, which define the general shape of the nodes, which enable various analysis such as graph traversal, graph query.
- Files can reference each other via `fref` or markdown links. These are edges in the network.

The result is a knowledge graph with type safety, autocompletion, and a static site generator built in.

```
                      Vault (graph)
          ┌──────────────┴──────────────┐
          │                             │
     ┌────┴────┐                   ┌────┴────┐
     │  alice  │  ───assignee───▶  │  task-1 │
     │ (Person)│                   │  (Task) │
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
     Edges  = fref("/path/to/file.td")
```

Typedown draws from:

- Notion's relational databases
- Obsidian's linked vault model
- TypeScript's structural type system
- Semantic web technology, which allows machines to parse and process human nodes.

If you have used any of these, the mental model will feel familiar.

## What a vault looks like

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
assignee: fref("./people/alice.td")
assignee_name: self.assignee.name
---

Assigned to ${self.assignee.name}.
```

## What you get

Here is what Typedown gives you across the language, editor, and build pipeline.

**Language** - the Typedown format gives your notes structure that tools can understand:

- Typed frontmatter with schemas, inheritance, and nullable fields
- Computed fields with expressions, dot access, and closures
- File references that link documents into a traversable graph

**Editor** - an LSP server brings IDE-grade tooling to your existing editor:

- Autocompletion for schema fields, types, and file references
- Type checking with inline error diagnostics
- Go-to-definition, semantic renaming, and hover info
- Neovim, VS Code, and Zed extensions

**Build** - a CLI that turns your vault into a polished static site:

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

- **Graph queries and computed collections** to filter, sort, and aggregate across the vault
- **Graph analysis** for knowledge notes: backlinks, orphan detection, cluster visualization
- **Publish to VS Code Marketplace** and Zed extension registry
- **Incremental builds** for faster production builds on large vaults

See the [changelog](https://github.com/huydo862003/typerighter/blob/main/CHANGELOG.md) for what shipped recently.

## Learn more

- [Getting started](01-getting-started/index): install, scaffold a project, write your first document, and deploy
- [Typedown Syntax](02-typedown-syntax/index): vaults, schemas, expressions, references, and vault queries
- [Custom Theming](03-custom-theming/01-custom-theming): custom components, layout slots, and full layout replacement
- [FAQ](06-faq): common questions about schemas, deployment, and how Typedown compares to other tools

## Contact

Typedown is open source under the MIT license.

- [GitHub](https://github.com/huydo862003/typerighter) for source code and releases
- [Issues](https://github.com/huydo862003/typerighter/issues) for bug reports and feature requests
- [Discussions](https://github.com/huydo862003/typerighter/discussions) for questions and ideas
