# Development

For full project setup, see the [root DEVELOPMENT.md](../../DEVELOPMENT.md).

## Dependencies

- **Rust**: To build the extension WASM
- **wasm32-wasip1 target**: WASM compilation target
- **Zed**

All provided automatically by `nix develop` from the repo root.

## Setup

No additional setup needed beyond the root project.

## Development

- Extension source: `src/typedown.rs`
- Tree-sitter queries: `languages/*/`
- Grammar references: `extension.toml`
- Build WASM: `cd editors/zed && cargo build`

## Testing

### Non-Nix

Requires Rust installed via [rustup](https://rustup.rs/) (not homebrew or system packages).

Install the extension locally in Zed via `zed: install dev extension`, pointing to the `editors/zed` directory. Zed handles the WASM compilation automatically.

### NixOS

Zed's `install dev extension` requires `rustup`, which is not available on NixOS ([zed-industries/zed#42353](https://github.com/zed-industries/zed/issues/42353)). Use the install script instead:

```bash
pnpm run build:local
```

This builds the extension WASM and grammar WASMs (using wasi-sdk clang, same pattern as [nix-zed-extensions](https://github.com/DuskSystems/nix-zed-extensions)) into `editors/zed/`. Then use `zed: install dev extension` pointing to `editors/zed/`.

For a release build, pass `--release`.

## Release

Releases are handled by `publish.sh` from the repo root. CI builds the WASM and packages the extension automatically.

## Tree-sitter

Syntactic highlighting in Zed uses Tree-sitter. The grammar lives in `packages/tree-sitter/`. Zed extensions reference the grammar via `extension.toml` and include query files under `languages/typedown/`.

See also: [Tree-sitter research](https://huydo862003.github.io/loupe/research/analysis/syntactic-analysis.html)
