# Stable Hasher

This module implements session-independent and architecture-independent hashing/sorting.

It draws inspiration from `rustc`, check my related docs here:

- [dboxide findings of `rustc`](https://huydo862003.github.io/dboxide/research/resources/rustc/SUMMARY.html)
- [`compiler/rustc_data_structures/src/stable_hash.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_data_structures/src/stable_hash.rs) - trait definitions

## Why stable hashing

The incremental cache needs to detect whether a query result changed between sessions.

Comparing old and new values directly would require deserializing the old value from disk (expensive for large values like ASTs, HTML strings, or maps with 1000+ entries). Instead, we store a 16-byte fingerprint alongside each
cached result.

On reload, we recompute the result, hash it, and compare fingerprints, avoiding the deserialization cost.

This is the same approach used by `rustc`'s incremental compilation.

## Performance considerations

Stable hashing is the most expensive part of `db.dump()` (cache serialization).
`rustc` acknowledges this: "computing fingerprints is quite costly. It is the main reason why incremental compilation can be slower than non-incremental compilation". (`rustc` dev guide, incremental compilation)

Key design choices to keep hashing fast:

- **Use BTreeMap/BTreeSet for String/PathBuf-keyed maps**: HashMap/HashSet require O(n log n) sorting on every hash call (via `StableCompare`) to ensure deterministic iteration order. BTreeMap is already sorted by `Ord`, so iteration is O(n). Profiling showed that HashMap sorting accounted for 65% of dump time on a 1272-file vault (the bottleneck was `std::path::compare_components` called during quicksort of PathBuf keys)

- **HashMap/HashSet is still used for interned-type keys** (e.g `Symbol`, `LazyType`) where `Ord` is session-dependent (numeric IDs change between sessions). These maps are typically small, so the sorting cost is acceptable.

- **Possible future optimization: `no_hash` / `skip_cache`**: `rustc` offers a `no_hash` modifier that skips fingerprinting for queries whose results are expensive to hash but cheap to recompute. A `skip_cache` variant that also  skips serialization would further reduce dump time for export queries
