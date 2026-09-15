# Typedown

![Status](https://img.shields.io/badge/status-alpha-orange)
[![npm](https://img.shields.io/npm/v/typerighter)](https://www.npmjs.com/package/typerighter)
![License](https://img.shields.io/badge/license-GPL-green)
<a href="https://github.com/huydo862003/Fck-AI-Slop#plan"><img src="https://img.shields.io/badge/Human%20slop-90EE90"></a>

Your documents are a typed graph. Typedown is the language for it.

Typedown draws from Notion's relational databases, Obsidian's linked vault model, and TypeScript's structural type system. If you want the flexibility of plain files with the structure of a database, this is for that.

A vault is a collection of `.td` files. Each file is a node. Schemas define node types. File references (`fref`) are edges. The type checker validates everything, and the editor gives you autocompletion and diagnostics as you write.

```yaml
# _types/Person.td - defines the shape of a person
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

```yaml
# people/alice.td - a person
---
_type: Person
name: "Alice Chen"
role: "developer"
email: "alice@example.com"
---

Alice is a backend developer focused on authentication systems.
```

```yaml
# tasks/auth.td - a task that links to a person
---
_type: Task
title: "Implement auth"
assignee: fref("people/alice.td")
assignee_name: self.assignee.name
---

Assigned to ${self.assignee.name}.
```

The schema, the content, and the references all live in plain files. The type checker catches mistakes. The LSP gives you autocompletion for fields, types, and file paths. The static site generator turns the vault into a searchable website.

## What you get

**Language**: typed frontmatter with schemas, inheritance, and nullable fields. Computed fields with expressions, dot access, and closures. File references that link documents into a traversable graph.

**Editor**: autocompletion for schema fields, types, and file references. Type checking with inline diagnostics. Go-to-definition, semantic renaming, and hover info. Works in Neovim, VS Code, and Zed via LSP.

**Build**: static site generation with search and sidebar navigation. Syntax highlighting, math rendering, and callout blocks. Per-page icons, breadcrumbs, and prev/next navigation.

## Quick start

```sh
npm install -g typerighter
typerighter init
typerighter dev
```

Open `http://localhost:8686` and start editing `.td` files. Changes reload automatically.

**[Read the full documentation](https://huydo862003.github.io/typerighter/)** for editor setup, authoring guide, and reference.

> The npm package name `typedown` is unavailable (npm considers it too similar to `typedoc`). The entry package is [`typerighter`](https://www.npmjs.com/package/typerighter) and scoped packages are under [`@typerighter`](https://www.npmjs.com/org/typerighter).

## Editor setup

| Editor | Guide |
| --- | --- |
| Neovim | [editors/nvim/README.md](editors/nvim/README.md) |
| VS Code | [editors/vscode/README.md](editors/vscode/README.md) |
| Zed | [editors/zed/README.md](editors/zed/README.md) |

## Contributing

- [DEVELOPMENT.md](DEVELOPMENT.md) for dev setup, building, releasing, and architecture
- [doc/](doc/) for internal design and research documentation
- [Issues](https://github.com/huydo862003/typerighter/issues) for bug reports and feature requests
- [Discussions](https://github.com/huydo862003/typerighter/discussions) for questions and ideas
- [Changelog](https://github.com/huydo862003/typerighter/blob/main/CHANGELOG.md) for release history

## License

GPL
