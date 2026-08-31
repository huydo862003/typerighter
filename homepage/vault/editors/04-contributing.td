For contributors working on the editor integrations themselves.

## Neovim

Load the plugin from the local repo for development:

```sh
nvim -u editors/nvim/local_init.lua path/to/file.td
```

This uses the debug build of the LSP binary at `target/debug/typedown-lsp`.

## VS Code

Open the `editors/vscode/` directory in VS Code and press F5 to launch the extension development host. See `editors/vscode/DEVELOPMENT.md` for details.

## Zed

See `editors/zed/DEVELOPMENT.md` for build and test instructions.
