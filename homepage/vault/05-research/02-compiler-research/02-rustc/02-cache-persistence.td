---
_label: "Cache Persistence"
_icon: icon.document
---

This document covers how `rustc` physically organizes, loads, and saves its incremental compilation cache across sessions. It complements [Incremental Compilation in Detail](01-incremental-compilation), which covers the conceptual algorithm.

Sources:

- Rustc dev guide: [Incremental Compilation in Detail](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html)
- [`compiler/rustc_incremental/src/persist/fs.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs) - session directory management
- [`compiler/rustc_incremental/src/persist/load.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/load.rs) - cache loading
- [`compiler/rustc_incremental/src/persist/save.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/save.rs) - cache writing
- [`compiler/rustc_middle/src/dep_graph/serialized.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/serialized.rs) - dep-graph format
- [`compiler/rustc_middle/src/query/on_disk_cache.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/query/on_disk_cache.rs) - query-cache format

## Physical Organization

- In `rustc`, the **root cache directory** is specified via the `-C incremental=<path>` flag passed to `rustc`.
- When using Cargo, this is managed automatically - set via `incremental = true/false` in `Cargo.toml` under `[profile.*]`, and the cache is placed under `target/{profile}/incremental/`.

Inside the **cache directory**, the layout is ([`fs.rs:224-246`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs#L224-L246)):

```
<incr-comp-dir or root-cache-dir>/
  {crate-name-and-disambiguator}/     <- one per crate
    s-{timestamp}-{random}-working/   <- active session (in progress)
    s-{timestamp}-{SVH}/              <- finalized session (previous runs)
```

> Note that **on Windows**, [rustc uses a hack to "insert a weird prefix that makes in windows tolerate long paths"](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs#L229-L232). It seems that this prefix is `\\?\`?

### Directory of a Compilation Session

A **compilation session** is one invocation of the compiler on a crate.

- Each session gets its own subdirectory named `s-{timestamp}-{random}-working` while active.
- On successful completion it is renamed to `s-{timestamp}-{SVH}`, where SVH (Stable Version Hash) is a hash of the crate's content.
- If the compiler crashes, the `-working` directory is left behind and cleaned up on the next run.

Sessions follow a **copy-on-write** protocol ([`fs.rs:4-96`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs#L4-L96)):

- A new session starts as a `-working` directory, populated by hard-linking files from the previous finalized session. Only files that change are rewritten.
- On success, the `-working` directory is renamed to a finalized session directory (named with the crate's stable version hash).
- Multiple compiler processes can run concurrently for the same crate without interfering - each works on its own private copy.

### Picking a Source Directory to Reuse

At startup, the compiler scans the crate's directory for finalized session directories and picks the **most recently timestamped** one ([`fs.rs:518-557`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs#L518-L557)). The timestamp is embedded in the directory name (`s-{timestamp}-{SVH}`), so no stat calls are needed - it's just a string comparison over directory entries.

The SVH in the directory name is **not used for selection** - only the timestamp matters. Whether the selected session's cache is actually usable is determined later, when the dep-graph is loaded and its header is checked (compiler version + commandline args hash).

If the selected directory cannot be copied from (e.g. deleted mid-copy by a concurrent garbage collector), the compiler falls back to the next most recent candidate and retries. If no finalized session exists at all, it starts with an empty cache.

**Pitfalls**:

- **Clock skew**: if the system clock goes backwards (NTP adjustment, VM migration), a newer session can get an older timestamp and be bypassed in favor of a genuinely older one. The compiler has no defense against this.
- **Optimistic copy**: the source directory is selected and hard-linked before the dep-graph header is checked. If the cache turns out to be invalid (wrong compiler version, changed flags), the hard-linking was wasted work. Hard-linking is cheap on most filesystems, but on filesystems without hard link support (e.g. FAT), the files are fully copied instead - making an invalid cache selection genuinely costly.

### Lock Files and Crash Detection

Each session directory has a **lock file** that the compiler holds an exclusive lock on for the duration of the session. The lock serves two purposes:

1. **Crash detection** ([`fs.rs:55-70`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs#L55-L70)): a crashed process cannot run cleanup code, so stale `-working` directories cannot be deleted by the process that created them. Instead, on each startup, `garbage_collect_session_directories` scans for leftover `-working` directories and tries to acquire an exclusive lock on each one.

- Lock fails -> the owning process is still alive, leave it alone.
- Lock succeeds -> the owning process is dead, safe to delete.

OS-level file locks are automatically released when a process dies (crash, OOM, kill), so this works reliably without any explicit crash handling. To avoid a race between a process creating the lock file and actually locking it, only directories older than a few seconds are considered for garbage collection.

2. **Reader/writer coordination**: when one process wants to copy a finalized session directory (to start a new session from it), it acquires a **shared** lock on that directory's lock file. Any garbage-collecting process acquires an **exclusive** lock. This prevents a collector from deleting a directory that another process is actively copying from.

Each session directory contains three binary files (`.bin` signals raw binary format, not text or JSON):

- `dep-graph.bin` - the dependency graph (nodes, edges, fingerprints)
- `query-cache.bin` - cached query result values
- `work-products.bin` - CGU artifact registry

Some noteworthy points:

- A dummy lockfile is created solely for representing folder synchronization.
  > Locking a whole directory is unreliably, because the lock API requires calling the file lock API on an open file descriptor. However, opening a directory is unstable on Linux.
- "There is still a small time window between the original process creating the lock file and actually locking it. In order to minimize the chance that another process tries to acquire the lock in just that instance, only session directories that are older than a few seconds are considered for garbage collection".

### Dependency and Change Tracking (`dep-graph`)

[`serialized.rs:1-40`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/serialized.rs#L1-L40) - the module comment describes the format in full.

Stores the dependency graph: nodes, edges, and fingerprints. No query result data is stored here - only the graph structure and hashes needed for change detection. Fully decoded upfront into memory on load - no random access into the file itself.

Each node contains:

- `DepKind` - which query this node represents.
- Key fingerprint - a stable hash of the query key (e.g. a `DefPathHash`). This serves as the node's stable identity across sessions - no numeric ID is persisted.
- Value fingerprint - a hash of the query's return value, used to detect whether the result changed since the previous session.
- Edges - indices of the nodes this node depends on.

```
[ file header                   ]  <- compiler version + commandline args hash
[ NodeInfo, NodeInfo, NodeInfo  ]  <- streamed node records, written as queries finish
[ ...                           ]
[ total_node_count: u64         ]  <- last 16 bytes, appended after all nodes are written
[ total_edge_count: u64         ]
```

> The counts are at the end rather than the start because nodes are streamed to disk as queries finish - the total is not known until writing is complete. Putting them first would require buffering the entire graph in memory or a second pass to rewrite the header.

Each node record corresponds to one `NodeInfo` (a dep-node plus its value fingerprint and edges). On disk it is split into two parts:

- A fixed-size `SerializedNodeHeader` ([`serialized.rs:432`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/serialized.rs#L432)): 38 bytes packed as `DepKind` discriminant (2 bytes) + key fingerprint (16 bytes) + value fingerprint (16 bytes) + edge count and edge byte-width metadata (remaining bits).
- A variable-length edge list immediately following the header.

The total node/edge counts at the end of the file allow the decoder to pre-allocate arrays upfront before reading any records ([`serialized.rs:300-346`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/serialized.rs#L300-L346)). The dep-graph file is then read sequentially from start to end - there is no random access into individual node records. All lookups after load go through the in-memory `SerializedDepGraph` arrays.

### Computed Result Store (`query-cache`)

[`on_disk_cache.rs:52-116`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/query/on_disk_cache.rs#L52-L116)

Stores the serialized return values of cached queries. On load, only the footer index is decoded into memory; result blobs are accessed lazily by offset via `mmap`. Values and their lookup index are kept in separate regions of the file:

```
[ query result blob ]  <- raw serialized value for some query
[ query result blob ]  <- raw serialized value for another query
[ ...               ]
[ Footer            ]  <- index: maps each dep-node -> byte offset of its result blob
[ footer_pos: u64   ]  <- last 8 bytes: byte offset of the Footer
```

On load, the compiler reads the last 8 bytes to find the footer position, jumps to it, and decodes the full index into memory ([`on_disk_cache.rs:159-165`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/query/on_disk_cache.rs#L159-L165)). Individual result blobs are then accessed directly by offset on demand.

### Compiled Artifact Registry (`work-products`)

Stores the set of compiled artifact files produced per CGU. Fully loaded upfront on every startup - the list is small (one entry per CGU) and always needed in full. Simple sequential encoding with no index.

Each entry maps a `WorkProductId` (stable hash of the CGU name) to a `WorkProduct` (the artifact file paths for that CGU, e.g. `"o" -> "/path/to/foo.o"`), allowing the compiler to look up "given this CGU, where are its compiled files from last session?" ([`graph.rs:1110-1135`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/graph.rs#L1110-L1135)).

On an incremental rebuild, if the CGU's dep-node is green and the symbol hash matches, the saved files are reused directly. Otherwise the CGU is recompiled and a new entry is written.

## Load Algorithm

[`load.rs:40-225`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/load.rs#L40-L225)

At startup, the compiler:

1. Scans the crate's directory for finalized session directories (`s-{timestamp}-{SVH}`) and picks the most recently timestamped one as the source ([`fs.rs:518-557`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs#L518-L557)).
   - If the copy fails (e.g. a concurrent garbage collector deleted the directory mid-copy), it retries with the next most recent candidate.
   - If no finalized session exists, it starts with an empty cache and skips to step 4.
2. Hard-links files from the selected source directory into the new `-working` directory. Only files that change during this session will be rewritten.
3. Reads `dep-graph.bin` from the hard-linked copy.
   - Checks the file header: if the compiler version or command-line arguments hash has changed, the entire cache is discarded.
   - If valid, fully decodes it upfront into a `SerializedDepGraph` ([`serialized.rs:99-126`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/serialized.rs#L99-L126)): flat arrays of nodes, value fingerprints, and edges held in memory. It is wrapped in an `Arc` and never mutated after construction. Only fingerprints and indices are stored - no query result values.
4. Reads `work-products.bin` fully upfront. The list is small (one entry per CGU) and always needed in full, so no index or lazy loading is used. Entries whose artifact files no longer exist on disk are pruned.
5. Memory-maps `query-cache.bin` but only decodes its footer index (dep-node -> byte offset) into memory. The actual result blobs remain in the mmap and are decoded lazily on demand by offset.

At this point, two dep-graphs coexist in memory:

- `previous` - the `SerializedDepGraph` decoded from disk: flat arrays of nodes, value fingerprints, and edges. Used for fingerprint lookups and color marking. Never mutated.
- `current` - a new empty graph being built for this session, populated as queries execute.

They are **never merged**. Green nodes from the previous graph are copied into the current one as they are marked.

## Write Algorithm

Dev guide: "Once the compilation session has finished...the new graph is serialized out to disk, alongside the query result cache".

The dep-graph is streamed to a staging file (`dep-graph.part.bin`) as each query finishes ([`save.rs`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/save.rs)).

At the end of the session:

1. **Cache promotion** runs first: walks all green dep-nodes and loads any query results not yet in memory.
   - Dev guide: "Before emitting the new result cache it will walk all green dep-nodes and make sure that their query result is loaded into memory".
   - Without this, those results silently disappear from the new cache, causing unnecessary recomputation next session.
2. The query-cache is written out with all in-memory results.
3. The `-working` directory is renamed to a finalized session directory.

If the compiler exits with errors, the `-working` directory is left unfinalized and cleaned up on the next run.

## Garbage Collection

Two levels of GC run on each startup ([`fs.rs:613`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs#L613)):

**Session directories**: `garbage_collect_session_directories` scans the crate directory and keeps only the most recent finalized session directory. All older finalized directories that can be exclusively locked are deleted via `remove_dir_all` (which removes the `.bin` files inside). Stale `-working` directories older than ~10 seconds that can be exclusively locked are also deleted.

**Artifact files**: during load, any work product entry whose artifact files are missing is passed to `delete_workproduct_files`, which deletes whatever artifact files still exist for that entry and drops the entry from the in-memory map ([`load.rs:35-38`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/load.rs#L35-L38)).

## IO Techniques

Concrete techniques observed in the source code.

**Hard-linking unchanged files** ([`fs.rs:24-26`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/fs.rs#L24-L26)): when starting a new session, the previous session's files are hard-linked into the new `-working` directory. Only files that are actually rewritten get new disk writes.

**Streaming dep-graph writes** ([`serialized.rs:565-984`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/serialized.rs#L565-L984)): each dep-graph node is written to disk as its query completes - no in-memory accumulation of the full graph.

- Nodes are streamed into `dep-graph.part.bin` during the session. The `.part` suffix signals the file is incomplete. At the end of the session it is renamed to `dep-graph.bin` once all nodes and the trailing counts are written.
  > Remark: This is probably to avoid unnecessary memory usage/recomputation at the end?

**mmap for cache reads** ([`load.rs:91-100`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/load.rs#L91-L100), [`on_disk_cache.rs:152`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/query/on_disk_cache.rs#L152)): both the dep-graph and query-cache files are memory-mapped via `Mmap`. The compiler does not read them into a heap buffer.

**Footer-based random access** ([`on_disk_cache.rs:159-165`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/query/on_disk_cache.rs#L159-L165)): the query-cache footer (an index of `dep-node -> byte offset`) is decoded once on startup. Individual results are then read by seeking directly to their offset.

**Deferred reverse index construction** ([`serialized.rs:159-166`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_middle/src/dep_graph/serialized.rs#L159-L166)): after loading the dep-graph, the per-`DepKind` reverse index (`LazyNodeIndex`) is not built upfront. Each kind's index is constructed on first lookup via `OnceLock`.

## Pitfalls

**The cache can silently shrink** without cache promotion. Intermediate query results that are green but never accessed during a session are not in memory when the new cache is written, so they are dropped. The next session that needs them recomputes from scratch.

**Whole-cache invalidation happens more often than expected**. Any change to the compiler version or command-line arguments discards the entire cache - not just affected nodes. This is intentional and conservative: the file header check rejects mismatches at the binary level.

**Artifact files can go missing independently of the dep-graph**. A work-product entry can survive while its `.o` file has been deleted (e.g. by `cargo clean` on a specific target). The compiler handles this by checking file existence on load and pruning stale entries ([`load.rs:56-88`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_incremental/src/persist/load.rs#L56-L88)). However, this check is not atomic - if a file is deleted after the existence check but before the backend copies it out during codegen, `link_or_copy` returns an error. For non-object files this is a compiler error; for `.o` files it is a fatal abort (`emit_fatal`) with no fallback to recompiling the CGU ([`write.rs:895-944`](https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_codegen_ssa/src/back/write.rs#L895-L944)).

**The query-cache does not store upstream crate results**. Results from external crates live in their compiled `.rmeta` artifacts and are read directly from there. Only results computed in the local crate are persisted in the local cache.
