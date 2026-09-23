---
_label: "RDFS: Vocabularies for RDF"
_icon: icon.document
---

RDF vocabularies can be defined at [different levels of formality](./vocabulary.md):

- **Informal**: A web page listing terms and human-readable descriptions. Only humans can interpret them.
- **Semi-formal (RDFS)**: The vocabulary is defined as RDF triples themselves, using constructs like `rdfs:domain` and `rdfs:range`. An RDFS-aware processor can reason over these.
- **Formal ([OWL](../owl.md))**: Same as RDFS but with richer constraints (cardinality, disjointness, equivalence). An OWL-aware processor has more built-in inference rules.

RDFS (RDF Schema) is the semi-formal level. It provides constructs for defining the meaning of terms used in [RDF](./core.md) graphs, sitting between raw RDF (no semantics beyond triples) and OWL (full ontology language).

## Constructs

The meanings of these constructs are **hardcoded into RDFS-aware processors**.

Vocabulary authors use them to describe their own IRIs, and the processor applies its built-in inference rules accordingly.

### Basic Notions

Everything is a resource classified by `rdf:type`.

- A **class** is a category (subjects and objects are instances of classes).
- A **property** is a relationship (used as the predicate). A property is just an instance of the class `rdf:Property`, the same way `Bob` is an instance of `Person`:

```
<Bob> rdf:type <Person> .              (Bob is an instance of Person)
<Person> rdf:type rdfs:Class .         (Person is an instance of Class)
<foaf:knows> rdf:type rdf:Property .   (knows is an instance of Property)
<rdf:type> rdf:type rdf:Property .     (type is itself an instance of Property)
```

Similarly, `rdf:Property` is a class whose instances are properties:

```
<foaf:knows> rdf:type rdf:Property .  (knows is a Property)
```

### Constructs

| Construct                                                                    | Kind       | Triple form                | Purpose                                                  |
| ---------------------------------------------------------------------------- | ---------- | -------------------------- | -------------------------------------------------------- |
| [`rdfs:Class`](https://www.w3.org/TR/rdf12-schema/#ch_class)                 | A class    | `C rdf:type rdfs:Class`    | `C` (a resource) is an RDF class                         |
| [`rdf:Property`](https://www.w3.org/TR/rdf12-schema/#ch_property)            | A class    | `P rdf:type rdf:Property`  | `P` (a resource) is an RDF property                      |
| [`rdf:type`](https://www.w3.org/TR/rdf12-schema/#ch_type)                    | A property | `I rdf:type C`             | `I` (a resource) is an instance of `C` (a class)         |
| [`rdfs:subClassOf`](https://www.w3.org/TR/rdf12-schema/#ch_subclassof)       | A property | `C1 rdfs:subClassOf C2`    | `C1` (a class) is a subclass of `C2` (a class)           |
| [`rdfs:subPropertyOf`](https://www.w3.org/TR/rdf12-schema/#ch_subpropertyof) | A property | `P1 rdfs:subPropertyOf P2` | `P1` (a property) is a sub-property of `P2` (a property) |
| [`rdfs:domain`](https://www.w3.org/TR/rdf12-schema/#ch_domain)               | A property | `P rdfs:domain C`          | Domain of `P` (a property) is `C` (a class)              |
| [`rdfs:range`](https://www.w3.org/TR/rdf12-schema/#ch_range)                 | A property | `P rdfs:range C`           | Range of `P` (a property) is `C` (a class)               |

> The two different prefixes (`rdf:` and `rdfs:`) are a historical artefact preserved for backward compatibility.

Example:

```
<Person> <type> <Class> .
<is a friend of> <type> <Property> .
<is a friend of> <domain> <Person> .
<is a friend of> <range> <Person> .
<is a good friend of> <subPropertyOf> <is a friend of> .
```

Note that properties are themselves resources. `<is a friend of>` is typically used as a predicate, but here it appears as the subject of triples (assigning type, domain, range) and as the object of a triple (describing `<is a good friend of>`).

## Entailment

RDFS enables **[entailment](https://www.w3.org/TR/rdf12-semantics/#dfn-entail)**: Deriving new triples from existing ones through logical inference.

Example: If `foaf:knows` has `rdfs:domain foaf:Person` and `rdfs:range foaf:Person`, then from `<Bob> foaf:knows <Alice>` a system can infer that both Bob and Alice are of type `foaf:Person`.

For a formal specification of RDFS semantics, see the [RDF Semantics](https://www.w3.org/TR/rdf12-semantics/) document. For more comprehensive semantic modeling, see [OWL](../owl.md), which is itself an RDF vocabulary and can be used in combination with RDFS.

## Popular Vocabularies

Vocabularies get their value from reuse: The more a vocabulary's IRIs are reused by others, the more valuable it becomes (network effect). Prefer reusing existing IRIs over inventing new ones.

- **[FOAF](http://xmlns.com/foaf/spec/)**: One of the first worldwide RDF vocabularies, for describing people and social networks.
- **[Dublin Core](https://www.dublincore.org/specifications/dublin-core/dcmi-terms/)** (`dcterms`): General-purpose metadata (creator, publisher, title).
- **[schema.org](https://schema.org/)**: Vocabulary developed by major search providers for marking up web pages so search engines understand the content.
- **[SKOS](https://www.w3.org/TR/skos-reference/)**: W3C recommendation (since 2009) for publishing classification schemes, terminologies, and thesauri. The Library of Congress published its Subject Headings as SKOS.
