A Typedown project has three layers: a config file, schema definitions, and content files. This page explains how they fit together.

## The big picture

A Typedown project is a directory with a `typedown.yaml` config file. The config points to a vault directory that holds all your content.

```
my-project/
  typedown.yaml          # project config
  vault/                 # vault root (all content lives here)
    _types/              # schema definitions
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

There are three kinds of things in a vault:

1. **Content files**: regular `.td` files that become pages on the site. They have frontmatter and a markdown body.
2. **Schema files**: `.td` files in `_types/` that define the shape of content. They declare what fields a resource has and their types.
3. **Internal files**: `.td` files in `_`-prefixed directories (like `_partials/`, `_drafts/`). They are excluded from the site but can be imported by other files.

## Config: typedown.yaml

The config file sits at the project root, not inside the vault.

```yaml
version: "1.0.0"
vault:
  root_dir: "vault"
repo: "https://github.com/user/project"
site:
  title: "My Knowledge Base"
  description: "A typedown site"
```

- `vault.root_dir`: path to the vault directory, relative to `typedown.yaml`. Use `"."` if the vault root is the same directory as the config.
- `repo`: optional repository URL shown in the generated site.
- `site.title`: site title for HTML and the sidebar header.
- `site.description`: site description for HTML meta tags.

## Schemas: the `_types/` directory

Schema files live under `_types/`. The directory can be nested for organization. The file name (without extension) becomes the schema name. Schema names use PascalCase.

```
_types/
  Person.td          -> schema name: Person
  tracker/
    Task.td          -> schema name: Task
    Milestone.td     -> schema name: Milestone
```

Content files reference a schema with `_type: Person`. See [Schemas](03-schemas) for how to define and extend schemas.

## Internal directories

Any directory whose name starts with `_` (other than `_types/`) is internal. Files inside are excluded from the sidebar, search, and site navigation. They still participate in the type system and can be imported by other files using `_imports`.

Common internal directories:

- `_partials/` for shared configuration or reusable data
- `_assets/` for images, PDFs, and other binary files
- `_drafts/` for work-in-progress content

## Index files

A file named `index.td` in any directory serves as the landing page for that directory. `vault/people/index.td` renders at `/people/`.

## Ordering

Files and directories in the sidebar are sorted alphabetically by default. Prefix a name with a number to control the order: `01-getting-started.td` sorts before `02-schemas.td`. The number prefix is stripped from the display name.

## Assets

Binary files (images, PDFs) can live anywhere in the vault. The editor paste command (`TypedownPaste` in Neovim) saves clipboard images to `_assets/` next to the current file.

Reference assets in the markdown body:

```markdown
![logo](_assets/logo.png)
```

Or in the frontmatter as a file reference:

```yaml
avatar: fref("_assets/photo.png")
```
