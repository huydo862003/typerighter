# Change Log

Full changelog: [CHANGELOG.md](https://github.com/huydo862003/typerighter/blob/main/CHANGELOG.md)
## [0.40.2] - 2026-09-25

### Added

- Hover on `_type`/`_extends` values shows schema name and field list with types
- Hover on markdown links/images shows target label, schema, and path
- Hover on field values shows declared type instead of literal value
- Go-to-definition for markdown links and images, resolves relative paths from current file
- Config validation diagnostics published to editor for invalid typedown.yaml
- Completion: required/optional markers on field suggestions, required sorted first
- Completion: `_meta` snippet with tab stops, hidden when already present
- Completion: `fref` inserts `fref("$1")` with cursor inside quotes
- `_meta` builtin type validation (`title: string?`, `description: string?`, `image: string?`)
- `_meta` nullable, consistent with `_label`/`_icon`
- `_meta` data flows end-to-end from Rust extraction through RPC to HTML output
- Centralized URL resolution via `resolve_vault_url` and `prepend_base_path` in export/utils.rs
- HTML and markdown emitters resolve relative URLs in links and images
- `_meta.image` resolved on Rust side via `resolve_vault_url`
- `is_external_url` moved to shared `db::utils`
- Dangling link check in go-to-definition returns None for nonexistent targets

## [0.40.1] - 2026-09-24

### Fixed

- Inlcude dist in rpc-client

## [0.40.0] - 2026-09-24

### Added

- **site.origin config** - New `site.origin` field in typedown.yaml (e.g. `origin: "example.com"`, defaults to https). When set, enables absolute URLs in sitemap, canonical links, Open Graph tags, and JSON-LD.
- **site.lang config** - New `site.lang` field for the HTML lang attribute, defaults to "en".
- **Sitemap improvements** - `<lastmod>` dates from file modification time. Absolute URLs when origin is configured.
- **robots.txt** - Auto-generated when origin is set, with sitemap reference. User's `public/robots.txt` takes priority.
- **RSS feed** - `feed.xml` generated when origin is set, containing the 50 most recently modified pages with title, excerpt, and date.
- **SEO meta tags** - `meta generator`, `meta robots`, `meta author`, `og:site_name`, `og:url`, RSS feed auto-discovery link.
- **JSON-LD structured data** - Article schema with headline, description, author, dateModified, and URL. BreadcrumbList schema for nested pages.
- **_meta builtin** - Per-page SEO overrides via `_meta: { title, description, image }`. Type-validated with autocompletion support.
- **SEO module** - New `seo/` directory with focused modules: `page.ts` (data extraction), `jsonld.ts` (structured data), `sitemap.ts` (sitemap/robots), `feed.ts` (RSS).
- **init prompts for origin** - `typerighter init` now asks for site origin with a note about sitemap/robots usage.

### Fixes

- **pnpm 12 on NixOS** - Updated `packageManager` from pnpm 11.9.0 to 12.3.4. The old version caused "dynamically linked executable" errors on NixOS.

### Dependencies

- bump html-escape 0.2.14 to 0.2.15
- bump vscode-languageclient 10.1.0 to 10.1.1
- bump unescaper 0.1.10 to 0.2.0
- bump @vitejs/plugin-vue 6.0.8 to 6.0.9
- bump @lucide/vue 1.44.0 to 1.47.0
- bump eslint 10.9.1 to 10.11.0
- bump vitest 5.0.0 to 5.0.1
- bump rust-overlay, nixpkgs

## [0.39.0] - 2026-09-23

### Fixed

* crates/typedown-lang
  - Syntax error messages now show descriptive labels ("an identifier", "a field name", "a value") instead of generic "a token" for 17 syntax kinds
  - Complete BTreeMap to HashMap migration for Project.files across all crates, eliminating expensive PathBuf component-by-component comparison

* crates/typedown-server
  - Frontmatter code action no longer generates duplicate `---` delimiters when markers already exist
  - Frontmatter code action includes `_label` and `_icon` with sensible defaults in schema template

### Perf

* crates/typedown-incremental
  - Cache write reduced from 29.5s to 30ms by wrapping dep graph, interned blobs, and query cache writes with BufWriter
  - Cache load: `promote_cached` uses `fingerprint_map` index for O(1) name lookups instead of scanning all dep graph nodes per ingredient
  - PathBuf `StableCompare` overridden to use raw byte comparison instead of component-by-component iteration

* packages/typerighter
  - SSG prerender parallelized with native worker_threads pool (45s to 8.6s for 1284 pages)
  - `WorkerPool` generic utility at `src/node/lib/worker-pool/` for distributing tasks across worker threads
  - Vue SSR uses `shallowRef` for siteData to avoid deep-proxying the content tree
  - CLI no longer hangs after build completes

## [0.38.1] - 2026-09-20

### Fixed

* packages/typerighter
  - Improve search result for more context and remove unnecessary groupings
  - Auto focus search box when sidebar opens

## [0.38.0] - 2026-09-20

### Fixed

* crates/typedown-incremental
  - Identity maps now seeded from previous session's cache, preventing duplicate entry IDs for derived structs (TdSchemaType, etc.) after cache roundtrip
  - No-hash query memos promoted during cache load so their derived identities survive across sessions
  - SKIPPED fingerprint deps re-executed during cross-session green check instead of blindly trusted
  - Dep graph serialization format extended with derived_identities per query memo

* packages/typerighter
  - Vault path resolution in Vite transform now uses path segment boundaries, fixing false matches when parent directory names contain the vault root name
  - `--port` flag now correctly passed to Vite dev server (was ignored due to string-to-number conversion and hardcoded default)
  - `--fresh` flag added to CLI, RPC, and LSP binaries to skip cache loading

* crates/typedown-server
  - `--root`, `--addr`, `--port`, `--fresh`, `--help` CLI flags added to typedown-rpc and typedown-lsp via pico-args
  - `TYPEDOWN_NO_CACHE` env var skips cache loading in both RPC and LSP servers

### Added

* packages/typerighter
  - Playwright e2e test suite: 13 tests across HMR, navigation, and server startup, run against both root and base-path fixtures
  - `data-testid` attributes on sidebar and content elements for robust test selectors
  - `path.stripPrefix` shared utility for safe vault-relative path extraction

* crates/typedown-server
  - 39 integration tests restructured into `cache/` (16 tests) and `multi_session/` (11 tests) modules
  - Multi-session flow tests: RPC-to-LSP, LSP-to-RPC, 3-session chains, concurrent dumps
  - `server_simulation` module with `run_rpc_queries`, `run_lsp_queries`, `collect_type_errors` helpers
  - Zero-type-error regression test for cache corruption bug
  - Cache roundtrip tests assert export headers are non-null and parse results have content

* CI
  - E2E workflow with Playwright on GitHub Actions

## [0.37.1] - 2026-09-19

### Fixed

* packages/typerighter
  - Dev server now serves `fref()` assets correctly when `base_path` is set
  - Removed unused debounced sidebar fetch

## [0.37.0] - 2026-09-19

### Refactored

* crates/typedown-server
  - Unified FS notifications: 4 content events collapsed into `content_updated` / `content_deleted` with post-batch stat verification
  - Schema notifications collapsed into `schema_updated` / `schema_deleted`
  - New directories scanned for files to fix inotify race condition
  - Removed event deduplication in favor of processing all events in order

* packages/typerighter
  - Single handler per notification type, every handler invalidates the individual file
  - Removed `renamed_to` field from content notifications

### Fixed

* packages/typerighter
  - Sidebar folder labels aligned with file icons
  - Reduced tree nesting indent (22px to 16px)
  - Hidden horizontal scrollbar on sidebar and drawer (`overflow-x: clip`)
  - Drawer header sticky on scroll
  - Sidebar search bar sticky above scrollable tree
  - Bottom breathing space on sidebar nav tree

## [0.36.0] - 2026-09-18

### Fixed

* crates/typedown-server
  - Send all FS events in order instead of deduplicating (fixes HMR when OS emits both Create and Modify)

* packages/typerighter
  - All content notifications (created, changed, deleted, renamed) now invalidate the individual file for HMR

## [0.35.0] - 2026-09-18

### Fixed

* crates/typedown-lang
  - Escape curly braces in HTML output to prevent Vue template interference

* packages/typerighter
  - File cache now invalidated on `content_created` events, fixing stale HMR after atomic writes
  - Reverted aggressive brace escaping that broke HMR
  - Frontmatter link icons align to first line of multiline text

## [0.34.3] - 2026-09-18

### Fixed

* crates/typedown-lang
  - Improve diagnostic messages

## [0.34.2] - 2026-09-16

### Fixed

* crates/typedown-lang
  - Intraword underscore (`a_b`) lexed as text per CommonMark rules

* packages/typerighter
  - Curly braces in content no longer eaten by Vue template compiler
  - Frontmatter link icons align to first line of multiline text

### Added

* crates/typedown-lang
  - `__` for bold and `___` for bold italic (matching `**`/`***`)

## [0.34.1] - 2026-09-16

### Fixed

* crates/typedown-incremental
  - Skip cache promotion for no_hash queries so they always recompute on reload

## [0.34.0] - 2026-09-16

### Feat

* crates/typedown-server, packages/typerighter
  - Support mermaid rendering with lazy load to avoid excessive bundled size

### Fixed

* crates/typedown-server
  - Cache persistence: eagerly load all derived entries at startup to prevent dangling IDs across sessions
  - Cache dump wrapped in catch_unwind to clear corrupted cache instead of persisting it
  - Centralized cache directory path into `get_cache_dir`

## [0.33.2] - 2026-09-16

### Fixed

* packages/typerighter
  - Scale h3-h6 properly

## [0.33.1] - 2026-09-16

### Fixed

- Callout titles with quotes no longer split into separate tokens (e.g. `"frontend"` rendered correctly)

## [0.33.0] - 2026-09-16

### Fixed

- File create/delete/rename now triggers sidebar and page updates (OS batching was merging Create+Modify into a single Modified event)
- Dot access completions appear immediately when typing `.` (registered trigger characters)

## [0.32.7] - 2026-09-15

### Fixed

* packages/typerighter
  - Resizable mobile drawer with shared sidebar width
  - Wider default sidebar (272px to 300px)
  - Overscroll padding on sidebar for scrolling past last item
  - Resize handle clamped to 85vw on small screens

## [0.32.6] - 2026-09-15

### Fixes

* crates/typedown-lang
  -  Hash enum discriminant in StableHash for TdTypeEnum and TdObjectEnum

## [0.32.5] - 2026-09-15

### Fixes

* packages/typerighter
  - FOUC for fonts and modified time

## [0.32.4] - 2026-09-15

### Fixes

* packages/typerighter
  - Nested checkbox formatting

## [0.32.3] - 2026-09-15

### Documentation

* packages/typerighter
  - Broken link in README

## [0.32.2] - 2026-09-15

### Perf

* packages/typerighter
  - Avoid prerendering in parallel to prevent OOM on large vaults

## [0.32.1] - 2026-09-15

### Fixes

* crates/typedown-server
  - Invalidate diagnostics for typedown.yaml when it changes

* packages/typerighter
  - Allows scrolling into and cycling through search results

## [0.32.0] - 2026-09-15

### Feat

* packages/typerighter
  - Improve design of SSG

## [0.31.0] - 2026-09-14

### Fixed

* packages/typerighter, crates/typedown-lang, crates/typedown-server
  - File rename now updates sidebar, reroutes if viewing renamed page, and correctly invalidates incremental cache (stale type errors after rename)
  - File deletion reroutes to parent directory when viewing the deleted page
  - `index.td` now respects `_label` in page title
  - Sidebar metadata (`_label`, `_icon`) reactive on content changes
  - Glossary shows `description` and `summary` fields as excerpt
  - `${}` interpolation completions include scope variables (`self`, `fref`, etc.) alongside file suggestions
  - Code block overflow scrolling inside `details` callout
  - Schema rename properly invalidates client cache (delete old + create new)
  - macOS file rename events (`RenameMode::Any`) handled correctly
  - `RedNode::text()` pre-allocates buffer to avoid intermediate string allocations
  - Sidebar metadata refresh debounced to coalesce rapid saves
  - Page HMR detects when a real page replaces a directory index (e.g. `index.td` created)

## [0.30.8] - 2026-09-13

### Documentation

* packages/typerighter
  - Improve typerighter npm README

* editors/vscode
  - Improve vscode README and CHANGELOG

## [0.30.7] - 2026-09-13

### Fixes

* packages/typerighter
  - Direct accesses to fake directory index pages return 404

## [0.30.6] - 2026-09-12

### Features

* crates/typedown-lang
  - Add 55+ new icon entries across 7 categories:
    - Arrows and direction: `arrow_left`, `arrow_right`, `arrow_up`, `arrow_down`, `arrow_left_right`, `arrow_up_down`, `chevron_left`, `chevron_right`, `chevron_up`, `chevron_down`, `external_link`, `move`, `refresh`, `rotate`
    - Actions: `plus`, `minus`, `x`, `edit`, `save`, `download`, `upload`, `share`, `copy`, `print`, `filter`, `sort`, `undo`, `redo`, `log_in`, `log_out`
    - Media: `play`, `pause`, `stop`, `volume`
    - Layout: `list`, `grid`, `sidebar`, `menu`, `more`, `maximize`, `minimize`
    - Status: `success`, `warning`, `error`, `help`, `loading`
    - Combat and gaming: `sword`, `swords`, `crosshair`, `skull`
    - Connectivity: `wifi`, `bluetooth`, `power`

## [0.30.5] - 2026-09-12

* crates/typedown-server
  - Generic dot-access completion: `icon.` suggests icon names, works for any expression with fields
  - Expression completion: suggests variables in scope (builtins, file names, imports, closure params) in value positions

## [0.30.4] - 2026-09-12

### Fixes

* packages/typerighter
  - Prev and next navigation should prioritize index on top
  - Make TOC items preserve formatting
  - In prev and next cards, format index as folder title

## [0.30.3] - 2026-09-12

### Fixes

* crates/typedown-lang
  - Fix reactivity problems for sidebar and prev/next navigation

## [0.30.2] - 2026-09-12 (retracted)

### Fixes

* crates/typedown-lang
  - Preserve inline code, math, and interpolation i>

## [0.30.1] - 2026-09-12 (retracted)

### Fixes

* packages/typerighter
  - Dev server middleware now passes through Vite internal requests (`/@vite/client`, `/@id/...`) when `base_path` is set

## [0.30.0] - 2026-09-11 (retracted)

### Features

* crates/typedown-lang
  - `find_transitive_referrers`: find all files that transitively reference a given file via fref
  - Per-resource page data virtual module (`@typedown/pages?resource=<path>`) for granular HMR

* crates/typedown-server
  - `affectedFiles` field in content change notification for transitive fref invalidation
  - fref completions inside `${}` interpolation in markdown body
  - fref completion labels now show human-readable `_label` instead of file paths

* packages/typerighter
  - TOC hot reload via per-resource virtual module
  - Transitive HMR: changing a file invalidates all files that reference it
  - Sidebar scroll container with `overscroll-behavior: contain`
  - Skip-to-content link and keyboard-accessible sidebar resize
  - Header and footer use subtle background color for visual separation

### Fixes

* crates/typedown-lang
  - Preserve special characters (`.`, `:`, `#`, `+`) in TOC heading titles
  - Skip non-content files (SVGs, assets) in `references()` to avoid lexer panics
  - Move fref icon inside `<a>` tag to fix rendering inside `<p>` elements

* packages/typerighter
  - Remove `max-width` centering on content area
  - Heading scroll-margin-top accounts for sticky header offset

* editors/nvim
  - Better error messages in `TypedownPasteAsset` for clipboard and file write failures

## [0.29.0] - 2026-09-11 (retracted)

### Fixes

* crates/typedown-lang
  - Restore HirValueKind to non-discriminantly-only stable hashing to avoid broken cache

* crates/typedown-incremental
  - Enable cache dumping again

## [0.28.2] - 2026-09-11 (retracted)

### Fixes

* packages/typerighter
  - Broken assets transform

## [0.28.1] - 2026-09-11 (retracted)

### Fixes

* crates/typedown-incremental
  - Temporarily disable cache dumping to investigate broken cache loading

## [0.28.0] - 2026-09-10 (retracted)

### Features

* crates/typedown-lang
  - Support relative fref paths (`./` and `../`) resolved from the current file's directory

### Perf

* crates/typedown-incremental
  - Lax return type constraint on derived queries (accept any Id type, not just DerivedId)

* crates/typedown-lang
  - Optimize scope and runtime: interned scopes, streamlined symbol/HIR accessors, reduced overhead in ingredient storage (10min+ -> 3sec on large vaults)

### Fixes

* crates/typedown-lang
  - Allow indexing on string literal types (`"abc"[0]` no longer reports "not indexable")
  - Report missing `)` diagnostic when a call expression is interrupted by an outer context (e.g. `${fref('./...')}` with misplaced quote)

* editors/nvim
  - Rename `:TypedownPaste` to `:TypedownPasteAsset`
  - Use relative fref path (`./_assets/...`) in pasted asset references
  - Show warning when no supported image is found in clipboard

## [0.27.1] - 2026-09-09 (retracted)

### Perf

* crates/typedown-lang
  - Deduplicate singleton scopes to reduce cache bloat
  - Optimize stable hash of HIR
  - Use revision-based counter to perform more optimized LRU eviction

## [0.27.0] - 2026-09-08 (retracted)

### Features

* crates/typedown-lang
  - Support linked images / badge syntax: `[![alt](img-url)](link-url)`
  - Body-only files (no frontmatter) now produce a proper empty product instead of null

### Perf

* crates/typedown-lang
  - O(1) `StableHash` for `RedNode` via `FileRedNode` wrapper (offset + kind + text_len instead of recursive tree walk)

* crates/typedown-server
  - Timeout cache dump to prevent hanging on large vaults (10s limit)
  - Graceful SIGINT handling via `ctrlc` for cache persistence

* packages/typerighter
  - Instant sidebar via disk scan (bypass RPC for initial load)
  - Lazy site data loading (app mounts immediately, sidebar fills via HMR)
  - Disk-based search indexing (1251 RPC calls replaced with `globSync`)
  - Eagerly populate site data and search index in production builds

### Fixes

* crates/typedown-lang
  - Add `noopener` to all external link `rel` attributes

* crates/typedown-server
  - Gracefully handle write stream failure in TCP transport

* packages/typerighter
  - Auto-detect `basePath` from `typedown.yaml` for preview and dev commands
  - Fix HMR issues for sidebar virtual module
  - Support multiple notification handlers in RPC client
  - Eagerly fetch site data and search index during production builds

## [0.26.0] - 2026-09-07 (retracted)

### Perf

* crates/typedown-incremental
  - Split ingredient storage into typed arrays with composite DepId
  - Replace decoder DashMap with Vec<AtomicU64> for lock-free deserialization
  - Bulk-write StableHash for str/OsStr instead of byte-by-byte
  - Identity hasher for integer-keyed DashMaps
  - AtomicU32 verified_at to avoid write locks in green check
  - Point-lookup green check instead of set collection
  - Enable thin LTO for release builds

### Fixes

* crates/typedown-lang
  - Fix schema export including internal fields
  - Remove unused Set<T> wrapper

## [0.25.0] - 2026-09-06 (retracted)

### Fixes

* crates/typedown-lang
  - Failed exporting schemaless files

* packages/rpc-client
  - Use a handrolled rpc client

### Perf

* crates/typedown-server
  - Drop jsonrpsee and uses multithreading instead of tokio

## [0.24.1] - 2026-09-05 (retracted)

### Fixes

* crates/typedown-lang
  - Exported html for custom container contains unclosed templates

## [0.24.0] - 2026-09-05 (retracted)

### Perf

* packages/typerighter
  - Search indexing reads directly from disk instead of going through RPC, ~100x faster for large vaults
  - Virtual modules (site data, pages, search index) loaded lazily on first access instead of blocking startup

### Fixes

* packages/typerighter
  - Search bar styling

## [0.23.0] - 2026-09-05 (retracted)

### Features

* packages/typerighter
  - Add site footer with author and license from `typedown.yaml`
  - Add keyboard shortcut system (`Ctrl+K` / `Cmd+K` to toggle search)
  - Support `site.nav` links in `typedown.yaml` for header navigation

* crates/typedown-lang
  - Support `site.nav` config with icon syntax (`icon.book`)

### Refactor

* crates/typedown-lang
  - Parse `typedown.yaml` config via native typedown YAML parser, drop `yaml-rust2`
  - Replace markdown-it with Rust HTML emitter for markdown-to-HTML conversion

* packages/typerighter
  - Replace markdown-it pipeline with lightweight post-processor (shiki + Temml only)
  - Drop 14 npm dependencies (`markdown-it`, `@mdit-vue/*`, `@mdit/plugin-*`, `katex`, etc.)

### Fixes

* packages/typerighter
  - Fix `page.frontmatter` access error
  - Fix left sidebar overflow-x
  - Fix search toggle focus on `Ctrl+K` / `Cmd+K`
  - Use template refs for search toggle instead of DOM class query

## [0.22.2] - 2026-09-03 (retracted)

### Perf

* packages/typerighter
  - Avoid waterfall when fetching search indexes, site data, etc.

### Fixes

* packages/typerighter
  - `index.html` is no longer required and is ignored

## [0.22.1] - 2026-09-03 (retracted)

### Chore

* crates/typedown-server
  - Switch Linux release binary to static linking (musl) for NixOS/Alpine compatibility

## [0.22.0] - 2026-09-03 (retracted)

### Chore

* editors/vscode
  - Rename publisher to `huydo862003`
  - Add missing metadata
  - Unify the vscode extension to use 1 vsix variant to bypass vscode marketplace hassels

## [0.21.1] - 2026-09-02 (retracted)

### Perf

* packages/typerighter
  - Remove all dead dependencies

### Chore

* packages/typerighter
  - Bump packages to resolve vulnerabilities

## [0.21.0] - 2026-09-02 (retracted)

### Feat

* packages/typerighter
  - `typedown({ root })`: mount a vault as a subpath of an existing Vite app
  - `typerighter init` accepts `--name`, `--title`, `--description`, `--yes` flags for non-interactive scaffolding
  - `typerighter build <dir>` now resolves `<dir>` as the project root

### Fixes

* crates/typedown-lang
  - Inline code blocks (`` `{ a: b }` ``) no longer emit invalid code range indicator diagnostics

## [0.20.4] - 2026-09-02 (retracted)

### Fixes

* crates/typedown-lang
  - Icon handling in markdown body

## [0.20.3] - 2026-09-01 (retracted)

### Perf

* crates/typedown-lang
  - Add `export_resource_meta`: lightweight metadata export
* crates/typedown-server
  - Add `list_sidebar` RPC endpoint
* packages/typerighter
  - `fetchSiteData` uses `listSidebar` for content tree

## [0.20.2] - 2026-09-01 (retracted)

### Feat

* packages/typerighter
  - Folders sort above files in sidebar by default
  - Schemaless files support `_label` and `_icon`
  - Display `schemaLabel` in page eyebrow
  - Folder icon and label inherit from index file

## [0.20.1] - 2026-09-01 (retracted)

### Fixes

* crates/typedown-incremental
  - Fix false cycle panics during concurrent query execution

## [0.20.0] - 2026-09-01 (retracted)

### Feat

* crates/typedown-lang
  - Support schema label and icon

## [0.19.0] - 2026-08-31 (retracted)

### Feat

* crates/typedown-lang
  - Support `|` operator for enum

## [0.18.0] - 2026-08-31 (retracted)

### Feat

* crates/typedown-lang
  - **Page icons**: add `_icon` built-in field with `Icon` type and `icon` built-in module
  - LSP autocompletion for `_icon` values

* packages/typerighter
  - Render page icons in sidebar nav and page header

### Fixes

* crates/typedown-lang
  - Formatter deleting `$$` and code blocks

## [0.17.5] - 2026-08-31 (retracted)

### Fixes

* packages/typerighter
  - FOUD issues when accessing the website

## [0.17.4] - 2026-08-31 (retracted)

### Fixes

* packages/typerighter
  - Broken font imports

## [0.17.3] - 2026-08-31 (retracted)

### Perf

* packages/typerighter
  - Optimize css loading by splitting fonts out of main.css

## [0.17.2] - 2026-08-31 (retracted)

### Fixes

* crates/typedown-lang
  - Don't strip _label when export header

## [0.17.1] - 2026-08-31 (retracted)

### Fixes

* packages/typerighter
  - Properly copy public dir to output

## [0.17.0] - 2026-08-31 (retracted)

### Feat

* crates/typedown-lang
  - Excess field check
* crates/typedown-incremental
  - Cache GC, LRU eviction, fallible getters, `specialize!` proc macro

### Refactor

* Remove nightly Rust, switch to stable toolchain

### Fixes

* packages/typerighter
  - Base path routing fixes

## [0.16.0] - 2026-08-27 (retracted)

### Feat

* crates/typedown-server
  - Inlay hints, create linked resource command, enhanced completions
* editors/vscode
  - `createLinkedResource` command support
* homepage
  - New documentation site

## [0.15.1] - 2026-08-26 (retracted)

### Fixes

* editors/vscode
  - Fixed paste handler SnippetString escaping

## [0.15.0] - 2026-08-26 (retracted)

### Breaking changes

* crates/typedown-lang
  - Schema inheritance (`_extends`), schema/product type split, modules (`_imports`)

## [0.14.0] - 2026-08-25 (retracted)

* packages/typerighter
  - Improve type system, layout, frontmatter, add glossary view

## [0.13.0] - 2026-08-24 (retracted)

* packages/typerighter
  - Add github corner icon if there's a repo config

## [0.12.0] - 2026-08-24 (retracted)

BREAKING CHANGE: Retouch vault configuration and organization

## [0.11.0] - 2026-08-22 (retracted)

* crates/typedown-lang
  - Support existential type, default/computed in schema, ? operator

## [0.10.0] - 2026-08-14 (retracted)

* crates/typedown-lang
  - Allow blank lines in lists, no longer require trailing space after blockquote marker

## [0.9.0] - 2026-08-13 (retracted)

* packages/typerighter
  - SEO meta tags, sitemap, robots.txt, favicon scaffolding, public dir support

## [0.8.0] - 2026-08-13 (retracted)

* packages/typerighter
  - Breadcrumb collapse, prev/next navigation, TdDropdown, search improvements

## [0.7.0] - 2026-08-13 (retracted)

* crates/typedown-lang / packages/typerighter
  - `_label` as canonical display name, `repo` config, prev/next navigation

## [0.6.0] - 2026-08-12 (retracted)

* crates/typedown-lang
  - Container shorthand syntax, kebab-case identifiers

## [0.5.0] - 2026-08-11 (retracted)

* Various parser, LSP, and CLI fixes

## [0.4.0] - 2026-08-09 (retracted)

* crates/typedown-lang
  - Custom components, code ranges, `.md` file support
* packages/typerighter
  - Templating, frontmatter rendering, document search, sidebar improvements
* editors
  - Updated grammar for container syntax

## [0.3.1] - 2026-08-04 (retracted)

* packages/typerighter
  - Fix broken hydration, pre-render virtual index page

## [0.1.0] - 2026-08-01 (retracted)

First major version with core compiler, language services, and static site generator
