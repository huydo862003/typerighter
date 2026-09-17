---
_label: "RDF Vocabulary"
_icon: icon.document
---

Based on:

- [W3C RDF 1.2 Primer](https://www.w3.org/TR/rdf12-primer/): An informative Group Note Drafts and no guarantee is made regarding its claims.
- [W3C RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/): A normative specs, as oppose to the above specs.

## Context

For context, see [./data-model.md].

This section repeats most of the materials covered there.

- RDF is simply **a set of triples**, with **IRIs used as unique identifiers**.
- On its own, it is just groups of IRI triples with no interpretation given.
- **[RDF vocabularies](https://www.w3.org/TR/rdf12-concepts/#dfn-rdf-vocabulary)** give interpretation over the IRIs.

There are 3 major ways vocabularies may be defined, differing in the inference power they grant to processors.

At every level, the core idea is the same:

1. Someone picks IRIs.
2. They publish what they mean.
3. Others reuse them.

### Informal Vocabulary Definition

Approach: A web page listing terms and their human-readable descriptions.

For example, [Dublin Core](https://www.dublincore.org/specifications/dublin-core/dcmi-terms/) publishes a page describing terms like `dcterms:title` and `dcterms:creator`.

This is only for humans: an RDF processor cannot read or reason about informal definitions.

How it would be used:

1. The developer reads the vocabulary page to understand what each IRI means.
2. The developer writes RDF data using those IRIs.
3. The processor stores and queries the triples.

Correctness depends entirely on the developer using the IRIs consistently with their documented meaning.

## Semi-formal Vocabulary Definition ([RDFS](./rdfs.md))

Approach: The vocabulary is defined _as RDF triples themselves_.

For example, [FOAF](http://xmlns.com/foaf/spec/) publishes an RDF file containing triples like:

```
<foaf:knows> <rdf:type> <rdf:Property> .
<foaf:knows> <rdfs:domain> <foaf:Person> .
```

This works because the RDF spec defines a small set of special IRIs (`rdf:type`, `rdfs:domain`, `rdfs:range`, etc.) whose meaning is **hardcoded into RDFS-aware processors**.

Vocabulary authors use these special IRIs to describe their own IRIs, and the processor applies its built-in inference rules to them.

> Remark: The meaning of the core IRIs (`rdfs:domain`, etc.) lives in the processor's code, not in more RDF. The vocabulary triples just provide input for those built-in rules. This is like built-ins in a high-level functional programming language: you compose `rdfs:domain`, `rdfs:subClassOf`, etc to express complex vocabulary definitions without touching the underlying implementation, just like using `map` or `filter` without caring how they work internally.

How it would be used:

1. The developer writes RDF data using the vocabulary's IRIs (same as informal).
2. The developer also feeds the vocabulary's RDF file into the processor.
3. The processor reasons over the vocabulary triples alongside the data triples - e.g. inferring that `<Bob>` is a `foaf:Person` because he appears as the subject of `foaf:knows`.

The processor enforces some of the vocabulary's meaning automatically, rather than relying purely on the developer.

## Formal Vocabulary Definition ([OWL](../owl.md))

Approach: Same as RDFS, the vocabulary is defined as RDF triples, but with richer constraints: cardinality, disjointness, equivalence, etc.

An **OWL-aware processor** has **more built-in inference rules** than an **RDFS-aware one**.

How it would be used:

1. The developer writes RDF data using the vocabulary's IRIs (same as informal).
2. The developer also feeds the vocabulary's OWL file into the processor (same as semi-formal).
3. The processor reasons over the vocabulary triples with a richer set of inference rules - e.g. inferring that two classes are disjoint, or that a property can have at most one value.

The processor enforces more of the vocabulary's meaning automatically than RDFS can.
