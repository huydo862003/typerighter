---
_label: "Configuration"
_icon: icon.settings
description: "All typedown.yaml fields and their defaults."
---

All site configuration lives in a single `typedown.yaml` file at the root of your project. It controls the vault location, site metadata, navigation, theming, and custom component registration. Each section below shows one part of the config; see the [full example](#full-example) at the bottom for everything together.

## version

Required. Schema version of the config file. Currently the only valid value is `"1.0.0"`.

```yaml{title=typedown.yaml}
version: "1.0.0"
```

## vault

Where your content lives.

```yaml{title=typedown.yaml}
vault:
  root_dir: "docs"
```

| Field | Default | Description |
|---|---|---|
| `root_dir` | `"."` | Directory containing `.td` content files, relative to project root |

## repo

Enables "Edit this page" links. Accepts a URL string or an object when `typedown.yaml` is nested inside the repo.

```yaml{title=typedown.yaml}
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

Controls the site title, SEO metadata, and navigation. These fields affect the generated HTML, sitemap, and RSS feed. See [Deploying](02-deploying) for how `origin` and `base_path` interact with hosting platforms.

```yaml{title=typedown.yaml}
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

```yaml{title=typedown.yaml}
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

Register Vue components for use in `.td` content files and layout slots. See [Custom components](../02-typedown-syntax/05-custom-components) for how to use them in the markdown body.

```yaml{title=typedown.yaml}
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

Controls the page layout. The simplest form replaces the default layout entirely with a custom `.vue` file:

```yaml{title=typedown.yaml}
theme:
  layout: "./theme/Layout.vue"
```

To keep the default layout but inject components into specific zones, use the object form with `slots`:

```yaml{title=typedown.yaml}
theme:
  layout:
    slots:
      layout_top: "AnnouncementBar"
      doc_after: "FeedbackWidget"
```

The object form also supports a custom layout component alongside slots, and a custom not-found page:

```yaml{title=typedown.yaml}
theme:
  layout:
    component: "./theme/Layout.vue"
    not_found: "./theme/NotFound.vue"
    slots:
      layout_top: "AnnouncementBar"
```

| Field | Description |
|---|---|
| `component` | Path to a custom layout `.vue` file (alternative to the string shorthand) |
| `slots` | Map of slot names to registered component names |
| `not_found` | Path to a custom 404 page `.vue` file |

See [Theming API](../03-custom-theming/02-theming-api) for available slot names and [Custom Theming](../03-custom-theming/01-custom-theming) for how to build layout components.

## Full example

```yaml{title=typedown.yaml}
version: "1.0.0"

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
