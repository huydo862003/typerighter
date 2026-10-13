---
_label: "Configuration"
---

# Configuration

All site configuration lives in `typedown.yaml` at your project root.

## Full example

```yaml
version: "1"

vault:
  root_dir: "vault"

site:
  title: "My Site"
  description: "Built with Typerighter"
  origin: "https://example.com"
  lang: "en"
  base_path: "/"
  author: "Your Name"
  license: "MIT"

  nav:
    - title: "Guide"
      link: "/guide/index"
    - title: "API"
      link: "/api/composables"

theme:
  layout: "./theme/Layout.vue"

  components:
    - "./components"

  layout:
    slots:
      doc-before: MyBanner
```

## Fields

### vault

| Field | Default | Description |
|---|---|---|
| `root_dir` | `"vault"` | Directory containing `.td` content files |

### site

| Field | Default | Description |
|---|---|---|
| `title` | `""` | Site title shown in `useSiteConfig().title` |
| `description` | `""` | Site description for SEO |
| `origin` | - | Full URL like `https://example.com`, enables sitemap and RSS |
| `lang` | `"en"` | HTML `lang` attribute |
| `base_path` | `"/"` | URL base path for subdirectory hosting |
| `author` | - | Site author name |
| `license` | - | License name |
| `nav` | `[]` | Top-level navigation links |
| `repo` | - | Repository config for "Edit this page" links |

### theme

| Field | Description |
|---|---|
| `layout` | Path to your custom layout `.vue` file |
| `components` | Directory or explicit component entries |
| `layout.slots` | Slot components to inject into the default theme layout |

## Repository config

```yaml
site:
  repo:
    url: "https://github.com/user/repo"
    root: "docs"   # path from git root to typedown.yaml
```

Available via `useSiteConfig().repo`.

## Nav items

```yaml
site:
  nav:
    - title: "Guide"
      link: "/guide/index"
      icon: "book"
    - title: "GitHub"
      link: "https://github.com/user/repo"
```

Available via `useSiteConfig().nav`.
