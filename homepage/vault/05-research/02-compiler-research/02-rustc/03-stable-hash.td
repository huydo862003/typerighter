---
_label: "StableHash Infrastructure"
_icon: icon.document
---

This document covers how `rustc` hashes values in a way that is stable across compilation sessions, which is required for fingerprint-based change detection.

The conceptual need for stable hashing is described in [Incremental Compilation in Detail](01-incremental-compilation). This document covers how it is actually implemented.

Sources:

- Rustc dev guide: [Incremental Compilation in Detail](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html)
- [`compiler/rustc_data_structures/src/stable_hash.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_data_structures/src/stable_hash.rs) - trait definitions
- [`compiler/rustc_middle/src/dep_graph/graph.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/graph.rs) - fingerprint computation

## The Problem

Rust's standard `Hash` trait is not stable across sessions. It can hash memory addresses, pointer values, or anything tied to the current process state. Two objects that are logically identical will not produce the same hash if session-local details differ.

For incremental compilation, fingerprints must be consistent: hashing the same value in session 1 and session 2 must produce the same bits, so the compiler can tell whether a query result changed.

## The `StableHash` Trait

[`stable_hash.rs:76-77`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_data_structures/src/stable_hash.rs#L76-L77)

`rustc` defines a separate `StableHash` trait. Its doc comment states these requirements:

- **Session-independent**: must not hash memory addresses, `DefId` values, or any other session-local state.
- **Architecture-independent**: must produce the same bits on any host (endianness, pointer size). The `StableHasher` handles this internally.
- **Consistent with equality**: `x == y` implies `stable_hash(x) == stable_hash(y)`, and `x != y` implies `stable_hash(x) != stable_hash(y)`. This second condition is stricter than the standard `Hash` trait, which only requires the first.

For simple types (integers, strings, paths), `StableHash` delegates to the standard hasher since their values are already stable. For session-local types like `DefId`, it requires a **hashing context** (`Hcx`) to translate them into stable equivalents before hashing.

`HashMap` and `HashSet` explicitly do NOT implement `StableHash` because their iteration order is unstable ([`stable_hash.rs:593-594`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_data_structures/src/stable_hash.rs#L593-L594)):

```rust
impl<V> !StableHash for std::collections::HashSet<V> {}
impl<K, V> !StableHash for std::collections::HashMap<K, V> {}
```

## The Hashing Context (`StableHashCtxt`)

[`stable_hash.rs:23-36`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_data_structures/src/stable_hash.rs#L23-L36)

The context passed into `stable_hash` provides two key operations:

- `def_path_hash(def_id)` - converts an unstable `DefId` into its stable `DefPathHash` before feeding it to the hasher.
- `stable_hash_span(span)` - converts a `Span` (which references session-local byte offsets) into a stable representation.

This means that when any type containing a `DefId` implements `StableHash`, it does not hash the `DefId` integer directly - it calls `hcx.def_path_hash(def_id)` and hashes that instead. The context is the translation layer between session-local IDs and stable identifiers.

Dev guide: "whenever something is hashed that might change in between compilation sessions (e.g. a `DefId`), we instead hash its stable equivalent".

## `StableHashControls`

[`stable_hash.rs:623-630`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_data_structures/src/stable_hash.rs#L623-L630)

The context also carries a `StableHashControls` struct (currently just `hash_spans: bool`) that lets callers opt out of hashing certain data. The comment states: "Whenever a `StableHash` implementation caches its result, it needs to include `StableHashControls` as part of the key, to ensure that it does not produce an incorrect result".

This is used when producing fingerprints that should not change just because line numbers shifted.

## The `StableHasher` Algorithm

`StableHasher` wraps `SipHasher128`, which is SipHash 1-3 with a 128-bit output. It is initialized with a fixed key of `(0, 0)` so the output is deterministic across processes and platforms:

```rust
StableHasher { state: SipHasher128::new_with_keys(0, 0) }
```

SipHash was chosen because it is fast (designed for short keys), has good collision resistance for non-cryptographic use, and was already the algorithm behind Rust's standard `HashMap` (so it was well-understood). The 128-bit output reduces collision probability to negligible levels for fingerprinting. Rustc explicitly accepts that a collision would produce an incorrect incremental result and treats it as an acceptable risk given the 2^128 output space.

## Enum Discriminant Stability

For enums, `StableHash` impls hash the discriminant index first, then the variant fields. This means that inserting a new variant in the middle of an enum shifts all following discriminants, invalidating fingerprints for every value of those variants.

Rustc is careful about this. The practical rule is: new variants go at the end, or the enum uses explicit discriminant values so insertion does not shift existing ones.

## Connection to Fingerprints

[`graph.rs:160-167`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/graph.rs#L160-L167), [`graph.rs:483-489`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/graph.rs#L483-L489)

When a query finishes, its return value is fingerprinted via `hash_result`, which calls `stable_hash` with a `StableHashCtxt` backed by `TyCtxt`:

```rust
pub fn hash_result<R: StableHash>(hcx: &mut StableHashState<'_>, result: &R) -> Fingerprint {
    let mut stable_hasher = StableHasher::new();
    result.stable_hash(hcx, &mut stable_hasher);
    stable_hasher.finish()
}
```

This fingerprint is stored in the dep-graph node as the value fingerprint and compared against the stored fingerprint from the previous session to decide green vs red.

Dev guide: "Each time a new query result is computed, the query engine will compute a 128 bit hash value of the result".
