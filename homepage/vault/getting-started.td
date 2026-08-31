Install Typedown and write your first document.

## Install

```sh
npm install -g typerighter
```

## Create a project

```sh
mkdir my-project && cd my-project
typerighter init
```

This creates:

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

```sh
typerighter dev
```

Open `http://localhost:8686`. You should see your vault rendered as a site with a sidebar, search, and syntax highlighting.

## Write a file

Open `vault/hello.td`. A `.td` file has two parts: YAML frontmatter and a markdown body.

```yaml
---
_type: Article
title: "My first document"
tags:
  - "getting-started"
---

This is the body. It supports **bold**, _italic_, `code`, tables, math ($E = mc^2$), and more.
```

The frontmatter between `---` markers holds structured data. The `_type` field tells Typedown which schema this file conforms to. The body is standard markdown with some extensions.

Save the file. The dev server reloads automatically.

## Build for production

```sh
typerighter build
```

This generates a static site in `dist/`. Deploy it anywhere that serves static files.

## Next steps

- [Set up your editor](editors) for autocompletion and type checking
- [Understand the concepts](concepts/index) behind vaults, schemas, and references
