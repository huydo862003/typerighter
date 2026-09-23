---
_label: "Neo4j Internals: Native Graph Processing"
_icon: icon.document
---

Part of the Neo4j internals series: **Native Graph Processing** | [Native Graph Storage](./12-native-graph-storage.md) | [Transactions, Availability, Scale](./13-internals-transactions-availability-scale.md)

These sections peek under the hood of graph database implementation, using Neo4j as the reference. Neo4j has both native graph processing and [native graph storage](./4-concept-graph-database.md), and is open source.

A graph database has **native graph processing** if it uses **index-free adjacency**: Each node maintains direct references to its adjacent nodes, acting as a micro-index of nearby nodes.

## Index-Free Adjacency vs Global Indexes

|                              | Native (index-free adjacency)          | Non-native (global indexes)                      |
| ---------------------------- | -------------------------------------- | ------------------------------------------------ |
| "Alice's friends"            | O(1) per hop, follow pointers          | O(log n) per hop, index lookup                   |
| "Who is friends with Alice?" | O(1) per hop, follow incoming pointers | O(m log n), scan index for each potential friend |
| m-step traversal             | O(m), independent of graph size        | O(m log n), degrades with graph size             |

With index-free adjacency, bidirectional joins are effectively **precomputed** and stored as relationships. No index lookup needed. This is why graph databases can traverse in either direction (tail to head, or head to tail) equally cheaply.

Index lookups are fine for small networks, but too costly for complex queries over larger graphs. Graph databases make relationships concrete and navigable at query time instead of relying on index indirection.
