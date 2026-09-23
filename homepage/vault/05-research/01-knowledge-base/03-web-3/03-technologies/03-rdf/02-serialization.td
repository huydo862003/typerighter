---
_label: "RDF Serialization Formats"
_icon: icon.document
---

RDF is an **abstract model**, NOT a file format.

Multiple serializations exist to represent this abstract model in a textual format. Different ways of writing down the same graph lead to exactly the same triples and are thus logically equivalent.

| Family | Format    | Based on  | Strengths                                  |
| ------ | --------- | --------- | ------------------------------------------ |
| Turtle | N-Triples | -         | One triple per line, good for streaming    |
| Turtle | N-Quads   | N-Triples | N-Triples with named graph support         |
| Turtle | Turtle    | N-Triples | Most human-readable, good for hand-writing |
| Turtle | TriG      | Turtle    | Turtle with named graph support            |
| -      | JSON-LD   | JSON      | Web-friendly, used by schema.org           |
| -      | RDFa      | HTML      | Embeds RDF in HTML pages                   |
| -      | RDF/XML   | XML       | Original format, verbose, mostly legacy    |

The use cases for each:

- The Turtle family covers all the basic concepts for serializing RDF:
  - **N-Triples**: Streaming, bulk processing, and line-oriented tools (e.g. `grep`, `wc`).
  - **N-Quads**: Same as N-Triples, but when you need named graphs.
  - **Turtle**: Reading, writing, and editing RDF by hand.
  - **TriG**: Same as Turtle, but when you need named graphs.
- **JSON-LD**: Web APIs and schema.org markup.
- **RDFa**: Embedding RDF directly in HTML pages for search engines.
- **RDF/XML**: Legacy systems that expect XML.

## Turtle Family

### N-Triples

N-Triples is the simplest format. The rules are:

- Each line represents exactly one triple, terminated by a period.
- Every IRI must be written in full inside `<>`, with no prefixes or shortcuts allowed.
- Each line is fully self-contained, so no parser state is needed between lines.

This design enables line-oriented processing: files can be `grep`-ed for an IRI, `wc -l`-ed to count triples, or `cat`-ed together and the result remains valid N-Triples.

```
<http://example.org/bob#me> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://xmlns.com/foaf/0.1/Person> .
<http://example.org/bob#me> <http://xmlns.com/foaf/0.1/knows> <http://example.org/alice#me> .
<http://example.org/bob#me> <http://schema.org/birthDate> "1990-07-04"^^<http://www.w3.org/2001/XMLSchema#date> .
<http://www.wikidata.org/entity/Q12418> <http://purl.org/dc/terms/title> "Mona Lisa" .
<http://www.wikidata.org/entity/Q12418> <http://purl.org/dc/terms/title> "La Joconde"@fr .
```

Literals are written as follows:

- Datatypes are appended via `^^`, e.g. `"1990-07-04"^^<...xsd#date>`.
- String literals (`xsd:string`) may omit the datatype: `"Mona Lisa"` is equivalent to `"Mona Lisa"^^xsd:string`.
- Language-tagged strings use `@lang` after the closing quote, e.g. `"La Joconde"@fr`. Their implicit datatype is `rdf:langString` and is never written explicitly.

### N-Quads

