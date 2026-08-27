# Typedown Vault

A Typedown vault is a directory containing a `typedown.yaml` file at its root. This file is the entrypoint: its presence marks the directory as a Typedown vault and configures where the vault's content is located.

A vault is purely an organization convention: it defines how [Typedown](./typedown.md) files are arranged on disk & how cross-file definitions are resolved. It has no meaning in the [Typedown abstract model](./typedown-model.md), which only concerns itself with the resource graph and its contents.

## Vault Layout

A typical vault looks like:

```
my-project/
  typedown.yaml
  vault/
    _types/
      Person.td
      Task.td
    _partials/
      colors.td
    _assets/
      logo.png
    people/
      alice.td
      bob.td
    tasks/
      setup-ci.td
    index.td
```

### Content Files

Content files are `.td` files that represent resources. They can live at any depth under the vault root.

### Schema Files

Schema files live under `_types/` and define the shape of resources. The `_types/` directory can be nested arbitrarily:

```
_types/
  Person.td
  tracker/
    Task.td
    Milestone.td
```

Schema names use PascalCase (`Person`, `Task`). The file name (without extension) becomes the schema name that resources reference with `_type`.

### Internal Directories

Any directory whose name starts with `_` (other than `_types/`) is an **internal directory**. Files inside internal directories are excluded from content discovery: they don't appear in the sidebar, search index, or site navigation. They do participate in the type system and can be imported by other files.

Common internal directories:

- `_partials/` for shared configuration or reusable data (imported via `_imports`)
- `_assets/` for images, PDFs, and other binary files
- `_drafts/` for work-in-progress content

### Assets

Assets (images, PDFs, etc.) can live anywhere in the vault. The paste handler in editor integrations saves clipboard images to `_assets/` next to the current file by convention.

See [Typedown](./typedown.md) for how individual files are structured.

## typedown.yaml

`typedown.yaml` (or `typedown.yml`) holds global vault and site configuration. It has the following fields:

```yaml
version: "1.0.0"
vault:
  root_dir: "vault"
repo: "https://github.com/user/project"
site:
  title: "My Knowledge Base"
  description: "A typedown site"
```

- `version`: the Typedown format version.
- `vault.root_dir`: path to the vault root directory, relative to the location of `typedown.yaml`. Use `"."` if the vault root is the same directory as `typedown.yaml`.
- `repo`: optional repository URL, shown in the generated site.
- `site.title`: the site title, used in the HTML title and sidebar header.
- `site.description`: the site description, used in HTML meta tags.
