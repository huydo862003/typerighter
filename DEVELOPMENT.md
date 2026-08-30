# Development

> This project is developed exclusively on NixOS. The setup and tooling on other systems may be unreliable.

## Dependencies

Absolutely required dependencies to author the core crates (Rust) & packages (Node):

- Rust
  - **Rust stable** (1.95+): Compiler, LSP server, Zed extension
  - **wasm32-wasip1 target**: Compile the Zed extension to WASM
  - **wasm32-unknown-unknown target**: Compile the RPC client to browser WASM
  - **wasm-pack**: Build and package the RPC client WASM module for JS consumption
- Node
  - **Node.js** (22+): Tree-sitter grammar build tooling
  - **pnpm** (11+): Node package manager

### Recommended DX

- Rust
  - **rust-src**: Standard library source for rust-analyzer go-to-definition
  - **rust-analyzer**: IDE support
  - **clippy**: Linting
  - **rustfmt**: Formatting
  - **cargo-edit**: Version bumping in publish script
  - **cargo-watch**: Watch mode for iterative development
- Other
  - **clangd**: IDE support for C scanner code
- Decent Typescript environments

### Tree-sitter Grammar Authoring

- **tree-sitter CLI** (0.26+): Generate and test tree-sitter grammars
- **clang** (21+): C compiler for tree-sitter external scanners
- **wasi-sdk** (25+): Compile tree-sitter grammars to WASM. Must target wasm32-wasip1 (Zed requires this for grammar loading)

### Docs Authoring

- Rust
  - **mdbook**: Build design documentation
  - **mdbook-mermaid**: Mermaid diagram support in mdbook

### Recommended Editors (for testing extensions locally)

- **Neovim**
- **VS Code** or **VSCodium**
- **Zed**

## Nix (recommended)

Nix automates the entire setup. Currently only tested on x86_64 Linux. The wasi-sdk derivation hardcodes the x86_64-linux binary, so aarch64-linux and macOS will fail until platform-specific URLs are added to `flake.nix`.

```bash
nix develop
```

This drops you into a shell with all dependencies above. If you use [direnv](https://direnv.net/), add `use flake` to `.envrc` for automatic activation.

## Non-Nix

Install each dependency manually:

1. **Rust** via [rustup](https://rustup.rs/):

   ```bash
   rustup install stable
   rustup default stable
   rustup component add rust-src rust-analyzer clippy rustfmt
   rustup target add wasm32-wasip1 wasm32-unknown-unknown
   ```

2. **Node.js** (22+) and **pnpm**:

   ```bash
   corepack enable
   pnpm install
   ```

3. **tree-sitter CLI**: https://tree-sitter.github.io/tree-sitter/creating-parsers/tool-setup

4. **wasi-sdk**: https://github.com/WebAssembly/wasi-sdk/releases

   Set `TREE_SITTER_WASI_SDK_PATH` to the extracted directory.

5. **clang** and **clangd**: Install via your system package manager.

6. **cargo-edit**, **cargo-watch**, and **wasm-pack**:

   ```bash
   cargo install cargo-edit cargo-watch
   cargo install wasm-pack
   ```

7. **mdbook** and **mdbook-mermaid** (only needed for docs):

   ```bash
   cargo install mdbook mdbook-mermaid
   ```

## Building

`pnpm` is the task runner for the entire project. All build commands are defined in the root `package.json` and delegate to cargo, tree-sitter, and sub-package scripts as needed.

- `pnpm run build`: Build everything (Rust crates + Node packages)
- `pnpm run build:local`: Build everything using local sources
- `pnpm run build:staging`: Build everything using staging release binaries
- `pnpm run dev`: Watch mode for iterative development

## Per-package development

If you only intend to work on a specific package or editor extension, see the individual DEVELOPMENT.md files:

- Core Rust crates (`typedown-lang`, `typedown-server`, `typedown-incremental`, etc.): This file
- Tree-sitter grammars: [packages/tree-sitter/DEVELOPMENT.md](packages/tree-sitter/DEVELOPMENT.md)
- Neovim plugin: [editors/nvim/DEVELOPMENT.md](editors/nvim/DEVELOPMENT.md)
- VS Code extension: [editors/vscode/DEVELOPMENT.md](editors/vscode/DEVELOPMENT.md)
- Zed extension: [editors/zed/DEVELOPMENT.md](editors/zed/DEVELOPMENT.md)

## Releasing

Run the publish script from the repo root (must be inside `nix develop`):

```bash
./publish.sh
```

The script will:

1. Prompt for a bump type (patch, minor, major, prepatch, preminor, premajor, prerelease)
2. Compute the new version using semver
3. For production releases: open `$EDITOR` to write a CHANGELOG.md entry
4. Bump versions across all files (VERSION, Cargo.toml, package.json, version.lua)
5. Commit, tag, and push

Tags:
- Production releases: `v{VERSION}` (e.g. `v0.2.0`)
- Staging releases: `staging/v{VERSION}` (e.g. `staging/v0.2.0-rc.1`)

CI picks up the tag and builds release artifacts automatically.
