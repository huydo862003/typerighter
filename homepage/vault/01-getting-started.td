---
_label: "Getting Started"
_icon: icon.rocket
description: "Install Typedown, create a project, and write your first document."
---

This page gets you from zero to a running dev server in under a minute. Editor setup, authoring concepts, and deployment are covered in the [Guide](02-guide/index).

## Install

Typedown ships as a single npm package. Install it globally so the CLI is available everywhere.

```sh
npm install -g typerighter
```

## Create a project

Scaffold a new project with `typerighter init`. This creates the vault directory, a sample schema, and the config file.

```sh
mkdir my-project && cd my-project
typerighter init
```

The scaffolded structure looks like this:

```
my-project/
  typedown.yaml          # project config
  vault/
    _types/
      Article.td         # sample schema
    hello.td             # sample content
  index.html
  package.json
```

## Start the dev server

Run the dev server and open the browser. Changes to `.td` files reload automatically.

```sh
typerighter dev
```

Open `http://localhost:8686`. You should see your vault rendered as a site with a sidebar, search, and syntax highlighting.

## Write a file

Open `vault/hello.td`. A `.td` file has two parts: YAML frontmatter between `---` markers and a markdown body after.

```yaml
---
_type: Article
title: "My first document"
tags:
  - "getting-started"
---

This is the body. It supports **bold**, _italic_, `code`, tables, math ($E = mc^2$), and more.
```

The `_type` field tells Typedown which schema this file conforms to. The editor uses it to offer autocompletion and type checking for every field. Save the file and the dev server reloads automatically.

## Next steps

- [Set up your editor](02-guide/01-editor-setup) for autocompletion and type checking
- [Learn the concepts](02-guide/index) behind vaults, schemas, and references
- [Deploy your site](02-guide/04-deploying) to GitHub Pages, Netlify, or any static host
