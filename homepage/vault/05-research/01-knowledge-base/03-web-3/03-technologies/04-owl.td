---
_label: "OWL: Ontologies and Reasoning"
_icon: icon.document
---

## Resources

### Specs

- [W3C OWL 2 Overview](https://www.w3.org/TR/owl2-overview/) - Maps the OWL 2 document suite and explains the three profiles (EL, QL, RL) which trade expressiveness for computational tractability.
- [W3C OWL 2 Primer](https://www.w3.org/TR/owl2-primer/) - Gentle intro with examples. Covers classes, properties, restrictions, and reasoning.

### Books

- _Semantic Web for the Working Ontologist_ (Allemang & Hendler) - The practical guide. Covers RDF, RDFS, OWL, and SPARQL with a focus on building real ontologies.
- _A Semantic Web Primer_ (Antoniou & van Harmelen) - More academic. Good for understanding the theoretical foundations.

### Others

- [W3C OWL Guide (OWL 1)](https://www.w3.org/TR/owl-guide/) - Older but still the most comprehensive walkthrough with wine ontology examples.
- [Manchester Pizza Ontology Tutorial](http://owl.cs.manchester.ac.uk/owltutorialjuly2014/) - Hands-on: Build an ontology from scratch using Protege. The de facto "hello world" for OWL.

### In Semantic Web

- OWL ontologies enable the **automatic inference** discussed in [The Graph Space](../../graph-database/graph-databases-book/3-the-graph-space.md). Property graphs have no equivalent, reasoning must be custom code.

## RDFS vs OWL

RDFS (RDF Schema) and OWL are both for defining vocabularies, but at different levels:

- **RDFS**: Lightweight. Classes, subclasses, domain/range of properties. Enough for simple taxonomies.
- **OWL**: Full reasoning. Cardinality constraints, transitive/symmetric properties, equivalence, disjointness. Needed when you want machines to infer new facts.

Most real-world ontologies use a mix: RDFS for the basics, OWL where inference is needed.
