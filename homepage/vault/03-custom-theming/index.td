---
_icon: icon.palette
description: "The Typedown default theme: layout, navigation, search, keyboard shortcuts, and appearance."
---

Typerighter ships a default theme optimized for documentation. It gives you a sidebar, full-text search, breadcrumbs, prev/next navigation, dark mode, and syntax highlighting with no configuration required.

This section covers how to use and navigate the default theme. For customizing or replacing it, see [Custom Theming](01-custom-theming).

## Keyboard shortcuts

| Action | Shortcut |
|---|---|
| Search | `⌘K` / `Ctrl+K` |
| Commands | `⌘/` / `Ctrl+/` |
| Go to previously accessed page | `⌘[` / `Ctrl+[` |
| Go to next accessed page | `⌘]` / `Ctrl+]` |
| Next page | `Ctrl+→` |
| Previous page | `Ctrl+←` |
| Toggle sidebar | `⌘\` / `Ctrl+\` |
| Close | `Esc` |

Open the command palette with `⌘/` to browse all available commands, navigate to pages, and access recently visited pages.

## Page history

The theme tracks the pages you visit across sessions. Press `⌘[` or `Ctrl+[` to return to the previously accessed page. This uses a custom history stack stored in `localStorage`, separate from the browser's native back button, so it persists across tabs and browser restarts.

Recently visited pages appear at the bottom of every page and in the command palette when the search query is empty.
