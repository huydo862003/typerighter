---
_label: "Graph Databases by Ian Robison, Jim Webber &amp; Emil Eifrem"
_icon: icon.document
---

This is a book aims at practicality, focus on managing and querying highly connected data using schema-free graph models.

The book is authored by engineers at Neo4J.

Key takeaways:

> - Graph databases excel at **highly connected data** where relational databases and aggregate NoSQL stores struggle: Query patterns that require traversing various records via links.
> - Graph databases are more adept at querying **graph-like properties**, like all transitive friends of a person, where relational joins degrade exponentially with depth. Graph traversal stays constant (O(1) per hop with index-free adjacency).
> - The **property graph model** (nodes + relationships, both with key-value properties) is simple yet covers the vast majority of use cases.
> - Graph databases are **schema-free and additive**: New node types and relationships can be added without breaking existing queries.

The book covers the followings:

- The **Cypher query language**.
- The **property graph data model**.
- Best practices/commons pitfalls and architectural patterns when working with graph databases.
- How to design data with the graph data model?

## A Little of History

At around 1999, the founders of Neo4j were struggling with an enterprise content management system where **50% of their engineering effort** was spent "fighting" relational databases. While SQL handled discrete data well, it was extremely slow and complex for managing **connected data**, which represents most of their use cases.

Failing to find an existing solution on the web (using "Altavista", an antique search engine), the team built a native graph database from scratch. Their goal was to combine the reliability of relational databases (**ACID transactions**) with a **graph-centric model** for the 21st century (?).

Since then, graph databases are now mission-critical in logistics (routing), finance (entitlements), airlines, and healthcare.

Basically, the philosophy is to embrace "graph thinking" to empower data connectivity, which allows unlocking insights that traditional databases cannot.
