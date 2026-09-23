---
_label: "Incremental Compilation"
_icon: icon.document
---

Link: https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html

They remark that:

> **The incremental compilation scheme** is, in essence, a **surprisingly simple extension** to the overall **query system**. It relies on the fact that:
>
> 1. Queries are pure function - given the same inputs, a query will always yield the same result, and
> 2. The query model structures compilation in an acyclic graph that makes dependencies between individual computations explicit.

They briefly discuss the **red-green algorithm** for incremental query system here: https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html#improving-accuracy-the-red-green-algorithm.

However, I want to focus on **persistence** of incremental computation here. **Persistence** means saving the query result cache to disk so it survives across compiler invocations. Without it, incremental compilation only helps within a single compiler process - once the process exits, **all cached results are gone** and the next build starts from scratch. Persistence is what makes incremental compilation **useful in practice**: the next `cargo build` can read the cache from disk and **skip recomputing queries whose inputs have not changed**.

## Persistence Challenges

The sections above describe the underlying algorithm, however, another thing need to be addressed: **Persistence across compilation**.

- The compiler process exits when it finishes - taking the query context and its result cache with it. -> Another compilation round will have to compute from scratch.
- To make incremental compilation useful across sessions, that data must be persisted to disk, which introduces a whole new set of challenges:

**Persistence** introduces further challenges:

1. The query result cache lives on disk, so it is not readily available for change comparison when the next session starts.
2. A subsequent session starts with a new version of the code that may have arbitrary changes applied. All kinds of IDs and indices generated from a global sequential counter (e.g. `NodeId`, `DefId`) might have shifted, **making the persisted results not immediately usable because the same numeric ID might now refer to a completely different thing**.
   > Remark: I think this is the primary problem.
3. **Persisting to disk is not free**, so **not every piece of information should be cached between sessions**.
   > It's desirable that **fixed-sized, plain-old-data is preferred over complex structures** that require an expensive serialization and deserialization step.

## Stable Identifiers: Bridging the Gap Between Sessions

The **ID instability problem** (challenge 2 above) is the primary one.

**Example**: Imagine a file with two functions, `foo` and `bar`.

1. In **session 1**, these two functions are assigned `DefId(0)` and `DefId(1)` accordingly.
2. The compiler caches the result of `type_of(DefId(1))` as `i32`.
3. In **session 2**, the user inserts a new function before `foo`, so the counter shifts: `foo` becomes `DefId(1)` and `bar` becomes `DefId(2)`.

**Outcome**: The cached entry for `DefId(1)` now points to `foo`, not `bar` - the persisted result is silently wrong.

**Solution**:

- The compiler **should never persist** **numeric IDs** directly.
- Instead, the compiler uses **stable identifiers** derived from the hierarchical structure of the source code rather than from a global counter.
  > I recall that in AML, FQN (fully qualified name) is used for this purpose.

Concretely, in `rustc`:

- Instead of `DefId`s, `DefPath`s, which are based on the path to the identified item, are used instead.
  - **`DefPath`**: A path-based representation of a definition (e.g. `std::collections::HashMap`).
  - Adding a new function in the middle of a file does not change the `DefPath` of existing definitions, so cached data keyed by `DefPath` remains valid.
- **`DefPathHash`**: A 128-bit hash of the `DefPath`. It is `Copy` and cheap to compare, making it the practical unit of stable identity stored on disk.
  - On the next compilation session, the compiler deserializes cached data keyed by `DefPathHash` and maps it back to the current session's `DefId` values via a hash table lookup. This way, the cached results remain usable despite arbitrary source changes.
- **`HirId`**: Not every HIR node is a definition-level item, so a `DefId` alone is not enough to identify sub-items within one owner. `HirId` pairs a `DefPath` with a `LocalId` that identifies something locally within its owner, so internal identifiers remain stable even if the owning item moves in the file.

## `Fingerprints` and `StableHash`: Change Detection from Disk Without Full Deserialization

