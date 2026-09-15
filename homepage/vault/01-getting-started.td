---
_label: "Getting Started"
_icon: icon.rocket
description: "Install Typedown, create a project, set up your editor, and write your first document."
---

This page walks through installation, project setup, editor configuration, and writing your first document. By the end you will have a running dev server with typed content.

## Install

Typedown ships as a single npm package. Install it globally so the CLI is available everywhere.

```sh
npm install -g typerighter
```

## Create a project

Scaffold a new project with `typerighter init`. This creates the vault directory, a sample schema, and the config file.

```sh
mkdir my-project && cd my-project
typerighter init
```

The scaffolded structure looks like this:

```
my-project/
  typedown.yaml          # project config
  vault/
    _types/
      Article.td         # sample schema
    hello.td             # sample content
  index.html
  package.json
```

## Start the dev server

Run the dev server and open the browser. Changes to `.td` files reload automatically.

```sh
typerighter dev
```

Open `http://localhost:8686`. You should see your vault rendered as a site with a sidebar, search, and syntax highlighting.

## Set up your editor

All editors use the same LSP server, so features are consistent: autocompletion, type checking, go-to-definition, semantic rename, hover info, and diagnostics.

### Neovim

The Neovim plugin registers the LSP, tree-sitter grammars, and file type detection automatically. Requires Neovim 0.10+ and `curl` on your PATH for automatic binary download.

**lazy.nvim**

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

**mini.deps**

```lua
MiniDeps.add("huydo862003/typerighter")
local base = vim.fn.stdpath("data") .. "/site/pack/deps/opt/typerighter/editors/nvim"
vim.opt.runtimepath:prepend(base)
vim.filetype.add({ extension = { td = "typedown" } })
for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
  vim.cmd.source(f)
end
```

**NixOS**: On NixOS, the auto-downloaded binary will not work because it is dynamically linked. Install via the flake instead:

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

**Commands**: `TypedownFormat` (format file), `TypedownLint` (show diagnostics), `TypedownPasteAsset` (paste clipboard image as asset).

**Keybindings**: Standard LSP keybindings work: `gd` (go to definition), `gr` (find references), `K` (hover), `<leader>ca` (code actions), `<leader>rn` (rename).

### VS Code

The VS Code extension provides syntax highlighting, the full LSP client, a paste handler for images, and list continuation on enter.

The extension is not yet published on the VS Code Marketplace. Download the `.vsix` file for your platform from the [GitHub Releases](https://github.com/huydo862003/typerighter/releases) page, then install it manually:

1. Open VS Code
2. `Ctrl+Shift+P` -> "Extensions: Install from VSIX..."
3. Select the downloaded `.vsix` file

Standard VS Code keybindings apply: `Ctrl+Space` (completions), `F2` (rename), `Ctrl+.` (code actions), `Ctrl+Shift+F` (format), `Ctrl+Shift+V` (paste image as asset).

### Zed

The Zed extension provides tree-sitter grammars and LSP integration. Install from the Zed extension marketplace.

::: info
Zed does not support custom command handlers in extensions. Features that require editor-side prompts (like "Create linked resource") and list continuation on enter are not available in Zed.
:::

## Write a file

Open `vault/hello.td`. A `.td` file has two parts: YAML frontmatter between `---` markers and a markdown body after.

```yaml
---
_type: Article
title: "My first document"
tags:
  - "getting-started"
---

This is the body. It supports **bold**, _italic_, `code`, tables, math ($E = mc^2$), and more.
```

The `_type` field tells Typedown which schema this file conforms to. The editor uses it to offer autocompletion and type checking for every field. Save the file and the dev server reloads automatically.

## Build for production

When you are ready to deploy, build a static site. The output goes to `dist/` and can be served by any static file host.

```sh
typerighter build
```

## Next steps

Now that your project is running and your editor is set up, learn how vaults, schemas, and references work together in the [Guide](02-guide/index).
