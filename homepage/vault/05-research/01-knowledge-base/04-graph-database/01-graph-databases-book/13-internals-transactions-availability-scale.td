---
_label: "Neo4j Internals: Transactions, Availability, Scale"
_icon: icon.document
---

Part of the Neo4j internals series: [Native Graph Processing](./11-native-graph-processing.md) | [Native Graph Storage](./12-native-graph-storage.md) | **Transactions, Availability, Scale**

## Transactions

Neo4j supports full **ACID transactions**:

- Every modification runs inside a transaction.
- Write-ahead log ensures durability: Changes are logged before being applied to store files.
- Transactions can span multiple nodes, relationships, and properties.

## Recoverability

- Write-ahead log for crash recovery.
- On restart, Neo4j replays uncommitted transactions from the log.

## Availability

- Master-slave replication with automatic failover.
- Slaves serve reads immediately. Writes go to master and propagate.
- On master failure, a slave is elected as new master.

## Scale

- **Reads**: Scale horizontally by adding slaves.
- **Writes**: Scale vertically (single master).
- **Cache sharding**: Route requests to instances with relevant portions of the graph in memory.
- **Graph sharding** (splitting the graph across machines): Remains an open research problem. Cutting a graph inevitably severs relationships, degrading traversal performance.
