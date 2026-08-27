Typedown is a typed markdown language for structured content.

A typedown document is a structured markdown document. It has YAML frontmatter with a specified schema to describe its properties and its relationship with other documents. Based on this, editor features like autocompletion, go-to-definition, and semantic renaming work out of the box. A static site generator with search is built on top.

## What it looks like

A schema defines the shape of your content:

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

## Quick start

Install `typerighter` and scaffold a new project:

```sh
npm install -g typerighter
typerighter init
```

This creates a `typedown.yaml` config and a `vault/` directory with a starter `index.td`. Run the dev server:

```sh
typerighter dev
```

Build for production:

```sh
typerighter build
```

## What you get

- **Typed frontmatter**: define schemas for your content. The editor gives you autocompletion, type checking, and go-to-definition.
- **Schema inheritance**: extend schemas with `_extends` to build type hierarchies.
- **Computed fields**: use expressions in frontmatter values that reference other fields or linked documents.
- **File references**: link documents together with `fref("path/to/file.td")` and traverse the graph with dot access.
- **Modules**: organize internal files in `_`-prefixed directories and import them with `_imports`.
- **Static site generation**: build a full static site with search, sidebar navigation, and syntax highlighting.
- **Editor support**: Neovim, VS Code, and Zed extensions with LSP integration.

Read the [guide](guide/index) to learn more, or jump to the [references](references/index).
