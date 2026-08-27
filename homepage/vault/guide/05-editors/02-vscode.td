The VS Code extension provides:

- TextMate grammar for syntax highlighting
- LSP client for all language features
- Paste handler that saves clipboard images to `_assets/` and inserts `fref`
- List continuation on enter (continue bullet, dedent on empty item)

## Installation

The extension is not yet published on the VS Code Marketplace.
Download the `.vsix` file for your platform from the [GitHub Releases](https://github.com/huydo862003/typerighter/releases) page, then install it manually:

1. Open VS Code
2. `Ctrl+Shift+P` -> "Extensions: Install from VSIX..."
3. Select the downloaded `.vsix` file

## Tips

The extension supports all standard VS Code LSP features:

- `Ctrl+Space` for completions
- `F2` to rename a symbol
- `Ctrl+.` for code actions
- `Ctrl+Shift+F` to format the file
- `Ctrl+Shift+V` to paste clipboard images as assets
