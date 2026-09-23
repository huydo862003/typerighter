---
_label: "Glossary"
_icon: icon.book
---

Rustc-specific terms used across the rustc research docs.

## CGU (Codegen Unit)

A partition of a crate's code that is compiled independently by the backend (LLVM). `rustc` splits a crate into CGUs to enable parallel compilation and incremental reuse. Each CGU maps to a set of symbols (functions, statics) and produces its own artifact files.

## DefId

A numeric identifier for a definition (function, struct, trait, etc.) within a specific compilation session. `DefId` values are session-local - the same definition may get a different `DefId` in the next session if the source changes. They must never be persisted directly to disk.

## DefPath

A path-based, stable representation of a definition - e.g. `std::collections::HashMap`. Unlike `DefId`, `DefPath` is derived from the source structure and does not shift when unrelated code is added or removed.

## DefPathHash

A 128-bit hash of a `DefPath`. It is `Copy`, cheap to compare, and stable across sessions. This is the practical unit of identity used when persisting anything keyed by definition to disk. On load, the compiler maps `DefPathHash` back to the current session's `DefId` via a hash table lookup.

## DepNode

A node in the dependency graph. Represents one unit of computation (a query invocation). Each `DepNode` carries a `DepKind` (which query) and a key fingerprint (which input). Edges between nodes encode "this computation read the result of that computation".

## Fingerprint

A 128-bit hash (`u64, u64`) produced by `StableHasher`. Used in two roles per dep node:

- **Key fingerprint**: identifies which node this is (a stable hash of the query key, e.g. a `DefPathHash`).
- **Value fingerprint**: a hash of the query's return value, used for change detection across sessions.

## HirId

An identifier for a HIR node that is not a definition-level item. Conceptually a pair of `(DefPath, LocalId)` - the `DefPath` identifies the owning item, and the `LocalId` identifies something within it. Stable across sessions as long as the owning item's path does not change.

## SVH (Stable Version Hash)

A hash of the crate's content used to name finalized session directories. A session directory named `s-{timestamp}-{SVH}` is considered finalized and valid. This lets the compiler quickly identify the most recent valid cache for a given crate.

## Work Product

A record of the compiled artifact files produced for one CGU in a session. Stored in `work-products.bin`. Contains the CGU name and a map of file type to file path (e.g. `"o" -> "/path/to/foo.o"`). On an incremental rebuild, if a CGU's dep-node is green and the symbol hash matches, its work product files are reused directly instead of recompiling.

> Source: [`graph.rs:1110-1135`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/graph.rs#L1110-L1135)
