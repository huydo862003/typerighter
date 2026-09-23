---
_label: "Graph Technologies: Semantic Web vs Property Graph"
_icon: icon.document
---

For a detailed comparison of RDF vs property graphs (structural differences, performance, reasoning), see [The Graph Space: Property Graph vs RDF](./graph-database/graph-databases-book/3-the-graph-space.md#property-graph-vs-rdf).

## Key Differences

Both represent data as nodes connected by named edges and can model the same data. The real differences:

- **Property graphs** optimize for **developer ergonomics and traversal performance**. Properties live directly on nodes and edges. Schema-free and ad-hoc.
- **Semantic web (RDF)** optimizes for **global interoperability and machine reasoning**. Everything is a triple. Formal ontologies (OWL) enable automatic inference. Standardized by W3C.

The choice is about ecosystem and purpose, not structural capability.
