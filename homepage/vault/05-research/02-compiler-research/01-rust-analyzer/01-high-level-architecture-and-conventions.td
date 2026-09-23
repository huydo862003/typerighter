---
_label: "High-Level Architecture & Conventions"
_icon: icon.document
---

This section describes the architecture of `rust-analyzer`.

Official site: [Link](https://rust-analyzer.github.io/book/contributing/architecture.html).

- `rust-analyzer` input/output:
  - Input (Ground state): Source code data from the client. Everything is kept in memory.
    - Mapping from file paths to their contents.
    - Project structure metadata represented as a crate graph (crate roots, `cfg` flags, crate dependencies)
  - Output (Derived state): "Structure semantic model" of the code.
    - A representation of the project that is fully resolved - type-wise and reference-wise.

- Optimizations:
  - Incremental:
    - Input can be a delta of changes.
    - Output can be a fresh code model.
  - Lazy: The output is computed on-demand.

## Parser - `parser` Crate

- A hand-written recursive descent tree-agnostic parser.
- Output: A sequence of events like "start node" and "finish node", based on [kotlin's parser](https://github.com/JetBrains/kotlin/blob/4d951de616b20feca92f3e9cc9679b2de9e65195/compiler/frontend/src/org/jetbrains/kotlin/parsing/KotlinParsing.java), which can be used to learn about dealing with syntax errors and incomplete input.
- Some traits (`TreeSink` and `TokenSource`) are used to bridge the tree-agnostic parser with `rowan` trees.

### Architecture Invariant - Tree-Agnostic Parser

- The parser functions as a pure transformer, converting one flat stream of events into another.
- Dual independence: The parser is not locked into:
  - A specific tree structure (output format).
  - A specific token representation (input format).
- Benefits:
  - Token independence allows using the same logic to parse:
    - Standard source code (text -> tokens).
    - Macro expansion (token trees -> tokens).
    - Synthetic code generated programmatically.
  - Tree independence allows easily varying the syntax tree implementation + light-parsing.
    - Avoid allocation of tree nodes.
    - For tasks like "find all function names in this 10k line file," the parser can simply emit "DefineName" events. A listener catches those names and ignores everything else, finishing the task in a fraction of the time.

### Architecture Invariant - Infallible Parser

- Parsing never fails.
- Parser returns `(T, Vec<Error>)`.

## Syntax Tree Structure & Parser - `syntax` Crate

Based on [`libsyntax-2.0`](../03-libsyntax).

- [`rowan`](https://github.com/rust-analyzer/rowan): The underlying library used to construct the raw, untyped syntax trees (Green/Red trees).
- `ast` internal crate: Provide a **type-safe API** layer on top of the raw `rowan` tree.
- `ungrammar` internal crate: A grammar description format used to automatically generate the `syntax_kinds` and `ast` modules.

### Architecture Invariant - `syntax` Crate as an API Boundary

- The `syntax` crate knows nothing about `salsa` and LSP. It's an API boundary.
- Benefits:
  - Allows it to be used for lightweight tooling without needing a full build or semantic analysis.

### Architecture Invariant - Syntax Tree as a Value Type

- The syntax tree is self-contained, defined solely by its contents without relying on global context (like interners).
- Pure syntax: Unlike traditional compiler trees, it strictly excludes semantic information (such as type inference data).
- Benefits:
  - IDE optimization: Critical for tools like `rust-analyzer`, where assists and refactors require frequent tree modifications.
  - Simplified transformation: Keeping the tree "dumb" (purely structural) allows for easy code manipulation without the complexity of managing semantic state during edits.

### Architecture Invariant - Syntax Tree per File

- A syntax tree is built for a single file.
- Benefits: Enable parallel parsing of all files.

### Architectural Invariant - Incomplete Syntax Tree

- Syntax trees are designed to tolerate incomplete or invalid code (common during live editing).
- AST accessor methods return `Option` types to safely handle missing data.

## Query database - `base-db` Crate

- [`salsa`](https://github.com/salsa-rs/salsa): A crate used for incremental and on-demand computation.
  - `salsa` resembles a key-value store.
  - `salsa` can compute derived values with specified functions.
- Define most input queries.

### Architecture Invariant - File-System Agnostic

- Nothing is known about the file system & file paths.
- `FileId`: An opaque type that represents a file.

## Analyzer (Macro expansion, Name resolution, Type inference) - `hir-xxx` Crates

- `hir-expand`: Macro expansion.
- `hir-def`: Name resolution.
- `hir_ty`: Type inference (Why does this one uses underscore?).

- Define various IRs of the core.
- `hir-xxx` is ECS-based (Entity-Component-System):
  - ECS architecture: Instead of rich objects, compiler entities (like functions or structs) are represented as raw integer IDs (handles), similar to game entities.
  - Database-driven: You cannot access data directly from an ID. You must query the central Salsa database (e.g., `db.function_data(id)`), which stores the actual content in "component" arrays.
- Zero abstraction: The code is intentionally explicit about database access. It avoids helper methods to keep dependency tracking transparent and overhead low.
- These crates "lower" (translate) Rust syntax into logic predicates, allowing the `chalk` engine to solve complex trait bounds and type inference.

### Architecture Invariant - Incremental

- The separation of "Identity" (ID) from "Data" in ECS allows `rust-analyzer` to update only changed data without breaking references to the ID elsewhere, enabling millisecond-level updates.

## High-Level IR - `hir` Crate

- An API boundary for consuming `rust-analyzer` as a library.

- `hir` acts as the high-level API boundary, wrapping internal raw IDs (ECS-style) into semantic structs (e.g., `Function`) to provide a familiar object-oriented interface for library consumers.
- "Thin handle" Pattern: These structs hold no data (only the ID) and require the `db` to be passed into every method call (e.g., `func.name(db)`), effectively bridging the stateless handles with the stateful Salsa database.
- Analogy: Internally, the ECS-style code is like SQL & `hir` is like ORM.
  - Syntax inversion (object-oriented vs functional):
    - In pure ECS, logic lives in external systems (e.g. `db.function_visibility(id)`).
    - The `hir` crate inverts this to an object-oriented style (`func.visibility(db)`), making the API discoverable via IDE autocomplete.
  - Encapsulated "joins":
    - Pure ECS requires you to manually query multiple tables to piece together information (e.g., get parent module ID -> look up module data -> find visibility).
    - The `hir` crate abstracts these complex multi-step database lookups into single, coherent methods.
  - Semantic types:
    - ECS deals with efficient storage (raw `u32` IDs).
    - `hir` deals with high-level meaning, exposing semantic types (like struct `Type`) rather than implementation details (like struct `TypeId`).

### Architecture Invariant - Inert Data Structure

- `hir` presents a fully resolved, inert view of the code, abstracting away the dynamic computations occurring in internal crates.
  - "Inert" here is relative to the `db` object.
- Syntax-to-HIR bridge: It manages the complex one-to-many mapping between raw syntax and semantic definitions (via the `Semantics` type).
- The "Uber-IDE" pattern: To resolve a specific syntax node to an HIR entity (essential for "Go to Definition"), it employs a recursive strategy used by Roslyn and Kotlin: it resolves the syntax parent to a HIR owner, then queries that owner's children to re-identify the target node.

## IDE - `ide-xxx` Crates

- Top-level API boundary: The ultimate entry point for external clients (LSP servers, text editors) to interact with rust-analyzer.
- `ide` consumes the semantic model provided by the `hir` crate to implement concrete user features like code completion, goto definition, and refactoring.
- `ide` is protocol-agnostic, designed to be used via LSP, custom protocols (like FlatBuffers), or directly as a library within an editor.

- This crate introduces the concept of change over time:
  - `AnalysisHost`: The mutable state container where you `apply_change`.
  - `Analysis`: An immutable, transactional snapshot of the state used for querying.

- Modular Architecture:
  - `ide`: The public facade and home for smaller features.
  - `ide-db`: Shared infrastructure (e.g. reference search).
  - `ide-xxx`: Isolated crates for major features (completion, diagnostics, assists, SSR).

### Architecture Invariant - View Layer

- View/ViewModel layer:
  - `ide` acts as the "View" (MVC) or "ViewModel" (MVVM), translating complex compiler data into simple, editor-friendly terms (offsets, text labels) rather than internal definitions or syntax trees.
  - The API is built with POD types. All inputs and outputs are conceptually serializable (no complex object graphs or HIR types exposed).
- The boundary is explicitly drawn at the "UI" level, following the philosophy popularized by the Language Server Protocol. - It talks in the language of the text editor & not the language of the compiler.

## `rust-analyzer` Crate

- Define the binary for the language server -> The entry point.
- It acts like the network/protocol adapter tha connects the pure logic of the `ide` crate to the outside world. -> Functional core, imperative shell.

### Architecture Invariant - LSP & JSON Awareness

- `rust-analyzer` is the only place where LSP types and JSON serialization exist.
- Lower crates (`ide`, `hir`) remain pure and protocol-agnostic. They are forbidden from deriving `Serialize` or `Deserialize` for LSP purposes.
- `rust-analyzer` maintains its own set of serializable types. It manually converts the `ide` crate's Rust-native data structures (like `TextRange`) into LSP's wire-format structures (like `Range` with line/character) before sending them over the wire.

### Architecture Invariant - Protocol-Wise Statelessness

- The server is stateless, in the sense that it doesn't know about the previous requests.

## Utilities - `stdx` Crate

- `rust-analyzer` avoids small helper crates.
- `stdx` is the crate to store all small reusable utilities.

## Macro Crates

- Core abstraction (`tt`): Macros are defined purely as `TokenTree` -> `TokenTree` transforms, isolated from other compiler parts. The `tt` crate defines this structure (single tokens or delimited sequences).
- Declarative macros (`mbe`): The `mbe` crate implements "Macros By Example" (`macro_rules!`). It handles parsing, expansion, and the translation between the IDE's syntax trees and the raw token trees.
- Procedural Macros: Proc-macros run in a separate process to isolate the IDE from user code crashes.
  - Server (`proc-macro-srv`): Load the dynamic libraries (built by Cargo) and executes the macros.
  - Client (`proc-macro-api`): Communicate with the server, sending/receiving Token Trees.

### Architecture Invariant - Isolation

- Because arbitrary macro code can panic or segfault (crashing the editor), `rust-analyzer` executes them in a separate process. This allows the main IDE to survive fatal errors and recover gracefully.
- `salsa`'s incremental system assumes all functions are pure (deterministic). Since proc-macros can be non-deterministic (e.g., reading external files or random numbers), they violate this core assumption and require special handling to prevent database corruption or infinite invalidation loops.

## Virtual File System - `vfs-xxx` and `paths` Crates

- Virtual file system (VFS): These crates provide an abstraction layer that generates consistent snapshots of the file system, insulating the compiler from raw, messy OS paths.
- The architecture does not assume a single unified file system. A single `rust-analyzer` process can serve multiple remote machines simultaneously, meaning the same path string could exist on two different machines and refer to different content.
- "Witness" API: To resolve this ambiguity, path APIs generally require a "file system witness" (an existing anchor path) to identify which specific file system context the operation targets.

## Interning - `intern` Crate

- Use `Arc` (Atomic Reference Counting) to ensure identical data (like strings or paths) is stored only once in memory.
- Optimized for "value types" that are defined by their content (e.g., `std::vec::Vec`), rather than "entities" defined by an ID (e.g., `Function #42`).
- `db`-independent: Unlike `salsa`'s integer IDs, `Interned<T>` owns its data, allowing access and inspection without needing a reference to the compiler database (`db`).
- Interning enables instant equality checks by comparing memory pointers instead of scanning content, which is critical for frequently compared items like file paths.
- Interning serves as a lower-level optimization layer for static, immutable data that doesn't require the full overhead of incremental dependency tracking.

## Architectural Policies

### Stability Guarantees

- `rust-analyzer` avoids new stability guarantees to move fast.
- The internal `ide` API is explicitly unstable.
- Stability is only guaranteed at the **LSP** level (managed by the protocol) and **input** level (Rust language/Cargo).
- De-facto stability: `rust-project.json` became stable implicitly by virtue of having users - a lesson to explicitly mark APIs as unstable/opt-in before release.

### Code Generation

- The API for syntax trees (`syntax::ast`) and manual sections (features, assists, config) are generated automatically.
- To simplify builds, `rust-analyzer` does not use itself for codegen. It uses `syn` and manual string parsing instead.

### 3. Cancellation (Concurrency)

- The problem: If the user types while the IDE is computing (e.g., highlighting), the result is immediately stale.
- The solution: The **salsa** database maintains a global revision counter.
  - When input changes, the counter is bumped.
  - Old threads checking the counter notice the mismatch and **panic** with a special `Canceled` token.
- The `ide` boundary catches this panic and converts it into a `Result<T, Canceled>`.

### Testing Strategy

- Tests are concentrated on three system boundaries:
  - Outer (`rust-analyzer` crate): "Heavy" integration tests via LSP/stdio. Validates the protocol but is slow (reads real files).
  - Middle (`ide` crate): The most important layer. Tests `AnalysisHost` (simulating an editor) against expectations.
  - Inner (`hir` crate): Tests semantic models using rich types and **snapshot testing** (via the `expect` crate).

#### Key Testing Invariants

- Data-driven: Tests use string fixtures (representing multiple files) rather than calling API setup functions manually. This allows significant API refactorings.
- No `libstd`: Tests do not link to `libstd`/`libcore` to ensure speed; all necessary code is defined within the test fixture.

### Error Handling

- No IO in core: Internal crates (`ide`, `hir`) are pure and never fail (no `Result`). They return partial data plus errors: `(T, Vec<Error>)`.
- Panic resilience: Since bugs are inevitable, every LSP request is wrapped in `catch_unwind` so a crash in one feature doesn't kill the server.
- Macros: Uses `always!` and `never!` macros to handle impossible states gracefully.

### Observability

- Profiling: Includes a custom low-overhead hierarchical profiler (`hprof`) enabled via env vars (`RA_PROFILE`).

### Serialization

- The trap of ease: While `#[derive(Serialize)]` is easy to add, it creates rigid IPC boundaries (backward compatibility contracts) that are extremely difficult to change later.
- To strictly preserve internal flexibility, types in core crates like `ide` and `base_db` are not serializable by design.
- Serialization is forced to the "edge" (the client). External clients must define their own stable schemas (e.g., `rust-project.json`) and manually convert them into internal structures, isolating the core compiler from protocol versioning issues.
