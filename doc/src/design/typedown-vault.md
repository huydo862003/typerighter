# Typedown Vault

A Typedown vault is a directory containing a `typedown.yaml` file at its root. This file is the entrypoint: its presence marks the directory as a Typedown vault and configures where the vault's content is located.

A vault is purely an organization convention: it defines how [Typedown](./typedown.md) files are arranged on disk & how cross-file definitions are resolved. It has no meaning in the [Typedown abstract model](./typedown-model.md), which only concerns itself with the resource graph and its contents.

## Vault Layout

A typical vault looks like:

```
my-vault/
├── typedown.yaml
├── _types/
│   ├── person.td
│   └── artwork.td
├── bob.td
└── mona-lisa.td
```

Content files live directly under the vault root. Type schema files live under `_types/`, which may be nested arbitrarily. Assets (images, PDFs, etc.) conventionally live under `_assets/`, though they may appear anywhere in the vault.

See [Typedown](./typedown.md) for how individual files are structured.

### Naming Conventions

Typedown uses **snake_case** throughout:

- **File names**: all `.td` files use snake_case (e.g. `my_note.td`, `blog_post.td`).
- **YAML keys**: all property names in the frontmatter use snake_case (e.g. `birth_date`, `first_name`, `topic_interest`).

### typedown.yaml

`typedown.yaml` (or `typedown.yml`) holds global vault configuration. It has the following fields:

- `version`: the Typedown format version.
- `vault`: configuration for the vault.
  - `root_dir`: path to the vault root directory, relative to `typedown.yaml`.

```yaml
version: 1.0.0
vault:
  root_dir: vault
```