Red-green marking requires knowing **whether a query result has changed since the previous session** (backdating). Doing this naively has two problems:

1. **Loading old results just to compare them is wasteful**.
   - The new result is already computed in memory: Loading the old one from disk also pollutes interners with data that will likely never be used again.
     -> Deserializing the old one just to throw it away after comparison is extra work.
   - Deserialization touches interners: structures that deduplicate strings, paths, and symbols by storing them in a shared pool.
     -> Unnecessarily bloat the interners.
     -> Loading stale data from a previous session fills those pools with entries that belong to the old session and will almost certainly never be referenced again, wasting memory.
2. **Not every result should be stored on disk**.
   - If a query result comes from an upstream crate compiled separately, it is already encoded in that crate's compiled artifact.
   - Caching it again in the local incremental store is redundant, as the next session can read it from the crate directly.
   - Storing it twice wastes both disk space and the time spent serializing and deserializing it.

**Solution**: The compiler avoids both problems with **Fingerprints**.

- **Fingerprint**: a 128-bit hash computed over each query result.
- Idea:
  - Instead of storing the full result, **only its fingerprint is stored alongside the dependency graph**.
    -> Since fingerprints are just bytes, loading the full set is cheap.
  - When red-green marking needs to check if a result changed, it compares the **already-loaded previous fingerprint** against **the fingerprint of the newly computed result**. No old result needs to be deserialized.
  - NOTE: The hashing must be done in a **stable way**: anything that could shift between sessions (e.g. a `DefId`) is replaced with its stable equivalent (e.g. the corresponding `DefPath`) before hashing. This is what the `StableHash` infrastructure provides, ensuring fingerprints from two different sessions are still comparable.

This approach has two known trade-offs:

1. **Hash collisions**: Two different results could produce the same fingerprint, causing the system to miss an update. The risk is mitigated by using a high-quality hash function with a 128-bit output, making collisions negligible in practice.
   > Remark: So `rustc` just accepts wrong compilation?
2. **Fingerprint computation is expensive**: It is the primary reason incremental compilation can be _slower_ than non-incremental compilation - a high-quality hash function is required, and all identifiers must be mapped to their stable equivalents during hashing.

## Two DepGraphs: Old and New

Dependency tracking involves two graphs at once: the one built during the previous session (loaded from disk, immutable) and the one being built for the current session.

When a query is invoked, the compiler first tries to mark the corresponding node green in the **old graph**. The mapping from a current query key to a node in the old graph is done via fingerprints: each dep-node is identified by a fingerprint of its query key. Since fingerprints are stable across sessions, computing one in the current session lets the compiler locate the matching node in the old graph. If no node with that fingerprint exists, the query key refers to something that did not exist in the previous session.

Once a node is successfully marked green, it is **copied from the old graph into the new graph**, along with its edges. This copy step is necessary because the normal dependency tracking system can only record edges by actually running a query, but running the query is exactly what we want to avoid when the result is already cached.

At the end of the session:

- Unchanged nodes have been copied from the old graph into the new one.
- Changed nodes have been added to the new graph by the tracking system as their queries were re-executed.

The new graph is then serialized to disk alongside the query result cache, ready to serve as the old graph in the next session.

## Cache Promotion

The system has a subtle property: if all inputs of a dep-node are green, the node itself can be marked green without loading its cached result from disk. Applied transitively, this means intermediate results are often never loaded. Consider:

```
input(A) <-- intermediate_query(B) <-- leaf_query(C)
```

If `leaf_query(C)` is needed, the compiler loads its result from cache. `intermediate_query(B)` is never loaded. When the compiler writes the new result cache at the end of the session, only in-memory results are written, so `intermediate_query(B)` goes missing from the new cache.

The next session that actually needs `intermediate_query(B)` will have to recompute it, even though a valid cached result existed just before.

To prevent the result cache from silently shrinking, the compiler performs **cache promotion**: before writing the new cache, it walks all green dep-nodes and ensures their query results are loaded into memory, so they are included in the serialized output.
