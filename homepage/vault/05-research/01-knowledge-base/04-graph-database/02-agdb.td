---
_label: "agdb - Agnesoft Graph Database"
_icon: icon.database
---

## What is agdb

[agdb](https://github.com/agnesoft/agdb) is an embeddable, persistent graph database written in Rust. It stores data as a property graph - nodes and edges with typed key-value properties - and exposes queries as native code objects rather than a text-based query language. This makes it "application native": queries are constructed using Rust structs and a builder pattern, with no string parsing or DSL involved.

It can run embedded (as a library crate), as a standalone server, or as a cloud-hosted SaaS.

## Key Properties

- **No query language**: Queries are Rust objects built with `QueryBuilder`. No text parsing, no SQL/Cypher/Gremlin. The same query objects work across all client SDKs (Rust, Python, JS, PHP, etc.).
- **Persistent, platform-agnostic storage**: File-based storage that is transferable between platforms.
- **Memory-mapped**: Optionally memory-mapped for fast reads.
- **ACID compliant**: Transactions with rollback support.
- **Schema-less**: Typed key-value properties on nodes and edges, but no fixed schema. Allows flexible data evolution without migrations.
- **Regex support**: Optional regex matching for text comparisons in queries.
- **Server and cluster modes**: Built-in TLS, OpenAPI clients generated for any language.
- **Derive macro**: `#[derive(UserValue)]` maps Rust structs to/from database records automatically.

## Comparison with Other Graph Databases

| Property | agdb | Neo4j | SurrealDB | Memgraph |
| --- | --- | --- | --- | --- |
| Language | Rust | Java | Rust | C++ |
| Query interface | Native objects (builder) | Cypher (text) | SurrealQL (text) | Cypher (text) |
| Embeddable | Yes (library crate) | No (server only) | Yes | No (server only) |
| Schema | Schema-less (typed KV) | Optional schema | Schema-less | Optional schema |
| ACID | Yes | Yes | Yes | Yes |
| Storage | File-based, memory-mapped | Custom store | RocksDB / memory | In-memory + WAL |
| Cluster | Yes | Yes (Enterprise) | Yes | Yes (Enterprise) |
| Maturity | Young (v0.12, active dev) | Mature (10+ years) | Growing | Growing |
| Ecosystem | Small, Rust-centric | Large, broad language support | Growing | Moderate |

**Key trade-off:** agdb trades query language universality for type safety and zero-parsing overhead. You lose the ability to write ad-hoc queries in a REPL or share queries as text, but gain compile-time checked queries with no serialization cost.

**When to consider agdb over Neo4j/Memgraph:**

- Embedding a graph store inside a Rust application without running a separate server process
- Wanting compile-time query validation instead of runtime string parsing
- Small-to-medium datasets where the JVM overhead of Neo4j is disproportionate

**When NOT to use agdb:**

- Need a mature, battle-tested system for production at scale
- Need ad-hoc querying (analysts writing Cypher in a notebook)
- Need a large ecosystem of tools, visualizers, and integrations

## API

### Setup

```rust
use agdb::{Db, DbId, QueryBuilder, UserValue};

let mut db = Db::new("my_graph.agdb")?;
```

### Define a user type

```rust
#[derive(Debug, UserValue)]
struct Person {
    name: String,
    age: u64,
}
```

### Insert nodes

```rust
// Insert a node with an alias
db.exec_mut(QueryBuilder::insert().nodes().aliases("people").query())?;

// Insert nodes with values
let people = db.exec_mut(
    QueryBuilder::insert()
        .nodes()
        .values(vec![
            Person { name: "Alice".into(), age: 30 },
            Person { name: "Bob".into(), age: 25 },
        ])
        .query()
)?;
```

### Insert edges

```rust
// Connect "people" root to each person node
db.exec_mut(
    QueryBuilder::insert()
        .edges()
        .from("people")
        .to(&people)
        .query()
)?;
```

### Query nodes

```rust
// Select all values from inserted nodes
let result = db.exec(
    QueryBuilder::select()
        .values(Person::db_keys())
        .ids(&people)
        .query()
)?;

// Deserialize back to Rust structs
let persons: Vec<Person> = result.try_into()?;
```

### Search with conditions

```rust
use agdb::{Comparison, CountComparison};

// Find people older than 28
let result = db.exec(
    QueryBuilder::select()
        .values(Person::db_keys())
        .ids(
            QueryBuilder::search()
                .from("people")
                .where_()
                .key("age")
                .value(Comparison::GreaterThan(28.into()))
                .query()
        )
        .query()
)?;
```

### Search with regex

```rust
// Find people whose name matches a pattern
let result = db.exec(
    QueryBuilder::select()
        .values(Person::db_keys())
        .ids(
            QueryBuilder::search()
                .from("people")
                .where_()
                .key("name")
                .regex("^A.*")
                .query()
        )
        .query()
)?;
```

### Remove nodes

```rust
db.exec_mut(
    QueryBuilder::remove()
        .ids(&people)
        .query()
)?;
```

### Transactions

```rust
// Multiple mutations in a single transaction (all-or-nothing)
db.transaction_mut(|transaction| -> Result<(), anyhow::Error> {
    transaction.exec_mut(
        QueryBuilder::insert().nodes().aliases("root").query()
    )?;
    transaction.exec_mut(
        QueryBuilder::insert()
            .nodes()
            .values(vec![Person { name: "Eve".into(), age: 22 }])
            .query()
    )?;
    Ok(())
})?;
```

## Sources

- [agdb GitHub repository](https://github.com/agnesoft/agdb)
- [agdb on crates.io](https://lib.rs/crates/agdb)
- [agdb API docs](https://docs.rs/agdb)
