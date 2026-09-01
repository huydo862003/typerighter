A vault is the top-level container for all your content. Think of it as a typed graph: files are nodes, schemas are node types, and file references are edges.

## Project structure

A Typedown project is a directory with a `typedown.yaml` config file. The config points to a vault directory that holds everything.

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

The `typedown.yaml` config sits at the project root:

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
