---
_label: "Graph Database Pros"
_icon: icon.document
---

Three key advantages over relational databases and NoSQL stores:

## Performance

With relational databases, join-heavy queries get slower as the dataset grows. With graph databases, **performance stays relatively constant** regardless of total dataset size. This is because queries are localized: Execution time is proportional to the portion of the graph traversed, not the size of the overall graph.

This makes graph databases especially compelling for connected data, where relational joins would otherwise deteriorate by orders of magnitude.

## Flexibility

The graph data model is **schema-free and naturally additive**: You can add new node types, relationships, and subgraphs without breaking existing queries or application logic. This means:

- Structure and schema emerge alongside your understanding of the domain, rather than being imposed upfront when you know least about the data.
- Fewer migrations, reducing maintenance overhead and risk.
- No need to model the domain in exhaustive detail ahead of time.

> Remark: Well, I think there's maybe an unknown trade-off.

## Agility

Graph databases align well with **incremental, iterative software delivery**. The schema-free model lets the data evolve in step with the application.

The tradeoff: No schema means no schema-oriented governance (like relational constraints). Instead, governance is applied programmatically through tests that validate the data model, queries, and business rules. This fits naturally with test-driven development practices.

> Remark: Well, I think there's maybe an unknown trade-off for this point too.
