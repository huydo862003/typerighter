---
_label: "Configuration"
_icon: icon.settings
description: "All typedown.yaml fields and their defaults."
---

All site configuration lives in `typedown.yaml` at your project root.

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
  layout: "./theme/Layout.vue"
  components:
    - "./components"
  layout:
    slots:
      doc_after: "FeedbackWidget"
```

## version

Required. Currently `"1"`.

## vault

| Field | Default | Description |
|---|---|---|
| `root_dir` | `"."` | Directory containing `.td` content files, relative to project root |

## repo

Repository config for "Edit this page" links. Accepts a string shorthand or an object.

```yaml
# String shorthand (typedown.yaml at repo root)
repo: "https://github.com/user/repo"

# Object form (typedown.yaml nested in a subdirectory)
repo:
  url: "https://github.com/user/repo"
  root: "docs"
```

| Field | Description |
|---|---|
| `url` | Repository URL |
| `root` | Path from the git root to the directory containing `typedown.yaml` |

The edit link is constructed as `{url}/edit/main/{root}/{vault.root_dir}/{filepath}`.

## site

| Field | Default | Description |
|---|---|---|
| `title` | `""` | Site title, available via `useSiteConfig().title` |
| `description` | `""` | Site description for SEO metadata |
| `origin` | - | Full URL like `https://example.com`. Enables sitemap, RSS, and canonical URLs |
| `lang` | `"en"` | HTML `lang` attribute |
| `base_path` | `"/"` | URL base path for subdirectory hosting (e.g. `"/docs"`) |
| `author` | - | Site author name |
| `license` | - | License name |
| `public_dir` | `"public"` | Directory for static assets copied to output as-is |

### site.nav

Top-level navigation links shown in the header.

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

External links (starting with `http`) show an arrow icon. Max 4 nav items recommended.

## theme

| Field | Description |
|---|---|
| `layout` | Path to a custom layout `.vue` file, or an object with `slots` |
| `components` | List of component directories or explicit entries |

### theme.layout

Replace the default layout:

```yaml
theme:
  layout: "./theme/Layout.vue"
```

Or inject components into slots of the default layout:

```yaml
theme:
  layout:
    slots:
      layout_top: "AnnouncementBar"
      doc_after: "FeedbackWidget"
```

See [Theming](06-theming) for available slots.

### theme.components

Register Vue components for use in `.td` content files and layout slots.

```yaml
theme:
  components:
    - "./components"              # scan directory for .vue files
    - name: MyWidget              # explicit entry
      source: "./ui/MyWidget.vue"
```

Directory entries scan for `.vue` files and register each by filename. Explicit entries register a single component by name.
