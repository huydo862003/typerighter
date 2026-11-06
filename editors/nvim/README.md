# Typedown for Neovim

Neovim plugin for the Typedown language (`.td` files). Provides LSP support, tree-sitter highlighting, and automatic server binary download.

## Requirements

- Neovim 0.10+
- `curl` on your PATH (for automatic binary download, not needed if using Nix)

## Installation

Since the plugin lives in `editors/nvim/` inside the monorepo, you need to add that subdirectory to the runtime path and explicitly source the plugin files.

### lazy.nvim

```lua
{
  "huydo862003/typerighter",
  tag = "v0.1.0",
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

### Vanilla (no plugin manager)

Clone the repo and add the plugin subdirectory to your runtime path:

```bash
git clone --depth 1 --branch v0.1.0 https://github.com/huydo862003/typerighter \
  ~/.local/share/nvim/site/pack/typedown/start/typerighter
```

Then add to your `init.lua`:

```lua
local base = vim.fn.stdpath("data") .. "/site/pack/typedown/start/typerighter/editors/nvim"
vim.opt.runtimepath:prepend(base)
vim.filetype.add({ extension = { td = "typedown" } })
for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
  vim.cmd.source(f)
end
```

## LSP binary resolution

The plugin resolves the `typedown-lsp` binary in this order:

1. **System PATH** if `typedown-lsp` is already installed (e.g. via Nix)
2. **Auto-download** from the GitHub release matching the plugin version, cached in `~/.local/share/nvim/typedown/`

### NixOS

On NixOS, the auto-downloaded binary will not work because it is dynamically linked. Install `typedown-lsp` via Nix instead.

Add the flake input to your system configuration:

```nix
# flake.nix
{
  inputs.typerighter = {
    url = "github:huydo862003/typerighter";
    inputs.nixpkgs.follows = "nixpkgs";
  };
}
```

Then add the package to your system or home-manager packages:

```nix
environment.systemPackages = [
  typerighter.packages.${system}.default
];
```

This puts both `typedown-lsp` and `typedown-rpc` on your PATH. The nvim plugin will pick them up automatically.

## Development

For local development against a debug build:

```bash
cd ~/projects/typerighter
nvim -u editors/nvim/local_init.lua path/to/file.td
```

This sources your normal config, then overlays the local plugin with `vim.g.typedown_dev = true` so it uses `target/debug/typedown-lsp` instead of downloading a release binary.
