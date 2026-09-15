---
_label: "Vault and documents"
_icon: icon.folder
description: "The project structure, configuration, file anatomy, and how content forms a graph."
---

A vault is the top-level container for all your content. This page covers how a project is organized, what goes in the config file, and how individual documents are structured.

## Project structure

A Typedown project is a directory with a `typedown.yaml` config file. The config points to a vault directory that holds all content, schemas, and assets.

```
my-project/
  typedown.yaml          # project config
  vault/                 # vault root
    _types/              # schema definitions (node types)
      Person.td
      Task.td
    _partials/           # internal files (not published)
      colors.td
    _assets/             # images, PDFs, etc.
      logo.png
    people/              # content directories
      alice.td
      bob.td
    tasks/
      setup-ci.td
    index.td             # landing page
```

Three kinds of things live in a vault:

1. **Content files**: `.td` files that become pages on the site. They are the nodes of the graph.
2. **Schema files**: `.td` files in `_types/` that define node types. They declare what fields a resource has.
3. **Internal files**: `.td` files in `_`-prefixed directories (like `_partials/`, `_drafts/`). They are excluded from the site but can be imported by other files.

## Configuration

The `typedown.yaml` config sits at the project root. It tells Typedown where the vault is and what metadata to use for the generated site.

```yaml
version: "1.0.0"
vault:
  root_dir: "vault"
repo: "https://github.com/user/project"
site:
  title: "My Knowledge Base"
  description: "A typedown site"
```

| Key | Description |
| --- | --- |
| `vault.root_dir` | Path to the vault directory, relative to `typedown.yaml`. Use `"."` if the vault root is the same directory. |
| `repo` | Optional repository URL shown in the site header. |
| `site.title` | Site title for HTML and the sidebar. |
| `site.description` | Site description for HTML meta tags. |

## Internal directories

Any directory starting with `_` (other than `_types/`) is internal. Internal files are excluded from the sidebar, search, and site navigation. They still participate in the type system and can be imported using `_imports`.

Common internal directories:

- `_partials/` for shared configuration or reusable data
- `_assets/` for images, PDFs, and other binary files
- `_drafts/` for work-in-progress content

## Index files and ordering

A file named `index.td` in any directory serves as the landing page for that directory. `vault/people/index.td` renders at `/people/`.

Files and directories sort alphabetically in the sidebar. Prefix a name with a number to control the order:

```
01-getting-started.td
02-vault-structure.td
03-schemas.td
```

The number prefix is stripped from the display name.

## Anatomy of a `.td` file

Every `.td` file is a node in the vault graph. A document has two parts: a YAML frontmatter that holds structured data, and a markdown body that holds prose.

```yaml
---
_type: Person
_label: "Alice Chen"
_icon: icon.user
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

The frontmatter is the block between `---` markers. The body is everything after the closing `---`. Both are optional: a file with no `---` markers is treated as body-only.

## Frontmatter

The frontmatter is a YAML mapping of key-value pairs. Every value is an expression: strings, numbers, booleans, lists, and computed values all work.

```yaml
---
_type: Task
title: "Implement auth"
priority: 3
done: false
tags:
  - "backend"
  - "security"
assignee: fref("people/alice.td")
---
```

Keys starting with `_` are reserved for built-in directives that control how Typedown treats the file:

| Key | Purpose |
| --- | --- |
| `_type` | Schema this document conforms to |
| `_label` | Display name shown in the sidebar and links |
| `_icon` | Page icon (e.g., `icon.rocket`) |
| `_extends` | Parent schema (schema files only) |
| `_imports` | File imports into scope |

## Markdown body

The body supports standard markdown plus Typedown extensions. Here is a quick tour of what is available.

**Text formatting**: `**bold**`, `_italic_`, `~~strikethrough~~`, `` `inline code` ``

**Math**: inline $E = mc^2$ with `$E = mc^2$`, and block math with `$$..$$`.

**Code blocks** with syntax highlighting and optional language labels.

**Tables**, **blockquotes**, **task lists**, and **callout blocks** (note, tip, warning, danger, details, info).

**String interpolation** with `${...}` to embed expressions in prose:

```markdown
This task is assigned to ${self.assignee.name}.
```

The expression is evaluated at build time and replaced with the result.

**Container shorthands** like `[[toc]]` and `[[directory-index]]` insert built-in components.

See the [Markdown reference](../03-references/02-markdown-body) for the full syntax.

## Next steps

Documents get their structure from schemas. The next page covers how to define node types, use the type system, write expressions, and link files together with references.
