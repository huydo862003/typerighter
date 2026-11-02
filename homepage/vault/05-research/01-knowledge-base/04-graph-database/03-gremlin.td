---
_label: "Gremlin - Graph Traversal Language"
_icon: icon.hourglass
---

## What is Gremlin

Gremlin is the graph traversal language of [Apache TinkerPop](https://tinkerpop.apache.org/). A Gremlin query is a step-by-step description of how to walk a property graph - each step in the chain either moves the traverser, transforms results, filters, or branches. This makes it fundamentally different from declarative languages like Cypher, which describe *what* pattern to match rather than *how* to traverse.

Gremlin is not tied to a single database. Any TinkerPop-enabled system speaks Gremlin, including JanusGraph, Amazon Neptune, Azure Cosmos DB, DataStax Enterprise, and others.

## Key Properties

- **Imperative/traversal-centric.** Queries are method chains that describe step-by-step graph walks.
- **Vendor-neutral.** Runs on any TinkerPop-enabled graph system - switch backends without changing queries.
- **Turing complete.** Supports loops (`repeat`), branching (`choose`, `coalesce`), recursion, and arbitrary computation during traversal.
- **Language variants.** Native bindings for JVM (Java/Groovy), Python, JavaScript, .NET, Go. Queries look idiomatic in each language.
- **OLTP and OLAP.** Same traversal language works for real-time queries (OLTP) and batch analytics (OLAP via Spark/Flink).

## Gremlin vs Cypher

| Aspect | Gremlin | Cypher |
| --- | --- | --- |
| Paradigm | Imperative (traversal steps) | Declarative (pattern matching) |
| Origin | Apache TinkerPop | Neo4j (now openCypher/GQL) |
| Query shape | Method chain: `g.V().out().values()` | ASCII art: `(a)-[:KNOWS]->(b)` |
| Write ops | Inline mutation steps: `addV`, `addE`, `property` | Dedicated clauses: `CREATE`, `MERGE`, `SET` |
| Looping | `repeat().until()` / `repeat().times()` | `*` in variable-length paths (limited) |
| Backends | JanusGraph, Neptune, Cosmos DB, DataStax, etc. | Neo4j, Memgraph, Neptune (openCypher) |
| Strengths | Complex multi-hop traversals, algorithmic queries | Pattern matching, readability, ad-hoc exploration |
| Weaknesses | Verbose for simple pattern matches | Limited procedural control flow |

**Rule of thumb:** If the query is "find all X that look like this pattern", Cypher is more natural. If the query is "starting here, walk the graph this way, accumulating state", Gremlin is more natural.

## Supported Databases

- **JanusGraph** - open source, distributed, built on TinkerPop
- **Amazon Neptune** - managed, supports both Gremlin and openCypher
- **Azure Cosmos DB** - Gremlin API mode on Microsoft's multi-model database
- **DataStax Enterprise** - Cassandra-backed graph with TinkerPop support
- **PuppyGraph** - query engine that adds Gremlin over existing data stores
- **TinkerGraph** - in-memory reference implementation (bundled with TinkerPop)

## API

### Traversal source

All queries start from `g`, the traversal source bound to a graph:

```groovy
g = traversal().withRemote('conf/remote-graph.properties')
```

### Add vertices and edges

```groovy
// Add vertices with labels and properties
g.addV('person').property('name', 'Alice').property('age', 30).as('alice').
  addV('person').property('name', 'Bob').property('age', 25).as('bob').
  addE('knows').from('alice').to('bob')
```

### Basic traversal

```groovy
// Get names of all people Alice knows
g.V().has('person', 'name', 'Alice').out('knows').values('name')
// => Bob

// Get names of friends-of-friends
g.V().has('name', 'Alice').out('knows').out('knows').values('name')
```

### Filter steps

```groovy
// People older than 28
g.V().hasLabel('person').has('age', gt(28)).values('name')

// People whose name starts with 'A'
g.V().hasLabel('person').has('name', TextP.startingWith('A')).values('name')
```

### Path traversal

```groovy
// Show the full path from Alice to any vertex within 3 hops
g.V().has('name', 'Alice').repeat(out()).times(3).path().by('name')
```

### Repeat / until (loops)

```groovy
// Find shortest path from Alice to someone named 'Eve'
g.V().has('name', 'Alice').
  repeat(out('knows')).
  until(has('name', 'Eve')).
  path().by('name')

// Traverse exactly 2 levels deep
g.V().has('name', 'Alice').repeat(out()).times(2).values('name')
```

### Aggregation

```groovy
// Count vertices by label
g.V().groupCount().by(label)
// => {person: 4, project: 2}

// Group people by age
g.V().hasLabel('person').group().by('age').by('name')
```

### Subgraph / project

```groovy
// Project specific fields
g.V().hasLabel('person').
  project('name', 'friend_count').
    by('name').
    by(out('knows').count())
// => [{name: Alice, friend_count: 3}, ...]
```

### Mutations

```groovy
// Update a property
g.V().has('name', 'Alice').property('age', 31)

// Remove an edge
g.V().has('name', 'Alice').outE('knows').where(inV().has('name', 'Bob')).drop()

// Remove a vertex and all its edges
g.V().has('name', 'Bob').drop()
```

### Branch / choose

```groovy
// Conditional traversal
g.V().hasLabel('person').
  choose(has('age', gt(30))).
    option(true, values('name')).
    option(false, constant('young'))
```

### Practical example: recommendation query

```groovy
// "People who know Alice's friends but Alice doesn't know directly"
g.V().has('name', 'Alice').as('alice').
  out('knows').out('knows').
  where(neq('alice')).
  where(without('alice_friends')).
  dedup().
  values('name')
```

## Sources

- [Apache TinkerPop - Gremlin](https://tinkerpop.apache.org/gremlin.html)
- [TinkerPop Reference Documentation](https://tinkerpop.apache.org/docs/current/reference/)
- [The Gremlin Console Tutorial](https://tinkerpop.apache.org/docs/current/tutorials/the-gremlin-console/)
- [Gremlin vs Cypher comparison](https://www.puppygraph.com/blog/gremlin-vs-cypher)
