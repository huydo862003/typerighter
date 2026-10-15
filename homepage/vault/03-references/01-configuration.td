---
_label: "Configuration"
_icon: icon.settings
description: "All typedown.yaml fields and their defaults."
---

All site configuration lives in a single `typedown.yaml` file at the root of your project.

## version

**Required.** Schema version of the config file. Currently the only valid value is `"1"`.

```yaml
version: "1"
```

## vault

Where your content lives.

```yaml
vault:
  root_dir: "docs"
```

| Field | Default | Description |
|---|---|---|
| `root_dir` | `"."` | Directory containing `.td` content files, relative to project root |

## repo

Enables "Edit this page" links. Accepts a URL string or an object when `typedown.yaml` is nested inside the repo.

```yaml
# Simple: typedown.yaml is at the repo root
repo: "https://github.com/user/repo"

# Nested: typedown.yaml is inside a subdirectory
repo:
  url: "https://github.com/user/repo"
  root: "docs"
```

| Field | Description |
|---|---|
| `url` | Repository URL |
| `root` | Path from the repo root to the directory containing `typedown.yaml` |

The edit link points to `{url}/edit/main/{root}/{vault.root_dir}/{filepath}`.

## site

Controls the site title, SEO metadata, and navigation.

```yaml
site:
  title: "My Site"
  description: "A knowledge base built with Typerighter"
  origin: "https://example.com"
  lang: "en"
  base_path: "/"
```

| Field | Default | Description |
|---|---|---|
| `title` | `""` | Site title shown in the header and browser tab |
| `description` | `""` | Default meta description for SEO |
| `origin` | | Production URL (e.g. `https://example.com`). Required for sitemap and canonical URLs |
| `lang` | `"en"` | HTML `lang` attribute |
| `base_path` | `"/"` | URL prefix for subdirectory hosting (e.g. `"/docs"`) |
| `author` | | Site author name |
| `license` | | License name |
| `public_dir` | `"public"` | Static assets directory, copied to output as-is |

### site.nav

Header navigation links. External links (starting with `http`) show an arrow icon.

```yaml
site:
  nav:
    - title: "Guide"
      link: "/guide/index"
      icon: icon.book
    - title: "GitHub"
      link: "https://github.com/user/repo"
```

| Field | Required | Description |
|---|---|---|
| `title` | yes | Display text |
| `link` | yes | URL path or external URL |
| `icon` | no | Icon from the `icon` module (e.g. `icon.book`) |

## theme

Customize the look and behavior of your site with custom layouts and Vue components.

### theme.components

Register Vue components for use in `.td` content files and layout slots.

```yaml
theme:
  components:
    - "./components"              # scan directory for .vue files
    - name: MyWidget              # explicit entry
      source: "./ui/MyWidget.vue"
```

Directory entries discover components two ways:
- `ComponentName.vue` files directly in the directory
- `ComponentName/index.vue` subdirectories

Explicit entries register a single component by name.

### theme.layout

Replace the default layout entirely with a custom `.vue` file:

```yaml
theme:
  layout: "./theme/Layout.vue"
```

Or keep the default layout and inject components into its slots:

```yaml
theme:
  layout:
    slots:
      layout_top: "AnnouncementBar"
      doc_after: "FeedbackWidget"
```

See [Theming](07-theming) for available slot names.

## Full example

```yaml
version: "1"

vault:
  root_dir: "vault"

repo: "https://github.com/user/repo"

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
  components:
    - "./components"
  layout:
    slots:
      doc_after: "FeedbackWidget"
```
