The Neovim plugin registers the LSP, tree-sitter grammars, and file type detection automatically. Requires Neovim 0.10+ and `curl` on your PATH (for automatic binary download, not needed on NixOS).

## Installation with lazy.nvim

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

## Installation with mini.deps

```lua
MiniDeps.add("huydo862003/typerighter")
local base = vim.fn.stdpath("data") .. "/site/pack/deps/opt/typerighter/editors/nvim"
vim.opt.runtimepath:prepend(base)
vim.filetype.add({ extension = { td = "typedown" } })
for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
  vim.cmd.source(f)
end
```

## NixOS

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

## Commands

| Command | Description |
| --- | --- |
| `TypedownFormat` | Format the current file via the LSP |
| `TypedownLint` | Show diagnostics in the location list |
| `TypedownPaste` | Paste clipboard image as an asset and insert `fref` |

## Tips

Standard LSP keybindings work as usual:

| Key | Action |
| --- | --- |
| `gd` | Go to definition |
| `gr` | Find references |
| `K` | Hover (show type info) |
| `<leader>ca` | Code actions |
| `<leader>rn` | Rename symbol |

List continuation works out of the box: pressing enter on a list item continues the list, pressing enter on an empty item exits the list.
