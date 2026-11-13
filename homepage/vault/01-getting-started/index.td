---
_label: "Getting Started with Typerighter"
_icon: icon.rocket
description: "Scaffold a project with npx, write your first typed document, set up LSP in your editor, and deploy a static site in minutes."
---

Start from scratch: scaffold a project, write your first document, set up your editor, and deploy.

## Scaffold a project

Run `npx typerighter init` in an empty directory. The CLI prompts for a project name, site title, and description, then writes the full project structure. No global install needed.

```sh
mkdir my-site && cd my-site
npx typerighter init
```

The result:

```
my-site/
  typedown.yaml          # site config
  vault/
    _types/
      Article.td         # article schema (title, description, tags)
      Note.td            # note schema (title, description, tags, author)
    notes/
      index.td           # glossary index of all notes
      getting-started.td # schemas, body interpolation, frefs
      core-concepts.td   # schema syntax, fref, vault query reference
    index.td             # home page with file structure and commands
    components.td        # built-in component demos
  public/
    favicon.svg
    robots.txt
  package.json
```

Install dependencies, then start the dev server:

```sh
npm install
npm run dev
```

Open `http://localhost:8686`. The site is live with a sidebar, search, and syntax highlighting.

![Scaffold home page with sidebar, file structure, and commands table](assets/initial-page.png)

## Write your first document

Open `vault/notes/getting-started.td`. Every `.td` file has two parts: a YAML frontmatter block between `---` markers that holds structured data, and a markdown body after it. The `_type` field connects the file to a schema - a template that declares which fields are allowed and what types they have, so the editor can validate your input and offer autocompletion.

```yaml{title=vault/notes/getting-started.td}
---
_type: Note
title: "Getting started"
description: "A quick tour of typedown basics"
tags:
  - "intro"
author: "You"
---

This file uses `_type: Note`, defined in `_types/Note.td`.
The `title`, `description`, `tags`, and `author` fields above are all type-checked.
```

Open the page at `http://localhost:8686/notes/getting-started` to see the result:

![Getting started page showing typed frontmatter, schema validation, and body interpolation](assets/hello-page.png)

## Edit and see changes live

Try editing `vault/notes/getting-started.td` by adding a new section to the markdown body and saving:

```markdown{title=vault/notes/getting-started.td}
## Why Typedown?

- **Typed content**: every field is validated against a schema
- **File references**: link documents together with `fref`
- **Live reload**: edits appear instantly in the browser
```

The browser updates instantly without a full reload:

![Live editing with instant browser updates](assets/live-edit.png)

## Deploy

When you are ready to publish, build the site:

```sh
npm run build
```

The output goes to `dist/`. It is a self-contained static site that needs no server. See [Deploying](02-deploying) for platform-specific instructions for GitHub Pages, Netlify, Vercel, and Cloudflare Pages.

## What comes next

As your vault grows, Typedown's type system keeps it consistent. Schemas validate fields, file references (`fref`) link documents into a traversable graph, and vault queries let you compute relationships automatically - for example, finding all tasks assigned to a person without maintaining that list by hand:

```yaml{title=vault/people/alice.td}
---
_type: Person
name: "Alice"
assigned_tasks: vault.from_nodes(Task).filter((task) -> task.assignee == self)
---
```

Explore these features in the guides:

- [Set up your editor](01-editor-setup) for autocompletion, type checking, and diagnostics
- [Learn Typedown Syntax](../02-typedown-syntax/index) to understand vaults, schemas, expressions, and vault queries
- [Configure your site](03-configuration) for custom nav, SEO, and subpath deployments