N-Quads extends N-Triples with support for [RDF datasets](./data-model.md#rdf-dataset). An optional fourth element on each line holds the graph IRI. Lines without a fourth element belong to the default graph and are otherwise identical to N-Triples.

```
<http://example.org/bob#me> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://xmlns.com/foaf/0.1/Person> <http://example.org/bob> .
<http://www.wikidata.org/entity/Q12418> <http://purl.org/dc/terms/title> "Mona Lisa" <https://www.wikidata.org/wiki/Special:EntityData/Q12418> .
<http://example.org/bob> <http://purl.org/dc/terms/publisher> <http://example.org> .
```

Like N-Triples, N-Quads is used for exchanging large RDF datasets with line-oriented tools.

### Turtle

Turtle is a superset of N-Triples: every valid N-Triples document is valid Turtle. It adds readability shortcuts on top:

- `PREFIX`/`BASE`: declare namespace prefixes so short forms like `foaf:knows` expand to full IRIs.
- `;`: repeat the same subject with a new predicate-object pair.
- `,`: repeat the same subject and predicate with a new object.
- `a`: shorthand for `rdf:type`.
- `#`: line comment.
- `_:label`: a blank node with a local reusable label.
- `[ ... ]`: an anonymous blank node inline (syntactic sugar for a fresh `_:` label plus extra triples).

```turtle
PREFIX foaf: <http://xmlns.com/foaf/0.1/>
PREFIX schema: <http://schema.org/>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
PREFIX dcterms: <http://purl.org/dc/terms/>
PREFIX wd: <http://www.wikidata.org/entity/>

<http://example.org/bob#me>
    a foaf:Person ;                          # rdf:type shorthand
    foaf:knows <http://example.org/alice#me> ;
    schema:birthDate "1990-07-04"^^xsd:date ;
    foaf:topic_interest wd:Q12418 .

wd:Q12418
    dcterms:title "Mona Lisa", "La Joconde"@fr ;   # comma: same subject+predicate
    dcterms:creator <http://dbpedia.org/resource/Leonardo_da_Vinci> .
```

Language and direction (RDF 1.2), see [RDF Data Model: Literals](./data-model.md#literals):

- `"value"@lang`: a language-tagged string.
- `"value"@lang--rtl` (or `--ltr`): a directional language-tagged string.

```turtle
wd:Q12418
    dcterms:title "Mona Lisa"@en ;
    dcterms:title "La Joconde"@fr ;
    dcterms:title "موناليزا"@ar--rtl .
```

Turtle 1.2 provides three syntaxes for reifying triples, see [RDF Data Model: Triple Terms](./data-model.md#triple-terms):

- `{| P O ; ... |}`: an **unnamed inline annotation** attached directly to the preceding triple.
- `<< S P O >>` in subject position: an **unnamed reifier** referencing a triple term.
- `~ <iri>` or `~ _:label` after a triple: a **named reifier** that can be referenced elsewhere.

```turtle
# Unnamed inline annotation
<bob#me> foaf:topic_interest wd:Q12418
    {| dcterms:date "1998-10-04"^^xsd:date |} .

# Named reifier
<bob#me> foaf:topic_interest wd:Q12418
    ~ <http://example.org/bob#interest-1> .

<http://example.org/bob#interest-1>
    a prov:Influence ;
    dcterms:date "1998-10-04"^^xsd:date .
```

### TriG

TriG extends Turtle with support for [RDF datasets](./data-model.md#rdf-dataset). Triples are wrapped in `GRAPH <iri> { ... }` to assign them to a named graph. The `GRAPH` keyword is optional. Triples outside any block form the default graph. In RDF 1.2, every Turtle document is also valid TriG (the names are kept for historical reasons).

```trig
PREFIX foaf: <http://xmlns.com/foaf/0.1/>
PREFIX dcterms: <http://purl.org/dc/terms/>
PREFIX wd: <http://www.wikidata.org/entity/>

GRAPH <http://example.org/bob> {
    <http://example.org/bob#me>
        a foaf:Person ;
        foaf:topic_interest wd:Q12418 .
}

GRAPH <https://www.wikidata.org/wiki/Special:EntityData/Q12418> {
    wd:Q12418
        dcterms:title "Mona Lisa" ;
        dcterms:creator <http://dbpedia.org/resource/Leonardo_da_Vinci> .
}

# Default graph
<http://example.org/bob>
    dcterms:publisher <http://example.org> ;
    dcterms:rights <http://creativecommons.org/licenses/by/3.0/> .
```

## JSON-LD

JSON-LD provides a JSON syntax for RDF graphs and datasets. It is designed for minimal friction when converting existing JSON to RDF and for use in Web APIs and schema.org markup.

A `@context` object maps friendly JSON keys to full IRIs and specifies datatypes, so the rest of the document reads like regular JSON. The `@context` can be inlined or referenced as an external URL.

Each JSON object represents an RDF resource, with the following special keys:

- `@id`: the IRI of the resource.
- `@type`: maps to `rdf:type`.
- Nested objects express related resources.
- `@graph`: serializes a full RDF dataset.

```json
{
  "@context": {
    "foaf": "http://xmlns.com/foaf/0.1/",
    "Person": "foaf:Person",
    "knows": { "@id": "foaf:knows", "@type": "@id" },
    "birthdate": {
      "@id": "http://schema.org/birthDate",
      "@type": "http://www.w3.org/2001/XMLSchema#date"
    },
    "dcterms": "http://purl.org/dc/terms/",
    "title": "dcterms:title",
    "creator": { "@id": "dcterms:creator", "@type": "@id" }
  },
  "@id": "http://example.org/bob#me",
  "@type": "Person",
  "birthdate": "1990-07-04",
  "knows": "http://example.org/alice#me",
  "interest": {
    "@id": "http://www.wikidata.org/entity/Q12418",
    "title": "Mona Lisa",
    "creator": "http://dbpedia.org/resource/Leonardo_da_Vinci"
  }
}
```

The context controls how values are interpreted:

- `"@type": "@id"`: the value is treated as an IRI, not a plain string.
- `"@type": "<datatype IRI>"`: a datatype is attached to the literal value.
- `"@reverse": "<property IRI>"`: the triple direction is inverted, so `"key": "x"` becomes `<x> <property> <current resource>`.

## RDFa

RDFa embeds RDF triples as attributes inside HTML (or XML) so search engines can extract structured data while crawling. The RDF lives directly in the page the browser already renders, with no separate file needed.

RDFa introduces the following HTML attributes:

- `prefix`: declares namespace shorthands, similar to Turtle `PREFIX`.
- `resource`: specifies the IRI of the subject (or object when used without `property`).
- `typeof`: asserts `rdf:type` for the enclosing element's resource.
- `property`: names the predicate IRI; the object is taken from `href`, `resource`, or element content.
- `datatype`: provides an explicit datatype for a literal; HTML semantics can supply it implicitly (e.g. the `<time>` element implies `xsd:date`).

```html
<body
  prefix="foaf: http://xmlns.com/foaf/0.1/ schema: http://schema.org/ dcterms: http://purl.org/dc/terms/"
>
  <div resource="http://example.org/bob#me" typeof="foaf:Person">
    <p>
      Bob knows
      <a property="foaf:knows" href="http://example.org/alice#me">Alice</a> and
      was born on
      <time property="schema:birthDate" datatype="xsd:date">1990-07-04</time>.
    </p>
    <p>
      Bob is interested in
      <span
        property="foaf:topic_interest"
        resource="http://www.wikidata.org/entity/Q12418"
        >the Mona Lisa</span
      >.
    </p>
  </div>
  <div resource="http://www.wikidata.org/entity/Q12418">
    <p>
      The <span property="dcterms:title">Mona Lisa</span> was painted by
      <a
        property="dcterms:creator"
        href="http://dbpedia.org/resource/Leonardo_da_Vinci"
        >Leonardo da Vinci</a
      >.
    </p>
  </div>
</body>
```

## RDF/XML

RDF/XML is the original RDF serialization from the late 1990s, written in XML. It is verbose and today mostly appears in legacy systems.

RDF/XML is structured as follows:

- All triples are nested inside an `<rdf:RDF>` root element that carries `xmlns:` namespace declarations.
- Each `<rdf:Description rdf:about="...">` block groups triples that share a subject.
- Property subelements name the predicate: IRI objects use `rdf:resource="..."`, literal objects are placed as element content.
- The `rdf:datatype` attribute attaches a datatype; omitting it defaults to `xsd:string`.
- Namespace prefixes apply only to XML element and attribute names, so full IRIs are required in `rdf:about`, `rdf:resource`, and `rdf:datatype` attribute values.

```xml
<?xml version="1.0" encoding="utf-8"?>
<rdf:RDF xmlns:dcterms="http://purl.org/dc/terms/"
         xmlns:foaf="http://xmlns.com/foaf/0.1/"
         xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:schema="http://schema.org/">
  <rdf:Description rdf:about="http://example.org/bob#me">
    <rdf:type rdf:resource="http://xmlns.com/foaf/0.1/Person"/>
    <schema:birthDate rdf:datatype="http://www.w3.org/2001/XMLSchema#date">1990-07-04</schema:birthDate>
    <foaf:knows rdf:resource="http://example.org/alice#me"/>
    <foaf:topic_interest rdf:resource="http://www.wikidata.org/entity/Q12418"/>
  </rdf:Description>
  <rdf:Description rdf:about="http://www.wikidata.org/entity/Q12418">
    <dcterms:title>Mona Lisa</dcterms:title>
    <dcterms:creator rdf:resource="http://dbpedia.org/resource/Leonardo_da_Vinci"/>
  </rdf:Description>
</rdf:RDF>
```
