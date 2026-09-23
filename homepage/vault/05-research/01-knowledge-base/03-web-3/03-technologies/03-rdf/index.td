---
_label: "RDF: Meaning as Triples"
_icon: icon.document
---

RDF (Resource Description Framework) is a framework for expressing machine-readable information about [resources](./uri/rfc-3986.md) (can be anything as defined like the URI spec). RDF enables automated processing and exchange of information across applications without losing meaning.

In the Semantic Web stack, XML provides structure but its tags have no machine-interpretable meaning: `<creator>` is just a string to an XML parser. RDF adds a **universal data model** (triples with IRIs) on top, so every relationship and resource has a globally unique identifier. This makes data interoperable and mergeable across sources. In that sense, **RDF is metadata about resources, described in a way machines can reason about**.

> Remark: Think of it like this - With XML, data can be displayed to human & human can write programs to process the XML documents. Interpretation of XML is still under human's discretion. With RDF, there's a standard way to interpret the XML documents.

RDF is another graph model like property graphs. For how RDF compares to property graphs, see [The Graph Space](../../graph-database/graph-databases-book/3-the-graph-space.md#property-graph-vs-rdf).

RDF is recommended by W3C.

- Enriching web pages with machine-readable metadata (schema.org).
- Linking datasets across sources (e.g. paintings to artist information in Wikidata).
- Standards-compliant database data exchange.
- Cross-dataset querying via SPARQL.

Subpages:

- [Data Model](./rdf/data-model.md): Triples, RDF terms, named graphs, semantics.
- [Serialization Formats](./rdf/serialization.md): Turtle, N-Triples, TriG, N-Quads, JSON-LD, RDFa, RDF/XML.
- [RDFS](./rdf/rdfs.md): Vocabularies and class hierarchies for RDF.

See also [OWL](./owl.md) for full ontology language and [SPARQL](./sparql.md) for querying.

## Semantics of RDF Graphs

Motivation:

- Syntax alone is not enough to reliably merge data across sources.
- Two datasets can use the same IRI to mean different things, or different IRIs to mean the same thing.

RDF addresses this with a **model-theoretic semantics** ([RDF Semantics](https://www.w3.org/TR/rdf12-semantics/)):

- Every IRI denotes some thing in the world (its **referent**).
- A triple `S P O` is true when the referent of `S` and the referent of `O` are actually related by the relation denoted by `P`.
- A graph is true when all its triples are true.

That is all plain RDF semantics defines, without inference rules & class hierarchies, just truth conditions.

This foundation lets software check consistency and draw conclusions. A **reasoner** can derive triples that are logically implied by existing ones (**entailment**), or detect contradictions. For example, `ex:bob ex:age "forty"^^xsd:integer .` is inconsistent because `"forty"` does not satisfy `xsd:integer`.

RDFS and OWL build inference rules on top of this, but are not part of RDF's core semantics. Each introduces **privileged IRIs** whose meaning is hardcoded into processors:

- **RDFS**: adds rules via `rdfs:domain`, `rdfs:subClassOf`, etc. For example, `foaf:knows rdfs:domain foaf:Person .` lets a reasoner derive `ex:bob rdf:type foaf:Person .` from `ex:bob foaf:knows ex:alice .`
- **OWL**: adds richer rules (cardinality, disjointness, equivalence) on top of RDFS.

Axioms are not written in RDF. They are formal rules in the spec, implemented in processor code. What you write in RDF are **vocabulary triples** that act as inputs to those rules:

- The axiom "if `P rdfs:domain C` and `X P Y`, infer `X rdf:type C`" lives in the spec.
- The triple `foaf:knows rdfs:domain foaf:Person .` is a normal RDF triple that triggers it.

RDF Schema also draws no hard line between classes and instances: the same IRI can serve both roles at once.

## Linked Data

RDF data is widely published on the Web as [Linked Data](https://www.w3.org/wiki/LinkedData): datasets connected by shared IRIs and queryable via SPARQL.

Notable examples include Wikidata, DBpedia, WordNet, Europeana, and VIAF.

Different publishers often mint different IRIs for the same entity. `owl:sameAs` lets anyone assert that two IRIs refer to the same resource, so processors can merge data across datasets:

```turtle
<http://dbpedia.org/resource/Leonardo_da_Vinci>
    owl:sameAs <http://viaf.org/viaf/24604287/> .
```

## Resources

### Specs

- [W3C RDF 1.2 Primer](https://www.w3.org/TR/rdf12-primer/) is a gentle intro with examples (Warning: It's an informative Group Note Drafts and no guarantee is made regarding its claims).
- [W3C RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/) is the formal abstract data model. This is a normative specs, as oppose to the above specs.

### Others

- [Baeldung: Introduction to RDF](https://www.baeldung.com/cs/rdf-intro) is concise and covers triples, serialization formats (Turtle, JSON-LD, RDF/XML), and SPARQL basics.
- [Enterprise Knowledge: The RDF Guide](https://enterprise-knowledge.com/the-resource-description-framework-rdf/) is practical and application-focused, good on why RDF matters for knowledge graphs.
