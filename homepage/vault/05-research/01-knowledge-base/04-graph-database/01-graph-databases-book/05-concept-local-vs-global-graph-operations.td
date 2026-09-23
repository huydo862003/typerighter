---
_label: "Local vs Global Graph Operations"
_icon: icon.document
---

There are two fundamentally different ways to work with graph data:

## Local Operations

Start at a specific node and traverse outward. Only visit a small portion of the graph.

Examples:

- "Who does Alice follow?"
- "Find friends-of-friends of Bob"
- "What's the shortest path from X to Y?"

These are fast because they touch few nodes. This is what **graph databases** are built for: Real-time, low-latency responses.

## Global Operations

Operate on the **entire graph**. Must touch most or all nodes, often in multiple passes.

Examples:

- **PageRank**: Score every node by importance based on incoming links. Requires iterating over all nodes multiple times until scores converge.
- **Community detection**: Find clusters/groups by scanning everyone's connections.
- **Connected components**: Identify all disconnected subgraphs.
- **Betweenness centrality**: Find which nodes act as bridges between different parts of the graph.

These are slow by nature: You can't answer "who is the most important person in the network" without looking at everyone. This is what **graph compute engines** are built for: Offline batch processing.

## Significance

A graph database trying to run a global algorithm would block all other queries while scanning the entire dataset. A graph compute engine can't serve real-time app requests because it's designed for batch throughput, not low latency. Different tools for different jobs.
