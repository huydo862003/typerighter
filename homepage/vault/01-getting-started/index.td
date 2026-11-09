---
_label: "Getting Started with Typerighter"
_icon: icon.rocket
description: "Install the Typerighter CLI, scaffold a project with schemas and vault structure, write your first typed document, set up LSP in your editor, and deploy a static site in minutes."
---

Go from zero to a live site in five steps: install the CLI, scaffold a project, write your first document, set up your editor, and deploy.

## Install

Typedown ships as a single npm package. Install it globally so the CLI is available everywhere.

```sh
npm install -g typerighter
```

## Scaffold a project

Create a new directory and run `typerighter init` inside it. The CLI prompts for a project name, site title, and description, then writes the full project structure.

```sh
mkdir my-site && cd my-site
typerighter init
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
    components.td        # built-in component demos (flashcard, glossary)
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

Open `vault/notes/getting-started.td`. Every `.td` file has two parts: a YAML frontmatter block between `---` markers and a markdown body after it.

```yaml
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

The `_type` field tells Typedown which schema this file conforms to. The editor uses it to validate every field and offer autocompletion. Open the page at `http://localhost:8686/notes/getting-started`:

![Getting started page showing typed frontmatter, schema validation, and body interpolation](assets/hello-page.png)

## Edit and see changes live

Add a new section to `vault/notes/getting-started.td` and save. The browser updates instantly without a full reload.

```markdown
## Why Typedown?

- **Typed content**: every field is validated against a schema
- **File references**: link documents together with `fref`
- **Live reload**: edits appear instantly in the browser
```

The scaffold also includes a components page with live demos of `flashcard` and `glossary-index`:

![Components page with flashcard demo and container fence syntax](assets/live-edit.png)

## Deploy

When you are ready to publish, build the site:

```sh
npm run build
```

The output goes to `dist/`. It is a self-contained static site that needs no server. See [Deploying](02-deploying) for platform-specific instructions for GitHub Pages, Netlify, Vercel, and Cloudflare Pages.

## Next steps

- [Set up your editor](01-editor-setup) for autocompletion, type checking, and diagnostics
- [Learn Typedown Syntax](../02-typedown-syntax/index) to understand vaults, schemas, and expressions
- [Configure your site](03-configuration) for custom nav, SEO, and subpath deployments
