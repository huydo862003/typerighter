## [0.4.2] - 2026-08-09

* packages/rpc-server
  - Detect NixOS and install properly for rpc-server

## [0.4.1] - 2026-08-09

* packages/typerighter
  - Fix the index listing to correctly link to children dir

## [0.4.0] - 2026-08-09

* crates/typedown-lang
  - Support custom components `::: name {prop="value" flag}`
  - Support code ranges in code block (` ```js{1,3,5-8} `) with `language()` and `line_ranges()`
  - Rename callout to container across the AST, formatter, and exporter
  - Treat `{` and `}` as lexer tokens
  - Add container title support
  - Support typeless `.td` files
  - Support `.md` files alongside `.td`
  - Remove non-standard CommonMark syntax extensions
* packages/tree-sitter/typedown-md
  - Grammar and external scanner for container props and slot separators
  - Corpus coverage for containers, nested containers, and code block ranges
  - Fix container title parsing
* packages/typerighter
  - Add templating and snippet support
  - Render frontmatter in the default theme
  - Implement document search with MiniSearch
  - Show file modification time and improve theming
  - Improve sidebar directory tree navigation
  - Add file icons to directory listings
* editors
  - Update Neovim highlight queries and the VS Code TextMate grammar for container syntax

## [0.3.1] - 2026-08-04

* packages/typerighter
  - Use Vite manifest to resolve client entry (fixes broken hydration)
  - Pre-render virtual index page in SSG build
  - Clear output directory before building to prevent stale assets

## [0.3.0] - 2026-08-04

[0.2.3] - 2026-08-04

* crates/typedown-server
  - Add close() to WASM RPC client for safe cleanup
* packages/typerighter
  - Migrate CLI to cac framework with --help, --version, and options
  - Clean build output with phase summaries, timing, and progress percentage
  - Add file logging to .typedown/.local/logs/
  - Detect existing project in init and generate .gitignore
  - Introduce AppContext for resource lifecycle management
  - Drop consola, use picocolors directly

## [0.2.2] - 2026-08-04

* packages/typerighter
  - Avoid the server hanging after build
  - Improve the server logging for phases

## [0.2.1] - 2026-08-04

* packages/typerighter
  - Bundle typerighter into ssr build to avoid dual vue instances

## [0.2.0] - 2026-08-03

* packages/typerighter
  - Fix build and dev command failure for cli

## [0.1.6] - 2026-08-02 (Deprecated)

* packages/typerighter
  - No longer inject html

## [0.1.5] - 2026-08-02 (Deprecated)

* packages/typerighter
  - Split cli to a vite endtrypoint and properly import it

## [0.1.4] - 2026-08-02 (Deprecated)

* packages/rpc-server
  - Support unref() to avoid keeping the event loop alive

* packages/typerighter
  - Support CLI
  - Prevent building from keeping the event loop alive by unref() the server
  - Use random port for RPC server

* crates/typedown-lang
  - Support exporting assets to markdown

## [0.1.3] - 2026-08-02 (Deprecated)

* packages/rpc-server
  - Prioritize binary on PATH

## [0.1.2] - 2026-08-02 (Deprecated)

* packages/rpc-server
  - Fall back to system PATH when typedown-rpc binary is not in node_modules

* editors/nvim
  - Add list[T] and dict[K,V] syntax highlighting for index expressions
  - Fix devicons icon key (use file extension 'td' instead of filetype 'typedown')

* editors/zed
  - Add list[T] and dict[K,V] syntax highlighting for index expressions

* editors/vscode
  - Add list[T] and dict[K,V] syntax highlighting via tmLanguage pattern

## [0.1.1] - 2026-08-01 (Deprecated)

* crates/\*, packages/\*, editors/\*
  - Report unresolved identifiers as errors when used as field values
  - Skip diagnostics for non-td files (fixes false errors on typedown.yaml)
  - Add Nix flake build for typedown-lsp and typedown-rpc
  - Fix repo URLs in npm packages

## [0.1.0] - 2026-08-01 (Deprecated)

First major version with core compiler + language services and static site generator
