# Development

For full project setup, see the [root DEVELOPMENT.md](../../DEVELOPMENT.md).

## Dependencies

- **Neovim** (0.10+)
- **Rust**: To build `typedown-lsp`
- **Node.js** and **pnpm**: To build tree-sitter grammars

All provided automatically by `nix develop` from the repo root.

## Setup

```bash
pnpm install
```

## Development

- Build tree-sitter grammars: `pnpm --filter tree-sitter-typedown run build`
- Build LSP: `cargo build -p typedown-lsp`
- Launch with local build: `nvim -u editors/nvim/local_init.lua path/to/file.td`
- Confirm LSP attached: `:LspInfo` inside Neovim

`local_init.lua` sets `vim.g.typedown_dev = true`, which tells the plugin to use `target/debug/typedown-lsp` instead of downloading a binary.

Syntactic highlighting uses Tree-sitter. The grammar lives in `packages/tree-sitter/` and the query files live in `queries/typedown/` inside this plugin. Once the grammar is built, register the parser with nvim-treesitter and the query files will provide highlighting, injections, and folds.

## Testing

### Local build

```bash
nvim -u editors/nvim/local_init.lua path/to/file.td
```

### Staging release

1. Push a staging tag via `./publish.sh` from the repo root (choose a `pre*` bump type). CI builds and uploads the prerelease binaries automatically.

2. Launch with `staging_init.lua`:

   ```bash
   nvim -u editors/nvim/staging_init.lua path/to/file.td
   ```

   The plugin reads `version.lua` (derived from `VERSION`), constructs the `staging/vX.Y.Z-label.N` release URL, and downloads the matching binary automatically on first launch.

## Release

Releases are handled by `publish.sh` from the repo root. It bumps the version in `VERSION` alongside all other derived version files and pushes the tag that triggers CI.
