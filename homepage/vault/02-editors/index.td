---
_icon: icon.code
---

Editor integrations for Typedown. All editors use the same LSP server, so features are consistent across editors.

## LSP features

- Syntax highlighting (via tree-sitter grammars and semantic tokens)
- Autocompletion for schema fields, type names, file references, and enum values
- Go-to-definition for identifiers, schema references, and `fref` paths
- Find references across the vault
- Semantic rename (renames propagate to all references, including `fref` paths)
- Diagnostics (type errors, unresolved references, lint warnings)
- Formatting (markdown body formatting)
- Code actions (schema initialization, create linked resource)
- Inlay hints (show `_label` for `fref` targets)
- Hover (show type info for fields, show label and schema for `fref` targets)

## Sections

- [Neovim](01-neovim) for installation, commands, and tips
- [VS Code](02-vscode) for installation and features
- [Zed](03-zed) for installation
- [Contributing](04-contributing) for developing the editor plugins
