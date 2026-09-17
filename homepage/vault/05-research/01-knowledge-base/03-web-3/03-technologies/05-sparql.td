---
_label: "SPARQL: Query Language for RDF"
_icon: icon.document
---

## Resources

### Specs

- [W3C SPARQL 1.1 Query Language](https://www.w3.org/TR/sparql11-query/) - The formal spec.
- [W3C SPARQL 1.1 Update](https://www.w3.org/TR/sparql11-update/) - The write side: INSERT, DELETE, LOAD, CLEAR.

### In Semantic Web

- SPARQL is to RDF what Cypher is to property graphs. For the comparison, see [The Graph Space](../../graph-database/graph-databases-book/3-the-graph-space.md#property-graph-vs-rdf).
- SPARQL queries pattern-match on triples, while Cypher pattern-matches on nodes and relationships. Structurally similar, but SPARQL has built-in support for reasoning (if the triple store supports inference).

### Live Playgrounds

- [Wikidata Query Service](https://query.wikidata.org/) - Run SPARQL queries against Wikidata's RDF graph in the browser.
- [DBpedia SPARQL Endpoint](https://dbpedia.org/sparql) - Query Wikipedia's structured data as RDF.

### Tutorials

- [Apache Jena SPARQL Tutorial](https://jena.apache.org/tutorials/sparql.html) - Fast, practical course. Covers SELECT, FILTER, OPTIONAL, UNION with runnable examples.
- [SPARQL.dev](https://sparql.dev/) - Cheat sheet / quick reference. Good for looking up syntax while writing queries.
- [Wikidata SPARQL Tutorial](https://www.wikidata.org/wiki/Wikidata:SPARQL_tutorial) - Learn by querying real data (Wikidata). The most motivating way to learn.
