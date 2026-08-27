Editor integrations for Typedown. All editors use the same LSP server, so features are consistent across editors.

## Features

The LSP provides:

- Syntax highlighting (via tree-sitter grammars and semantic tokens)
- Autocompletion for schema fields, type names, and file references
- Go-to-definition for identifiers, schema references, and `fref` paths
- Find references across the vault
- Semantic rename (renames propagate to all references, including `fref` paths)
- Diagnostics (type errors, unresolved references, lint warnings)
- Formatting (markdown body formatting via `textDocument/formatting`)
- Code actions (schema initialization for empty frontmatter)

## Neovim

The Neovim plugin registers the LSP, tree-sitter grammars, and file type detection automatically. Requires Neovim 0.10+ and `curl` on your PATH (for automatic binary download, not needed on NixOS).

### Installation with lazy.nvim

```lua
{
  "huydo862003/typerighter",
  tag = "v0.15.1",
  lazy = false,
  init = function()
    local base = vim.fn.stdpath("data") .. "/lazy/typerighter/editors/nvim"
    vim.opt.runtimepath:prepend(base)
    vim.filetype.add({ extension = { td = "typedown" } })
    for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
      vim.cmd.source(f)
    end
  end,
}
```

### Installation with mini.deps

```lua
MiniDeps.add("huydo862003/typerighter")
local base = vim.fn.stdpath("data") .. "/site/pack/deps/opt/typerighter/editors/nvim"
vim.opt.runtimepath:prepend(base)
vim.filetype.add({ extension = { td = "typedown" } })
for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
  vim.cmd.source(f)
end
```

### NixOS

On NixOS, the auto-downloaded binary will not work because it is dynamically linked. Install via the flake instead:

```nix
# flake.nix
{
  inputs.typerighter = {
    url = "github:huydo862003/typerighter";
    inputs.nixpkgs.follows = "nixpkgs";
  };
}
```

```nix
environment.systemPackages = [
  typerighter.packages.${system}.default
];
```

This puts both `typedown-lsp` and `typedown-rpc` on your PATH. The plugin picks them up automatically.

### Commands

| Command | Description |
| --- | --- |
| `TypedownFormat` | Format the current file via the LSP |
| `TypedownLint` | Show diagnostics in the location list |
| `TypedownPaste` | Paste clipboard image as an asset and insert `fref` |

Standard LSP keybindings (`gd`, `gr`, `K`, etc.) work as usual.

## VS Code

The VS Code extension provides:

- TextMate grammar for syntax highlighting
- LSP client for all language features
- Paste handler that saves clipboard images to `_assets/` and inserts `fref`

Install from the VS Code marketplace or build from source.

## Zed

The Zed extension provides tree-sitter grammars and LSP integration.

See `editors/zed/README.md` for installation instructions.

## Developing the editor plugins

This section is for contributors working on the editor integrations themselves.

### Neovim

Load the plugin from the local repo for development:

```sh
nvim -u editors/nvim/local_init.lua path/to/file.td
```

This uses the debug build of the LSP binary at `target/debug/typedown-lsp`.

### VS Code

Open the `editors/vscode/` directory in VS Code and press F5 to launch the extension development host. See `editors/vscode/DEVELOPMENT.md` for details.

### Zed

See `editors/zed/DEVELOPMENT.md` for build and test instructions.
