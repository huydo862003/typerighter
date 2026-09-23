# Development

See [README.md](README.md) for an overview of what Typedown is and how to install it.

> This project is developed exclusively on NixOS. The setup and tooling on other systems may be unreliable.

## Dependencies

Absolutely required dependencies to author the core crates (Rust) & packages (Node):

- Rust
  - **Rust stable** (1.95+): Compiler, LSP server, Zed extension
  - **wasm32-wasip1 target**: Compile the Zed extension to WASM
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
   rustup target add wasm32-wasip1
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

6. **cargo-edit** and **cargo-watch**:

   ```bash
   cargo install cargo-edit cargo-watch
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

## Dependency graph

- `typedown-macros` and `typedown-types` contain common utils, which are the lowest common denominator that everyone depends upon.
  - They can be depended upon by other crates.
  - They must not depend on any other crates.
- `typedown-incremental` contains the incremental engine.
  - It must not depend on any other crates, except for `typedown-macros` and `typedown-types`.
  - It can be depended upon by everyone, EXCEPT FOR `typedown-macros` and `typedown-types`.
- `typedown-lang` contains the AST structure, parser, typechecking, and evaluation logic for typedown.
  - It depends on `typedown-incremental`, `typedown-macros`, and `typedown-types`.
  - It must not depend on `typedown-server`/`typedown-build`.
  - It can only be depended upon by `typedown-server`/`typedown-build`.
- `typedown-server` contains the LSP server for typedown while `typedown-build` contains the build server.
  - It can depend on any other crates.
  - It can not be depended upon by others.

## Design and research documentation

Internal design and research docs live in [doc/](doc/) (built with mdbook):

- [Design](doc/src/design.md): compiler design, syntax model, vault model
- [Research](doc/src/research.md): graph databases, Semantic Web, JSON-RPC, Vite plugins, YAML 1.2

Earlier research was documented in the [dboxide](https://github.com/huydo862003/dboxide) repo ([design docs](https://github.com/huydo862003/dboxide/tree/main/doc/src/design)). Tree-sitter grammar research is in the [loupe](https://github.com/huydo862003/loupe) repo.

For linting and formatting, we follow [Google's markdown style guide](https://google.github.io/styleguide/docguide/style.html) with some divergences. See [crates/typedown-lang/README.md](crates/typedown-lang/README.md) for details.

## Common pitfalls

These are some lessons learnt during the development of the project. Some comments in the code are also marked with `TIL`.

### Visitor pattern for serialization/hashing

There are two naive approaches to serialization, and a third that combines the best of both.

**Approach 1: Serializer knows every type**: a single serializer has a method per type. Adding a new type means modifying the serializer.

**Approach 2: Each type serializes itself**: every type must know the byte format, and changing the format means updating every type.

**Approach 3: Visitor (double dispatch)**: the type decides WHAT to write (which fields, in what order). The serializer decides HOW to write it (byte format, endianness, buffering). Neither depends on the other's internals.

```rust
trait Serializer {
    fn emit_str(&mut self, v: &str);
    fn emit_u32(&mut self, v: u32);
}

trait Serialize {
    fn serialize(&self, s: &mut impl Serializer);
}

impl Serialize for Person {
    fn serialize(&self, s: &mut impl Serializer) {
        s.emit_str(&self.name);
        s.emit_u32(self.age);
    }
}
```

This is how `std::hash` works (`Hash`/`Hasher`), how rustc does it (`Encodable`/`Encoder`), and how serde does it (`Serialize`/`Serializer`).

### HMR in Vite

Two problems:
- **Race condition**: Vite's watcher fires before Rust finishes re-indexing, so we suppress `handleHotUpdate` for `.td` files and let the Rust RPC events drive invalidation instead.
- **Client-side hot reload**: `.td` files become Vue SFCs, so Vue's HMR runtime handles `import.meta.hot.accept()` automatically.

Making data hot-reloadable: every piece of dynamic data lives in its own virtual module. The app entry imports each one and accepts HMR updates. When data changes on the server side, we invalidate the virtual module and push an HMR update.

Things to avoid:
1. Don't inline data as JSON in the app entry (forces full reload on change)
2. Don't use `const` for data that needs HMR (closures capture the binding, not the value)
3. Don't put `import.meta.glob` in the app entry (invalidating the app module to re-scan forces a full reload)
4. Don't `provide()` a plain object (use `shallowRef` for reactivity)
5. Don't use full reloads for data changes (reserve for structural changes like `basePath`)

### Parameterized types vs universal types

- **Parameterized type** (type constructor): `List :: Type -> Type`. Not a type by itself, needs args applied. This is what our `arity` system implements.
- **Universal type**: `forall T. T -> T :: Type`. Already a concrete type. Values of this type are polymorphic functions.
- **Higher-kinded type (HKT)**: a type variable ranging over type constructors. We don't have this.
- **Existential type**: `exists T. SchemaProperty[T]` means "there is some `T`, but I don't tell you which".

### Static types vs runtime objects

We were conflating three things in a single `TdTypeLike` trait: static type shape (typechecker), runtime object shape (evaluator), and user mental model of runtime shape. The fix: split into two systems (`TdStaticType` and `TdRuntimeObject`).

### Stable specialization via autoderef

We needed ad-hoc overloading on stable Rust. Rust method resolution prefers the least-deref match, so we implement the same method name on different traits at different reference depths. Only works in macro-generated code where the concrete type is known.

Based on: https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html

### Incremental cache: identity map dedup

The identity map only deduplicates derived structs within a single parent query execution. For singleton-like structs (builtin scope, project scope), always create them through a dedicated `#[query_derived]` function so the identity map lives in one place.

### Incremental cache: StableHash on derived structs

Macro-generated `StableHash` reads every field via `try_<field>(db)`, then recursively hashes nested derived struct fields. This can be expensive and can deadlock under concurrent execution if fingerprints are computed eagerly.

Known mitigations:
- `HirValueKind`: discriminant-only hash since the key already captures identity
- `FileRedNode`: O(1) hash via `(offset, kind, text_len)` instead of recursive tree walk

### LSP: dynamic vs static registration

When a client advertises `dynamicRegistration: true` for `workspace.fileOperations` (as VSCode does), some clients ignore static capabilities declared in `InitializeResult`. The server must use `client/registerCapability` to dynamically register at runtime.
