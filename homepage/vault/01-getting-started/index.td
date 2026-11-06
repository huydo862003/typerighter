---
_label: "Getting Started"
_icon: icon.rocket
description: "Install Typedown, scaffold a project, write your first document, and deploy to the web."
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
      Article.td         # sample schema
    hello.td             # sample content file
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

![Initial page after scaffold](assets/initial-page.png)

## Write your first document

Open `vault/hello.td`. Every `.td` file has two parts: a YAML frontmatter block between `---` markers and a markdown body after it.

```yaml
---
_type: Article
title: "Hello, world"
description: "The starting point of your vault"
---

Welcome to your new typedown site.
```

The `_type` field tells Typedown which schema this file conforms to. The editor uses it to validate every field and offer autocompletion. Open the page at `http://localhost:8686/hello`:

![hello.td rendered in the browser](assets/hello-page.png)

## Edit and see changes live

Add a new section to `vault/hello.td` and save. The browser updates instantly without a full reload.

```markdown
## Why Typedown?

- **Typed content**: every field is validated against a schema
- **File references**: link documents together with `fref`
- **Live reload**: edits appear instantly in the browser
```

![Live edit reflected in the browser](assets/live-edit.png)

## Deploy

When you're ready to publish, build the site:

```sh
npm run build
```

The output goes to `dist/`. It's a self-contained static site - no server required. See [Deploying](02-deploying) for platform-specific instructions for GitHub Pages, Netlify, Vercel, and Cloudflare Pages.

## Next steps

- [Set up your editor](01-editor-setup) for autocompletion, type checking, and diagnostics
- [Learn Typedown Syntax](../02-typedown-syntax/index) to understand vaults, schemas, and expressions
- [Configure your site](03-configuration) for custom nav, SEO, and subpath deployments
