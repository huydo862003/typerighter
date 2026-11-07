---
_meta:
  title: "Typerighter & Typedown: Typed Knowledge Graph for Markdown Vaults"
  description: "Build a typed knowledge graph from plain .td files. Schema validation, file references, computed fields, LSP support, and a built-in static site generator."
---

Typerighter is a suite of software that is designed to aid in local-centric plain-text note taking workflows.

Its vision is to support the local-centric note taking workflow:

1. You write your note on your machine using whatever editor you like.
2. You save your notes & commit to a version control system.
3. You can optionally backup your notes on various platforms or published your notes as live web pages.

Because everything stays local, you have every possible choice to do with your note, be it backing up, publishing, sharing, versioning.

As the notes become more complex, you may find the needs to:

- Link notes together into a knowledge base.
- Define metadata for notes.
- Retrieve notes based on metadata.

Typerighter provides tools to achieve these easier, **without locking you into any note editing or storage platforms**:

- Authoring knowledge base:

  - Typerighter provides Typedown, a markdown-variant plain text format that allows defining note metadata and cross-refs.
  - Typedown language services & editor plugins (${fref("01-getting-started/01-editor-setup.td")}) to gain quality of life features like auto-completion, navigating the knowledge base via links, etc.
  - Typedown allows defining note schemas, so it can detect notes that violate the consistency of the knowledge base.
- Publishing knowledge base: Typerighter provides a CLI that can output a website based on your notes which can be published.
- Querying knowledge base: Typedown has a feature called vault query (${fref("02-typedown-syntax/07-vault-query.td")}), which allows you to traverse the knowledge graph and extract meaningful information.

## Backstory on the Web Revolution

Traditionally, documents are treated as if they are a collection of files: People write documents, and save them alongside the other documents.

The Web 2.0 technology has revolutionized this a long time ago: A document may contain links to other documents, making each document a potential entrypoint to a potentially large graph of documents. As human, we typically enter one document, and jump to another one that this one may point to.

However in Web 2.0, for machines, this ability is somewhat limited: Yes, they can find links in the documents and build an understanding of the structure of the document graph. However, they lack a way to unambiguously extract the knowledge latent in the graph. That's why Web 3.0 was born, to facilitate human-independent reasoning about the contents of documents.

## About Typedown

Typedown is a format to write your notes in and linking them within a **vault** (a collection of notes and assets).

Your documents are a typed graph. Typedown is the language for it.

A vault is a collection of `.td` files. Each file is a node. Schemas define node types. File references (`fref`) are edges. The result is a knowledge graph with type safety, autocompletion, and a static site generator built in.

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

- **Graph queries and computed collections** to filter, sort, and aggregate across the vault
- **Graph analysis** for knowledge notes: backlinks, orphan detection, cluster visualization
- **Publish to VS Code Marketplace** and Zed extension registry
- **Incremental builds** for faster production builds on large vaults

See the [changelog](https://github.com/huydo862003/typerighter/blob/main/CHANGELOG.md) for what shipped recently.

## Learn more

- [Getting started](01-getting-started/index): install Typedown, create a project, set up your editor, and write your first file
- [Typedown Syntax](02-typedown-syntax/index): understand vaults, documents, schemas, expressions, and references
- [Default theme](03-custom-theming/index): navigation, search, shortcuts, and appearance
- [Custom Theming](03-custom-theming/01-custom-theming): custom components, layout slots, and full layout replacement
- [Typedown Syntax](02-typedown-syntax/index): look up exact syntax for frontmatter, markdown, types, icons, and the CLI

## Contact

Typedown is open source under the MIT license.

- [GitHub](https://github.com/huydo862003/typerighter) for source code and releases
- [Issues](https://github.com/huydo862003/typerighter/issues) for bug reports and feature requests
- [Discussions](https://github.com/huydo862003/typerighter/discussions) for questions and ideas
