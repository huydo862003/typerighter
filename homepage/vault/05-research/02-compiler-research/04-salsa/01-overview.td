---
_label: "Overview"
_icon: icon.document
---

Official site: [Link](https://salsa-rs.github.io/salsa/).

## What is Salsa?

A Rust framework for writing incremental, on-demand programs:

- Adapts to input changes while maintaining up-to-date outputs.
- Originally developed from incremental recompilation techniques in `rustc`.
- Tracks dependencies and memoizes results for efficient recomputation.
- Primary use case: Compilers and language tooling responding to source code modifications without full recompilation.

## Core Concepts

### Queries

The fundamental mechanism in Salsa:

- Functions whose results are cached (memoized) and tracked for dependencies.

Two types of queries:

1. Input queries: Values provided from outside the system. When these change, dependent computations are invalidated.
2. Derived queries: Computed from other queries (inputs or derived). Results are cached and recomputed only when dependencies change.

### Database

The central runtime that manages query execution, dependency tracking, and memoization. All queries execute through the database, which coordinates:

- Active function tracking.
- Dependency recording.
- Cache invalidation.
- Result storage.

### Tracked Structures

Salsa manages tracked and interned structs:

- Maintain state throughout computations.
- Integrate with the dependency tracking system.
- Enable Salsa to know when they change and which computations depend on them.

## How It Works

### Dependency Tracking

When a query executes:

- Salsa automatically records which other queries it reads.
- Creates a dependency graph.

When an input changes:

1. Salsa marks affected queries as "potentially stale".
2. On next access, performs a "maybe changed after" check.
3. Only recomputes if dependencies actually changed.
4. Reuses cached results when possible.

### Memoization

Query results are cached with their dependencies. Salsa stores:

- The computed result.
- Which queries were read during computation.
- The "revision" when the computation occurred.

### Incremental Updates

The framework uses a revision counter that increments when inputs change.

For each cached result, Salsa can determine:

- Was this result computed before or after the last change?
- Have any of its dependencies changed since computation?
- Can we reuse the cached value?

Result: Efficient incremental updates where only affected computations run.

### Cycle Handling

When circular dependencies occur (Query A depends on B, B depends on A):

- Salsa detects the cycle.
- Provides mechanisms to handle it.
- Prevents infinite loops.

## Example Workflow

```rust
// Input query - set from outside
#[salsa::input]
struct SourceText {
    #[return_ref]
    text: String,
}

// Derived query - computed from inputs
#[salsa::tracked]
fn parse(db: &dyn Db, input: SourceText) -> Ast {
    // Parse the text
    // Salsa automatically tracks that this depends on `input.text()`
}

#[salsa::tracked]
fn check_types(db: &dyn Db, input: SourceText) -> TypeCheckResult {
    let ast = parse(db, input);  // Depends on parse
    // Type check the ast
}

// Later, when input changes:
// 1. Set new input: input.set_text(db).to(new_text)
// 2. Access check_types(db, input)
// 3. Salsa sees parse() needs recompute (input changed)
// 4. Salsa recomputes parse(), then check_types()
// 5. If parse() result unchanged, check_types() reuses cache
```

## Key Features

### On-Demand Evaluation

- Queries computed lazily.
- If no one asks for a result, it doesn't get computed.
- Contrasts with systems that eagerly recompute everything after changes.

### Durability

- Queries can be marked as "durable" if their inputs rarely change.
- Salsa uses durability levels to optimize invalidation checks.

### LRU Caching

- Memoized results can be evicted using LRU (Least Recently Used) strategy.
- Manages memory by evicting old results.
- Results recomputed if accessed after eviction.

### Parallel Execution

- Multiple queries execute concurrently.
- Salsa coordinates access and handles race conditions through its runtime.

### Cancellation

- Long-running queries cancelled if inputs change during execution.
- Avoids wasted computation.

## Runtime Architecture

The Salsa runtime manages:

- Active function tracking: Which queries are currently executing.
- Revision management: Monotonic counter for tracking change epochs.
- Dependency graph: Records which queries depend on which.
- Memoization storage: Cached results indexed by query + inputs.
- Worker coordination: Handles concurrent query execution.

## Advantages

1. Automatic dependency tracking: No manual specification of what depends on what.
2. Fine-grained invalidation: Only recompute what actually changed.
3. Transparent caching: Memoization handled by framework, not user code.
4. Parallel-friendly: Built-in support for concurrent execution.
5. Memory efficient: LRU eviction prevents unbounded cache growth.

## Integration with `rust-analyzer`

`rust-analyzer` uses Salsa in its `base-db` crate for incremental compilation.

Example uses:

- Input queries: File contents, crate graph configuration.
- Derived queries: Parse trees, name resolution, type inference.

Benefits:

- When a file changes, only affected analysis passes rerun.
- Editing one function: type checking other functions reuses cached results.
- Responsive even on large codebases.
- Most edits trigger minimal recomputation.

## Comparison: Reactive vs Traditional

Traditional approach:
```
File change -> Reparse entire project -> Recheck all types -> Update all diagnostics
```

Salsa approach:
```
File change -> Mark queries as potentially stale
User requests diagnostics -> Salsa recomputes only affected queries -> Return result
```

The Salsa approach is faster because:
- Only computes what's actually needed (on-demand).
- Reuses unchanged intermediate results (memoization).
- Granular invalidation (file-level, not project-level).

## Learning Resources

- [Tutorial: calc language](https://salsa-rs.github.io/salsa/tutorial.html): End-to-end example building a compiler/interpreter
- [Plumbing documentation](https://salsa-rs.github.io/salsa/plumbing.html): Internal mechanisms and algorithms
- [Community](https://salsa.zulipchat.com): Zulip instance for discussions

## Summary

Salsa provides incremental computation through:

1. Query-based architecture: All computations are queries that depend on other queries.
2. Automatic dependency tracking: Framework records dependencies during execution.
3. Smart invalidation: Only recomputes when dependencies actually change.
4. Transparent memoization: Caches results without explicit cache management.
5. On-demand evaluation: Computes only what's requested, when requested.

Result:

- Responsive compilers and language servers.
- Efficient change handling.
- Real-time feedback practical even for large codebases.
