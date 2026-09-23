---
_label: "Modeling Connectivity in Various Data Models"
_icon: icon.document
---

This document compares how relational databases, aggregate NoSQL stores, and graph databases handle connected data.

## Context: Aggregate NoSQL Stores & Graph Databases

Aggregate NoSQL databases that organize data around **aggregates**. An aggregate (from Domain-Driven Design) is a cluster of related data treated as a single unit for storage and retrieval.

For example, an "Order" aggregate bundles the order, its line items, shipping address, and payment info together. In a relational database this would be spread across multiple tables joined by foreign keys, but in an aggregate store the entire order is stored and fetched as one unit.

Three main types:

- **Key-value stores** (e.g. Redis, DynamoDB): Data is a blob associated with a key. The store doesn't care what's inside.
- **Document stores** (e.g. MongoDB, CouchDB): Data is a structured document (JSON/BSON). The store understands the internal structure.
- **Column-family stores** (e.g. Cassandra, HBase): Data grouped by columns rather than rows, optimized for reading specific attributes across many records.

These stores excel at reading/writing a single aggregate, but struggle when querying _across_ aggregates or following relationships between them. That's exactly where graph databases shine.

The fourth NoSQL type is graph databases themselves, which are the opposite: They optimize for relationships rather than aggregates.

## Relational Databases and Relationships

Ironically, relational databases deal poorly with relationships. "Relations" only exist as a means of joining tables via foreign keys: They can't carry semantics, weight, or properties of their own.

Problems:

- Join tables cause accidental complexity.
  - The relational model itself doesn't allow relationships between tables to have properties. Junction tables must be used in this case. However, they are usually sparse and large for graph databases' use cases.
  - Junction tables mix both business and metadata.
  - Example: A simple "Alice is friends with Bob" requires a dedicated table with only foreign keys.
- Queries degrade with depth due to increased joins.
  - "Bob's friends" is cheap (1 join).
  - "Friends-of-friends" needs 4 joins.
  - At 4-6 degrees of depth, cost grows exponentially.
- Reciprocal queries are asymmetric.
  - Some types of queries are cheap, some are not.
  - Example: "What did customer X buy?" is cheap, but "which customers bought product Y?" scans the entire join table.
- Schema is rigid and brittle: Sparse nullable columns and special-case code are required to handle ad-hoc connections. Migrations are costly as the application evolves.

## NoSQL Aggregate Stores and Relationships

Most NoSQL aggregate stores do not store relationships. A **well-known workaround** is to have a field in the aggregates store another's id in it.

Aggregate stores fake relationships by embedding foreign aggregate IDs inside fields (e.g. `order: 1234` inside a user document). This looks graph-like but has fundamental problems:

- Relationships aren't first-class.
  - The application must build and maintain connections manually, the database is agnostic of the relationships, so relationship-involving queries are not well-optimized.
  - ACID may not even hold. If it fails to update/delete references in tandem, dangling references accumulate.
- Forward-only links: References only point one way. "What did Alice order?" is easy (look up her order IDs). "Who bought product X?" requires a brute-force scan of the entire dataset (O(n)).
- Denormalization is expensive: Adding backward links (e.g. `friended_by`) trades write latency and disk for read convenience, and each hop still requires an index lookup (`O(log n)` per hop) since aggregates have no locality.
- Performance still degrades with depth: Immediate friends are feasible. Friends-of-friends or deeper traversals become impractical because every hop is a fake relationship resolved through index lookups, not direct pointers.

### Performance Comparison

| Operation                            | Aggregate Store                      | Graph Database                       |
| ------------------------------------ | ------------------------------------ | ------------------------------------ |
| "Bob's friends" (forward lookup)     | Index lookups per friend             | O(1) per hop, follow direct pointers |
| "Who is friends with Bob?" (reverse) | Brute-force scan, O(n)               | O(1) per hop, follow incoming edges  |
| Friends-of-friends (depth 2+)        | Impractical, each hop = index lookup | O(1) per hop, stays constant         |

The core difference: Graph databases provide **index-free adjacency** (nodes physically point to neighbors). Aggregate stores must resolve every relationship through index lookups or full scans. Many systems try to simulate graph-like processing on aggregate stores, but end up doing it in batch rather than real-time.

## Graph Databases and Relationships

In relational and aggregate stores, connections are **implicit**: The application infers and builds them. In a graph database, **connected data is stored as connected data**. Where there are connections in the domain, there are connections in the data.

### Flexibility

A graph naturally accommodates semi-structured, densely connected domains. New nodes and relationship types can be added without breaking the existing network or migrating data. A social network can mix FRIEND_OF, COLLEAGUE_OF, BOSS_OF, MARRIED_TO, LOVES, and DISLIKES in the same graph with no schema changes.

### Performance

Benchmark from _Neo4j in Action_ (Partner & Vukotic): Finding friends-of-friends in a social network of 1,000,000 people with ~50 friends each:

| Depth | RDBMS (s)  | Neo4j (s) | Records returned |
| ----- | ---------- | --------- | ---------------- |
| 2     | 0.016      | 0.01      | ~2,500           |
| 3     | 30.267     | 0.168     | ~110,000         |
| 4     | 1,543.505  | 1.359     | ~600,000         |
| 5     | Unfinished | 2.132     | ~800,000         |

At depth 2, both are fine. At depth 3, the RDBMS takes 30 seconds (unusable for real-time) while Neo4j takes 168ms. At depth 5, the RDBMS can't finish at all. Both aggregate stores and relational databases fall short when managing connected data: Anything beyond a shallow traversal is slow because of index lookups. Graphs use index-free adjacency to keep traversal fast regardless of depth.

### Multi-Domain Graphs

Graphs are naturally multi-dimensional. Different domains can be joined into a single graph, enabling queries that cross domain boundaries, for example:

- **Purchase history**: User -\[PLACED\]-> Order -\[CONTAINS\]-> Product. A MOST\_RECENT linked list supports browsing order history.
- **Geospatial data**: Modeled using **R-Trees**, graph-like indexes that describe bounded boxes around geographies, forming overlapping location hierarchies (e.g. postal code, district, city, country). This allows "people who live nearby" queries.
- **Recommendations**: Combine all of the above. "Find ice cream flavors liked by people who live near user X, enjoy espresso, but dislike Brussels sprouts" is a single graph traversal joining purchase patterns, social connections, and geospatial data.
- **Fraud detection**: Patterns outside the norm for a given user are easily detected by comparing against graph-wide buying patterns.

These pattern-matching queries are extremely difficult in SQL, laborious against aggregate stores, and perform poorly in both. Graph databases are optimized for exactly this kind of traversal, providing millisecond responses in many cases.
