---
_label: "XML: Structure Without Meaning"
_icon: icon.document
---

> Note: Only covering enough XML concepts to understand RDF and OWL. Not a deep dive into XML itself.

XML stands for eXtensible Markup Language, designed to store and transport data & self-descriptive.

W3C recommends XML.

> Remark: I find XML to be verbose and user-unfriendly though. But maybe it's one of the first universal and portable data transport format.

- [Core](./xml/core.md): Syntax, rules, namespaces, properties.
- [DOM: HTML DOM for XML](./xml/dom.md)
- [XPath: CSS Selectors for XML](./xml/xpath.md)
- [XSLT: Template Engine for XML](./xml/xslt.md)
- [XQuery: SQL for XML](./xml/xquery.md)
- [XPointer: Fragment Identifiers for XML](./xml/xpointer.md)
- [XLink: Hyperlinks for XML](./xml/xlink.md)

## Resources

### Specs

- [W3C XML 1.0 Spec](https://www.w3.org/TR/xml/) - The definitive spec. Verbose but authoritative.
- [W3C XML Namespaces](https://www.w3.org/TR/xml-names/) - How XML avoids naming collisions using URIs. Critical for understanding how RDF serializes to XML.

### Tutorials

- [W3Schools XML Tutorial](https://www.w3schools.com/xml/) - Practical, example-driven intro. Good for syntax basics.
- [MDN: XML Introduction](https://developer.mozilla.org/en-US/docs/Web/XML/XML_introduction) - Brief, developer-focused overview.

### In Semantic Web

- XML provides structure but no meaning: `<name>` could mean anything without external agreement. This is exactly the gap that RDF fills by adding URIs to every concept. See [RDF: Meaning as Triples](../rdf.md).
