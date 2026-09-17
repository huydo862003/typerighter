---
_label: "The Concept of Graph"
_icon: icon.document
---

A **graph** is a collection of **nodes** (entities) and **relationships** (edges) connecting them. This schema-free structure can model virtually anything: Social networks, supply chains, road systems, etc.

For example, a social network where users follow each other:

```mermaid
graph LR
    A -->|FOLLOWS| B
    B -->|FOLLOWS| A
    C -->|FOLLOWS| A
    C -->|FOLLOWS| B
```

**Relationships** carry **semantic** meaning - we can immediately see who follows whom and whether it's mutual.

Graphs can also mix entity types. For instance, adding posts to a user node as a linked list:

```mermaid
graph TD
    A -->|LATEST| Post_3
    Post_3 -->|PREV| Post_2
    Post_2 -->|PREV| Post_1
```

`LATEST` points to the most recent post & `PREV` chains form a timeline.

Different concerns (social connections + content) coexist naturally in the same graph.
