## [0.12.1] - 2026-08-24

* packages/typerighter
  - Fix build failure: type schema files in `_types/` now correctly serve the not-found page instead of crashing the bundler
  - Vault diagnostics in dev mode now print after the server URL instead of before
  - Vault check runs async after server starts, no longer blocking startup
  - Improve transform error logging: errors now include file path and message

## [0.12.0] - 2026-08-24

BREAKING CHANGE: Retouch vault configuration and organization
- Remove `assets_dir` config
- Remove `content_dir` and `schema_dir` config
- Unify to 1 `root_dir` config

## [0.11.5] - 2026-08-23

* packages/typerighter
  - Properly layout the show more button in a frontmatter list

## [0.11.4] - 2026-08-22

* packages/tree-sitter
  - Add syntax highlighting for ? postfix operator

## [0.11.3] - 2026-08-22

* packages/typerighter
  - Style frontmatter widget list as list of bullet point instead of inline list

## [0.11.2] - 2026-08-22

* packages/typerighter
  - Fix broken static asset serving in vite dev server

## [0.11.1] - 2026-08-22

* packages/typerighter
  - Fix broken ssg

## [0.11.0] - 2026-08-22

* crates/typedown-lang
  - Support existential type
  - Support default and computed in schema definition
  - Support ? operator for optional, remove optional property in schema definition

## [0.10.2] - 2026-08-14

* crates/typedown-lang
  - Container block incorrectly consuming ::: when parsing

## [0.10.1] - 2026-08-14

* crates/typedown-lang
  - Allow tables in markdown to indent

* packages/tree-sitter
  - Add syntax highlighting for container shorthand

* editors/nvim
  - Fix local plugin loader to prioritize our dev version for easy dev testing

## [0.10.0] - 2026-08-14

* crates/typedown-lang
  - Allow blank lines in lists before indented blocks
  - No longer require trailing space between blockquote and content

## [0.9.0] - 2026-08-13

* crates/typedown-lang
  - Add `public_dir` to vault config under `site`, defaults to `public`

* packages/typerighter
  - Add SEO meta tags: OpenGraph, Twitter Card, canonical URL, JSON-LD structured data
  - Add sitemap.xml generation during build
  - Add robots.txt scaffolding in init
  - Add favicon.svg scaffolding from brand icon in init
  - Add public directory support via Vite `publicDir`
  - Add client-side KaTeX rendering for page titles and sidebar tree labels
  - Add per-page document title and meta description updates
  - Fix syntax highlighting: always use dark Shiki theme for code blocks
  - Fix prev/next title overflow with 2-line clamp
  - Fix breadcrumb Home never truncated
  - Fix breadcrumb ellipsis button vertical alignment
  - Fix folder icon color to sienna for visual distinction from file icons
  - Fix scroll-to-top on page navigation without hash
  - Default dev server port to 8686
  - Increase sidebar tree MAX_VISIBLE from 4 to 20
  - Use `_label` as canonical display name, remove `title` from example schemas

## [0.8.0] - 2026-08-13

* packages/typerighter
  - Collapse too long breadcrumbs
  - Add previous/next page navigation
  - Add reusable TdDropdown component with click-outside handling
  - Add `typerighter/shared` package export for generated templates
  - Add centralized index URL helpers and `stripTrailingSlash`
  - Hide empty and builtin-prefixed frontmatter fields
  - Include virtual index pages in prev/next navigation

## [0.7.0] - 2026-08-13

* crates/typedown-lang
  - Rewrite markdown exporter to strip source indentation from nested blocks
  - Report unresolved identifiers in interpolations during vault check
  - Add optional `repo` field to `typedown.yaml`
  - Use `_label` as the canonical display name, fall back to parent directory for index files

* packages/typerighter
  - Add previous/next page navigation
  - Add `basePath` and `repo` to site config
  - Render relation lists as bullet lists instead of inline
  - Fix directory and index page routing to use `/index` suffix
  - Fix folder label overflow in sidebar tree
  - Fix frontmatter row alignment for multiline values

## [0.6.0] - 2026-08-12

* crates/typedown-lang
  - Add container shorthand syntax `[[identifier {props}]]`
  - Support kebab-case identifiers in container blocks and shorthands
  - Allow empty container blocks (`::: note` followed by `:::`)
  - Treat unclosed `[` and `![` before newline as plain text instead of errors

* packages/typerighter
  - Add tooltip component with floating-vue-style dark theme
  - Add TOC active heading highlight via IntersectionObserver
  - Add search keyboard navigation (arrow keys + enter) and loading spinner
  - Add search result grouping by page
  - Add resizable sidebar and table columns
  - Add `[[directory-index]]` custom component for inline directory listings
  - Add deterministic pill colors for select/multiselect frontmatter fields
  - Improve code block styling: full-bleed on mobile, border-radius, full-width highlights
  - Improve inline code styling: border-radius, link color, heading size
  - Fix breadcrumb overflow with ellipsis and `aria-hidden` on separators
  - Fix code blocks inside containers and blockquotes breaking layout
  - Fix timestamp alignment in sidebar tree and content nav
  - Fix search input overflow when sidebar is narrow
  - Replace hardcoded font sizes with design tokens across all components
  - Replace unicode checkbox characters with lucide icons
  - Extract TOC into standalone `TdToc` component
  - Move `TdDirectoryIndex` to custom components directory

## [0.5.1] - 2026-08-12

* packages/typerighter
  - Improve sidebar nested item stylings
  - Improve index file styling
  - No longer strip index from path

## [0.5.0] - 2026-08-11

* crates/typedown-lang
  - Fix infinite loop when a container block is nested inside a list item
  - Fix nested list multi-indentation
  - Fix bullet/ordered/toggle list markers not followed by spaces being treated as lists
  - Fix empty list item diagnostics and list space diagnostics
  - Fix HIR lowering for inline math in strings
  - Fix `[...]` now parsed as text instead of a link attempt
  - Support horizontal rules and backslash escape
  - Drop toggle list syntax
  - Require Debug on incremental ingredient trait and all implementors
  - Add readable query names in debug builds
* crates/typedown-server
  - Fix ref-cell panic in WASM RPC client
  - Improve check command with vault error reporting
  - Exclude non-relevant files from directory scan
  - Offload RPC work off the main thread
  - Load cache on RPC startup for faster responses
  - Improve name resolver performance with a fast path
* packages/typerighter
  - Fix CLI error reporting and improve check command output
* editors/nvim
  - Remove conflicting C-v keymap

## [0.4.4] - 2026-08-09

* crates/typedown-types
  - Wrong exhaustion check of filestream

## [0.4.3] - 2026-08-09

* packages/rpc-server
  - Make install.js more robust on nixos

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
