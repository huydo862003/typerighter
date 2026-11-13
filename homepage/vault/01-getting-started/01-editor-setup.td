---
_label: "Editor setup"
_icon: icon.code
description: "Set up Neovim, VS Code, or Zed for autocompletion, type checking, and diagnostics."
---

All editors use the same LSP server, so you get the same features everywhere:

- Autocompletion for field names, schema types, and file references
- Type checking and inline diagnostics
- Go-to-definition, find references, and semantic rename
- Hover info, formatting, and code actions

## Quick setup

The fastest way to set up your editor is the `setup-editor` command. It detects installed editors and configures them automatically:

```sh
npx typerighter setup-editor
```

If you prefer to set things up manually, or need editor-specific configuration, see the sections below.

## Neovim

The Neovim plugin registers the LSP, tree-sitter grammars, and file type detection automatically. Requires Neovim 0.10+ and `curl` on your PATH for automatic binary download.

### lazy.nvim

```lua
{
  "huydo862003/typerighter",
  -- pin to a release tag, e.g. "v0.57.6"
  tag = "v0.57.6",
  lazy = false,
  submodules = false,
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

### mini.deps

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

```nix{title=flake.nix}
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

### Commands and keybindings

| Command | Description |
| --- | --- |
| `TypedownFormat` | Format the current file via the LSP |
| `TypedownLint` | Show diagnostics in the location list |
| `TypedownPasteAsset` | Paste clipboard image as an asset and insert `fref` |

Standard LSP keybindings work: `gd` (go to definition), `gr` (find references), `K` (hover), `<leader>ca` (code actions), `<leader>rn` (rename).

List continuation works out of the box: pressing enter on a list item continues the list, pressing enter on an empty item exits the list.

## VS Code

The VS Code extension provides syntax highlighting, the full LSP client, a paste handler for clipboard images, and list continuation on enter.

### Installation

The extension is published on the VS Code Marketplace and Open VSX (for VS Code forks like VSCodium). Install it from the extensions panel by searching for "typedown", or run:

```sh
typerighter setup-editor vscode
```

You can also install from the command line directly:

```sh
code --install-extension huydo862003.typedown
```

### Keybindings

Standard VS Code keybindings apply: `Ctrl+Space` (completions), `F2` (rename), `Ctrl+.` (code actions), `Ctrl+Shift+F` (format), `Ctrl+Shift+V` (paste image as asset).

## Zed

The Zed extension provides tree-sitter grammars and LSP integration. Install from the Zed extension marketplace.

::: info
Zed does not support custom command handlers in extensions. Features that require editor-side prompts (like "Create linked resource") and list continuation on enter are not available in Zed.
:::
