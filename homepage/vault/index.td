---
_meta:
  title: "Typerighter & Typedown: Typed Knowledge Graph for Markdown Vaults"
  description: "Build a typed knowledge graph from plain .td files. Schema validation, file references, computed fields, LSP support, and a built-in static site generator."
---

Typerighter is a **suite of plugins/add-ons** that integrate with existing plain text editors to facilitate:

- Authoring structured markdown notes.
- Linking markdown notes into a graph.
- Building your knowledge graph.
- Exporting a live website for the knowledge base.

Typerighter is not a centralized solution and does not introduce a new platform:

- You still use your favorite editors and your notes are plain markdown files with YAML frontmatter in git.
- Essentially, Typerighter doesn't require you to change your workflow or impose any intrusive changes to your setup.
- Furthermore, you can choose to quit using Typerighter later without having to make heavy migrations.

::: note
Typerighter's philosophy is to enhance current note taking flows and be supplementary to that only.
:::

Typerighter currently provides:

- Editor plugins that give you autocompletion, type checking, and go-to-definition. (See ${fref("./01-getting-started/01-editor-setup.td")})
- An SSG with a default theming so you can export a website out of the knowledge base. (See ${fref("./03-custom-theming/01-custom-theming.td")})
- A CLI tool that allows vault (a folder for storing notes) initialization, building and developing the SSG out of the vault.

## How it works

In Typerighter, we use `.td` files as an acronym for "typedown" (Typed markdown). However, `.md` files are equally accepted.

A project has a `typedown.yaml` config file at the root that points to the vault directory. The vault is a folder of `.td` files that form a knowledge network:

- Each file in the vault is a node in the network.
- Each file can optionally have a schema, which defines the general shape of the node and enables graph traversal and querying.
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

A project starts with a `typedown.yaml` config that tells Typerighter where the vault lives:

```yaml{title="typedown.yaml"}
version: "1"
vault:
  root_dir: "vault"
```

Inside the vault, a `_types/` directory holds schemas, files that declare the shape of a node:

```yaml{title="vault/_types/Person.td"}
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

A content file uses `_type` to reference a schema. Its frontmatter fields are type-checked and it can have a full markdown body:

```yaml{title="vault/people/alice.td"}
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

Link files together with `fref` and traverse the graph. Computed fields follow references at build time:

```yaml{title="vault/tasks/task-1.td"}
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

![Scaffold home page with sidebar navigation, file structure, and commands table](01-getting-started/assets/initial-page.png)

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

Scaffold a new project with `npx`, no global install needed:

```sh{title="Scaffold a new project"}
npx typerighter init
```

The CLI prompts for a project name and description, then writes the full vault structure:

```
my-site/
  typedown.yaml          # site config
  vault/
    _types/
      Article.td         # article schema
      Note.td            # note schema
    notes/
      index.td           # notes index
      getting-started.td # example document
    index.td             # home page
  package.json
```

Start the dev server:

```sh{title="Start dev server"}
npm run dev
```

Build for production:

```sh{title="Build for production"}
npm run build
```

See [Getting Started](01-getting-started/index) for a full walkthrough: editor setup, writing documents, and deploying your site.

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
